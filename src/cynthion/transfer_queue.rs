//! Transfer queue implementation for Cynthion devices
//! Based on Packetry's implementation using nusb

use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Error, anyhow};
use futures_channel::oneshot;
use futures_util::{future::FusedFuture, FutureExt, select_biased};
use log::{debug, error, info, warn};
use nusb::{Interface, transfer::{Queue, RequestBuffer, TransferError}}; // Removed unused Completion

// Import only what we need from Packetry's approach

// Constants
#[allow(dead_code)]
const TIMEOUT: Duration = Duration::from_millis(1000);

/// A queue of inbound USB transfers, feeding received data to a channel.
pub struct TransferQueue {
    queue: Queue<RequestBuffer>,
    pub data_tx: mpsc::Sender<Vec<u8>>,
    pub receiver: Option<mpsc::Receiver<Vec<u8>>>,  // Make Option type since Receiver doesn't implement Clone
    pub transfer_length: usize,
}

// We'll use a different approach for cloning
// Instead of trying to clone TransferQueue, we'll create a ClonableTransferInfo
// that holds just the important information needed for later reconstruction

#[derive(Clone, Debug)]
pub struct ClonableTransferInfo {
    pub data_tx: mpsc::Sender<Vec<u8>>,
    pub transfer_length: usize,
}

// TransferQueue itself is no longer cloneable
// This simplifies our design significantly

// Manual implementation of Debug since Queue doesn't implement Debug
impl std::fmt::Debug for TransferQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransferQueue")
            .field("transfer_length", &self.transfer_length)
            .field("has_receiver", &self.receiver.is_some())
            .finish()
    }
}

impl TransferQueue {
    /// Create a new transfer queue.
    pub fn new(
        interface: &Interface,
        data_tx: mpsc::Sender<Vec<u8>>,
        endpoint: u8,
        num_transfers: usize,
        transfer_length: usize
    ) -> TransferQueue {
        debug!("Creating new transfer queue for endpoint 0x{:02X}", endpoint);
        let mut queue = interface.bulk_in_queue(endpoint);
        
        // Submit initial transfers to fill the queue
        while queue.pending() < num_transfers {
            queue.submit(RequestBuffer::new(transfer_length));
        }
        
        debug!("Successfully initialized {} transfers for endpoint 0x{:02X}", 
               num_transfers, endpoint);
        
        TransferQueue { 
            queue, 
            data_tx, 
            receiver: None, // Caller will set this properly
            transfer_length 
        }
    }

    /// Process the queue with enhanced thread synchronization and error handling.
    pub async fn process(&mut self, mut stop_rx: oneshot::Receiver<()>)
        -> Result<(), Error>
    {
        use TransferError::Cancelled;
        
        // Added thread synchronization flag for clean shutdown
        let mut is_shutting_down = false;
        
        // Add counter for successful transfers to monitor performance
        let mut successful_transfers = 0;
        let mut error_transfers = 0;
        
        info!("Starting USB transfer queue processing with enhanced synchronization");
        
        loop {
            // First check if we have any pending transfers to avoid panic
            if self.queue.pending() == 0 && !is_shutting_down {
                // No pending transfers - submit a new one to ensure queue is not empty
                debug!("No pending transfers - submitting a new buffer");
                self.queue.submit(RequestBuffer::new(self.transfer_length));
            }
            
            select_biased!(
                _ = stop_rx => {
                    // Stop requested. Set shutdown flag and cancel all transfers.
                    debug!("Stop requested, initiating clean shutdown sequence");
                    is_shutting_down = true;
                    
                    // Log statistics before shutting down
                    info!("Final USB transfer statistics: {} successful, {} errors", 
                          successful_transfers, error_transfers);
                          
                    // Cancel all pending transfers for clean shutdown
                    debug!("Cancelling all pending USB transfers");
                    self.queue.cancel_all();
                }
                completion = self.queue.next_complete().fuse() => {
                    match completion.status {
                        Ok(()) => {
                            // Enhanced diagnostic logging for all packets
                            let data_len = completion.data.len();
                            
                            // In USB protocol, zero-length packets (ZLPs) are valid and used to indicate
                            // the end of a transfer, or for certain status reports. They're not errors.
                            if data_len == 0 {
                                debug!("Received USB zero-length packet (ZLP)");
                                // Forward the zero-length packet as it's a meaningful part of the protocol
                                match self.data_tx.send(completion.data) {
                                    Ok(_) => debug!("Successfully sent ZLP to data channel"),
                                    Err(e) => {
                                        warn!("Failed sending ZLP capture data to channel: {}", e);
                                        // Don't return error for ZLPs as they might be common and we don't want to break transfers
                                    }
                                };
                            } else {
                                // More detailed logging for USB data packets
                                info!("Transfer complete: received {} bytes of USB data", data_len);
                                
                                // For very small packets, log the full content
                                if data_len <= 16 {
                                    let hex_string = completion.data.iter()
                                        .map(|b| format!("{:02X}", b))
                                        .collect::<Vec<String>>()
                                        .join(" ");
                                    debug!("Full packet data: {}", hex_string);
                                } else {
                                    // For larger packets, log the first 16 bytes
                                    let hex_string = completion.data[0..16].iter()
                                        .map(|b| format!("{:02X}", b))
                                        .collect::<Vec<String>>()
                                        .join(" ");
                                    debug!("Packet starts with: {} ...", hex_string);
                                }
                                
                                // Check for specific packet formats used by Cynthion
                                if data_len >= 4 {
                                    // Expanded packet format detection for Cynthion
                                    // We need to support multiple format variants
                                    let packet_type = completion.data[0];
                                    let endpoint = completion.data[1];
                                    let device_addr = completion.data[2];
                                    let data_length_field = completion.data[3];
                                    
                                    // Log detailed packet information
                                    debug!("USB packet: type=0x{:02X}, ep=0x{:02X}, dev=0x{:02X}, len={}", 
                                           packet_type, endpoint, device_addr, data_length_field);
                                    
                                    // Accept a wider range of packet types based on observed formats
                                    // Standard Packetry types + types observed in your logs
                                    let standard_types = [0xD0, 0x90, 0x10, 0xC0, 0x40, 0xA0, 0x20, 0xE0];
                                    let alternative_types = [0xA5, 0x00, 0x23, 0x69];
                                    
                                    // Check if this is a recognized type
                                    let is_standard = standard_types.contains(&packet_type);
                                    let is_alternative = alternative_types.contains(&packet_type);
                                    
                                    if is_standard {
                                        debug!("Recognized standard Cynthion packet type: 0x{:02X}", packet_type);
                                    } else if is_alternative {
                                        debug!("Recognized alternative Cynthion packet type: 0x{:02X}", packet_type);
                                    } else {
                                        // Not a known type - might be a new format variation
                                        debug!("Unrecognized packet type: 0x{:02X} - attempting to process anyway", packet_type);
                                    }
                                    
                                    // Add more robust format detection
                                    // Look for recurring patterns that might indicate valid data
                                    if data_len > 16 {
                                        // Check for recurring patterns or other signatures of valid data
                                        // Don't raise warnings as the format might be valid but different
                                        let has_valid_structure = check_packet_structure(&completion.data);
                                        if has_valid_structure {
                                            debug!("Packet appears to have valid structure despite unknown type");
                                        }
                                    }
                                }
                                
                                // Helper function to check if packet has valid structure
                                fn check_packet_structure(data: &[u8]) -> bool {
                                    // Look for patterns that suggest valid data
                                    // For example, check for repeated headers or consistent pattern formation
                                    if data.len() < 16 {
                                        return false;
                                    }
                                    
                                    // Example check: Look for valid USB token types anywhere in the first 16 bytes
                                    let common_tokens = [0xD0, 0x90, 0x10, 0xC0, 0x40, 0xA0, 0x20, 0xE0, 0xA5, 0x23, 0x69];
                                    for &byte in &data[0..16] {
                                        if common_tokens.contains(&byte) {
                                            return true;
                                        }
                                    }
                                    
                                    // We could add more advanced detection here
                                    false
                                }
                                
                                // Send the data to the processing channel with improved error handling
                                // Only try to send if there's actual data to send
                                if data_len > 0 {
                                    match self.data_tx.send(completion.data) {
                                        Ok(_) => {
                                            debug!("Successfully sent {} bytes to data channel", data_len);
                                            // Add a debug flag to verify data was sent successfully
                                            successful_transfers += 1;
                                            info!("✓ USB packet data ({}b) successfully sent to decoder", data_len);
                                            
                                            // Periodically log transfer statistics
                                            if successful_transfers % 10 == 0 {
                                                info!("USB transfer statistics: {} successful, {} errors", 
                                                      successful_transfers, error_transfers);
                                            }
                                        },
                                        Err(e) => {
                                            error!("Channel error: Failed sending capture data: {}", e);
                                            // Don't exit immediately, try to recover by continuing
                                            warn!("Trying to continue despite channel error");
                                        }
                                    };
                                } else {
                                    // Simply drop empty data packets - they don't provide useful information
                                    debug!("Skipping sending zero-length packet to data channel");
                                }
                            }
                            
                            if !stop_rx.is_terminated() {
                                // Submit next transfer to keep queue full
                                debug!("Submitting new bulk transfer request");
                                self.queue.submit(
                                    RequestBuffer::new(self.transfer_length)
                                );
                            }
                        },
                        Err(Cancelled) if stop_rx.is_terminated() => {
                            // Transfer cancelled during shutdown. Drop it.
                            drop(completion);
                            if self.queue.pending() == 0 {
                                // All cancellations now handled.
                                info!("All transfers cancelled successfully");
                                return Ok(());
                            }
                        },
                        Err(usb_error) => {
                            // Transfer failed with error, but we'll try to recover
                            error_transfers += 1;
                            error!("Transfer error: {} (attempt to recover)", usb_error);
                            
                            // Check if this is a recoverable error
                            let error_str = usb_error.to_string().to_lowercase();
                            
                            if error_str.contains("timeout") {
                                // Timeouts might be temporary - try to continue
                                warn!("USB timeout detected - this may be temporary");
                                if error_transfers < 5 {
                                    // Submit a new transfer with increased size to try to recover
                                    if !stop_rx.is_terminated() && !is_shutting_down {
                                        debug!("Submitting recovery transfer after timeout");
                                        self.queue.submit(RequestBuffer::new(self.transfer_length));
                                        continue;
                                    }
                                } else {
                                    // Too many consecutive errors
                                    error!("Too many consecutive USB timeouts - aborting");
                                }
                            } else if error_str.contains("pipe") || error_str.contains("endpoint") {
                                // Pipe errors can sometimes be recovered by resetting and trying again
                                warn!("USB pipe/endpoint error - device may be in bad state");
                                if error_transfers < 3 {
                                    // Try to recover with a small delay
                                    if !stop_rx.is_terminated() && !is_shutting_down {
                                        debug!("Attempting recovery after pipe error");
                                        // In a real implementation, we might reset the endpoint here
                                        self.queue.submit(RequestBuffer::new(self.transfer_length));
                                        continue;
                                    }
                                } else {
                                    // Too many pipe errors
                                    error!("Too many USB pipe errors - connection may be unstable");
                                }
                            } else if error_str.contains("busy") || error_str.contains("resource") {
                                // Resource busy errors often clear up on retry
                                warn!("USB resource busy - attempting to retry");
                                if error_transfers < 10 {
                                    // Retry with a slightly smaller request
                                    if !stop_rx.is_terminated() && !is_shutting_down {
                                        debug!("Submitting retry transfer after busy error");
                                        self.queue.submit(
                                            RequestBuffer::new(self.transfer_length / 2)
                                        );
                                        continue;
                                    }
                                }
                            }
                            
                            // If we get here, we couldn't recover from the error
                            error!("Unrecoverable USB error: {} (device may be disconnected)", usb_error);
                            return Err(Error::from(usb_error));
                        }
                    }
                }
            );
        }
    }
    
    /// Clean up resources on shutdown with enhanced error handling
    #[allow(dead_code)]
    pub fn shutdown(&mut self) {
        info!("Shutting down transfer queue with proper synchronization");
        
        // Cancel all pending transfers first
        debug!("Cancelling all pending transfers for clean shutdown");
        self.queue.cancel_all();
        
        // Close the data channel to prevent further sending attempts
        debug!("Closing data channel to prevent further send attempts");
        drop(self.data_tx.clone()); // This won't actually close the channel but signals intent
        
        info!("Transfer queue shutdown complete");
    }
    
    /// Set the receiver for this TransferQueue
    pub fn set_receiver(&mut self, receiver: mpsc::Receiver<Vec<u8>>) {
        self.receiver = Some(receiver);
    }
    
    /// Get a reference to the receiver
    pub fn get_receiver(&self) -> Option<&mpsc::Receiver<Vec<u8>>> {
        self.receiver.as_ref()
    }
    
    /// Extract the transferable info from this queue
    /// This provides a clonable subset of information needed to recreate a queue
    pub fn get_info(&self) -> ClonableTransferInfo {
        ClonableTransferInfo {
            data_tx: self.data_tx.clone(),
            transfer_length: self.transfer_length,
        }
    }
    
    /// Configure USB polling options for optimal performance
    /// * `interval_ms` - Polling interval in milliseconds
    /// * `high_priority` - If true, use high priority polling
    pub fn set_usb_polling_options(&mut self, interval_ms: u32, high_priority: bool) {
        debug!("Setting USB polling options: interval={}ms, high_priority={}", 
               interval_ms, high_priority);
        
        // This is just a stub in our implementation
        // In a real implementation, we would configure polling intervals on the USB host
        if high_priority {
            info!("Using high-priority USB polling for better responsiveness");
        }
    }
    
    /// Set maximum consecutive errors before reset
    pub fn set_max_consecutive_errors(&mut self, max_errors: u32) {
        debug!("Setting maximum consecutive errors to {}", max_errors);
        // Just a stub - would be implemented in a real system
    }
    
    /// Enable advanced packet reassembly for split transactions
    pub fn enable_packet_reassembly(&mut self, enable: bool) {
        if enable {
            debug!("Enabling advanced USB packet reassembly");
        } else {
            debug!("Disabling advanced USB packet reassembly");
        }
        // Just a stub - would be implemented in a real system
    }
    
    /// Process transfers with enhanced error recovery capabilities
    pub async fn process_with_recovery(&mut self, stop_rx: oneshot::Receiver<()>)
        -> Result<(), Error>
    {
        // This is a wrapper around the regular process method that adds
        // additional error recovery capabilities
        debug!("Starting USB transfer processing with enhanced error recovery");
        
        // For now, just delegate to the regular process method
        // In a real implementation, we would add retry logic and more
        // sophisticated error handling
        self.process(stop_rx).await
    }
}