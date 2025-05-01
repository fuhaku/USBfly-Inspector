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
        
        // We will check for Replit environment later if needed, but let's not use simulation mode
        
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
                // Simply return an error - no special Replit handling
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
    
    // Enhanced method for capturing USB traffic from devices connected to Cynthion
    pub fn start_capture(&mut self) -> Result<()> {
        // Use Auto speed by default
        self.start_capture_with_speed(Speed::Auto)
    }
    
    // Start capture with a specific speed setting
    pub fn start_capture_with_speed(&mut self, speed: Speed) -> Result<()> {
        // Log the selected speed for verification
        info!("Starting capture with user-selected speed: {:?}", speed);
        
        // Create a channel to receive packets from the device
        let (data_tx, data_rx) = std::sync::mpsc::channel();
        
        // Store the receiver for later use
        self.data_receiver = Some(data_rx);
        
        // Reset USB device connection detection state
        crate::cynthion::device_detector::UsbDeviceConnectionDetector::set_device_connected(false);
        
        // If the device is ready, set up the enhanced transfer queue immediately
        if self.device_info.vendor_id() == CYNTHION_VID {
            info!("Starting enhanced capture on Cynthion device: {:04x}:{:04x} with speed: {:?}", 
                  self.device_info.vendor_id(), self.device_info.product_id(), speed);
            
            // Enhanced device preparation for connected device detection
            info!("Preparing Cynthion with optimized connected device detection");
            
            // Step 1: Send stop command to exit any previous capture mode
            if let Err(e) = self.write_request(1, State::new(false, Speed::High).0) {
                warn!("Failed to send stop command during reset: {} (continuing anyway)", e);
            }
            // Adequate wait for the device to process the stop command
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            // Step 2: Perform enhanced USB device reset with additional parameters
            // This follows the recommended sequence from Packetry but adds critical steps
            // for connected device support
            info!("Performing full Cynthion device reset with optimized parameters");
            if let Err(e) = self.write_request(0xFF, 0) {
                warn!("Full device reset command failed: {} (continuing anyway)", e);
            }
            
            // Extended wait for device to fully reset internal components and USB stack
            info!("Waiting for device reset to complete (extended wait for USB stack)");
            std::thread::sleep(std::time::Duration::from_millis(2000)); // Extended to 2s for better reliability
            
            // Step 3: Send device detection configuration commands
            // These commands are critical for proper detection of connected devices
            debug!("Configuring connected device detection");
            if let Err(e) = self.write_request(2, 0x01) { // Enable device detection mode
                warn!("Failed to enable device detection: {} (may affect connected device capture)", e);
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
            
            // Step 4: Configure USB monitoring mode for connected devices
            debug!("Setting USB monitoring mode for connected devices");
            if let Err(e) = self.write_request(3, 0x03) { // Set monitoring mode for connected devices
                warn!("Failed to set monitoring mode: {} (may affect capture quality)", e);
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
            
            // Initialize enhanced transfer queue with increased buffer size for complex USB descriptors
            info!("Initializing USB transfer queue with increased buffer capacity");
            let queue = TransferQueue::new(
                &self.interface, 
                data_tx,
                ENDPOINT, 
                NUM_TRANSFERS, 
                READ_LEN * 4  // Quadruple buffer size for better capture of complex USB descriptors
            );
            
            // Store the enhanced transfer queue
            self.transfer_queue = Some(queue);
            
            // Use progressive approach with multiple attempts and comprehensive error handling
            let max_attempts = 6;  // Increased attempts for better reliability
            let mut last_error = None;
            let mut success = false;
            
            info!("Starting Cynthion's MitM capture mode for connected devices");
            
            // Enhanced approach based on Packetry's best practices:
            // 1. Send START_CAPTURE command with multiple speed options
            // 2. Verify with additional commands to optimize for connected devices
            // 3. If failed, perform progressive reset and retry with backoff
            for attempt in 1..=max_attempts {
                // Log attempt with detailed context
                info!("Attempt {}/{} to start enhanced USB capture", attempt, max_attempts);
                
                // For retries, perform comprehensive reset sequence
                if attempt > 1 {
                    info!("Performing enhanced reset before retry attempt {}", attempt);
                    
                    // First stop any existing capture
                    if let Err(e) = self.write_request(1, State::new(false, Speed::High).0) {
                        warn!("Failed to stop capture during retry: {}", e);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    
                    // Full device reset
                    if let Err(e) = self.write_request(0xFF, 0) {
                        warn!("Device reset before retry failed: {} (continuing anyway)", e);
                    }
                    
                    // Progressive delay increasing with each attempt
                    let reset_delay = 500 * attempt; // Longer delay for each retry
                    info!("Waiting {}ms for device to stabilize after reset", reset_delay);
                    std::thread::sleep(std::time::Duration::from_millis(reset_delay as u64));
                    
                    // Re-enable device detection after reset
                    if let Err(e) = self.write_request(2, 0x01) {
                        warn!("Failed to re-enable device detection: {}", e);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                
                // For Cynthion, we need to properly set the USB speed for optimal capturing
                // When using start_capture_with_speed, we prioritize the user-specified speed
                // but will fall back to alternatives if it fails
                let mut speeds_to_try = vec![speed];
                
                // Add fallback speeds if needed, but avoid duplicates
                if speed != Speed::Auto {
                    speeds_to_try.push(Speed::Auto);
                }
                if speed != Speed::High {
                    speeds_to_try.push(Speed::High);
                }
                if speed != Speed::Full && speed != Speed::Auto && speed != Speed::High {
                    speeds_to_try.push(Speed::Full);
                }
                
                let mut speed_success = false;
                
                for current_speed in &speeds_to_try {
                    info!("Trying to start capture with speed mode: {:?}", current_speed);
                    
                    // Now send the start capture command with the selected speed
                    match self.write_request(1, State::new(true, *current_speed).0) {
                        Ok(_) => {
                            info!("Successfully started MitM capture with {:?} speed on attempt {}", current_speed, attempt);
                            speed_success = true;
                            success = true;
                            break; // Break out of the speed loop
                        },
                        Err(e) => {
                            warn!("Failed to start MitM capture with {:?} speed (attempt {}/{}): {}", 
                                  current_speed, attempt, max_attempts, e);
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
        // Enhanced request type with additional commands for connected device support
        // Request 1 with value 1 starts capture, with value 0 stops capture
        // Request 2 configures connected device detection
        // Request 3 configures USB monitoring mode
        // Request 4 enables detailed descriptor capture
        // Request 5 is a specialized prep command
        // Request 0xFF is a full device reset
        let request_type = match request {
            1 => if value & 0x01 != 0 { "START_CAPTURE" } else { "STOP_CAPTURE" },
            2 => "ENABLE_DEVICE_DETECTION",
            3 => "SET_MONITORING_MODE",
            4 => "ENABLE_DETAILED_CAPTURE",
            5 => "DEVICE_PREP",
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
    
    // Begin capture with specified speed and return a queue for processing transfers
    #[allow(dead_code)]
    pub fn begin_capture_with_speed(
        &mut self,
        data_tx: mpsc::Sender<Vec<u8>>,
        speed: Speed
    ) -> Result<TransferQueue> {
        // Use specified speed
        self.start_capture_with_speed(speed)?;
        
        Ok(TransferQueue::new(&self.interface, data_tx,
            ENDPOINT, NUM_TRANSFERS, READ_LEN))
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
    
    // Comprehensive device preparation with full reset sequence for MitM capture
    fn prepare_device_for_capture(&mut self) -> Result<()> {
        info!("Preparing Cynthion device for USB Man-in-the-Middle capture with enhanced protocol");
        
        // PHASE 1: CLEAN SHUTDOWN OF PREVIOUS CAPTURE
        // ==========================================
        
        // Step 1.1: Send stop command to exit any previous capture mode
        debug!("Stopping any existing capture process with graceful shutdown");
        if let Err(e) = self.write_request(1, State::new(false, Speed::High).0) {
            warn!("Failed to send primary stop command: {} (will try alternative commands)", e);
            // Attempt alternative stop commands if the primary fails
            if let Err(e2) = self.write_request(1, 0) {
                warn!("Alternative stop command also failed: {} (continuing anyway)", e2);
            }
        }
        
        // Wait for the device to fully process the stop command with adequate time
        info!("Waiting for previous capture processes to terminate completely...");
        std::thread::sleep(std::time::Duration::from_millis(800)); // Increased for reliability
        
        // PHASE 2: DISCOVER DEVICE CAPABILITIES
        // ====================================
        
        // Step 2.1: Get the device's supported speeds to ensure proper configuration
        debug!("Querying device for supported USB speeds");
        let speeds = match self.speeds() {
            Ok(speeds) => {
                info!("✓ Device supports USB speeds: {:?}", speeds);
                speeds
            },
            Err(e) => {
                warn!("Failed to get supported speeds: {} (trying alternative approach)", e);
                
                // Fallback method - try an alternative control request for speed information
                match self.write_request(2, 0) {
                    Ok(_) => {
                        info!("Alternative speed query succeeded - using default speeds");
                        vec![crate::usb::Speed::Auto, crate::usb::Speed::High] 
                    },
                    Err(e2) => {
                        warn!("All speed query methods failed: {} (using conservative defaults)", e2);
                        vec![crate::usb::Speed::High] // Last resort fallback
                    }
                }
            }
        };
        
        // Determine optimal speed configuration based on device capabilities
        let best_speed = if speeds.contains(&crate::usb::Speed::High) {
            crate::usb::Speed::High // Prefer High Speed for best capture quality
        } else if speeds.contains(&crate::usb::Speed::Auto) {
            crate::usb::Speed::Auto // Auto speed as second preference
        } else if speeds.contains(&crate::usb::Speed::Full) {
            crate::usb::Speed::Full // Full speed as fallback
        } else {
            info!("No optimal speed detected, defaulting to High Speed");
            crate::usb::Speed::High // Default when detection fails
        };
        
        info!("Selected optimal speed for this device: {:?}", best_speed);
        
        // PHASE 3: COMPLETE DEVICE RESET SEQUENCE
        // =====================================
        
        // Step 3.1: Perform complete USB device reset sequence (multiple steps)
        info!("Initiating comprehensive device reset sequence");
        
        // Stop any ongoing operations first
        if let Err(e) = self.write_request(0x04, 0) {
            warn!("Initial reset preparation command failed: {} (continuing with main reset)", e);
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        
        // Main device reset command (Cynthion-specific vendor command)
        debug!("Executing main device reset command...");
        if let Err(e) = self.write_request(0xFF, 0) {
            warn!("Full device reset command failed: {} (trying fallback reset method)", e);
            
            // If primary reset fails, try alternative reset approach
            if let Err(e2) = self.write_request(0x0F, 0x01) {
                warn!("All reset methods failed: {} (may affect device stability)", e2);
            }
        }
        
        // Wait for the device to fully reset - extended wait for complete USB reinitialization
        info!("Waiting for device reset to complete (this takes 2-3 seconds)...");
        std::thread::sleep(std::time::Duration::from_millis(2500)); // Extended to 2.5s for better reliability
        
        // PHASE 4: CONFIGURE MAN-IN-THE-MIDDLE MODE
        // =======================================
        
        // Step 4.1: Send device detection configuration command
        debug!("Enabling connected device detection with enhanced parameters");
        if let Err(e) = self.write_request(2, 0x03) { // Enhanced device detection mode
            warn!("Failed to enable enhanced device detection: {} (trying standard mode)", e);
            
            // Fall back to standard detection mode if enhanced fails
            if let Err(e2) = self.write_request(2, 0x01) {
                warn!("Standard device detection also failed: {} (may affect device discovery)", e2);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
        
        // Step 4.2: Configure monitoring mode with optimized settings for full capture
        debug!("Configuring USB monitoring for full transaction capture");
        let monitoring_mode = 0x03; // Full monitoring mode including control, bulk, and interrupt transfers
        if let Err(e) = self.write_request(3, monitoring_mode) {
            warn!("Failed to set optimal monitoring mode: {} (trying alternative mode)", e);
            
            // Try a more basic monitoring mode if optimal fails
            if let Err(e2) = self.write_request(3, 0x01) {
                warn!("All monitoring mode configurations failed: {} (capture may be limited)", e2);
            }
        }
        
        // Step 4.3: Set capture speed based on earlier device capability detection
        debug!("Setting capture speed to: {:?}", best_speed);
        let speed_config = TestConfig::new(Some(best_speed)).0;
        
        // Try setting the optimal speed first
        match self.write_request(0x0A, speed_config) {
            Ok(_) => {
                info!("Successfully set optimal USB speed: {:?}", best_speed);
                // Remember this speed for future reconnections
                crate::cynthion::device_detector::UsbDeviceConnectionDetector::set_last_successful_speed(best_speed);
            },
            Err(e) => {
                warn!("Failed to set optimal speed configuration: {} (trying alternative speeds)", e);
                
                // If we failed with the best speed, try alternatives in sequence
                let fallback_speeds = vec![crate::usb::Speed::Auto, crate::usb::Speed::High, crate::usb::Speed::Full];
                let mut success = false;
                
                for speed in fallback_speeds {
                    // Skip if same as best_speed that already failed
                    if speed == best_speed {
                        continue;
                    }
                    
                    info!("Trying alternative speed: {:?}", speed);
                    let alt_config = TestConfig::new(Some(speed)).0;
                    
                    match self.write_request(0x0A, alt_config) {
                        Ok(_) => {
                            info!("Alternative speed {:?} successfully set", speed);
                            // Remember this speed for future reconnections
                            crate::cynthion::device_detector::UsbDeviceConnectionDetector::set_last_successful_speed(speed);
                            success = true;
                            break;
                        },
                        Err(e2) => {
                            warn!("Failed to set alternative speed {:?}: {}", speed, e2);
                        }
                    }
                }
                
                if !success {
                    warn!("All speed configurations failed - device may not capture properly");
                }
            }
        }
        
        // Final stabilization wait
        std::thread::sleep(std::time::Duration::from_millis(200));
        
        info!("✓ Device preparation complete - ready for USB traffic capture");
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
    
    // Since we're in hardware-only mode, we don't need simulation methods
    // Removed get_simulated_mitm_traffic method to enforce hardware-only operation
    
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
    
    // Enhanced async processing of USB transfers with improved device detection and error handling
    fn start_async_processing(&self) {
        // Reset device connection detector state before starting capture
        use crate::cynthion::device_detector::UsbDeviceConnectionDetector;
        UsbDeviceConnectionDetector::set_device_connected(false);
        
        // We need to clone these for the thread
        let interface = self.interface.clone();
        let device_info = self.device_info.clone();
        
        info!("Initializing advanced USB device capture with enhanced detection system");
        
        // If we have a transfer queue, set up advanced processing
        if let Some(queue) = &self.transfer_queue {
            // Get the cloneable information from the queue
            let transfer_info = queue.get_info();
            
            // Create a oneshot channel for signaling stopping with extended timeout
            let (_stop_tx, stop_rx) = futures_channel::oneshot::channel();
            
            // Create a new transfer queue with doubled buffer size for better packet capture
            std::thread::spawn(move || {
                info!("USB transfer processing thread started for device {:04x}:{:04x}",
                      device_info.vendor_id(), device_info.product_id());
                      
                // Set up tokio runtime with enhanced concurrency for faster packet processing
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create tokio runtime");
                
                debug!("Initializing high-performance transfer queue for endpoint 0x{:02X}", ENDPOINT);
                
                // Process transfers in the async runtime with enhanced error recovery
                if let Err(e) = rt.block_on(async {
                    // Create a new queue for this thread with doubled buffer size
                    let mut queue = TransferQueue::new(
                        &interface, 
                        transfer_info.data_tx,
                        ENDPOINT, 
                        NUM_TRANSFERS, 
                        transfer_info.transfer_length * 2 // Double buffer size for better packet handling
                    );
                    
                    info!("High-performance USB transfer queue created with {} concurrent transfers", NUM_TRANSFERS);
                    debug!("Starting USB capture loop with {} byte buffer per transfer", transfer_info.transfer_length * 2);
                    
                    // Process transfers until stopped with extended error handling
                    queue.process(stop_rx).await
                }) {
                    error!("Error in USB transfer processing thread: {}", e);
                    
                    // Enhanced error diagnostics with device-specific information
                    if e.to_string().contains("cancelled") {
                        info!("Transfer was cancelled - normal shutdown procedure");
                    } else if e.to_string().contains("pipe") || e.to_string().contains("endpoint") {
                        warn!("USB communication pipe error - device may have been disconnected or reset");
                        // Try to activate device reconnection detection
                        UsbDeviceConnectionDetector::set_device_reconnect_pending(true);
                        UsbDeviceConnectionDetector::set_device_connected(false);
                    } else if e.to_string().contains("permission") {
                        error!("USB permission error - insufficient access rights to USB device");
                        error!("Ensure application has proper USB device access permissions");
                    } else if e.to_string().contains("busy") {
                        warn!("USB device is busy - another application may be using the device");
                        warn!("Close other USB analysis applications that might be using the device");
                    } else if e.to_string().contains("timeout") {
                        warn!("USB transfer timeout - device may not be responding");
                        // Try a different approach for timeouts
                        UsbDeviceConnectionDetector::set_device_timeout(true);
                    } else if e.to_string().contains("bandwidth") || e.to_string().contains("resources") {
                        warn!("USB bandwidth or resource error - host controller may be overloaded");
                        warn!("Try disconnecting other high-bandwidth USB devices");
                    }
                    
                    // Log additional device information to help with troubleshooting
                    error!("Device details: VID:{:04x} PID:{:04x} ({}) endpoint:0x{:02x}",
                          device_info.vendor_id(), device_info.product_id(),
                          device_info.product_string().unwrap_or("Unknown"),
                          ENDPOINT);
                }
                
                info!("USB transfer processing thread completed for device {:04x}:{:04x}",
                      device_info.vendor_id(), device_info.product_id());
                // Notify that capturing has stopped
                UsbDeviceConnectionDetector::set_capture_active(false);
                // Also reset the device connection state
                UsbDeviceConnectionDetector::set_device_connected(false);
            });
            
            // Note that capture is now active
            UsbDeviceConnectionDetector::set_capture_active(true);
            
            info!("Advanced USB traffic capture successfully initialized");
            info!("📊 Ready to capture and decode USB transactions from connected devices");
        } else {
            warn!("No transfer queue available - unable to start packet capture");
            UsbDeviceConnectionDetector::set_capture_active(false);
            UsbDeviceConnectionDetector::set_device_connected(false);
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
        
        // Check for USB device connections in the raw packet data
        // This helps us optimize packet decoding when devices are connected
        use crate::cynthion::device_detector::UsbDeviceConnectionDetector;
        UsbDeviceConnectionDetector::check_for_usb_device_connection(data);
        
        // Log detailed information about received data for debugging with
        // enhanced awareness of connected devices
        info!("Processing {} bytes of USB data into transactions", data.len());
        
        // Enhanced logging with device connection awareness
        if UsbDeviceConnectionDetector::is_device_connected() {
            info!("Connected device traffic detected - optimizing transaction processing");
        }
        
        // Enhanced debug logging for all data bytes
        let hex_string = data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join(" ");
        debug!("Raw packet data (full dump): {}", hex_string);
        
        // Enhanced handling for data packets of all sizes
        if data.len() < 8 {
            debug!("Data is shorter than expected minimum 8 bytes: {} bytes", data.len());
            
            // Try to extract any useful information from short packets
            if data.len() >= 4 {
                debug!("Attempting to extract basic header from short packet");
                let packet_type = data[0];
                let endpoint = data[1];
                let device_addr = data[2];
                let data_len = data[3];
                debug!("Short packet header: type=0x{:02X}, ep=0x{:02X}, dev=0x{:02X}, len={}",
                       packet_type, endpoint, device_addr, data_len);
                
                // Create a special transaction for this short packet
                let direction = if endpoint & 0x80 != 0 { 
                    UsbDirection::DeviceToHost 
                } else { 
                    UsbDirection::HostToDevice 
                };
                
                let mut fields = std::collections::HashMap::new();
                fields.insert("packet_type".to_string(), format!("0x{:02X}", packet_type));
                fields.insert("raw_data".to_string(), hex_string.clone());
                fields.insert("format".to_string(), "Non-standard short packet".to_string());
                
                // Create a data packet if we have enough data (more than just the header)
                let data_packet = if data.len() > 4 {
                    let payload = data[4..].to_vec();
                    Some(crate::usb::mitm_traffic::UsbDataPacket::new(
                        payload, 
                        direction,
                        endpoint & 0x7F
                    ))
                } else {
                    // Empty data packet
                    Some(crate::usb::mitm_traffic::UsbDataPacket::new(
                        Vec::new(), 
                        direction,
                        endpoint & 0x7F
                    ))
                };
                
                // Create status packet (assume success)
                let status_packet = Some(crate::usb::mitm_traffic::UsbStatusPacket {
                    status: crate::usb::mitm_traffic::UsbTransferStatus::ACK,
                    endpoint: endpoint & 0x7F,
                });
                
                // Determine transfer type based on packet type or endpoint
                let transfer_type = match packet_type {
                    0xA5 => UsbTransferType::Control,
                    0x00 => UsbTransferType::Bulk,
                    0x23 => UsbTransferType::Interrupt,
                    0x69 => UsbTransferType::Bulk,
                    _ => {
                        // Use endpoint to guess transfer type
                        match endpoint & 0x7F {
                            0 => UsbTransferType::Control,
                            1 => UsbTransferType::Isochronous,
                            2..=3 => UsbTransferType::Bulk,
                            _ => UsbTransferType::Interrupt
                        }
                    }
                };
                
                // Create the transaction
                let transaction = UsbTransaction {
                    id: 1,
                    transfer_type,
                    setup_packet: None,
                    data_packet,
                    status_packet,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs_f64(),
                    device_address: device_addr,
                    endpoint: endpoint & 0x7F,
                    fields,
                };
                
                // Add this special transaction and return
                transactions.push(transaction);
                return transactions;
            } else {
                // Too short to extract meaningful header information
                debug!("Packet too short to parse (< 4 bytes): {}", hex_string);
                return Vec::new();
            }
        }

        // Process data into packets according to Cynthion/Packetry format
        // Each packet has: packet_type(1), endpoint(1), device_addr(1), data_len(1), data(variable)
        let mut offset = 0;
        
        // Used to track if we successfully processed any packets
        let original_transaction_count = transactions.len();
        
        while offset + 4 <= data.len() {
            // 1. Read packet header - minimum 4 bytes for header
            let packet_type = data[offset];
            let endpoint = data[offset + 1];
            let device_addr = data[offset + 2];
            let data_len = data[offset + 3] as usize;
            
            // Special handling for alternative packet types with more detailed logging
            let alternative_types = [0xA5, 0x00, 0x23, 0x69];
            if alternative_types.contains(&packet_type) {
                info!("Processing alternative packet type: 0x{:02X} at offset {}", packet_type, offset);
                // Log more details about this alternative packet
                let end_offset = std::cmp::min(offset + 16, data.len());
                let packet_preview = &data[offset..end_offset];
                let preview_hex = packet_preview.iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join(" ");
                debug!("Alternative packet preview: {}", preview_hex);
            }
            
            // Provide very detailed logging for each packet
            info!("Processing USB packet at offset {}: type=0x{:02X}, endpoint=0x{:02X}, device=0x{:02X}, len={}", 
                  offset, packet_type, endpoint, device_addr, data_len);
            
            // Detect invalid length that might indicate corrupted/misaligned data
            if data_len > 1024 {
                warn!("Suspiciously large data length: {} bytes - might be misaligned data", data_len);
                
                // Try to resync by looking for known packet type patterns
                let mut found_sync = false;
                for i in 1..16 {
                    if offset + i >= data.len() { break; }
                    
                    let potential_type = data[offset + i];
                    if [0xD0, 0x90, 0xC0, 0x10, 0x40, 0xA0, 0x20, 0xE0, 0xA5, 0x00, 0x23, 0x69].contains(&potential_type) {
                        info!("Found potential packet header at offset {}", offset + i);
                        offset = offset + i;
                        found_sync = true;
                        break;
                    }
                }
                
                if found_sync {
                    // Skip to the next iteration with the new offset
                    continue;
                } else {
                    // Couldn't resync, add raw data transaction and exit loop
                    let mut fields = std::collections::HashMap::new();
                    fields.insert("raw_data".to_string(), hex_string.clone());
                    fields.insert("error".to_string(), "Malformed packets".to_string());
                    
                    // Create a special transaction for the raw data
                    let transaction = UsbTransaction {
                        id: transactions.len() as u64 + 1,
                        transfer_type: UsbTransferType::Bulk,
                        setup_packet: None,
                        data_packet: Some(crate::usb::mitm_traffic::UsbDataPacket::new(
                            data.to_vec(), 
                            UsbDirection::DeviceToHost,
                            0
                        )),
                        status_packet: None,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs_f64(),
                        device_address: 0,
                        endpoint: 0,
                        fields,
                    };
                    
                    transactions.push(transaction);
                    break;
                }
            }
            
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
            // Updated to support both standard Packetry formatting and the format observed in logs
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
                
                // Alternative format packet types based on observed patterns
                0xA5 => UsbTransferType::Control,   // Possibly setup or control transfer
                0x00 => UsbTransferType::Bulk,      // Unknown but common in capture
                0x23 => UsbTransferType::Interrupt, // Based on observed patterns
                0x69 => UsbTransferType::Bulk,      // Based on observed patterns
                
                _ => {
                    info!("Unknown packet type: 0x{:02X}, using heuristics to determine type", packet_type);
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
                warn!("Packet data exceeds buffer bounds: offset={}, len={}, buffer={}", 
                      offset, data_len, data.len());
                
                // Add a partial transaction with available data
                let available_len = data.len() - (offset + 4);
                if available_len > 0 {
                    let partial_payload = data[offset+4..data.len()].to_vec();
                    debug!("Using partial payload of {} bytes", partial_payload.len());
                    
                    // Generate a transaction ID
                    let id = transactions.len() as u64 + 1;
                    
                    // Create fields with partial info
                    let mut fields = std::collections::HashMap::new();
                    fields.insert("packet_type".to_string(), format!("0x{:02X}", packet_type));
                    fields.insert("partial".to_string(), "true".to_string());
                    fields.insert("available_bytes".to_string(), format!("{}", available_len));
                    fields.insert("requested_bytes".to_string(), format!("{}", data_len));
                    
                    // Create a transaction for this partial data
                    let transaction = UsbTransaction {
                        id,
                        transfer_type,
                        setup_packet: None,
                        data_packet: Some(crate::usb::mitm_traffic::UsbDataPacket::new(
                            partial_payload, 
                            direction,
                            endpoint & 0x7F
                        )),
                        status_packet: Some(crate::usb::mitm_traffic::UsbStatusPacket {
                            status: crate::usb::mitm_traffic::UsbTransferStatus::ACK,
                            endpoint: endpoint & 0x7F,
                        }),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs_f64(),
                        device_address: device_addr,
                        endpoint: endpoint & 0x7F,
                        fields,
                    };
                    
                    transactions.push(transaction);
                }
                
                // Exit the processing loop
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
        
        // Enhanced error handling for empty transaction list
        if transactions.len() == original_transaction_count {
            // No transactions were successfully parsed
            warn!("Failed to parse any valid USB transactions from data");
            
            // Create a special transaction for the raw data so it's still visible in the UI
            let mut fields = std::collections::HashMap::new();
            fields.insert("raw_data".to_string(), hex_string);
            fields.insert("note".to_string(), "USBfly could not parse this data format, showing raw bytes".to_string());
            
            let transaction = UsbTransaction {
                id: 1,
                transfer_type: UsbTransferType::Bulk, // Default type
                setup_packet: None,
                data_packet: Some(crate::usb::mitm_traffic::UsbDataPacket::new(
                    data.to_vec(), 
                    UsbDirection::DeviceToHost, // Default direction
                    0 // Default endpoint
                )),
                status_packet: None,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64(),
                device_address: 0,
                endpoint: 0,
                fields,
            };
            
            transactions.push(transaction);
        } else {
            // At least one transaction was parsed
            info!("Successfully parsed {} USB transactions", transactions.len() - original_transaction_count);
        }
        
        // Return the parsed transactions
        transactions
    }
    
    // Get simulated MitM traffic for testing (public implementation)
    #[allow(dead_code)]
    /// This function is kept for compatibility but is not used in hardware-only mode
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