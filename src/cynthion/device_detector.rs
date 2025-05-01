//! USB device connection detector module for Cynthion connections
//! This helps identify when devices connect to a Cynthion device

use log::{debug, info};

/// A detection helper that identifies USB device connections through Cynthion traffic
pub struct UsbDeviceConnectionDetector {}

impl UsbDeviceConnectionDetector {
    /// Analyze raw USB data looking for device connection sequences
    pub fn check_for_usb_device_connection(data: &[u8]) {
        // We need at least a full USB packet to analyze
        if data.len() < 8 {
            return;
        }
        
        // Scan the data for device enumeration patterns
        // When a USB device is connected, the host will perform a specific enumeration sequence
        // Typically starting with GET_DESCRIPTOR for the device descriptor
        let mut offset = 0;
        
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
            
            // SETUP packets (0xD0) on EP0 (control endpoint) are the most interesting
            // for device detection, especially GET_DESCRIPTOR requests
            if packet_type == 0xD0 && (endpoint & 0x7F) == 0 && data_len >= 8 {
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
                }
            }
            
            // Move to the next packet
            offset += 4 + data_len;
        }
    }
}
