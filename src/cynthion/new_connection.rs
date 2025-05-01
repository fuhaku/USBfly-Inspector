//! Cynthion device connection handler using nusb
//! This is a clean reimplementation based on Packetry's approach

use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Result, bail, Context as AnyhowContext, Error};
use log::{info, error, warn, debug};
use nusb::{
    self,
    transfer::{
        Control,
        ControlType,
        Recipient,
    },
    DeviceInfo,
    Interface,
};

// Import the Speed enum from the usb module instead of the deprecated one
use crate::usb::Speed;

// Bitfield structures for device control
// We'll implement these manually since we're transitioning away from the bitfield crate

/// State structure for controlling Cynthion capture
struct State(u8);

impl State {
    fn new(enable: bool, speed: Speed) -> State {
        let mut value = 0u8;
        // Set enable bit (bit 0)
        if enable {
            value |= 0x01;
        }
        // Set speed bits (bits 1-2)
        value |= (speed as u8) << 1;
        State(value)
    }
}

/// TestConfig for Cynthion test device simulation
struct TestConfig(u8);

impl TestConfig {
    fn new(speed: Option<Speed>) -> TestConfig {
        let mut value = 0u8;
        if let Some(speed) = speed {
            // Set connect bit (bit 0)
            value |= 0x01;
            // Set speed bits (bits 1-2)
            value |= (speed as u8) << 1;
        }
        TestConfig(value)
    }
}
use crate::cynthion::transfer_queue::TransferQueue;

// Constants for Cynthion device connection
pub const CYNTHION_VID: u16 = 0x1d50;
pub const CYNTHION_PID: u16 = 0x615b;    // Original Cynthion firmware VID/PID
#[allow(dead_code)]
const CLASS: u8 = 0xff;                  // Vendor-specific class
#[allow(dead_code)]
const SUBCLASS: u8 = 0x10;               // USB analysis subclass
#[allow(dead_code)]
const PROTOCOL: u8 = 0x01;               // Cynthion protocol version
const ENDPOINT: u8 = 0x81;               // Bulk in endpoint for receiving data
const READ_LEN: usize = 0x4000;          // 16K buffer size for transfers
const NUM_TRANSFERS: usize = 4;          // Number of concurrent transfers
#[allow(dead_code)]
const TIMEOUT: Duration = Duration::from_millis(1000);

// Additional compatible devices
const ALT_CYNTHION_PID_1: u16 = 0x615c;
const ALT_CYNTHION_PID_2: u16 = 0x615d;
const GREATFET_ONE_PID: u16 = 0x60e6;

// Bitfield macros already defined in connection.rs

/// A Cynthion device that can be used for USB capture.
#[derive(Debug, Clone)]
pub struct CynthionDevice {
    device_info: DeviceInfo,
    interface_number: u8,
    #[allow(dead_code)]
    alt_setting_number: u8,
    #[allow(dead_code)]
    supported_speeds: Vec<Speed>,
}

impl CynthionDevice {
    // Check if the device is a supported Cynthion device
    pub fn is_supported(vid: u16, pid: u16) -> bool {
        (vid == CYNTHION_VID && pid == CYNTHION_PID) || // Primary Cynthion
        (vid == CYNTHION_VID && pid == ALT_CYNTHION_PID_1) || // Alt firmware version
        (vid == CYNTHION_VID && pid == ALT_CYNTHION_PID_2) || // Alt firmware version
        (vid == CYNTHION_VID && pid == GREATFET_ONE_PID)   // GreatFET One
    }

    // Find all compatible devices on the system
    pub fn find_all() -> Result<Vec<CynthionDevice>> {
        // Check if we're in forced hardware mode
        let force_hardware = std::env::var("USBFLY_FORCE_HARDWARE")
            .unwrap_or_else(|_| "0".to_string()) == "1";
            
        if force_hardware {
            info!("Listing devices in FORCE HARDWARE mode");
        }
        
        let devices = nusb::list_devices()?;
        
        let mut result = Vec::new();
        for device_info in devices {
            let device = match Self::from_device_info(device_info.clone()) {
                Some(device) => device,
                None => continue,
            };
            result.push(device);
        }
        
        Ok(result)
    }
    
    // Find all devices with force hardware mode enabled
    pub fn find_all_force_hardware() -> Result<Vec<CynthionDevice>> {
        // Set environment variable to indicate force hardware mode
        std::env::set_var("USBFLY_FORCE_HARDWARE", "1");
        // Call regular find_all
        Self::find_all()
    }
    
    // Create from device info if compatible
    fn from_device_info(device_info: DeviceInfo) -> Option<CynthionDevice> {
        // Check if this is a supported Cynthion device
        if !Self::is_supported(device_info.vendor_id(), device_info.product_id()) {
            return None;
        }
        
        // nusb has a different API than rusb - we need to open the device to inspect interfaces
        // For now we'll use interface 0, alt 0 for Cynthion devices - we'll check this when opening
        let selected_if = 0;
        let selected_alt = 0;
        
        info!("Found potential Cynthion device: VID:{:04x} PID:{:04x}",
             device_info.vendor_id(), device_info.product_id());
        
        // Create device with empty speed list - will be populated when opened
        Some(CynthionDevice {
            device_info,
            interface_number: selected_if,
            alt_setting_number: selected_alt,
            supported_speeds: Vec::new(),
        })
    }
    
    // Open the device for communication
    pub fn open(&self) -> Result<CynthionHandle> {
        let device = self.device_info.open()?;
        
        // Attempt to claim the interface
        // In nusb, this creates an Interface object we can use
        let interface = match device.claim_interface(self.interface_number) {
            Ok(interface) => interface,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to claim interface {}: {}", 
                          self.interface_number, e));
            }
        };
        
        // Create the connection handle
        Ok(CynthionHandle {
            interface,
            device_info: self.device_info.clone(),
            transfer_queue: None,
            data_receiver: None,
            pending_data_tx: None,
            capture_on_connect: false,
        })
    }
    
    // Get device information
    pub fn vendor_id(&self) -> u16 {
        self.device_info.vendor_id()
    }
    
    pub fn product_id(&self) -> u16 {
        self.device_info.product_id()
    }

    pub fn manufacturer(&self) -> &str {
        self.device_info.manufacturer_string()
            .unwrap_or("Unknown Manufacturer")
    }
    
    pub fn product(&self) -> &str {
        self.device_info.product_string()
            .unwrap_or("Unknown Device")
    }
    
    pub fn serial_number(&self) -> &str {
        self.device_info.serial_number()
            .unwrap_or("N/A")
    }
    
    pub fn get_description(&self) -> String {
        let product = self.device_info.product_string()
            .unwrap_or("Unknown Device");
        
        format!("{} {:04x}:{:04x} (Interface {})", 
               product, 
               self.device_info.vendor_id(), 
               self.device_info.product_id(),
               self.interface_number)
    }
}

/// A handle to an open Cynthion device.
pub struct CynthionHandle {
    interface: Interface,
    device_info: DeviceInfo,
    transfer_queue: Option<TransferQueue>,
    data_receiver: Option<mpsc::Receiver<Vec<u8>>>,
    pending_data_tx: Option<mpsc::Sender<Vec<u8>>>,
    capture_on_connect: bool,
}

// Manual implementation of Clone since TransferQueue can't be directly cloned
impl Clone for CynthionHandle {
    fn clone(&self) -> Self {
        // We can clone the interface and device_info
        let mut cloned = CynthionHandle {
            interface: self.interface.clone(),
            device_info: self.device_info.clone(),
            transfer_queue: None, // Can't directly clone the transfer queue
            data_receiver: None,  // Can't directly clone the receiver
            pending_data_tx: None, // We'll create a new one if needed
            capture_on_connect: self.capture_on_connect, // Clone this flag
        };
        
        // If there was a transfer queue, we need to reconstruct it
        if let Some(queue) = &self.transfer_queue {
            // Get the transferable info
            let info = queue.get_info();
            
            // Create a new channel
            let (_tx, rx) = mpsc::channel();
            
            // Create a new TransferQueue for the clone
            // This won't be fully functional for transfers but will have the
            // same data_tx and transfer_length properties
            let mut new_queue = TransferQueue::new(
                &cloned.interface,
                info.data_tx.clone(),  // Use the original tx
                ENDPOINT,
                NUM_TRANSFERS,
                info.transfer_length
            );
            
            // Set the new receiver
            new_queue.set_receiver(rx);
            
            // Assign to the clone
            cloned.transfer_queue = Some(new_queue);
        }
        
        cloned
    }
}

// Manual implementation of Debug since Interface doesn't implement Debug
impl std::fmt::Debug for CynthionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CynthionHandle")
            .field("device_info", &format!("VID:{:04x} PID:{:04x}", 
                    self.device_info.vendor_id(), 
                    self.device_info.product_id()))
            .field("interface_number", &self.interface.interface_number())
            .field("transfer_queue", &self.transfer_queue)
            .finish()
    }
}

impl CynthionHandle {
    // Get the supported speeds from the device
    #[allow(dead_code)]
    fn speeds(&self) -> Result<Vec<crate::usb::Speed>> {
        // Import the Speed enum only within this method
        use crate::usb::Speed::*;
        
        let control = Control {
            control_type: ControlType::Vendor,
            recipient: Recipient::Interface,
            request: 2,
            value: 0,
            index: self.interface.interface_number() as u16,
        };
        
        let mut buf = [0; 64];
        let timeout = Duration::from_secs(1);
        
        let size = self.interface
            .control_in_blocking(control, &mut buf, timeout)
            .context("Failed retrieving supported speeds from device")?;
            
        if size != 1 {
            bail!("Expected 1-byte response to speed request, got {size}");
        }
        
        let mut speeds = Vec::new();
        // Each speed corresponds to a bit in the response
        // Auto = bit 0, High = bit 1, Full = bit 2, Low = bit 3
        if buf[0] & 0x01 != 0 { speeds.push(Auto); }
        if buf[0] & 0x02 != 0 { speeds.push(High); }
        if buf[0] & 0x04 != 0 { speeds.push(Full); }
        if buf[0] & 0x08 != 0 { speeds.push(Low); }
        
        Ok(speeds)
    }
    
    // Start capturing USB traffic with specified speed
    pub fn start_capture(&mut self) -> Result<()> {
        // Create a channel to receive packets from the device
        let (data_tx, data_rx) = std::sync::mpsc::channel();
        
        // Store the receiver for later use
        self.data_receiver = Some(data_rx);
        
        // If the device is ready, set up the transfer queue immediately
        if self.device_info.vendor_id() == CYNTHION_VID {
            info!("Starting capture on Cynthion device: {:04x}:{:04x} using High Speed mode", 
                  self.device_info.vendor_id(), self.device_info.product_id());
            
            // First ensure the device is not already in capture mode
            // by sending a stop command. This helps reset the device state.
            info!("Resetting Cynthion to ensure clean capture state");
            if let Err(e) = self.write_request(1, State::new(false, Speed::High).0) {
                warn!("Failed to reset Cynthion capture state: {} (continuing anyway)", e);
                // Don't return error, just continue and try to start capture
                std::thread::sleep(std::time::Duration::from_millis(500));
            } else {
                // Wait for the device to reset
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            
            // Initialize transfer queue before starting capture
            // This ensures we're ready to receive data as soon as capture starts
            info!("Initializing USB transfer queue");
            let queue = TransferQueue::new(
                &self.interface, 
                data_tx,
                ENDPOINT, 
                NUM_TRANSFERS, 
                READ_LEN
            );
            
            // Store the transfer queue
            self.transfer_queue = Some(queue);
            
            // Try to set the device to capture mode with multiple attempts if needed
            let max_attempts = 5; // Increase max attempts
            let mut last_error = None;
            let mut success = false;
            
            info!("Commanding Cynthion to start Man-in-the-Middle capture...");
            for attempt in 1..=max_attempts {
                match self.write_request(1, State::new(true, Speed::High).0) {
                    Ok(_) => {
                        info!("Successfully started capture on attempt {}", attempt);
                        success = true;
                        break;
                    },
                    Err(e) => {
                        warn!("Failed to start capture (attempt {}/{})): {}", 
                            attempt, max_attempts, e);
                        last_error = Some(e);
                        
                        // Wait longer between attempts
                        if attempt < max_attempts {
                            let backoff = 100 * attempt; // Increasing backoff
                            std::thread::sleep(std::time::Duration::from_millis(backoff));
                        }
                    }
                }
            }
            
            if success {
                // Create a background thread to handle transfers
                self.start_async_processing();
                
                // Wait a moment to ensure capture is fully initialized
                std::thread::sleep(std::time::Duration::from_millis(100));
                
                info!("Cynthion Man-in-the-Middle mode activated, ready to capture USB traffic");
                return Ok(());
            }
            
            // If we've tried multiple times and still failed, report the error
            if let Some(e) = last_error {
                error!("Failed to start capture after {} attempts", max_attempts);
                // Reset the transfer queue since it failed
                self.transfer_queue = None;
                return Err(anyhow::anyhow!("Failed to start capture: {}", e));
            } else {
                // This shouldn't happen, but just in case
                self.transfer_queue = None;
                return Err(anyhow::anyhow!("Failed to start capture after {} attempts", max_attempts));
            }
        } else {
            // Even if no real Cynthion device is connected yet, we'll still indicate capture is prepared
            // This way we'll be ready to capture traffic as soon as a device is connected
            info!("No Cynthion device connected yet, but capture will start when device connects.");
            
            // Store the transmitter for later use when a device connects
            self.pending_data_tx = Some(data_tx);
            
            // Mark that we should start capturing immediately when a device connects
            self.capture_on_connect = true;
            
            return Ok(());
        }
    }
    
    // Stop capturing USB traffic
    pub fn stop_capture(&mut self) -> Result<()> {
        self.write_request(1, State::new(false, Speed::High).0)
    }
    
    // Configure the built-in test device (if available)
    #[allow(dead_code)]
    pub fn configure_test_device(&mut self, speed: Option<Speed>) -> Result<()> {
        let test_config = TestConfig::new(speed);
        self.write_request(3, test_config.0)
            .context("Failed to set test device configuration")
    }
    
    // Helper method to send vendor requests to the device
    fn write_request(&mut self, request: u8, value: u8) -> Result<()> {
        // Request 1 with value 1 starts capture, with value 0 stops capture
        // Request 3 configures test device
        let request_type = match request {
            1 => if value & 0x01 != 0 { "START_CAPTURE" } else { "STOP_CAPTURE" },
            3 => "CONFIGURE_TEST_DEVICE",
            _ => "UNKNOWN"
        };
        
        debug!("Sending Cynthion control request: {} (req={}, val=0x{:02X})", 
               request_type, request, value);

        let control = Control {
            control_type: ControlType::Vendor,
            recipient: Recipient::Interface,
            request,
            value: u16::from(value),
            index: self.interface.interface_number() as u16,
        };
        
        // Use a longer timeout for the start capture command
        let timeout = if request == 1 && value & 0x01 != 0 {
            // Longer timeout for start capture
            Duration::from_secs(3)
        } else {
            Duration::from_secs(1)
        };
        
        // Send the control request and capture the bytes transferred
        match self.interface.control_out_blocking(control, &[], timeout) {
            Ok(bytes) => {
                debug!("Cynthion control request succeeded: {} bytes transferred", bytes);
                Ok(())
            },
            Err(e) => {
                error!("Cynthion control request failed: {}", e);
                Err(Error::from(e))
            }
        }
    }
    
    // Begin capture and return a queue for processing transfers
    #[allow(dead_code)]
    pub fn begin_capture(
        &mut self,
        data_tx: mpsc::Sender<Vec<u8>>
    ) -> Result<TransferQueue> {
        // Default to High speed for now
        self.start_capture()?;
        
        Ok(TransferQueue::new(&self.interface, data_tx,
            ENDPOINT, NUM_TRANSFERS, READ_LEN))
    }
    
    // End capture
    #[allow(dead_code)]
    pub fn end_capture(&mut self) -> Result<()> {
        self.stop_capture()
    }
    
    // Get device information
    #[allow(dead_code)]
    pub fn vendor_id(&self) -> u16 {
        self.device_info.vendor_id()
    }
    #[allow(dead_code)]
    pub fn product_id(&self) -> u16 {
        self.device_info.product_id()
    }
    
    #[allow(dead_code)]
    pub fn manufacturer(&self) -> &str {
        self.device_info.manufacturer_string()
            .unwrap_or("Unknown Manufacturer")
    }
    
    #[allow(dead_code)]
    pub fn product(&self) -> &str {
        self.device_info.product_string()
            .unwrap_or("Unknown Device")
    }
    
    #[allow(dead_code)]
    pub fn serial_number(&self) -> &str {
        self.device_info.serial_number()
            .unwrap_or("N/A")
    }
    
    // The old direct read implementation is replaced by the version below
    // that uses TransferQueue for better performance and reliability
    fn _deprecated_read_mitm_direct(&mut self) -> Result<Vec<u8>> {
        // This is just a placeholder stub to avoid duplicate method definitions
        Ok(Vec::new())
    }
    
    // Set read timeout for bulk operations
    pub fn set_read_timeout(&mut self, _duration: Option<Duration>) -> Result<()> {
        // nusb doesn't have a direct timeout setting, we'll just store it for future use
        // This is a stub method to maintain API compatibility
        Ok(())
    }
    
    // Check if this is a simulation
    pub fn is_simulation_mode(&self) -> bool {
        // Only use simulation mode when explicitly enabled by env var
        // AND we don't have a real device connected
        let sim_enabled = std::env::var("USBFLY_SIMULATION_MODE").unwrap_or_else(|_| "0".to_string()) == "1";
        
        // If we have a real device connection, never use simulation mode
        // to prevent showing simulated data with a real device
        if self.device_info.vendor_id() == CYNTHION_VID {
            return false;
        }
        
        sim_enabled
    }
    
    // Check if device is connected
    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        // For now, if we have an interface, we're considered connected
        true
    }
    
    // Clear capture buffer (simulation only)
    #[allow(dead_code)]
    pub fn clear_capture_buffer(&mut self) -> Result<()> {
        // Real hardware doesn't need to clear buffer as it streams constantly
        Ok(())
    }
    
    // This is just a forward to the actual implementation
    // for compatibility with places where &self is used instead of &mut self
    fn get_simulated_mitm_traffic(&self) -> Vec<u8> {
        // Create a mutable reference to self
        let mut me = self.clone();
        // Call the public implementation
        me.get_simulated_mitm_traffic_pub()
    }
    
    // Read MitM traffic using an improved approach based on Packetry
    pub fn read_mitm_traffic_clone(&mut self) -> Result<Vec<u8>> {
        // If we're in simulation mode, return simulated data
        if self.is_simulation_mode() {
            return Ok(self.get_simulated_mitm_traffic());
        }
        
        // If capture_on_connect is true, we should check if we need to start capture
        // This could happen if a device was connected after starting capture
        if self.capture_on_connect && self.transfer_queue.is_none() && self.device_info.vendor_id() == CYNTHION_VID {
            info!("Device connected while capture was waiting - initializing capture now");
            
            // Get the stored transmitter from pending_data_tx
            if let Some(data_tx) = self.pending_data_tx.take() {
                // Create a transfer queue for the bulk transfers
                let queue = TransferQueue::new(
                    &self.interface, 
                    data_tx.clone(),
                    ENDPOINT, 
                    NUM_TRANSFERS, 
                    READ_LEN
                );
                
                // Store the transfer queue
                self.transfer_queue = Some(queue);
                
                // Try to start the capture with multiple attempts
                let max_attempts = 3;
                let mut success = false;
                
                for attempt in 1..=max_attempts {
                    match self.write_request(1, State::new(true, Speed::High).0) {
                        Ok(_) => {
                            info!("Successfully started capture on newly connected device (attempt {})", attempt);
                            success = true;
                            break;
                        },
                        Err(e) => {
                            warn!("Failed to start capture on newly connected device (attempt {}/{}): {}", 
                                  attempt, max_attempts, e);
                            
                            // Wait briefly before retrying
                            if attempt < max_attempts {
                                std::thread::sleep(std::time::Duration::from_millis(100));
                            }
                        }
                    }
                }
                
                if success {
                    // Start async processing in a background thread
                    self.start_async_processing();
                } else {
                    // Failed to start capture on the new device
                    error!("Failed to start capture on newly connected device after {} attempts", max_attempts);
                    self.transfer_queue = None;
                }
            }
        }
        
        // Check if we have an active transfer queue
        if self.transfer_queue.is_none() {
            info!("Initializing transfer queue for Cynthion device: {:04x}:{:04x}", 
                 self.device_info.vendor_id(), self.device_info.product_id());
            
            // Create a proper channel for data transfer
            let (tx, rx) = mpsc::channel();
            
            // Create a new transfer queue with the transmitter
            let mut transfer_queue = TransferQueue::new(
                &self.interface, 
                tx,
                ENDPOINT, 
                NUM_TRANSFERS, 
                READ_LEN
            );
            
            // Set the receiver in the transfer queue
            transfer_queue.set_receiver(rx);
                
            // Store the transfer queue
            self.transfer_queue = Some(transfer_queue);
            
            // Start the capture with proper error handling
            match self.start_capture() {
                Ok(_) => {
                    info!("Successfully started USB traffic capture");
                    // Start async processing in a separate thread
                    self.start_async_processing();
                },
                Err(e) => {
                    error!("Failed to start USB traffic capture: {}", e);
                    // Reset the transfer queue since it failed
                    self.transfer_queue = None;
                    return Err(anyhow::anyhow!("Failed to start capture: {}", e));
                }
            }
            
            // Return empty data for this first call
            return Ok(Vec::new());
        }
        
        // For our new approach, we don't need to manually process transfers
        // They're handled by the async processing thread
        // Just check if there's data available in the channel
        if let Some(queue) = &self.transfer_queue {
            if let Some(receiver) = queue.get_receiver() {
                match receiver.try_recv() {
                    Ok(data) => {
                        // Successfully received data from the queue
                        return Ok(data);
                    },
                    Err(mpsc::TryRecvError::Empty) => {
                        // No data available yet, return empty vector
                        return Ok(Vec::new());
                    },
                    Err(mpsc::TryRecvError::Disconnected) => {
                        // Channel is disconnected, possibly device disconnected
                        warn!("Transfer queue channel disconnected - device may have been unplugged");
                        // Reset the transfer queue to force re-initialization
                        self.transfer_queue = None;
                        return Ok(Vec::new());
                    }
                }
            } else {
                // No receiver available (can happen for cloned connections)
                return Ok(Vec::new());
            }
        }
        
        // Fallback if no queue is available
        Ok(Vec::new())
    }
    
    // Start async processing of USB transfers in a background thread
    fn start_async_processing(&self) {
        // We need to clone these for the thread
        let interface = self.interface.clone();
        let device_info = self.device_info.clone();
        
        // If we have a transfer queue, set up async processing
        if let Some(queue) = &self.transfer_queue {
            // Get the cloneable information from the queue
            let transfer_info = queue.get_info();
            
            // Create a oneshot channel for signaling stopping
            // The stop_tx is stored for future use when we want to stop the transfer
            // For now it's unused but we'll need it when implementing proper shutdown
            let (_stop_tx, stop_rx) = futures_channel::oneshot::channel();
            
            // Create a new transfer queue with the same properties
            std::thread::spawn(move || {
                info!("USB transfer processing thread started for device {:04x}:{:04x}",
                      device_info.vendor_id(), device_info.product_id());
                      
                // Set up tokio runtime for async processing
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create tokio runtime");
                
                debug!("Initializing transfer queue in async processing thread for endpoint 0x{:02X}", ENDPOINT);
                
                // Process transfers in the async runtime
                if let Err(e) = rt.block_on(async {
                    // Create a new queue for this thread
                    let mut queue = TransferQueue::new(
                        &interface, 
                        transfer_info.data_tx,
                        ENDPOINT, 
                        NUM_TRANSFERS, 
                        transfer_info.transfer_length
                    );
                    
                    info!("USB transfer queue successfully created in processing thread");
                    debug!("Starting USB transfer processing loop - waiting for USB data packets");
                    
                    // Process transfers until stopped
                    // This will continuously poll for new USB data
                    queue.process(stop_rx).await
                }) {
                    error!("Error in transfer processing thread: {}", e);
                    
                    // Provide more detailed diagnostic information based on the error
                    if e.to_string().contains("cancelled") {
                        info!("Transfer was cancelled - this is normal during shutdown");
                    } else if e.to_string().contains("pipe") || e.to_string().contains("endpoint") {
                        warn!("USB communication pipe error - device may have been disconnected");
                    } else if e.to_string().contains("permission") {
                        error!("USB permission error - insufficient permission to access device");
                    } else if e.to_string().contains("busy") {
                        warn!("USB device is busy - another application may be using it");
                    }
                }
                
                info!("USB transfer processing thread completed");
                // Thread will exit when processing is complete or errors
            });
            
            info!("Started async transfer processing thread");
        } else {
            warn!("Cannot start async processing - no transfer queue available");
        }
    }
    
    // Process raw data into USB transactions (for nusb implementation)
    pub fn process_transactions(&mut self, data: &[u8]) -> Vec<crate::usb::mitm_traffic::UsbTransaction> {
        use crate::usb::mitm_traffic::{UsbTransaction, UsbTransferType, UsbDirection};
        
        // If data is empty, return empty vector
        if data.is_empty() {
            return Vec::new();
        }
        
        // For our nusb implementation, properly process the data
        let mut transactions = Vec::new();
        
        // Ensure we have enough data for at least one transaction (minimum 8 bytes)
        if data.len() < 8 {
            debug!("Data too short to contain valid USB transaction: {} bytes", data.len());
            return Vec::new();
        }
        
        // Process data into packets
        let mut offset = 0;
        while offset + 8 <= data.len() {
            // Read packet header
            let packet_type = data[offset];
            let endpoint = data[offset + 1];
            let direction = if endpoint & 0x80 != 0 { 
                UsbDirection::DeviceToHost 
            } else { 
                UsbDirection::HostToDevice 
            };
            
            // Extract device address
            let device_addr = data[offset + 2];
            
            // Identify transfer type based on packet_type
            let transfer_type = match packet_type {
                0xD0 => UsbTransferType::Control,   // SETUP token
                0x90 => UsbTransferType::Bulk,      // IN token (bulk)
                0xC0 => UsbTransferType::Interrupt, // IN token (interrupt)
                0x10 => UsbTransferType::Bulk,      // OUT token (bulk)
                0x40 => UsbTransferType::Interrupt, // OUT token (interrupt)
                _ => {
                    debug!("Unknown packet type: 0x{:02X}, assuming Bulk", packet_type);
                    UsbTransferType::Bulk  // Default to bulk
                }
            };
            
            // Calculate data length
            let data_len = if offset + 3 < data.len() {
                data[offset + 3] as usize
            } else {
                0
            };
            
            // Safety check for data bounds
            if offset + 4 + data_len > data.len() {
                debug!("Packet data exceeds buffer bounds: offset={}, len={}, buffer={}", 
                       offset, data_len, data.len());
                break;
            }
            
            // Extract data payload
            let payload = data[offset+4..offset+4+data_len].to_vec();
            
            // Generate a transaction ID
            let id = transactions.len() as u64 + 1;
            
            // Create a data packet for this transaction
            let data_packet = crate::usb::mitm_traffic::UsbDataPacket::new(
                payload, 
                direction,
                endpoint & 0x7F // Remove direction bit
            );
            
            // Create a status packet (assume success)
            let status_packet = crate::usb::mitm_traffic::UsbStatusPacket {
                status: crate::usb::mitm_traffic::UsbTransferStatus::ACK,
                endpoint: endpoint & 0x7F,
            };
            
            // Create and update the transaction with our data
            let mut transaction = UsbTransaction {
                id,
                transfer_type,
                setup_packet: None, // Will be filled below for control transfers
                data_packet: Some(data_packet),
                status_packet: Some(status_packet),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64(),
                device_address: device_addr,
                endpoint: endpoint & 0x7F, // Remove direction bit
                fields: {
                    let mut fields = std::collections::HashMap::new();
                    fields.insert("speed".to_string(), "High".to_string());
                    fields.insert("packet_type".to_string(), format!("0x{:02X}", packet_type));
                    if packet_type == 0xD0 {
                        fields.insert("setup".to_string(), "true".to_string());
                    }
                    fields
                },
            };
            
            // If this is a setup packet, create a setup packet structure
            if packet_type == 0xD0 && data_len >= 8 {
                // Get setup packet data
                let setup_data = &data[offset+4..offset+4+8]; // Standard setup packet is 8 bytes
                
                // Extract setup packet fields
                let bm_request_type = setup_data[0];
                let b_request = setup_data[1];
                let w_value = u16::from_le_bytes([setup_data[2], setup_data[3]]);
                let w_index = u16::from_le_bytes([setup_data[4], setup_data[5]]);
                let w_length = u16::from_le_bytes([setup_data[6], setup_data[7]]);
                
                // Determine request type and recipient
                let request_type = match (bm_request_type >> 5) & 0x03 {
                    0 => crate::usb::mitm_traffic::UsbControlRequestType::Standard,
                    1 => crate::usb::mitm_traffic::UsbControlRequestType::Class,
                    2 => crate::usb::mitm_traffic::UsbControlRequestType::Vendor,
                    _ => crate::usb::mitm_traffic::UsbControlRequestType::Reserved,
                };
                
                let recipient = match bm_request_type & 0x1F {
                    0 => crate::usb::mitm_traffic::UsbControlRecipient::Device,
                    1 => crate::usb::mitm_traffic::UsbControlRecipient::Interface,
                    2 => crate::usb::mitm_traffic::UsbControlRecipient::Endpoint,
                    3 => crate::usb::mitm_traffic::UsbControlRecipient::Other,
                    _ => crate::usb::mitm_traffic::UsbControlRecipient::Reserved,
                };
                
                // Determine standard request type for standard requests
                let standard_request = if request_type == crate::usb::mitm_traffic::UsbControlRequestType::Standard {
                    match b_request {
                        0x00 => Some(crate::usb::mitm_traffic::UsbStandardRequest::GetStatus),
                        0x01 => Some(crate::usb::mitm_traffic::UsbStandardRequest::ClearFeature),
                        0x03 => Some(crate::usb::mitm_traffic::UsbStandardRequest::SetFeature),
                        0x05 => Some(crate::usb::mitm_traffic::UsbStandardRequest::SetAddress),
                        0x06 => Some(crate::usb::mitm_traffic::UsbStandardRequest::GetDescriptor),
                        0x07 => Some(crate::usb::mitm_traffic::UsbStandardRequest::SetDescriptor),
                        0x08 => Some(crate::usb::mitm_traffic::UsbStandardRequest::GetConfiguration),
                        0x09 => Some(crate::usb::mitm_traffic::UsbStandardRequest::SetConfiguration),
                        0x0A => Some(crate::usb::mitm_traffic::UsbStandardRequest::GetInterface),
                        0x0B => Some(crate::usb::mitm_traffic::UsbStandardRequest::SetInterface),
                        0x0C => Some(crate::usb::mitm_traffic::UsbStandardRequest::SynchFrame),
                        _ => None,
                    }
                } else {
                    None
                };
                
                // Create description for the request
                let request_description = match standard_request {
                    Some(req) => format!("{}(w_value=0x{:04X}, w_index=0x{:04X}, w_length={})", 
                                         format!("{:?}", req).replace("UsbStandardRequest::", ""),
                                         w_value, w_index, w_length),
                    None => format!("Request: 0x{:02X} (Type: {:?}, Recipient: {:?})",
                                  b_request, request_type, recipient),
                };
                
                // Add the setup packet to the transaction
                transaction.setup_packet = Some(crate::usb::mitm_traffic::UsbSetupPacket {
                    bmRequestType: bm_request_type,
                    bRequest: b_request,
                    wValue: w_value,
                    wIndex: w_index,
                    wLength: w_length,
                    direction,
                    request_type,
                    recipient,
                    standard_request,
                    request_description,
                });
            }
            
            // Add the transaction to our list
            transactions.push(transaction);
            
            // Move to next packet
            offset += 4 + data_len;
        }
        
        // Return the parsed transactions
        transactions
    }
    
    // Get simulated MitM traffic for testing (public implementation)
    pub fn get_simulated_mitm_traffic_pub(&mut self) -> Vec<u8> {
        // Create a realistic simulated USB packet
        // Format: [packet_type, endpoint, device_addr, data_len, data...]
        
        // Generate random packet type (control, bulk, interrupt)
        let packet_type = match rand::random::<u8>() % 3 {
            0 => 0xD0, // SETUP token (control transfer)
            1 => 0x90, // IN token (bulk or interrupt IN)
            _ => 0x10, // OUT token (bulk or interrupt OUT)
        };
        
        // Generate random endpoint (1-15) with direction bit
        let mut endpoint = (rand::random::<u8>() % 15) + 1;
        if packet_type == 0x90 {
            endpoint |= 0x80; // Set direction bit for IN transfers
        }
        
        // Use device address 1 or 2 for simulated devices
        let device_addr = (rand::random::<u8>() % 2) + 1;
        
        // Randomize data size (4-64 bytes)
        let data_len = (rand::random::<u8>() % 60) + 4;
        
        // Create data buffer
        let mut data = Vec::with_capacity(data_len as usize + 4);
        
        // Add header
        data.push(packet_type);
        data.push(endpoint);
        data.push(device_addr);
        data.push(data_len);
        
        // Add simulated USB data
        if packet_type == 0xD0 {
            // For SETUP packets, use standard device request format
            // bmRequestType
            data.push(0x80); // Device-to-host
            // bRequest
            data.push(0x06); // GET_DESCRIPTOR
            // wValue
            data.push(0x01); // Descriptor index
            data.push(0x00); // Descriptor type (device)
            // wIndex
            data.push(0x00);
            data.push(0x00);
            // wLength
            data.push(0x12); // 18 bytes (standard device descriptor length)
            data.push(0x00);
        } else {
            // For other packets, generate random data
            for _ in 0..(data_len as usize) {
                data.push(rand::random::<u8>());
            }
        }
        
        // Return the packet
        data
    }
    
    // Process MitM traffic into transactions
    pub fn process_mitm_traffic(&mut self, data: &[u8]) -> Vec<crate::usb::mitm_traffic::UsbTransaction> {
        use crate::usb::mitm_traffic::{UsbTransaction, UsbTransferType, UsbDirection};
        
        // If data is empty, return empty vector
        if data.is_empty() {
            return Vec::new();
        }
        
        // For our nusb implementation, properly process the data
        let mut transactions = Vec::new();
        
        // Ensure we have enough data for at least one transaction (minimum 8 bytes)
        // Real implementation would use a more robust parsing approach
        if data.len() < 8 {
            return Vec::new();
        }
        
        // Process data into packets first
        let mut offset = 0;
        while offset + 8 <= data.len() {
            // Read packet header (simplified for this implementation)
            let packet_type = data[offset];
            let endpoint = data[offset + 1];
            let direction = if endpoint & 0x80 != 0 { 
                UsbDirection::DeviceToHost 
            } else { 
                UsbDirection::HostToDevice 
            };
            
            // Extract device address
            let device_addr = data[offset + 2];
            
            // Identify transfer type based on endpoint number (simplified)
            let transfer_type = match endpoint & 0x03 {
                0 => UsbTransferType::Control,
                1 => UsbTransferType::Isochronous,
                2 => UsbTransferType::Bulk,
                3 => UsbTransferType::Interrupt,
                _ => UsbTransferType::Control, // Default fallback
            };
            
            // Calculate data length - this would be more complex in real parsing
            let mut data_len = 4; // Default to a minimum length
            if offset + 3 < data.len() {
                data_len = data[offset + 3] as usize;
                // Ensure we don't exceed buffer boundaries
                if offset + 4 + data_len > data.len() {
                    data_len = data.len() - offset - 4;
                }
            }
            
            // Extract data payload
            let payload = if offset + 4 + data_len <= data.len() {
                data[offset+4..offset+4+data_len].to_vec()
            } else {
                Vec::new()
            };
            
            // Generate a unique ID for this transaction
            // In a real implementation, this would come from the device
            let id = transactions.len() as u32 + 1;
            
            // Create a data packet for this transaction
            let data_packet = crate::usb::mitm_traffic::UsbDataPacket::new(
                payload, 
                direction,
                endpoint & 0x7F // Remove direction bit
            );
            
            // Create a status packet (assume success)
            let status_packet = crate::usb::mitm_traffic::UsbStatusPacket {
                status: crate::usb::mitm_traffic::UsbTransferStatus::ACK,
                endpoint: endpoint & 0x7F,
            };
            
            // Create and update the transaction with our data
            let mut transaction = UsbTransaction {
                id: id.into(), // Convert to u64
                transfer_type,
                setup_packet: None, // We'd parse this from the data for control transfers
                data_packet: Some(data_packet),
                status_packet: Some(status_packet),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64(),
                device_address: device_addr,
                endpoint: endpoint & 0x7F, // Remove direction bit
                fields: {
                    let mut fields = std::collections::HashMap::new();
                    fields.insert("speed".to_string(), "High".to_string());
                    fields.insert("packet_type".to_string(), format!("0x{:02X}", packet_type));
                    fields
                },
            };
            
            // If this is a setup packet, create a setup packet structure
            if packet_type == 0xD0 {
                // Add a basic setup packet - in a real implementation we'd parse the data
                // Create a minimal setup packet with just the required fields
                transaction.setup_packet = Some(crate::usb::mitm_traffic::UsbSetupPacket {
                    bmRequestType: 0,  // Default
                    bRequest: 0,       // Default
                    wValue: 0,         // Default
                    wIndex: 0,         // Default
                    wLength: 0,        // Default
                    direction: direction,
                    request_type: crate::usb::mitm_traffic::UsbControlRequestType::Standard,
                    recipient: crate::usb::mitm_traffic::UsbControlRecipient::Device,
                    standard_request: None,
                    request_description: "Unknown Request".to_string(),
                });
            }
            
            // Fields are already added during initialization
            
            transactions.push(transaction);
            
            // Move to next packet
            offset += 4 + data_len;
        }
        
        transactions
    }
}

// Processing stream for converting USB capture data into packets
#[allow(dead_code)]
pub struct CynthionStream {
    receiver: mpsc::Receiver<Vec<u8>>,
    buffer: VecDeque<u8>,
    padding_due: bool,
}

impl CynthionStream {
    // Create a new processing stream
    #[allow(dead_code)]
    pub fn new(receiver: mpsc::Receiver<Vec<u8>>) -> CynthionStream {
        CynthionStream {
            receiver,
            buffer: VecDeque::new(),
            padding_due: false,
        }
    }
    
    // Process captured data into a formatted packet
    #[allow(dead_code)]
    pub fn next_packet(&mut self) -> Option<Vec<u8>> {
        loop {
            // First check if we have a complete packet in the buffer
            if self.buffer.len() >= 4 {
                // Check if we have enough bytes for the packet including its length
                let packet_len = self.peek_packet_len();
                if self.buffer.len() >= packet_len + 4 {
                    // Extract the packet
                    return Some(self.extract_packet(packet_len));
                }
            }
            
            // We don't have a complete packet, try to get more data
            match self.receiver.try_recv() {
                Ok(data) => {
                    // Add new data to our buffer
                    self.buffer.extend(data);
                },
                Err(_) => {
                    // No more data available right now
                    return None;
                }
            }
        }
    }
    
    // Helper to peek at the packet length
    #[allow(dead_code)]
    fn peek_packet_len(&self) -> usize {
        let mut len_bytes = [0u8; 4];
        for (i, &byte) in self.buffer.iter().take(4).enumerate() {
            len_bytes[i] = byte;
        }
        
        // Convert to u32 (little endian)
        u32::from_le_bytes(len_bytes) as usize
    }
    
    // Extract a complete packet from the buffer
    #[allow(dead_code)]
    fn extract_packet(&mut self, packet_len: usize) -> Vec<u8> {
        // Remove the length bytes
        for _ in 0..4 {
            self.buffer.pop_front();
        }
        
        // Now extract the actual packet data
        let mut packet = Vec::with_capacity(packet_len);
        for _ in 0..packet_len {
            packet.push(self.buffer.pop_front().unwrap());
        }
        
        packet
    }
}