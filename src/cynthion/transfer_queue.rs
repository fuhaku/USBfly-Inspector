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
    self,
    transfer::{
        self,
        Status as TransferStatus,
        RequestBuffer,
    },
    Interface,
};

// Constants
const MAX_DONE_TRANSFERS: usize = 16;  // Maximum number of completed transfers to keep in the done queue
const TIMEOUT: Duration = Duration::from_millis(1000);

// Define the bulk transfer type we'll use
type BulkTransfer = transfer::Bulk;

/// A queue of USB bulk transfers to a device.
pub struct TransferQueue {
    interface: Interface,
    data_tx: mpsc::Sender<Vec<u8>>,
    endpoint: u8,
    read_size: usize,
    active_transfers: VecDeque<(BulkTransfer, Vec<u8>)>,
    done_transfers: VecDeque<(BulkTransfer, Vec<u8>)>,
    transfer_id: usize,
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
        let mut queue = TransferQueue {
            interface: interface.clone(),
            data_tx,
            endpoint,
            read_size,
            active_transfers: VecDeque::with_capacity(num_transfers),
            done_transfers: VecDeque::with_capacity(MAX_DONE_TRANSFERS),
            transfer_id: 0,
        };

        // Initialize the transfer queue by submitting initial transfers
        queue.initialize_transfers(num_transfers);
        
        queue
    }
    
    /// Initialize the transfer queue with a set of transfers
    fn initialize_transfers(&mut self, num_transfers: usize) {
        for _ in 0..num_transfers {
            match self.submit_transfer() {
                Ok(_) => {
                    // Transfer submitted successfully
                }
                Err(e) => {
                    error!("Error submitting initial transfer: {}", e);
                    // Continue anyway, we'll try to make the best of what we have
                }
            }
        }
    }
    
    /// Submit a new bulk transfer request
    fn submit_transfer(&mut self) -> Result<()> {
        // Create a new buffer for the transfer
        let buffer = vec![0u8; self.read_size];
        
        // Create a new RequestBuffer of the appropriate size
        // The RequestBuffer API doesn't have from_vec, but it has "reuse" for existing buffers
        let request_buffer = RequestBuffer::reuse(buffer.clone(), self.read_size);
        
        // In nusb, we create a bulk IN transfer (for receiving data)
        // The endpoint address already includes the direction bit (0x80 for IN)
        // First we get a future, then poll it to get the transfer
        let future = self.interface.bulk_in(self.endpoint, request_buffer);
        
        // Poll the future until we get a result - based on Packetry's approach
        // Create a dummy waker and context to poll the future
        let waker = futures::task::noop_waker();
        let mut context = Context::from_waker(&waker);
        
        // We need to pin the future to poll it
        let mut pinned = Box::pin(future);
        
        // Poll the future and get the result
        let result = match Pin::new(&mut pinned).poll(&mut context) {
            Poll::Ready(result) => result,
            Poll::Pending => return Err(anyhow::anyhow!("Transfer future is still pending")),
        };
        
        let transfer = match result {
            Ok(transfer) => transfer,
            Err(e) => return Err(anyhow::anyhow!("Transfer error: {}", e)),
        };
        
        // Add to active transfers queue
        self.active_transfers.push_back((transfer, buffer));
        self.transfer_id += 1;
        
        Ok(())
    }
    
    /// Process completed transfers and resubmit them
    pub fn process_completed_transfers(&mut self) -> Result<()> {
        let mut processed_count = 0;
        
        // Check all active transfers for completion
        while let Some((mut transfer, mut buffer)) = self.active_transfers.pop_front() {
            match transfer.check_status() {
                // Completed successfully - process the data
                TransferStatus::Completed => {
                    let actual = transfer.actual_length().unwrap_or(0);
                    if actual > 0 {
                        debug!("Transfer complete: {} bytes", actual);
                        
                        // Resize buffer to actual size and send it to the processor
                        buffer.truncate(actual);
                        if let Err(e) = self.data_tx.send(buffer.clone()) {
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
                }
                
                // Still in progress - put it back in the queue
                TransferStatus::Queued | TransferStatus::Transferring => {
                    self.active_transfers.push_back((transfer, buffer));
                    break;  // No need to check further transfers
                }
                
                // Transfer failed or cancelled
                TransferStatus::Error(e) => {
                    error!("Transfer error: {}", e);
                    // Save for reuse anyway
                    self.done_transfers.push_back((transfer, buffer));
                }
                
                TransferStatus::Cancelled => {
                    warn!("Transfer was cancelled");
                    // Save for reuse
                    self.done_transfers.push_back((transfer, buffer));
                }
                
                // Other status
                other => {
                    warn!("Transfer in unexpected state: {:?}", other);
                    // Save for reuse
                    self.done_transfers.push_back((transfer, buffer));
                }
            }
        }
        
        // If we processed any transfers, submit new ones to keep the queue full
        if processed_count > 0 {
            // Reuse transfers from the done queue
            while let Some((transfer, mut buffer)) = self.done_transfers.pop_front() {
                // Reset the buffer
                buffer.resize(self.read_size, 0);
                
                // Resubmit the transfer with proper RequestBuffer
                let request_buffer = RequestBuffer::reuse(buffer.clone(), self.read_size);
                match transfer.submit(request_buffer) {
                    Ok(transfer) => {
                        // Put it back in the active queue
                        self.active_transfers.push_back((transfer, buffer));
                    }
                    Err(e) => {
                        error!("Failed to resubmit transfer: {}", e);
                        // If we couldn't resubmit, try to create a new transfer instead
                        if let Err(e) = self.submit_transfer() {
                            error!("Failed to create new transfer: {}", e);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Cancel all active transfers and clean up resources
    pub fn shutdown(&mut self) {
        info!("Shutting down transfer queue");
        
        // Cancel and drain all active transfers
        while let Some((transfer, _)) = self.active_transfers.pop_front() {
            if let Err(e) = transfer.cancel() {
                error!("Failed to cancel transfer: {}", e);
            }
        }
        
        // Clear the done queue
        self.done_transfers.clear();
    }
}