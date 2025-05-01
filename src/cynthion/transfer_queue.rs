//! Transfer queue implementation for Cynthion devices
//! Based on Packetry's implementation using nusb

use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Error, anyhow};
use futures_channel::oneshot;
use futures_util::{future::FusedFuture, FutureExt, select_biased};
use log::{debug, error, info, warn, trace};
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

    /// Process the queue, sending data to the channel until stopped.
    pub async fn process(&mut self, mut stop_rx: oneshot::Receiver<()>)
        -> Result<(), Error>
    {
        use TransferError::Cancelled;
        loop {
            select_biased!(
                _ = stop_rx => {
                    // Stop requested. Cancel all transfers.
                    debug!("Stop requested, cancelling all transfers");
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
                                    // Packet format for Cynthion typically begins with:
                                    // [packet_type, endpoint, device_addr, data_len, data...]
                                    let packet_type = completion.data[0];
                                    let endpoint = completion.data[1];
                                    let device_addr = completion.data[2];
                                    let data_length_field = completion.data[3];
                                    
                                    // Log detailed packet information
                                    debug!("USB packet: type=0x{:02X}, ep=0x{:02X}, dev=0x{:02X}, len={}", 
                                           packet_type, endpoint, device_addr, data_length_field);
                                    
                                    // Verify this looks like valid Cynthion MitM format
                                    // Valid USB packet types are 0xD0 (setup), 0x90 (IN), 0x10 (OUT), etc.
                                    let valid_types = [0xD0, 0x90, 0x10, 0xC0, 0x40, 0xA0, 0x20, 0xE0];
                                    if !valid_types.contains(&packet_type) {
                                        warn!("Unrecognized packet type: 0x{:02X} - may indicate incorrect format", packet_type);
                                    }
                                }
                                
                                // Send the data to the processing channel
                                match self.data_tx.send(completion.data) {
                                    Ok(_) => trace!("Successfully sent {} bytes to data channel", data_len),
                                    Err(e) => return Err(anyhow!("Failed sending capture data to channel: {}", e)),
                                };
                            }
                            
                            if !stop_rx.is_terminated() {
                                // Submit next transfer to keep queue full
                                trace!("Submitting new bulk transfer request");
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
                            // Transfer failed with error.
                            error!("Transfer error: {} (device may be disconnected)", usb_error);
                            return Err(Error::from(usb_error));
                        }
                    }
                }
            );
        }
    }
    
    /// Clean up resources on shutdown - cancel all pending transfers
    #[allow(dead_code)]
    pub fn shutdown(&mut self) {
        info!("Shutting down transfer queue");
        self.queue.cancel_all();
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
}