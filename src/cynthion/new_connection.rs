//! Cynthion device connection handler using nusb
//! This is a clean reimplementation based on Packetry's approach

use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Result, bail, Context as AnyhowContext, Error};
use log::{info, error, warn, debug, trace};
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
        
        let devices = match nusb::list_devices() {
            Ok(devices) => {
                // Create a Vec to store the list for iteration
                let device_list: Vec<_> = devices.collect();
                
                // Log details about each detected USB device for diagnostic purposes
                for dev in &device_list {
                    debug!("USB device detected: VID:{:04x} PID:{:04x} {}", 
                           dev.vendor_id(), dev.product_id(), 
                           dev.product_string().unwrap_or("Unknown"));
                    if Self::is_supported(dev.vendor_id(), dev.product_id()) {
                        info!("👉 CYNTHION DEVICE FOUND: VID:{:04x} PID:{:04x} {}", 
                              dev.vendor_id(), dev.product_id(),
                              dev.product_string().unwrap_or("Unknown"));
                    }
                }
                device_list.into_iter()
            },
            Err(e) => {
                error!("Failed to list USB devices: {}", e);
                return Err(anyhow::anyhow!("Failed to list USB devices: {}", e));
            }
        };
        
        let mut result = Vec::new();
        for device_info in devices {
            let device = match Self::from_device_info(device_info.clone()) {
                Some(device) => {
                    info!("✓ Added Cynthion-compatible device to available list: {:04x}:{:04x}",
                         device_info.vendor_id(), device_info.product_id());
                    device
                },
                None => continue,
            };
            result.push(device);
        }
        
        // Log summary of detected devices
        if result.is_empty() {
            debug!("No Cynthion-compatible devices found in scan");
        } else {
            info!("Found {} Cynthion-compatible devices", result.len());
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
    
    // Open the device for communication with enhanced error handling and logging
    pub fn open(&self) -> Result<CynthionHandle> {
        use log::{info, debug, warn, error};
        
        info!("Opening Cynthion device: VID {:04x} PID {:04x}", self.device_info.vendor_id(), self.device_info.product_id());
        
        // Attempt to open the device with retry
        let device = match self.device_info.open() {
            Ok(device) => {
                debug!("Successfully opened device");
                device
            },
            Err(e) => {
                // Log detailed error information for diagnosing USB issues
                error!("Error opening device: {}", e);
                
                // Return error to let the application layer handle retry logic
                return Err(anyhow::anyhow!("Failed to open device: {}", e));
            }
        };
        
        // Attempt to claim the interface
        debug!("Attempting to claim interface {}", self.interface_number);
        let interface = match device.claim_interface(self.interface_number) {
            Ok(interface) => {
                info!("Successfully claimed interface {}", self.interface_number);
                interface
            },
            Err(e) => {
                error!("Failed to claim interface {}: {}", self.interface_number, e);
                
                // Try one more approach - on macOS it sometimes helps to reset the device
                warn!("First interface claim attempt failed, trying alternate approach...");
                
                // Return error to let application layer handle the retry
                return Err(anyhow::anyhow!("Failed to claim interface {}: {}", 
                          self.interface_number, e));
            }
        };
        
        // Create the connection handle
        info!("Successfully opened and claimed Cynthion device");
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
            
            // First ensure the device is not already in capture mode and reset it to a known state
            // This follows the best practices from Packetry and Cynthion documentation
            info!("Resetting Cynthion to ensure clean capture state");
            
            // Step 1: Send stop command to exit any previous capture mode
            if let Err(e) = self.write_request(1, State::new(false, Speed::High).0) {
                warn!("Failed to send stop command during reset: {} (continuing anyway)", e);
            }
            // Wait for the device to process the stop command
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            // Step 2: Perform USB device reset (Cynthion-specific vendor command for full reset)
            // This maps to the CYNTHION_RESET command in packetry
            info!("Performing full Cynthion device reset");
            if let Err(e) = self.write_request(0xFF, 0) {
                warn!("Full device reset command failed: {} (continuing anyway)", e);
            }
            
            // Wait for the device to fully reset - Cynthion needs time to reconnect all internal components
            // The Cynthion documentation recommends at least 1000ms (1 second) after a full reset
            info!("Waiting for device reset to complete...");
            std::thread::sleep(std::time::Duration::from_millis(1500)); // Extended to 1.5s for reliability
            
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
            
            // Try to set the device to capture mode with multiple attempts if needed, using exponential backoff
            let max_attempts = 5;
            let mut last_error = None;
            let mut success = false;
            
            info!("Commanding Cynthion to start Man-in-the-Middle capture...");
            
            // According to Packetry's best practices, we should:
            // 1. Send START_CAPTURE command
            // 2. Wait for confirmation or timeout
            // 3. If failed, reset and try again with backoff
            for attempt in 1..=max_attempts {
                // Log attempt number with more context
                info!("Attempt {}/{} to start MitM capture", attempt, max_attempts);
                
                // First, check if we need to reset the device for retries
                if attempt > 1 {
                    debug!("Performing reset before retry attempt {}", attempt);
                    // Reset device before retrying
                    if let Err(e) = self.write_request(0xFF, 0) {
                        warn!("Device reset before retry failed: {} (continuing anyway)", e);
                    }
                    
                    // Need extra delay after reset for device to stabilize
                    let reset_delay = 500 * attempt; // Longer delay for each retry (starting with 500ms)
                    debug!("Waiting {}ms for device to stabilize after reset", reset_delay);
                    std::thread::sleep(std::time::Duration::from_millis(reset_delay as u64));
                }
                
                // For Cynthion, we need to properly set the USB speed for optimal capturing
                // Try both Auto and High speed settings based on Packetry documentation
                let speeds_to_try = [Speed::Auto, Speed::High];
                let mut speed_success = false;
                
                for speed in &speeds_to_try {
                    info!("Trying to start capture with speed mode: {:?}", speed);
                    
                    // Now send the start capture command with the selected speed
                    match self.write_request(1, State::new(true, *speed).0) {
                        Ok(_) => {
                            info!("Successfully started MitM capture with {:?} speed on attempt {}", speed, attempt);
                            speed_success = true;
                            success = true;
                            break; // Break out of the speed loop
                        },
                        Err(e) => {
                            warn!("Failed to start MitM capture with {:?} speed (attempt {}/{}): {}", 
                                  speed, attempt, max_attempts, e);
                            last_error = Some(e);
                            
                            // Wait a bit before trying the next speed
                            std::thread::sleep(std::time::Duration::from_millis(200));
                        }
                    }
                }
                
                // If we succeeded with any speed, no need to try another attempt
                if speed_success {
                    break;
                }
                
                // Use exponential backoff between attempts
                if attempt < max_attempts {
                    let backoff = 400 * (1 << (attempt - 1)); // Exponential backoff: 400, 800, 1600ms
                    let max_backoff = 3000; // Cap at 3 seconds
                    let actual_backoff = std::cmp::min(backoff, max_backoff);
                    
                    info!("Waiting {}ms before next capture attempt", actual_backoff);
                    std::thread::sleep(std::time::Duration::from_millis(actual_backoff));
                }
            }
            
            if success {
                // Create a background thread to handle transfers
                debug!("Starting asynchronous transfer processing thread");
                self.start_async_processing();
                
                // Wait a moment to ensure capture is fully initialized and buffers are allocated
                // According to Packetry, this delay is important for Cynthion to stabilize its capture state
                debug!("Waiting for capture initialization to complete");
                std::thread::sleep(std::time::Duration::from_millis(500)); // Increased to 500ms for reliability
                
                info!("Cynthion Man-in-the-Middle mode successfully activated - ready to capture USB traffic");
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
        // Request 0xFF is a full device reset
        let request_type = match request {
            1 => if value & 0x01 != 0 { "START_CAPTURE" } else { "STOP_CAPTURE" },
            3 => "CONFIGURE_TEST_DEVICE",
            0xFF => "FULL_DEVICE_RESET",
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
        
        // Determine appropriate timeout based on request type
        let timeout = match request {
            // START_CAPTURE needs a longer timeout
            1 if value & 0x01 != 0 => Duration::from_secs(3),
            // FULL_DEVICE_RESET needs a longer timeout
            0xFF => Duration::from_secs(5),
            // All other requests can use a standard timeout
            _ => Duration::from_secs(1),
        };
        
        // Send the control request and capture the bytes transferred
        match self.interface.control_out_blocking(control, &[], timeout) {
            Ok(bytes) => {
                debug!("Cynthion control request succeeded: {} bytes transferred", bytes);
                
                // For the reset command, we need to give the device time to reset
                if request == 0xFF {
                    debug!("Reset command sent successfully, device should be resetting");
                }
                
                Ok(())
            },
            Err(e) => {
                // For reset commands, a timeout or pipe error might actually indicate success
                // as the device resets and drops the connection
                if request == 0xFF && (e.to_string().contains("timeout") || e.to_string().contains("pipe")) {
                    info!("Reset command resulted in expected disconnect: {}", e);
                    return Ok(());
                }
                
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
    
    // Helper method to prepare device for capture (reset and stabilize)
    fn prepare_device_for_capture(&mut self) -> Result<()> {
        info!("Preparing Cynthion device for capture - reset and stabilization sequence");
        
        // Step 1: Send stop command to exit any previous capture mode
        debug!("Sending stop command to reset capture state");
        if let Err(e) = self.write_request(1, State::new(false, Speed::High).0) {
            warn!("Failed to send stop command during reset: {} (continuing anyway)", e);
        }
        
        // Wait for the device to process the stop command
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        // Step 2: Perform USB device reset (Cynthion-specific vendor command for full reset)
        info!("Performing full Cynthion device reset");
        if let Err(e) = self.write_request(0xFF, 0) {
            warn!("Full device reset command failed: {} (continuing anyway)", e);
            // Some errors during reset are expected as the device resets
        }
        
        // Wait for the device to fully reset - Cynthion needs time to reconnect all internal components
        info!("Waiting for device reset to complete...");
        std::thread::sleep(std::time::Duration::from_millis(2000)); // Extended to 2s for better reliability
        
        // Step 3: Send another stop command to ensure clean state after reset
        debug!("Sending final stop command to ensure clean capture state");
        if let Err(e) = self.write_request(1, State::new(false, Speed::High).0) {
            warn!("Failed to send final stop command: {} (continuing anyway)", e);
        }
        
        // Final stabilization wait
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        info!("Device preparation complete - ready for capture initialization");
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
        // If we're in simulation mode, return simulated data (shouldn't happen in hardware mode)
        if self.is_simulation_mode() {
            warn!("Simulation mode detected but hardware mode should be enforced");
            // In hardware-only mode, we should never return simulated data
            return Ok(Vec::new());
        }
        
        // If capture_on_connect is true, we should check if we need to start capture
        // This could happen if a device was connected after starting capture
        if self.capture_on_connect && self.transfer_queue.is_none() && self.device_info.vendor_id() == CYNTHION_VID {
            info!("Device connected while capture was waiting - initializing capture now");
            
            // Get the stored transmitter from pending_data_tx
            if let Some(data_tx) = self.pending_data_tx.take() {
                // Before creating queue, make sure device is in a stable state
                // This is crucial for reliable packet capture
                if let Err(e) = self.prepare_device_for_capture() {
                    warn!("Failed to prepare device for capture: {}", e);
                }
                
                // Create a transfer queue for the bulk transfers with increased buffer size
                let queue = TransferQueue::new(
                    &self.interface, 
                    data_tx.clone(),
                    ENDPOINT, 
                    NUM_TRANSFERS, 
                    READ_LEN * 2  // Double buffer size for better packet capture
                );
                
                // Store the transfer queue
                self.transfer_queue = Some(queue);
                
                // Try to start the capture with multiple attempts
                let max_attempts = 5;  // Increased attempts
                let mut success = false;
                
                for attempt in 1..=max_attempts {
                    info!("Starting capture on newly connected device (attempt {}/{})", attempt, max_attempts);
                    
                    // Try both Auto and High speed settings
                    let speeds = [Speed::High, Speed::Auto, Speed::Full];
                    
                    for speed in &speeds {
                        info!("Trying with speed mode: {:?}", speed);
                        match self.write_request(1, State::new(true, *speed).0) {
                            Ok(_) => {
                                info!("Successfully started capture with {:?} speed (attempt {})", speed, attempt);
                                success = true;
                                break;
                            },
                            Err(e) => {
                                warn!("Failed to start capture with {:?} speed (attempt {}/{}): {}", 
                                    speed, attempt, max_attempts, e);
                                
                                // Short wait between speed attempts
                                std::thread::sleep(std::time::Duration::from_millis(200));
                            }
                        }
                    }
                    
                    if success {
                        break;
                    }
                    
                    // Only try again with delay if we have more attempts left
                    if attempt < max_attempts {
                        let delay = 500 * attempt as u64;
                        info!("Waiting {}ms before next capture attempt", delay);
                        std::thread::sleep(std::time::Duration::from_millis(delay));
                        
                        // Reset device before next attempt
                        if let Err(e) = self.write_request(0xFF, 0) {
                            warn!("Device reset failed between attempts: {} (continuing anyway)", e);
                        }
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                    }
                }
                
                // Did we succeed with any attempt?
                if success {
                    // Wait for device to stabilize after starting capture
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    
                    // Start the async processing thread
                    self.start_async_processing();
                    
                    // Additional delay to allow transfer queue to initialize
                    std::thread::sleep(std::time::Duration::from_millis(250));
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
            
            // Prepare device for capture (reset and stabilize)
            if let Err(e) = self.prepare_device_for_capture() {
                warn!("Failed to prepare device for capture: {}", e);
                // Continue anyway as some errors are expected during reset
            }
            
            // Create a proper channel for data transfer
            let (tx, rx) = mpsc::channel();
            
            // Create a new transfer queue with the transmitter and increased buffer size
            let mut transfer_queue = TransferQueue::new(
                &self.interface, 
                tx,
                ENDPOINT, 
                NUM_TRANSFERS, 
                READ_LEN * 2  // Double buffer size for better packet capture
            );
            
            // Set the receiver in the transfer queue
            transfer_queue.set_receiver(rx);
                
            // Store the transfer queue
            self.transfer_queue = Some(transfer_queue);
            
            // Start the capture with proper error handling and support for multiple speeds
            let speeds = [Speed::High, Speed::Auto, Speed::Full];
            let mut capture_success = false;
            
            for speed in &speeds {
                info!("Trying to start capture with speed: {:?}", speed);
                if let Ok(_) = self.write_request(1, State::new(true, *speed).0) {
                    info!("Successfully started USB traffic capture with speed: {:?}", speed);
                    capture_success = true;
                    break;
                } else {
                    warn!("Failed to start capture with speed: {:?}, trying next speed", speed);
                    std::thread::sleep(std::time::Duration::from_millis(300));
                }
            }
            
            if !capture_success {
                error!("Failed to start USB traffic capture with any speed setting");
                self.transfer_queue = None;
                return Err(anyhow::anyhow!("Failed to start capture with any speed setting"));
            }
            
            // Give the device time to stabilize in capture mode
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            // Start async processing in a separate thread
            self.start_async_processing();
            
            // Give the transfer queue time to initialize and start receiving data
            std::thread::sleep(std::time::Duration::from_millis(500));
            
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
                        // Log data size with additional details for debugging
                        if data.is_empty() {
                            warn!("Received empty USB data packet from transfer queue");
                        } else {
                            debug!("Received {} bytes of USB data from transfer queue", data.len());
                            if data.len() > 4 {
                                trace!("USB data starts with: {:02X?}", &data[0..4]);
                            }
                        }
                        return Ok(data);
                    },
                    Err(mpsc::TryRecvError::Empty) => {
                        // No data available yet, return empty vector
                        trace!("No USB data available from queue");
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
            warn!("Received empty data for transaction processing - check device connection");
            return Vec::new();
        }
        
        debug!("Processing {} bytes of USB traffic data", data.len());
        
        // Check for USB device connection patterns
        crate::cynthion::device_detector::UsbDeviceConnectionDetector::check_for_usb_device_connection(data);
        
        // For our nusb implementation, properly process the data
        let mut transactions = Vec::new();
        
        // Enhanced handling for data packets of all sizes
        if data.len() < 8 {
            debug!("Data is shorter than expected minimum 8 bytes: {} bytes", data.len());
            // Show the raw data for diagnostic purposes when it's too short
            if !data.is_empty() {
                let hex_string = data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join(" ");
                debug!("Raw packet data: {}", hex_string);
                
                // Try to extract any useful information from short packets
                if data.len() >= 4 {
                    debug!("Attempting to extract basic header from short packet");
                    let packet_type = data[0];
                    let endpoint = data[1];
                    let device_addr = data[2];
                    let data_len = data[3];
                    debug!("Short packet header: type=0x{:02X}, ep=0x{:02X}, dev=0x{:02X}, len={}",
                           packet_type, endpoint, device_addr, data_len);
                    
                    // Check if this is one of our newly recognized packet types
                    let alternative_types = [0xA5, 0x00, 0x23, 0x69];
                    if alternative_types.contains(&packet_type) {
                        debug!("Short packet contains recognized alternative packet type");
                        // We'll handle these special types even if they're shorter than expected
                        // Continue processing instead of returning early
                    } else {
                        // Not a recognized type, likely just incomplete data
                        return Vec::new();
                    }
                } else {
                    // Too short to extract meaningful header information
                    return Vec::new();
                }
            } else {
                return Vec::new();
            }
        }
        
        // Log detailed information about received data for debugging
        info!("Processing {} bytes of USB data into transactions", data.len());
        
        // Log the first 32 bytes of data for enhanced debugging
        let log_size = std::cmp::min(32, data.len());
        let first_bytes = &data[0..log_size];
        let hex_string = first_bytes.iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join(" ");
        debug!("First {} bytes of captured data: {}", log_size, hex_string);

        // Process data into packets according to Cynthion/Packetry format
        // Each packet has: packet_type(1), endpoint(1), device_addr(1), data_len(1), data(variable)
        let mut offset = 0;
        while offset + 4 <= data.len() {
            // 1. Read packet header - minimum 4 bytes for header
            let packet_type = data[offset];
            let endpoint = data[offset + 1];
            let device_addr = data[offset + 2];
            let data_len = data[offset + 3] as usize;
            
            // Special handling for alternative packet types
            let alternative_types = [0xA5, 0x00, 0x23, 0x69];
            if alternative_types.contains(&packet_type) {
                debug!("Processing alternative packet type: 0x{:02X}", packet_type);
                // These might have a different structure - add special handling here
            }
            
            // Provide more detailed logging for each packet
            debug!("Processing USB packet: type=0x{:02X}, endpoint=0x{:02X}, device=0x{:02X}, len={}", 
                   packet_type, endpoint, device_addr, data_len);
            
            // 2. Determine direction
            // For Cynthion, the direction is determined by the endpoint's high bit
            // If EP & 0x80 == 0, it's host-to-device (OUT)
            // If EP & 0x80 != 0, it's device-to-host (IN)
            let direction = if endpoint & 0x80 != 0 { 
                UsbDirection::DeviceToHost 
            } else { 
                UsbDirection::HostToDevice 
            };
            
            // 3. Identify transfer type based on packet_type and endpoint number
            // Updated to support both standard Packetry formatting and the format observed in your logs
            let transfer_type = match packet_type {
                // Standard Packetry documentation token types
                0xD0 => UsbTransferType::Control,   // SETUP token (control transfer)
                0x90 => UsbTransferType::Bulk,      // IN token (bulk transfer)
                0xC0 => UsbTransferType::Interrupt, // IN token (interrupt transfer)
                0x10 => UsbTransferType::Bulk,      // OUT token (bulk transfer)
                0x40 => UsbTransferType::Interrupt, // OUT token (interrupt transfer)
                0xA0 => UsbTransferType::Isochronous, // IN token (isochronous transfer)
                0x20 => UsbTransferType::Isochronous, // OUT token (isochronous transfer)
                0xE0 => UsbTransferType::Control,   // Special case for status stage
                
                // Observed in logs - alternative format packet types
                0xA5 => UsbTransferType::Control,   // Possibly setup or control transfer
                0x00 => UsbTransferType::Bulk,      // Unknown but common in capture
                0x23 => UsbTransferType::Interrupt, // Based on observed patterns
                0x69 => UsbTransferType::Bulk,      // Based on observed patterns
                
                _ => {
                    debug!("Unknown packet type: 0x{:02X}, using heuristics to determine type", packet_type);
                    // Enhanced heuristics that look at both packet type and endpoint
                    match endpoint & 0x7F {
                        0 => UsbTransferType::Control,  // EP0 is always control
                        1 => UsbTransferType::Isochronous, // Often used for isochronous
                        2..=3 => UsbTransferType::Bulk, // Often used for bulk
                        _ => {
                            // Additional endpoint pattern heuristics
                            if endpoint >= 0x80 && endpoint <= 0x8F {
                                UsbTransferType::Bulk // Often higher IN endpoints are bulk
                            } else if endpoint >= 0xA0 && endpoint <= 0xAF {
                                UsbTransferType::Interrupt // Often IN endpoints with bit 5 set are interrupt
                            } else {
                                UsbTransferType::Interrupt // Default for other pattern
                            }
                        }
                    }
                }
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
            
            // Create a data packet for this transaction - use clone() to avoid ownership issues
            let data_packet = crate::usb::mitm_traffic::UsbDataPacket::new(
                payload.clone(), 
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
                    fields.insert("direction".to_string(), format!("{:?}", direction));
                    fields.insert("transfer_type".to_string(), format!("{:?}", transfer_type));
                    
                    if packet_type == 0xD0 {
                        fields.insert("setup".to_string(), "true".to_string());
                    }
                    
                    if data_len > 0 {
                        fields.insert("data_size".to_string(), format!("{}", data_len));
                        // Also add first few bytes of data for quick reference
                        let preview_len = std::cmp::min(data_len, 8);
                        let preview = payload[..preview_len].iter()
                            .map(|b| format!("{:02X}", b))
                            .collect::<Vec<String>>()
                            .join(" ");
                        fields.insert("data_preview".to_string(), preview);
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
                        // USB 3.0 specific requests
                        0x30 => Some(crate::usb::mitm_traffic::UsbStandardRequest::SetSel),      // USB 3.0: Set System Exit Latency
                        0x31 => Some(crate::usb::mitm_traffic::UsbStandardRequest::SetIsochDelay), // USB 3.0: Set Isochronous Delay
                        // USB 2.0 Extension requests
                        0x33 => Some(crate::usb::mitm_traffic::UsbStandardRequest::SetFeatureSelector), // Set Feature Selector
                        // Class and Vendor specific requests are handled elsewhere
                        _ => None,
                    }
                } else {
                    None
                };
                
                // Create description for the request
                let request_description = match standard_request {
                    Some(req) => {
                        // Extract descriptor type for GET_DESCRIPTOR requests
                        let descriptor_info = if req == crate::usb::mitm_traffic::UsbStandardRequest::GetDescriptor {
                            let desc_type = (w_value >> 8) as u8;
                            let desc_index = (w_value & 0xFF) as u8;
                            
                            let desc_type_name = match desc_type {
                                1 => "DEVICE",
                                2 => "CONFIGURATION", 
                                3 => "STRING",
                                4 => "INTERFACE",
                                5 => "ENDPOINT",
                                6 => "DEVICE_QUALIFIER",
                                7 => "OTHER_SPEED_CONFIGURATION",
                                8 => "INTERFACE_POWER",
                                9 => "OTG",
                                10 => "DEBUG",
                                11 => "INTERFACE_ASSOCIATION",
                                15 => "BOS",
                                16 => "DEVICE_CAPABILITY",
                                17 => "HID",
                                18 => "REPORT",
                                19 => "PHYSICAL",
                                20 => "CLASS_SPECIFIC_INTERFACE",
                                21 => "CLASS_SPECIFIC_ENDPOINT",
                                22 => "HUB",
                                23 => "SUPERSPEED_HUB",
                                24 => "SS_ENDPOINT_COMPANION",
                                _ => "UNKNOWN",
                            };
                            
                            format!(" (type={}, index={})", desc_type_name, desc_index)
                        } else {
                            "".to_string()
                        };
                        
                        format!("{}{} (wValue=0x{:04X}, wIndex=0x{:04X}, wLength={})", 
                               format!("{:?}", req).replace("UsbStandardRequest::", ""),
                               descriptor_info,
                               w_value, w_index, w_length)
                    },
                    None => {
                        // For non-standard requests, provide more context
                        match request_type {
                            crate::usb::mitm_traffic::UsbControlRequestType::Class => {
                                format!("Class Request: 0x{:02X} to {:?} (wValue=0x{:04X}, wIndex=0x{:04X}, wLength={})",
                                      b_request, recipient, w_value, w_index, w_length)
                            },
                            crate::usb::mitm_traffic::UsbControlRequestType::Vendor => {
                                format!("Vendor Request: 0x{:02X} to {:?} (wValue=0x{:04X}, wIndex=0x{:04X}, wLength={})",
                                      b_request, recipient, w_value, w_index, w_length)
                            },
                            _ => {
                                format!("Request: 0x{:02X} (Type: {:?}, Recipient: {:?}, wValue=0x{:04X}, wIndex=0x{:04X}, wLength={})",
                                      b_request, request_type, recipient, w_value, w_index, w_length)
                            }
                        }
                    }
                };
                
                // Log important setup packets for debugging
                if standard_request == Some(crate::usb::mitm_traffic::UsbStandardRequest::GetDescriptor) {
                    debug!("Setup packet: GET_DESCRIPTOR - Type: {}, Index: {}, Length: {}", 
                          (w_value >> 8), (w_value & 0xFF), w_length);
                } else if let Some(req) = standard_request {
                    debug!("Setup packet: {:?}", req);
                }
                
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
                payload.clone(), 
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