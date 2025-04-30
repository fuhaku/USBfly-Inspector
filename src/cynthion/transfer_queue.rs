//! Transfer queue implementation for Cynthion devices
//! Based on Packetry's implementation using nusb

use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::Duration;
use std::future::Future;
use std::task::{Context, Poll};
use std::pin::Pin;

use anyhow::Result;
use log::{debug, error, info, warn};
use nusb::{
    transfer::{
        TransferError,
        RequestBuffer,
        Completion,
    },
    Interface,
};

// Constants
const MAX_DONE_TRANSFERS: usize = 16;  // Maximum number of completed transfers to keep in the done queue
const TIMEOUT: Duration = Duration::from_millis(1000);

// For nusb, transfers are handled through TransferFuture and Completion
// When polled, the future resolves to a Completion<Vec<u8>>
type BulkTransfer = Completion<Vec<u8>>;

/// A queue of USB bulk transfers to a device.
// Can't derive Clone because Receiver doesn't implement Clone
pub struct TransferQueue {
    interface: Interface,
    data_tx: mpsc::Sender<Vec<u8>>,
    pub receiver: Option<mpsc::Receiver<Vec<u8>>>,  // Make Option type since Receiver doesn't implement Clone
    endpoint: u8,
    read_size: usize,
    #[allow(dead_code)]
    active_transfers: VecDeque<(BulkTransfer, Vec<u8>)>,
    #[allow(dead_code)]
    done_transfers: VecDeque<(BulkTransfer, Vec<u8>)>,
    transfer_id: usize,
}

// Manual implementation of Clone for TransferQueue
impl Clone for TransferQueue {
    fn clone(&self) -> Self {
        // Create a new transfer queue with the same properties but None for receiver
        TransferQueue {
            interface: self.interface.clone(),
            data_tx: self.data_tx.clone(),
            receiver: None,  // Can't clone the receiver
            endpoint: self.endpoint,
            read_size: self.read_size,
            active_transfers: self.active_transfers.clone(),
            done_transfers: self.done_transfers.clone(),
            transfer_id: self.transfer_id,
        }
    }
}

// Manual implementation of Debug since BulkTransfer doesn't implement Debug
impl std::fmt::Debug for TransferQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransferQueue")
            .field("endpoint", &self.endpoint)
            .field("read_size", &self.read_size)
            .field("active_transfers_count", &self.active_transfers.len())
            .field("done_transfers_count", &self.done_transfers.len())
            .field("transfer_id", &self.transfer_id)
            .finish()
    }
}

impl TransferQueue {
    /// Create a new transfer queue for the specified device interface
    pub fn new(
        interface: &Interface,
        data_tx: mpsc::Sender<Vec<u8>>,
        endpoint: u8,
        num_transfers: usize,
        read_size: usize,
    ) -> TransferQueue {
        debug!("Creating new transfer queue for endpoint 0x{:02X}", endpoint);
        
        // Create the queue without initializing the receiver yet
        // The caller will provide the receiver part of the channel
        let mut queue = TransferQueue {
            interface: interface.clone(),
            data_tx: data_tx.clone(),  // Use the provided transmitter
            receiver: None,  // Caller will set this properly
            endpoint,
            read_size,
            active_transfers: VecDeque::with_capacity(num_transfers),
            done_transfers: VecDeque::with_capacity(MAX_DONE_TRANSFERS),
            transfer_id: 0,
        };
        
        // Initialize the transfer queue by submitting initial transfers
        // with proper error handling
        match queue.initialize_transfers(num_transfers) {
            Ok(_) => {
                debug!("Successfully initialized {} transfers for endpoint 0x{:02X}", 
                       num_transfers, endpoint);
            },
            Err(e) => {
                error!("Error initializing transfers: {}", e);
                // Continue anyway, we'll try to make the best of what we have
            }
        }
        
        queue
    }
    
    /// Initialize the transfer queue with a set of transfers
    fn initialize_transfers(&mut self, num_transfers: usize) -> Result<()> {
        let mut success_count = 0;
        let mut last_error = None;
        
        for i in 0..num_transfers {
            match self.submit_transfer() {
                Ok(_) => {
                    // Transfer submitted successfully
                    success_count += 1;
                    debug!("Submitted transfer {}/{}", i+1, num_transfers);
                }
                Err(e) => {
                    error!("Error submitting initial transfer {}/{}: {}", i+1, num_transfers, e);
                    last_error = Some(e);
                    // Continue trying the remaining transfers
                }
            }
        }
        
        // If we didn't get any successful transfers, return the last error
        if success_count == 0 && last_error.is_some() {
            Err(anyhow::anyhow!("Failed to submit any transfers: {}", last_error.unwrap()))
        } else if success_count < num_transfers {
            // Partial success
            info!("Partially initialized transfer queue: {}/{} transfers submitted",
                 success_count, num_transfers);
            Ok(())
        } else {
            // Full success
            Ok(())
        }
    }
    
    /// Submit a new bulk transfer request with a timeout
    fn submit_transfer(&mut self) -> Result<()> {
        // Create a new buffer for the transfer
        let buffer = vec![0u8; self.read_size];
        
        // Create a new RequestBuffer of the appropriate size
        // The RequestBuffer API doesn't have from_vec, but it has "reuse" for existing buffers
        let request_buffer = RequestBuffer::reuse(buffer.clone(), self.read_size);
        
        // In nusb, we create a bulk IN transfer (for receiving data)
        // The endpoint address already includes the direction bit (0x80 for IN)
        let future = self.interface.bulk_in(self.endpoint, request_buffer);
        
        // Poll the future until we get a result - based on Packetry's approach
        // Create a dummy waker and context to poll the future
        let waker = futures::task::noop_waker();
        let mut context = Context::from_waker(&waker);
        
        // We need to pin the future to poll it
        let mut pinned = Box::pin(future);
        
        // Add a timeout to prevent hanging - try up to 3 attempts with a short delay
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(100); // 100ms timeout
        let max_attempts = 3;
        
        let mut completion = None;
        let mut attempts = 0;
        
        while attempts < max_attempts {
            attempts += 1;
            
            match Pin::new(&mut pinned).poll(&mut context) {
                Poll::Ready(comp) => {
                    completion = Some(comp);
                    break;
                },
                Poll::Pending => {
                    // Check if we've exceeded the timeout
                    if start_time.elapsed() > timeout {
                        // If we've tried too many times, give up
                        if attempts >= max_attempts {
                            return Err(anyhow::anyhow!("Transfer timed out after {} attempts", attempts));
                        }
                        
                        // Otherwise, sleep briefly and retry
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            }
        }
        
        // If we didn't get a completion, return an error
        let completion = match completion {
            Some(comp) => comp,
            None => return Err(anyhow::anyhow!("Failed to complete transfer after {} attempts", attempts)),
        };
        
        // Check if there was an error during transfer
        if let Err(e) = &completion.status {
            // All nusb error types - simplify for now
            return Err(anyhow::anyhow!("Transfer error: {} (device may be disconnected)", e));
        }
        
        // Add to active transfers queue
        self.active_transfers.push_back((completion, buffer));
        self.transfer_id += 1;
        
        Ok(())
    }
    
    /// Process completed transfers and resubmit them
    pub fn process_completed_transfers(&mut self) -> Result<()> {
        let mut processed_count = 0;
        
        // Check all active transfers for completion
        while let Some((transfer, buffer)) = self.active_transfers.pop_front() {
            // In nusb, we need to check the completion status in a different way
            // The Completion struct has a status field that contains the Result
            if transfer.status.is_ok() {
                // Successfully completed - extract the data from the buffer
                let data = &transfer.data;
                let actual = data.len();
                
                if actual > 0 {
                    debug!("Transfer complete: {} bytes", actual);
                    
                    // Clone the data and send it to the processor
                    if let Err(e) = self.data_tx.send(data.clone()) {
                        error!("Failed to send transfer data: {}", e);
                    }
                }
                
                // Save this transfer for reuse
                self.done_transfers.push_back((transfer, buffer));
                processed_count += 1;
                
                // Trim the done queue if it gets too large
                if self.done_transfers.len() > MAX_DONE_TRANSFERS {
                    self.done_transfers.pop_front();
                }
            } else if let Err(e) = &transfer.status {
                // Handle error based on the specific TransferError type
                match e {
                    TransferError::Cancelled => {
                        warn!("Transfer was cancelled");
                        // Save for reuse
                        self.done_transfers.push_back((transfer, buffer));
                    },
                    
                    // Transfer failed with other error
                    _ => {
                        error!("Transfer error: {}", e);
                        // Save for reuse anyway
                        self.done_transfers.push_back((transfer, buffer));
                    }
                }
            } else {
                // Still in progress or other status
                self.active_transfers.push_back((transfer, buffer));
                break;  // No need to check further transfers
            }
        }
        
        // If we processed any transfers, submit new ones to keep the queue full
        if processed_count > 0 {
            // In nusb, we can't reuse transfers from the done queue the same way
            // Instead, we'll create new transfers to replace the completed ones
            for _ in 0..processed_count {
                // Submit a new transfer to keep the queue full
                if let Err(e) = self.submit_transfer() {
                    error!("Failed to create new transfer: {}", e);
                }
            }
            
            // Clear the done queue after processing
            self.done_transfers.clear();
        }
        
        Ok(())
    }
    
    /// Clean up resources on shutdown - for nusb we just need to clear the queues
    pub fn shutdown(&mut self) {
        info!("Shutting down transfer queue");
        
        // With nusb Completion, we don't need to explicitly cancel transfers
        // Just clear out both queues
        self.active_transfers.clear();
        self.done_transfers.clear();
    }
    
    /// Get a reference to the receiver
    pub fn get_receiver(&self) -> Option<&mpsc::Receiver<Vec<u8>>> {
        self.receiver.as_ref()
    }
}