//! Enhanced USB device connection detector module for Cynthion connections
//! This helps identify when devices connect to a Cynthion device and improves capture reliability

use log::{debug, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use lazy_static::lazy_static;
use std::sync::Mutex;

// Import the Speed enum from the usb module instead of the deprecated module
use crate::usb::Speed;

// Global state variables for enhanced device and capture detection
lazy_static! {
    // Track if we've detected a connected device
    static ref DEVICE_CONNECTED: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    
    // Track if a capture session is currently active
    static ref CAPTURE_ACTIVE: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    
    // Track if a device reconnect is pending (used for error recovery)
    static ref DEVICE_RECONNECT_PENDING: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    
    // Track if a device timeout has occurred
    static ref DEVICE_TIMEOUT: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    
    // Keep track of the last successful USB speed setting for improved reconnection
    static ref LAST_SUCCESSFUL_SPEED: Arc<Mutex<Option<Speed>>> = Arc::new(Mutex::new(None));
}

/// An enhanced detection helper that identifies USB device connections through Cynthion traffic
/// and improves the capture process for connected devices
pub struct UsbDeviceConnectionDetector {}

impl UsbDeviceConnectionDetector {
    /// Check if a device has been detected as connected to Cynthion
    pub fn is_device_connected() -> bool {
        DEVICE_CONNECTED.load(Ordering::Relaxed)
    }
    
    /// Set the device connected status (can be used by external components)
    pub fn set_device_connected(connected: bool) {
        DEVICE_CONNECTED.store(connected, Ordering::Relaxed);
        if connected {
            info!("USB device connected to Cynthion - capture optimized for device traffic");
        } else {
            info!("No USB devices detected on Cynthion");
        }
    }
    
    /// Check if a capture session is currently active
    pub fn is_capture_active() -> bool {
        CAPTURE_ACTIVE.load(Ordering::Relaxed)
    }
    
    /// Set the capture active status
    pub fn set_capture_active(active: bool) {
        CAPTURE_ACTIVE.store(active, Ordering::Relaxed);
        if active {
            info!("📥 USB capture session started - traffic monitoring active");
        } else {
            info!("⏹️ USB capture session ended");
        }
    }
    
    /// Check if device reconnection is pending
    pub fn is_device_reconnect_pending() -> bool {
        DEVICE_RECONNECT_PENDING.load(Ordering::Relaxed)
    }
    
    /// Set the device reconnect pending status
    pub fn set_device_reconnect_pending(pending: bool) {
        DEVICE_RECONNECT_PENDING.store(pending, Ordering::Relaxed);
        if pending {
            info!("🔄 USB device reconnection pending - attempting to reestablish connection");
        }
    }
    
    /// Check if a device timeout has occurred
    pub fn is_device_timeout() -> bool {
        DEVICE_TIMEOUT.load(Ordering::Relaxed)
    }
    
    /// Set the device timeout status
    pub fn set_device_timeout(timeout: bool) {
        DEVICE_TIMEOUT.store(timeout, Ordering::Relaxed);
        if timeout {
            warn!("⏱️ USB device timeout detected - connection may be unreliable");
        }
    }
    
    /// Get the last successful USB speed setting
    pub fn get_last_successful_speed() -> Option<Speed> {
        if let Ok(speed) = LAST_SUCCESSFUL_SPEED.lock() {
            return *speed;
        }
        None
    }
    
    /// Set the last successful USB speed setting
    pub fn set_last_successful_speed(speed: Speed) {
        if let Ok(mut last_speed) = LAST_SUCCESSFUL_SPEED.lock() {
            *last_speed = Some(speed);
            info!("📊 Remembered successful USB speed: {:?} for future connections", speed);
        } else {
            warn!("Failed to record last successful speed - mutex lock failed");
        }
    }
    
    /// Enhanced analysis of raw USB data looking for device connection sequences
    /// This method specifically focuses on finding connected devices on a Cynthion
    pub fn check_for_usb_device_connection(data: &[u8]) {
        // We need at least a full USB packet to analyze
        if data.len() < 8 {
            return;
        }
        
        // Scan the data for device enumeration patterns
        // When a USB device is connected, the host will perform a specific enumeration sequence
        // Typically starting with GET_DESCRIPTOR for the device descriptor
        let mut offset = 0;
        let mut has_found_device = false;
        
        while offset + 8 <= data.len() {
            // Read packet header (Cynthion format)
            let packet_type = data[offset];
            let endpoint = data[offset + 1];
            let device_addr = data[offset + 2];
            let data_len = data[offset + 3] as usize;
            
            // Safety check for data bounds
            if offset + 4 + data_len > data.len() {
                break;
            }
            
            // Enhanced detection looking for different packet types
            // 1. SETUP packets (0xD0) on EP0 (control endpoint)
            // 2. IN/OUT packets on various endpoints
            // 3. Special device configuration packets
            if (packet_type == 0xD0 || packet_type == 0xC0 || packet_type == 0x80) && 
               (endpoint & 0x7F) == 0 && data_len >= 8 {
                // This is a SETUP packet - extract the setup packet data
                let setup_data = &data[offset+4..offset+4+8]; // Standard setup packet is 8 bytes
                
                // Standard USB setup packet fields
                let bm_request_type = setup_data[0];
                let b_request = setup_data[1];
                let w_value = u16::from_le_bytes([setup_data[2], setup_data[3]]);
                
                // Check if this is a standard request (bit 5-6 == 0)
                let is_standard = (bm_request_type >> 5) & 0x03 == 0;
                
                // GET_DESCRIPTOR (0x06) requests are particularly interesting for device detection
                if is_standard && b_request == 0x06 {
                    let desc_type = (w_value >> 8) as u8;
                    let desc_index = (w_value & 0xFF) as u8;
                    
                    match desc_type {
                        1 => {
                            // Device Descriptor - major indicator of USB device connection
                            info!("🔌 USB Device Connection Detected! Host requesting Device Descriptor");
                            info!("   Device Address: {} on endpoint {}", device_addr, endpoint & 0x7F);
                            has_found_device = true;
                            // Update global connection state
                            UsbDeviceConnectionDetector::set_device_connected(true);
                        },
                        2 => {
                            // Configuration Descriptor - follows device descriptor in enumeration
                            info!("📝 USB Device Configuration: Host requesting Configuration Descriptor");
                            info!("   Device Address: {} on endpoint {}", device_addr, endpoint & 0x7F);
                        },
                        3 => {
                            // String Descriptor - indicates device identification in progress
                            debug!("USB String Descriptor requested: index={}", desc_index);
                        },
                        _ => {
                            // Other descriptor types
                            debug!("USB Descriptor request: type={}, index={}", desc_type, desc_index);
                        }
                    }
                }
                // SET_ADDRESS (0x05) is also a key part of USB enumeration
                else if is_standard && b_request == 0x05 {
                    let address = w_value & 0x7F;
                    info!("📍 USB Address Assignment: Host setting device address to {}", address);
                }
                // SET_CONFIGURATION (0x09) completes the basic USB enumeration process
                else if is_standard && b_request == 0x09 {
                    let config = w_value & 0xFF;
                    info!("✅ USB Configuration Complete: Device {} configured with config {}", device_addr, config);
                    info!("   USB device is now fully enumerated and ready for operation");
                    has_found_device = true;
                    // Update global connection state
                    UsbDeviceConnectionDetector::set_device_connected(true);
                }
            }
            
            // Move to the next packet
            offset += 4 + data_len;
        }
        
        // Return true if we found any evidence of a connected device
        if has_found_device {
            debug!("Device connection evidence found in USB traffic");
        }
    }
}
