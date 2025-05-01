use std::collections::HashMap;
use log::{debug, info, warn};
use crate::usb::descriptors::UsbDevice;
use crate::usb::UsbDescriptorType;
use crate::usb::packet_types::recognize_packet_type;
use serde::{Deserialize, Serialize};
use serde_json;

// Speed enum from the previous decoder/mod.rs
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Speed {
    Auto = 0,
    High = 1,
    Full = 2,
    Low = 3,
    Super = 4,
    SuperPlus = 5,
}

impl Speed {
    #[allow(dead_code)]
    pub fn mask(&self) -> u8 {
        1 << (*self as u8)
    }
}

impl std::fmt::Display for Speed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Speed::Auto => write!(f, "Auto"),
            Speed::High => write!(f, "High Speed"),
            Speed::Full => write!(f, "Full Speed"),
            Speed::Low => write!(f, "Low Speed"),
            Speed::Super => write!(f, "Super Speed"),
            Speed::SuperPlus => write!(f, "Super Speed Plus"),
        }
    }
}

// Data structure to hold decoded USB data for display in the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedUSBData {
    // Type of USB data
    pub data_type: String,
    // Human-readable description
    pub description: String,
    // Decoded fields
    pub fields: HashMap<String, String>,
    // Optional additional info based on data_type
    pub details: Option<String>,
    // USB descriptors parsed from the data
    pub descriptors: Vec<crate::usb::descriptors::USBDescriptor>,
}

// The decoder module is responsible for decoding USB protocol data
// including descriptors, control transfers, data transfers, etc.

#[derive(Debug, Clone)]
pub struct UsbDecoder {
    // Current state of USB device
    pub device: UsbDevice,
    
    // Additional data structures for protocol state
    pub transaction_counter: u64,
    pub vendor_names: HashMap<u16, String>,
    pub device_names: HashMap<(u16, u16), String>,
    
    // USB Speed setting for parsing packets correctly
    pub current_speed: Speed,
    
    // State flags
    initialized: bool,
}

impl UsbDecoder {
    pub fn new() -> Self {
        UsbDecoder {
            device: UsbDevice::new(),
            transaction_counter: 0,
            vendor_names: Self::load_vendor_database(),
            device_names: Self::load_device_database(),
            current_speed: Speed::Auto, // Default to Auto speed
            initialized: false,
        }
    }
    
    // Set the current USB speed for decoding
    pub fn set_speed(&mut self, speed: Speed) {
        // Log detailed information about the speed change for debugging
        let old_speed = self.current_speed;
        if old_speed != speed {
            info!("✓ Changing USB decoder speed: {:?} → {:?}", old_speed, speed);
        } else {
            info!("✓ Confirming USB decoder speed: {:?} (unchanged)", speed);
        }
        
        // Critical step: Update the speed setting for all future packet decoding
        self.current_speed = speed;
        
        // Reset the decoder state when speed changes to ensure clean state
        if old_speed != speed {
            self.reset();
            info!("Reset decoder state due to speed change");
        }
    }
    
    // Process raw USB data and update decoder state with enhanced error handling
    pub fn process_data(&mut self, data: &[u8]) -> Result<(), String> {
        debug!("Processing USB data with enhanced decoder, length={}, speed={:?}", data.len(), self.current_speed);
        
        // Empty data is not a processable error
        if data.is_empty() {
            return Err("Empty data received".to_string());
        }
        
        // Log the first few bytes for debugging
        let display_len = std::cmp::min(16, data.len());
        let data_start = data[0..display_len].iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<String>>()
            .join(" ");
        debug!("Data starts with: {}", data_start);
        
        // Adjust packet format interpretation based on current speed setting
        info!("✓ Decoding USB packet with speed: {:?}", self.current_speed);
        
        // Apply speed-specific packet interpretation rules
        let packet_size_valid = match self.current_speed {
            Speed::Low => {
                // Low speed has smaller packet sizes
                data.len() <= 8
            },
            Speed::Full => {
                // Full speed has medium packet sizes
                data.len() <= 64
            },
            Speed::High => {
                // High speed has larger packet sizes
                data.len() <= 512
            },
            Speed::Super | Speed::SuperPlus => {
                // Super speed has the largest packet sizes
                data.len() <= 1024
            },
            // Auto adapts based on content
            Speed::Auto => true,
        };
        
        if !packet_size_valid && data.len() > 0 {
            // Log warning but continue processing - the data might still be valid
            // just not ideal for the current speed setting
            warn!("Packet size {} bytes may not be optimal for {:?} speed", 
                  data.len(), self.current_speed);
        }
        
        // Reset state if this is first data
        if !self.initialized {
            self.reset();
            self.initialized = true;
        }
        
        // Parse descriptors from data
        self.parse_descriptors(data)?;
        
        // Increment transaction counter
        self.transaction_counter += 1;
        
        Ok(())
    }
    
    // Parse USB descriptors from raw data
    fn parse_descriptors(&mut self, data: &[u8]) -> Result<(), String> {
        // Process the data to extract descriptors
        self.device.parse_descriptors(data)?;
        
        // Log successful parsing
        if let Some(dev) = &self.device.device {
            let vendor_name = self.get_vendor_name(dev.vendor_id);
            let device_name = self.get_device_name(dev.vendor_id, dev.product_id);
            
            info!("Decoded device: VID={:04x} PID={:04x} ({} {})",
                dev.vendor_id, dev.product_id, 
                vendor_name.unwrap_or_else(|| "Unknown".to_string()), 
                device_name.unwrap_or_else(|| "".to_string()));
        }
        
        Ok(())
    }
    
    // Reset decoder state
    pub fn reset(&mut self) {
        debug!("Resetting USB decoder state");
        self.device = UsbDevice::new();
        self.transaction_counter = 0;
    }
    
    // Get all descriptors as a formatted string
    #[allow(dead_code)]
    pub fn get_all_descriptors_text(&self) -> String {
        format!("{}", self.device)
    }
    
    // Get friendly vendor name for a vendor ID
    pub fn get_vendor_name(&self, vendor_id: u16) -> Option<String> {
        self.vendor_names.get(&vendor_id).cloned()
    }
    
    // Decode USB data into a structured format for UI display
    pub fn decode(&self, data: &[u8]) -> Option<DecodedUSBData> {
        // If we're in simulation mode or encounter packet format that's unsupported,
        // we should still show something useful rather than returning None
        
        // Create a clone of self to process the data without modifying state
        let mut decoder_clone = UsbDecoder::new();
        
        // Make sure we use the same speed setting in the clone - critical for proper packet interpretation
        decoder_clone.set_speed(self.current_speed);
        info!("✓ Using speed {:?} for USB packet decoding - crucial for proper protocol interpretation", self.current_speed);
        
        // Enhanced packet detection with complete coverage of all known packet types
        // This ensures we can recognize and decode all possible packets from Cynthion
        let is_standard_packet = data.len() > 2 && (
            // Standard packet types from Packetry
            data[0] == 0xD0 || data[0] == 0x90 || data[0] == 0xC0 || 
            data[0] == 0x10 || data[0] == 0x40 || data[0] == 0xA0 || 
            data[0] == 0x20 || data[0] == 0xE0 ||
            // Cynthion-specific packet types (expanded list)
            data[0] == 0x5A || data[0] == 0x69 || data[0] == 0x24 || 
            data[0] == 0x1C || data[0] == 0x04 || data[0] == 0x00 ||
            // Additional Cynthion packet types seen in logs
            data[0] == 0x83 || data[0] == 0xAA || data[0] == 0xEC ||
            data[0] == 0x0C || data[0] == 0x58 || data[0] == 0xB7 ||
            // Fallback - treat all packets with reasonable sizes as recognizable
            (data.len() >= 8 && data.len() <= 65538) // 8-65538 bytes is a reasonable USB packet size
        );
            
        // Enhanced MitM packet recognition for better interoperability
        let is_mitm_packet = data.len() > 2 && (
            // Standard MitM packet headers
            data[0] == 0x80 || data[0] == 0x81 || data[0] == 0x82 || data[0] == 0x83 ||
            // Extended MitM packet headers
            data[0] == 0x84 || data[0] == 0x85 || data[0] == 0x86 || data[0] == 0x87 ||
            data[0] == 0x88 || data[0] == 0x89 || data[0] == 0x8A || data[0] == 0x8B ||
            // Special handling for custom MitM formats (including GreatFET variants)
            (data.len() >= 8 && (data[0] & 0x80) != 0)
        );
        
        // Try packet type recognition using our enum
        let packet_type_name = if data.len() > 0 {
            match recognize_packet_type(data[0]) {
                Some(packet_type) => format!("{:?}", packet_type),
                None => format!("Unknown (0x{:02X})", data[0])
            }
        } else {
            "Empty".to_string()
        };
        
        debug!("Packet type recognized as: {}", packet_type_name);
            
        // Log what kind of packet we detected
        debug!("Decoding USB data, length={}, standard_format={}, mitm_format={}", 
               data.len(), is_standard_packet, is_mitm_packet);
        
        // If we have raw data with fields already set, use that
        if !is_standard_packet && !is_mitm_packet && data.len() > 0 {
            // Check if this is a raw data packet with fields already set
            // These are often created by our process_transactions() fallback mechanism
            if let Some(fields_data) = data.iter().position(|&b| b == b'{') {
                if let Some(fields_end) = data.iter().position(|&b| b == b'}') {
                    if fields_end > fields_data {
                        debug!("Detected raw data with embedded field information");
                        return Some(self.decode_with_embedded_fields(data));
                    }
                }
            }
        }
        
        // Verify that the packet size is appropriate for the current speed
        let max_packet_size = match self.current_speed {
            Speed::Low => 8,
            Speed::Full => 64,
            Speed::High => 512,
            Speed::Super | Speed::SuperPlus => 1024,
            Speed::Auto => 1024, // Auto uses the largest allowed
        };
        
        // Log packet size compatibility information
        if data.len() > max_packet_size && data.len() > 0 {
            warn!("Packet size {} exceeds max size {} for {:?} speed - may result in fragmented data", 
                  data.len(), max_packet_size, self.current_speed);
        }
        
        // Try to process the data with standard descriptor decoding
        let process_result = decoder_clone.process_data(data);
        
        if let Err(e) = process_result {
            // Log the error but don't return None yet
            debug!("Standard processing of USB data failed: {}", e);
            
            // Try more specialized decoding approaches
            if is_standard_packet {
                debug!("Using standard packet format decoder");
                return Some(self.decode_standard_packet(data));
            } else if is_mitm_packet {
                debug!("Using MitM packet format decoder");
                return Some(self.decode_mitm_fallback(data));
            } else {
                // If it doesn't look like a recognizable packet format, use raw decoding
                debug!("Using raw data decoder");
                return Some(self.decode_raw_data(data));
            }
        }
        
        // Extract descriptors from processed data
        let descriptors = decoder_clone.device.get_all_descriptors();
        if descriptors.is_empty() {
            debug!("No USB descriptors found in data, trying alternative decoding");
            
            // Try more specialized decoding approaches
            if is_standard_packet {
                debug!("Using standard packet format decoder");
                return Some(self.decode_standard_packet(data));
            } else if is_mitm_packet {
                debug!("Using MitM packet format decoder");
                return Some(self.decode_mitm_fallback(data));
            } else {
                // Last resort - try raw decoding
                debug!("Using raw data decoder");
                return Some(self.decode_raw_data(data));
            }
        }
        
        // Create DecodedUSBData structure
        let mut decoded = DecodedUSBData {
            data_type: "USB Descriptors".to_string(),
            description: "Decoded USB device descriptors".to_string(),
            fields: HashMap::new(),
            details: None,
            descriptors,
        };
        
        // Add the speed information used for decoding
        decoded.fields.insert("USB Speed".to_string(), format!("{:?}", self.current_speed));
        
        // Add basic device info to fields if available
        if let Some(dev) = &decoder_clone.device.device {
            decoded.fields.insert("VID".to_string(), format!("{:04X}", dev.vendor_id));
            decoded.fields.insert("PID".to_string(), format!("{:04X}", dev.product_id));
            decoded.fields.insert("Device Class".to_string(), dev.device_class.name().to_string());
            // Extract USB version major and minor from BCD format
            let usb_version_major = (dev.usb_version >> 8) & 0xFF;
            let usb_version_minor = (dev.usb_version >> 4) & 0xF;
            decoded.fields.insert("USB Version".to_string(), format!("{}.{}", usb_version_major, usb_version_minor));
            
            // Add vendor and product names if available
            if let Some(vendor) = self.get_vendor_name(dev.vendor_id) {
                decoded.fields.insert("Vendor".to_string(), vendor);
            }
            
            if let Some(product) = self.get_device_name(dev.vendor_id, dev.product_id) {
                decoded.fields.insert("Product".to_string(), product);
            }
            
            // Add additional details
            decoded.details = Some(format!("USB {} device with {} configuration(s)", 
                                          dev.usb_version_string(), 
                                          dev.num_configurations));
        }
        
        Some(decoded)
    }
    
    // Get friendly device name for a vendor/product ID pair
    pub fn get_device_name(&self, vendor_id: u16, product_id: u16) -> Option<String> {
        self.device_names.get(&(vendor_id, product_id)).cloned()
    }
    
    // Load vendor name database
    fn load_vendor_database() -> HashMap<u16, String> {
        let mut vendors = HashMap::new();
        
        // Add some common vendors
        vendors.insert(0x1d50, "Great Scott Gadgets".to_string());
        vendors.insert(0x0483, "STMicroelectronics".to_string());
        vendors.insert(0x046d, "Logitech".to_string());
        vendors.insert(0x045e, "Microsoft".to_string());
        vendors.insert(0x05ac, "Apple".to_string());
        vendors.insert(0x0763, "Cypress Semiconductor".to_string());
        vendors.insert(0x18d1, "Google".to_string());
        vendors.insert(0x22d9, "OPPO Electronics".to_string());
        vendors.insert(0x0b05, "ASUSTek Computer".to_string());
        vendors.insert(0x413c, "Dell".to_string());
        vendors.insert(0x03f0, "HP".to_string());
        vendors.insert(0x0461, "Primax Electronics".to_string());
        vendors.insert(0x13b1, "Linksys".to_string());
        vendors.insert(0x0603, "Novatek Microelectronics".to_string());
        
        // TODO: In the full implementation, this would load from a database file
        vendors
    }
    
    // Load device name database
    fn load_device_database() -> HashMap<(u16, u16), String> {
        let mut devices = HashMap::new();
        
        // Add Cynthion devices
        devices.insert((0x1d50, 0x615c), "Cynthion USB Analyzer".to_string());
        devices.insert((0x1d50, 0x60e6), "Cynthion (DFU Mode)".to_string());
        devices.insert((0x1d50, 0x615b), "Cynthion USB Analyzer".to_string());
        
        // Add some common devices
        devices.insert((0x05ac, 0x8205), "MacBook Keyboard".to_string());
        devices.insert((0x05ac, 0x8002), "Apple Internal Keyboard/Mouse".to_string());
        devices.insert((0x05ac, 0x8242), "Apple IR Receiver".to_string());
        devices.insert((0x05ac, 0x1006), "Apple Keyboard".to_string());
        devices.insert((0x05ac, 0x0304), "Apple Optical USB Mouse".to_string());
        
        // TODO: In the full implementation, this would load from a database file
        devices
    }
    
    // Identify device class based on descriptor
    #[allow(dead_code)]
    pub fn identify_device_class(&self) -> Option<String> {
        self.device.device.as_ref().map(|d| {
            format!("{}", d.device_class.name())
        })
    }
    
    // Get list of all endpoints
    #[allow(dead_code)]
    pub fn get_endpoints(&self) -> Vec<String> {
        let mut endpoints = Vec::new();
        
        // Iterate through configurations and interfaces to find endpoints
        for config in &self.device.configurations {
            for interface in &config.interfaces {
                for endpoint in &interface.endpoints {
                    endpoints.push(format!(
                        "Endpoint 0x{:02x}: {} {}",
                        endpoint.endpoint_address,
                        endpoint.direction.name(),
                        endpoint.transfer_type.name()
                    ));
                }
            }
        }
        
        endpoints
    }
    
    // Get device strings
    #[allow(dead_code)]
    pub fn get_device_strings(&self) -> Vec<String> {
        let mut strings = Vec::new();
        
        // Add device descriptor strings
        if let Some(dev) = &self.device.device {
            if let Some(mfg) = &dev.manufacturer_string {
                strings.push(format!("Manufacturer: {}", mfg));
            }
            if let Some(prod) = &dev.product_string {
                strings.push(format!("Product: {}", prod));
            }
            if let Some(serial) = &dev.serial_number_string {
                strings.push(format!("Serial Number: {}", serial));
            }
        }
        
        // Add configuration strings
        for (i, config) in self.device.configurations.iter().enumerate() {
            if let Some(cfg_str) = &config.configuration_string {
                strings.push(format!("Configuration {}: {}", i, cfg_str));
            }
            
            // Add interface strings
            for (j, interface) in config.interfaces.iter().enumerate() {
                if let Some(if_str) = &interface.interface_string {
                    strings.push(format!("Interface {}.{}: {}", i, j, if_str));
                }
            }
        }
        
        strings
    }
    
    // Decode MitM packets when standard processing fails
    pub fn decode_mitm_fallback(&self, data: &[u8]) -> DecodedUSBData {
        let mut decoded = DecodedUSBData {
            data_type: "MitM Traffic".to_string(),
            description: "USB Man-in-the-Middle Traffic".to_string(),
            fields: HashMap::new(),
            details: None,
            descriptors: Vec::new(),
        };
        
        // Add the speed information used for decoding - essential for understanding packet structure
        decoded.fields.insert("USB Speed".to_string(), format!("{:?}", self.current_speed));
        
        if data.len() < 2 {
            decoded.description = "Invalid MitM Data (too short)".to_string();
            return decoded;
        }
        
        // Extract packet type and other data
        let packet_type = data[0];
        let device_address = if data.len() > 1 { data[1] } else { 0 };
        
        // Add basic packet info with recognized type if available
        match recognize_packet_type(packet_type) {
            Some(recognized_type) => {
                decoded.fields.insert("Packet Type".to_string(), 
                                    format!("0x{:02X} ({:?})", packet_type, recognized_type));
                // Set data_type based on recognized packet type
                decoded.data_type = format!("{:?} Packet", recognized_type);
            },
            None => {
                decoded.fields.insert("Packet Type".to_string(), format!("0x{:02X} (Unknown)", packet_type));
            }
        }
        
        decoded.fields.insert("Device Address".to_string(), format!("{}", device_address));
        
        // Identify packet type
        match packet_type {
            0x80 => {
                decoded.data_type = "Control Setup Packet".to_string();
                
                if data.len() >= 10 {
                    // Extract setup data
                    // bmRequestType(1) + bRequest(1) + wValue(2) + wIndex(2) + wLength(2)
                    let bm_request_type = data[2];
                    let b_request = data[3];
                    let w_value = (data[5] as u16) << 8 | (data[4] as u16);
                    let w_index = (data[7] as u16) << 8 | (data[6] as u16);
                    let w_length = (data[9] as u16) << 8 | (data[8] as u16);
                    
                    // Determine request direction
                    let direction = if (bm_request_type & 0x80) != 0 {
                        "Device-to-Host"
                    } else {
                        "Host-to-Device"
                    };
                    
                    // Determine request type
                    let req_type = match (bm_request_type >> 5) & 0x03 {
                        0 => "Standard",
                        1 => "Class",
                        2 => "Vendor",
                        _ => "Reserved",
                    };
                    
                    // Determine recipient
                    let recipient = match bm_request_type & 0x1F {
                        0 => "Device",
                        1 => "Interface",
                        2 => "Endpoint",
                        3 => "Other",
                        _ => "Reserved",
                    };
                    
                    // Add fields
                    decoded.fields.insert("Direction".to_string(), direction.to_string());
                    decoded.fields.insert("Request Type".to_string(), req_type.to_string());
                    decoded.fields.insert("Recipient".to_string(), recipient.to_string());
                    decoded.fields.insert("Request".to_string(), format!("0x{:02X}", b_request));
                    decoded.fields.insert("Value".to_string(), format!("0x{:04X}", w_value));
                    decoded.fields.insert("Index".to_string(), format!("0x{:04X}", w_index));
                    decoded.fields.insert("Length".to_string(), format!("{} bytes", w_length));
                    
                    // Add detailed description
                    decoded.details = Some(format!(
                        "{} {} request (0x{:02X}) to {} with Value=0x{:04X}, Index=0x{:04X}, Length={}",
                        direction, req_type, b_request, recipient, w_value, w_index, w_length
                    ));
                }
            },
            0x81 => {
                decoded.data_type = "Control Data Packet".to_string();
                
                if data.len() > 2 {
                    let data_len = data.len() - 2;
                    decoded.fields.insert("Data Length".to_string(), format!("{} bytes", data_len));
                    
                    if data_len > 0 {
                        // Add first few bytes of data as a sample
                        let sample_size = std::cmp::min(8, data_len);
                        let mut sample = String::new();
                        for i in 0..sample_size {
                            sample.push_str(&format!("{:02X} ", data[i + 2]));
                        }
                        if data_len > sample_size {
                            sample.push_str("...");
                        }
                        decoded.fields.insert("Data Sample".to_string(), sample);
                    }
                    
                    decoded.details = Some(format!("Control data packet with {} bytes", data_len));
                }
            },
            0x82 => {
                decoded.data_type = "Status Packet".to_string();
                
                if data.len() >= 3 {
                    let status = data[2];
                    let status_str = match status {
                        0 => "ACK (Success)",
                        1 => "NAK (Try Again)",
                        2 => "STALL (Error)",
                        3 => "DATA",
                        _ => "Unknown",
                    };
                    
                    decoded.fields.insert("Status".to_string(), status_str.to_string());
                    decoded.fields.insert("Status Code".to_string(), format!("0x{:02X}", status));
                    
                    decoded.details = Some(format!("USB status: {}", status_str));
                }
            },
            0x83 => {
                decoded.data_type = "Bulk/Interrupt Transfer".to_string();
                
                if data.len() > 2 {
                    let endpoint = data[1] & 0x7F;
                    let direction = if (data[1] & 0x80) != 0 { "IN" } else { "OUT" };
                    let data_len = data.len() - 2;
                    
                    decoded.fields.insert("Endpoint".to_string(), format!("0x{:02X}", endpoint));
                    decoded.fields.insert("Direction".to_string(), direction.to_string());
                    decoded.fields.insert("Data Length".to_string(), format!("{} bytes", data_len));
                    
                    if data_len > 0 {
                        // Add first few bytes of data as a sample
                        let sample_size = std::cmp::min(8, data_len);
                        let mut sample = String::new();
                        for i in 0..sample_size {
                            sample.push_str(&format!("{:02X} ", data[i + 2]));
                        }
                        if data_len > sample_size {
                            sample.push_str("...");
                        }
                        decoded.fields.insert("Data Sample".to_string(), sample);
                    }
                    
                    decoded.details = Some(format!(
                        "{} transfer on endpoint 0x{:02X} with {} bytes",
                        direction, endpoint, data_len
                    ));
                }
            },
            _ => {
                decoded.data_type = "Unknown Packet".to_string();
                decoded.details = Some(format!("Unknown packet type: 0x{:02X}", packet_type));
            }
        }
        
        decoded
    }
    
    // Decode raw USB data into structured format for display
    pub fn decode_raw_data(&self, data: &[u8]) -> DecodedUSBData {
        let mut decoded = DecodedUSBData {
            data_type: "Unknown".to_string(),
            description: "Raw USB Data".to_string(),
            fields: HashMap::new(),
            details: None,
            descriptors: Vec::new(),
        };
        
        // Add the speed information used for decoding - essential context for raw packet data
        decoded.fields.insert("USB Speed".to_string(), format!("{:?}", self.current_speed));
        
        if data.is_empty() {
            return decoded;
        }
        
        // Try to determine the type of data
        if data.len() >= 2 {
            let length = data[0];
            let descriptor_type = UsbDescriptorType::from(data[1]);
            
            decoded.fields.insert("Length".to_string(), format!("{} bytes", length));
            decoded.fields.insert("Type".to_string(), format!("{}", descriptor_type.name()));
            
            match descriptor_type {
                UsbDescriptorType::Device => {
                    decoded.data_type = "Device Descriptor".to_string();
                    decoded.description = "USB Device Information".to_string();
                    
                    // Extract basic device info
                    if data.len() >= 18 {
                        let vendor_id = (data[9] as u16) << 8 | (data[8] as u16);
                        let product_id = (data[11] as u16) << 8 | (data[10] as u16);
                        
                        decoded.fields.insert("Vendor ID".to_string(), format!("0x{:04x}", vendor_id));
                        decoded.fields.insert("Product ID".to_string(), format!("0x{:04x}", product_id));
                        
                        // Add vendor/product names if available
                        if let Some(vendor_name) = self.get_vendor_name(vendor_id) {
                            decoded.fields.insert("Vendor".to_string(), vendor_name);
                        }
                        
                        if let Some(device_name) = self.get_device_name(vendor_id, product_id) {
                            decoded.fields.insert("Product".to_string(), device_name);
                        }
                    }
                },
                UsbDescriptorType::Configuration => {
                    decoded.data_type = "Configuration Descriptor".to_string();
                    decoded.description = "USB Device Configuration".to_string();
                    
                    if data.len() >= 9 {
                        let total_length = (data[3] as u16) << 8 | (data[2] as u16);
                        let num_interfaces = data[4];
                        
                        decoded.fields.insert("Total Length".to_string(), format!("{} bytes", total_length));
                        decoded.fields.insert("Interfaces".to_string(), format!("{}", num_interfaces));
                    }
                },
                UsbDescriptorType::Interface => {
                    decoded.data_type = "Interface Descriptor".to_string();
                    decoded.description = "USB Interface Definition".to_string();
                    
                    if data.len() >= 9 {
                        let interface_num = data[2];
                        let alternate = data[3];
                        let num_endpoints = data[4];
                        let class_code = data[5];
                        
                        decoded.fields.insert("Interface".to_string(), format!("{}", interface_num));
                        decoded.fields.insert("Alt Setting".to_string(), format!("{}", alternate));
                        decoded.fields.insert("Endpoints".to_string(), format!("{}", num_endpoints));
                        decoded.fields.insert("Class".to_string(), format!("0x{:02x}", class_code));
                    }
                },
                UsbDescriptorType::Endpoint => {
                    decoded.data_type = "Endpoint Descriptor".to_string();
                    decoded.description = "USB Endpoint Definition".to_string();
                    
                    if data.len() >= 7 {
                        let endpoint_addr = data[2];
                        let attributes = data[3];
                        let max_packet_size = (data[5] as u16) << 8 | (data[4] as u16);
                        
                        let endpoint_num = endpoint_addr & 0x0F;
                        let direction = if (endpoint_addr & 0x80) != 0 { "IN" } else { "OUT" };
                        
                        decoded.fields.insert("Address".to_string(), format!("0x{:02x}", endpoint_addr));
                        decoded.fields.insert("Number".to_string(), format!("{}", endpoint_num));
                        decoded.fields.insert("Direction".to_string(), direction.to_string());
                        decoded.fields.insert("Attributes".to_string(), format!("0x{:02x}", attributes));
                        decoded.fields.insert("Max Packet Size".to_string(), format!("{} bytes", max_packet_size));
                    }
                },
                UsbDescriptorType::String => {
                    decoded.data_type = "String Descriptor".to_string();
                    decoded.description = "USB String Information".to_string();
                    
                    // Extract string if present (UTF-16LE encoding)
                    if data.len() > 2 {
                        let mut string_data = Vec::new();
                        for i in (2..data.len()).step_by(2) {
                            if i+1 < data.len() {
                                string_data.push(data[i]);
                            }
                        }
                        
                        // Try to convert to a UTF-8 string
                        match String::from_utf8(string_data) {
                            Ok(s) => {
                                decoded.fields.insert("String".to_string(), s);
                            },
                            Err(_) => {
                                decoded.fields.insert("String".to_string(), "[Invalid UTF-8]".to_string());
                            }
                        }
                    }
                },
                _ => {
                    // For other descriptor types, just show the raw data
                    decoded.data_type = format!("{} Descriptor", descriptor_type.name());
                    decoded.description = descriptor_type.description().to_string();
                    
                    let hex_data = data.iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<String>>()
                        .join(" ");
                    
                    decoded.details = Some(hex_data);
                }
            }
        } else {
            // For very short data, just show hex
            let hex_data = data.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<String>>()
                .join(" ");
            
            decoded.fields.insert("Raw Data".to_string(), hex_data);
        }
        
        decoded
    }
    
    // Decode standard packet format (Packetry/Cynthion format)
    fn decode_standard_packet(&self, data: &[u8]) -> DecodedUSBData {
        let mut decoded = DecodedUSBData {
            data_type: "USB Packet".to_string(),
            description: "USB Packet Data".to_string(),
            fields: HashMap::new(),
            details: None,
            descriptors: Vec::new(),
        };
        
        // Add the speed information used for decoding
        decoded.fields.insert("USB Speed".to_string(), format!("{:?}", self.current_speed));
        
        if data.len() < 4 {
            decoded.description = "Invalid Packet Data (too short)".to_string();
            return decoded;
        }
        
        // Extract header information
        let packet_type = data[0];
        let endpoint = data[1];
        let device_address = data[2];
        let data_len = data[3] as usize;
        
        // Add basic packet info
        decoded.fields.insert("Packet Type".to_string(), format!("0x{:02X}", packet_type));
        decoded.fields.insert("Endpoint".to_string(), format!("0x{:02X}", endpoint));
        decoded.fields.insert("Device Address".to_string(), format!("{}", device_address));
        decoded.fields.insert("Data Length".to_string(), format!("{}", data_len));
        
        // Determine direction from endpoint
        let direction = if endpoint & 0x80 != 0 {
            "Device to Host (IN)"
        } else {
            "Host to Device (OUT)"
        };
        decoded.fields.insert("Direction".to_string(), direction.to_string());
        
        // Identify transfer type with enhanced Cynthion packet support
        let transfer_type_str = match packet_type {
            // Standard USB packet types
            0xD0 => "Control (SETUP)".to_string(),
            0x90 => "Bulk (IN)".to_string(),
            0xC0 => "Interrupt (IN)".to_string(),
            0x10 => "Bulk (OUT)".to_string(),
            0x40 => "Interrupt (OUT)".to_string(),
            0xA0 => "Isochronous (IN)".to_string(),
            0x20 => "Isochronous (OUT)".to_string(),
            0xE0 => "Control (Status)".to_string(),
            
            // Cynthion-specific packet types seen in logs
            0x5A => "Cynthion Traffic (Data)".to_string(),
            0x83 => "Cynthion Bulk/Interrupt".to_string(),
            0xAA => "Cynthion Control".to_string(),
            0xEC => "Cynthion Status".to_string(),
            0x0C => "Cynthion Special".to_string(),
            0x58 => "Cynthion Enumeration".to_string(),
            0xB7 => "Cynthion Transfer".to_string(),
            
            // Alternative packet types
            0xA5 => "Alternative Control".to_string(),
            0x00 => "Alternative Bulk".to_string(),
            0x23 => "Alternative Interrupt".to_string(),
            0x69 => "Alternative Bulk".to_string(),
            0x24 => "GreatFET Special".to_string(),
            0x1C => "GreatFET Control".to_string(),
            0x04 => "Cynthion MitM".to_string(),
            
            // Fallback for unknown types
            _ => format!("Unknown (0x{:02X})", packet_type),
        };
        decoded.fields.insert("Transfer Type".to_string(), transfer_type_str);
        
        // Check if we have payload data
        if data.len() >= 4 + data_len && data_len > 0 {
            // For SETUP packets (control transfers), try to decode setup data
            if packet_type == 0xD0 && data_len >= 8 {
                // Extract setup packet fields
                let setup_data = &data[4..4+8]; // Standard setup packet is 8 bytes
                let bm_request_type = setup_data[0];
                let b_request = setup_data[1];
                let w_value = u16::from_le_bytes([setup_data[2], setup_data[3]]);
                let w_index = u16::from_le_bytes([setup_data[4], setup_data[5]]);
                let w_length = u16::from_le_bytes([setup_data[6], setup_data[7]]);
                
                // Add setup packet fields
                decoded.fields.insert("bmRequestType".to_string(), format!("0x{:02X}", bm_request_type));
                decoded.fields.insert("bRequest".to_string(), format!("0x{:02X}", b_request));
                decoded.fields.insert("wValue".to_string(), format!("0x{:04X}", w_value));
                decoded.fields.insert("wIndex".to_string(), format!("0x{:04X}", w_index));
                decoded.fields.insert("wLength".to_string(), format!("{}", w_length));
                
                // For standard requests, provide more specific information
                if (bm_request_type & 0x60) == 0 { // Standard request
                    match b_request {
                        0x06 => { // GET_DESCRIPTOR
                            let desc_type = (w_value >> 8) as u8;
                            let desc_index = (w_value & 0xFF) as u8;
                            let desc_type_str = match desc_type {
                                1 => "DEVICE",
                                2 => "CONFIGURATION",
                                3 => "STRING",
                                4 => "INTERFACE",
                                5 => "ENDPOINT",
                                6 => "DEVICE_QUALIFIER",
                                7 => "OTHER_SPEED_CONFIGURATION",
                                8 => "INTERFACE_POWER",
                                _ => "UNKNOWN",
                            };
                            decoded.description = format!("Get {} Descriptor (Index: {})", desc_type_str, desc_index);
                        },
                        0x05 => { // SET_ADDRESS
                            decoded.description = format!("Set Address: {}", w_value);
                        },
                        0x09 => { // SET_CONFIGURATION
                            decoded.description = format!("Set Configuration: {}", w_value);
                        },
                        0x01 => { // CLEAR_FEATURE
                            decoded.description = format!("Clear Feature: 0x{:04X}", w_value);
                        },
                        0x03 => { // SET_FEATURE
                            decoded.description = format!("Set Feature: 0x{:04X}", w_value);
                        },
                        0x00 => { // GET_STATUS
                            decoded.description = "Get Status".to_string();
                        },
                        _ => {
                            decoded.description = format!("Control Request: 0x{:02X}", b_request);
                        }
                    }
                } else {
                    // For class or vendor specific requests
                    if (bm_request_type & 0x60) == 0x20 { // Class request
                        decoded.description = format!("Class-specific Request: 0x{:02X}", b_request);
                    } else if (bm_request_type & 0x60) == 0x40 { // Vendor request
                        decoded.description = format!("Vendor-specific Request: 0x{:02X}", b_request);
                    }
                }
            }
            
            // Add hexdump of data payload
            let hex_dump = data[4..4+data_len].iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<String>>()
                .join(" ");
            
            let ascii_dump = data[4..4+data_len].iter()
                .map(|&b| if b >= 32 && b <= 126 { b as char } else { '.' })
                .collect::<String>();
                
            decoded.details = Some(format!("Hex: {}\nASCII: {}", hex_dump, ascii_dump));
        } else {
            decoded.details = Some("No data payload".to_string());
        }
        
        decoded
    }
    
    // Decode embedded field data from raw packets
    fn decode_with_embedded_fields(&self, data: &[u8]) -> DecodedUSBData {
        // Create basic structure
        let mut decoded = DecodedUSBData {
            data_type: "USB Raw Data".to_string(),
            description: "Raw USB Data with Embedded Fields".to_string(),
            fields: HashMap::new(),
            details: None,
            descriptors: Vec::new(),
        };
        
        // Convert data to string to try to extract JSON-like field data
        if let Ok(data_str) = std::str::from_utf8(data) {
            if let Some(fields_start) = data_str.find('{') {
                if let Some(fields_end) = data_str.find('}') {
                    if fields_end > fields_start {
                        let fields_json = &data_str[fields_start..=fields_end];
                        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(fields_json) {
                            if let Some(obj) = json_value.as_object() {
                                for (key, value) in obj {
                                    if let Some(val_str) = value.as_str() {
                                        decoded.fields.insert(key.clone(), val_str.to_string());
                                    } else {
                                        decoded.fields.insert(key.clone(), value.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Add hex dump of first 32 bytes of data
        let display_len = std::cmp::min(32, data.len());
        let hex_dump = data[0..display_len].iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<String>>()
            .join(" ");
        
        decoded.details = Some(format!("Raw data hex dump (first {} bytes): {}", display_len, hex_dump));
        
        // Set more specific description if we have packet type info
        if let Some(packet_type) = decoded.fields.get("packet_type") {
            decoded.description = format!("USB Packet Type: {}", packet_type);
            
            // Add more specific info if we have it
            if let Some(transfer_type) = decoded.fields.get("transfer_type") {
                decoded.data_type = format!("{} Transfer", transfer_type);
            }
        }
        
        decoded
    }
    
    // Note: Original decode_raw_data is used instead of this (commented out to avoid duplication)
    /*
    fn decode_raw_data(&self, data: &[u8]) -> DecodedUSBData {
        // This function implementation has been moved to use the existing public version
        // See the public decode_raw_data function above
        unreachable!()
    }
    */
}