//! Transfer queue implementation for Cynthion devices
//! Based on Packetry's implementation using nusb

use std::collections::VecDeque;
use std::sync::mpsc;

use anyhow::Result;
use log::{debug, error, info, warn};
use nusb::{
    self,
    transfer::{
        Bulk, 
        BulkIn,
        Status as TransferStatus,
        TransferFuture,
    },
    Interface,
    RequestBuffer,
};

// Maximum number of completed transfers to keep in the done queue
const MAX_DONE_TRANSFERS: usize = 16;

/// A queue of USB bulk transfers to a device.
pub struct TransferQueue {
    interface: Interface,
    data_tx: mpsc::Sender<Vec<u8>>,
    endpoint: u8,
    read_size: usize,
    active_transfers: VecDeque<(BulkIn, Vec<u8>)>,
    done_transfers: VecDeque<(BulkIn, Vec<u8>)>,
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
        
        // Convert Vec<u8> to RequestBuffer for nusb
        let request_buffer = RequestBuffer::from_vec(buffer.clone());
        
        // In nusb, we create a bulk IN transfer (for receiving data)
        // The endpoint address already includes the direction bit (0x80 for IN)
        // Use proper RequestBuffer but handle the future differently (no wait method available)
        // For async we'd use await, but in our sync context we'll use a blocking approach
        let transfer = self.interface.bulk_in_blocking(self.endpoint, request_buffer, TIMEOUT)
            .map_err(|e| anyhow::anyhow!("Transfer error: {}", e))?;
        
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
                let request_buffer = RequestBuffer::from_vec(buffer.clone());
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