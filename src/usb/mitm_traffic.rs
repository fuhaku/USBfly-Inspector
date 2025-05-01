use std::fmt;
use std::collections::HashMap;
use crate::usb::descriptors::{
    USBDescriptor, 
    DeviceDescriptor, 
    ConfigurationDescriptor, 
    InterfaceDescriptor, 
    EndpointDescriptor, 
    StringDescriptor
};
use serde::{Deserialize, Serialize};

// USB packet direction enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsbDirection {
    HostToDevice,
    DeviceToHost,
    Unknown,
}

impl fmt::Display for UsbDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsbDirection::HostToDevice => write!(f, "HOST→DEVICE"),
            UsbDirection::DeviceToHost => write!(f, "DEVICE→HOST"),
            UsbDirection::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

// USB endpoint types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsbTransferType {
    Control,
    Isochronous,
    Bulk,
    Interrupt,
    Unknown,
}

impl fmt::Display for UsbTransferType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsbTransferType::Control => write!(f, "Control"),
            UsbTransferType::Isochronous => write!(f, "Isochronous"),
            UsbTransferType::Bulk => write!(f, "Bulk"),
            UsbTransferType::Interrupt => write!(f, "Interrupt"),
            UsbTransferType::Unknown => write!(f, "Unknown"),
        }
    }
}

// USB Control Request Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsbControlRequestType {
    Standard,
    Class,
    Vendor,
    Reserved,
}

impl fmt::Display for UsbControlRequestType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsbControlRequestType::Standard => write!(f, "Standard"),
            UsbControlRequestType::Class => write!(f, "Class"),
            UsbControlRequestType::Vendor => write!(f, "Vendor"),
            UsbControlRequestType::Reserved => write!(f, "Reserved"),
        }
    }
}

// USB Control Request Recipient
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsbControlRecipient {
    Device,
    Interface,
    Endpoint,
    Other,
    Reserved,
}

impl fmt::Display for UsbControlRecipient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsbControlRecipient::Device => write!(f, "Device"),
            UsbControlRecipient::Interface => write!(f, "Interface"),
            UsbControlRecipient::Endpoint => write!(f, "Endpoint"),
            UsbControlRecipient::Other => write!(f, "Other"),
            UsbControlRecipient::Reserved => write!(f, "Reserved"),
        }
    }
}

// Standard USB control request codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsbStandardRequest {
    GetStatus = 0,
    ClearFeature = 1,
    Reserved1 = 2,
    SetFeature = 3,
    Reserved2 = 4,
    SetAddress = 5,
    GetDescriptor = 6,
    SetDescriptor = 7,
    GetConfiguration = 8,
    SetConfiguration = 9,
    GetInterface = 10,
    SetInterface = 11,
    SynchFrame = 12,
    // USB 3.0 specific requests
    SetSel = 48,        // 0x30: Set System Exit Latency (SuperSpeed USB)
    SetIsochDelay = 49, // 0x31: Set Isochronous Delay (SuperSpeed USB)
    // USB 2.0 Extensions
    SetFeatureSelector = 51, // 0x33: Set Feature Selector (USB 2.0 Extension)
    Unknown = 255,
}

impl From<u8> for UsbStandardRequest {
    fn from(value: u8) -> Self {
        match value {
            0 => UsbStandardRequest::GetStatus,
            1 => UsbStandardRequest::ClearFeature,
            2 => UsbStandardRequest::Reserved1,
            3 => UsbStandardRequest::SetFeature,
            4 => UsbStandardRequest::Reserved2,
            5 => UsbStandardRequest::SetAddress,
            6 => UsbStandardRequest::GetDescriptor,
            7 => UsbStandardRequest::SetDescriptor,
            8 => UsbStandardRequest::GetConfiguration,
            9 => UsbStandardRequest::SetConfiguration,
            10 => UsbStandardRequest::GetInterface,
            11 => UsbStandardRequest::SetInterface,
            12 => UsbStandardRequest::SynchFrame,
            48 => UsbStandardRequest::SetSel,        // 0x30
            49 => UsbStandardRequest::SetIsochDelay, // 0x31
            51 => UsbStandardRequest::SetFeatureSelector, // 0x33
            _ => UsbStandardRequest::Unknown,
        }
    }
}

impl fmt::Display for UsbStandardRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsbStandardRequest::GetStatus => write!(f, "GET_STATUS"),
            UsbStandardRequest::ClearFeature => write!(f, "CLEAR_FEATURE"),
            UsbStandardRequest::Reserved1 => write!(f, "RESERVED"),
            UsbStandardRequest::SetFeature => write!(f, "SET_FEATURE"),
            UsbStandardRequest::Reserved2 => write!(f, "RESERVED"),
            UsbStandardRequest::SetAddress => write!(f, "SET_ADDRESS"),
            UsbStandardRequest::GetDescriptor => write!(f, "GET_DESCRIPTOR"),
            UsbStandardRequest::SetDescriptor => write!(f, "SET_DESCRIPTOR"),
            UsbStandardRequest::GetConfiguration => write!(f, "GET_CONFIGURATION"),
            UsbStandardRequest::SetConfiguration => write!(f, "SET_CONFIGURATION"),
            UsbStandardRequest::GetInterface => write!(f, "GET_INTERFACE"),
            UsbStandardRequest::SetInterface => write!(f, "SET_INTERFACE"),
            UsbStandardRequest::SynchFrame => write!(f, "SYNCH_FRAME"),
            UsbStandardRequest::SetSel => write!(f, "SET_SEL"),
            UsbStandardRequest::SetIsochDelay => write!(f, "SET_ISOCH_DELAY"),
            UsbStandardRequest::SetFeatureSelector => write!(f, "SET_FEATURE_SELECTOR"),
            UsbStandardRequest::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

// USB Setup packet structure (8 bytes)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct UsbSetupPacket {
    // Note: These field names follow the USB spec naming convention which uses camelCase.
    // We're keeping the USB standard names despite Rust's convention for field naming.
    pub bmRequestType: u8,
    pub bRequest: u8,
    pub wValue: u16,
    pub wIndex: u16,
    pub wLength: u16,
    
    // Decoded request information
    pub direction: UsbDirection,
    pub request_type: UsbControlRequestType,
    pub recipient: UsbControlRecipient,
    pub standard_request: Option<UsbStandardRequest>,
    pub request_description: String,
}

impl UsbSetupPacket {
    pub fn new(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }
        
        #[allow(non_snake_case)]
        let bmRequestType = data[0];
        #[allow(non_snake_case)]
        let bRequest = data[1];
        #[allow(non_snake_case)]
        let wValue = u16::from_le_bytes([data[2], data[3]]);
        #[allow(non_snake_case)]
        let wIndex = u16::from_le_bytes([data[4], data[5]]);
        #[allow(non_snake_case)]
        let wLength = u16::from_le_bytes([data[6], data[7]]);
        
        // Decode direction
        let direction = if (bmRequestType & 0x80) != 0 {
            UsbDirection::DeviceToHost
        } else {
            UsbDirection::HostToDevice
        };
        
        // Decode request type
        let request_type = match (bmRequestType >> 5) & 0x03 {
            0 => UsbControlRequestType::Standard,
            1 => UsbControlRequestType::Class,
            2 => UsbControlRequestType::Vendor,
            _ => UsbControlRequestType::Reserved,
        };
        
        // Decode recipient
        let recipient = match bmRequestType & 0x1F {
            0 => UsbControlRecipient::Device,
            1 => UsbControlRecipient::Interface,
            2 => UsbControlRecipient::Endpoint,
            3 => UsbControlRecipient::Other,
            _ => UsbControlRecipient::Reserved,
        };
        
        // Decode standard request if applicable
        let standard_request = if request_type == UsbControlRequestType::Standard {
            Some(UsbStandardRequest::from(bRequest))
        } else {
            None
        };
        
        // Create a human-readable description
        let request_description = Self::create_request_description(
            direction, request_type, recipient, 
            bRequest, standard_request, wValue, wIndex, wLength
        );
        
        Some(UsbSetupPacket {
            bmRequestType,
            bRequest,
            wValue,
            wIndex,
            wLength,
            direction,
            request_type,
            recipient,
            standard_request,
            request_description,
        })
    }
    
    fn create_request_description(
        _direction: UsbDirection,
        request_type: UsbControlRequestType,
        recipient: UsbControlRecipient,
        b_request: u8,
        standard_request: Option<UsbStandardRequest>,
        w_value: u16,
        w_index: u16,
        _w_length: u16
    ) -> String {
        // For standard requests, we can give detailed information
        if let Some(std_request) = standard_request {
            match std_request {
                UsbStandardRequest::GetDescriptor => {
                    let descriptor_type = (w_value >> 8) as u8;
                    let descriptor_index = (w_value & 0xFF) as u8;
                    
                    match descriptor_type {
                        1 => format!("Get DEVICE Descriptor"),
                        2 => format!("Get CONFIGURATION Descriptor (Index: {})", descriptor_index),
                        3 => format!("Get STRING Descriptor (Index: {})", descriptor_index),
                        4 => format!("Get INTERFACE Descriptor (Index: {})", descriptor_index),
                        5 => format!("Get ENDPOINT Descriptor (Index: {})", descriptor_index),
                        6 => format!("Get DEVICE_QUALIFIER Descriptor"),
                        7 => format!("Get OTHER_SPEED_CONFIGURATION Descriptor"),
                        8 => format!("Get INTERFACE_POWER Descriptor"),
                        _ => format!("Get Unknown Descriptor (Type: {}, Index: {})", 
                                    descriptor_type, descriptor_index),
                    }
                },
                UsbStandardRequest::SetAddress => {
                    format!("Set Device Address: {}", w_value)
                },
                UsbStandardRequest::SetConfiguration => {
                    format!("Set Configuration: {}", w_value)
                },
                UsbStandardRequest::GetConfiguration => {
                    format!("Get Current Configuration")
                },
                UsbStandardRequest::GetStatus => {
                    let target = match recipient {
                        UsbControlRecipient::Device => "Device".to_string(),
                        UsbControlRecipient::Interface => format!("Interface {}", w_index),
                        UsbControlRecipient::Endpoint => {
                            let ep_num = w_index & 0x0F;
                            let ep_dir = if (w_index & 0x80) != 0 { "IN" } else { "OUT" };
                            format!("Endpoint {} {}", ep_num, ep_dir)
                        },
                        _ => "Unknown".to_string(),
                    };
                    format!("Get Status of {}", target)
                },
                UsbStandardRequest::SetInterface => {
                    format!("Set Interface: {}, Alternate Setting: {}", w_index, w_value)
                },
                UsbStandardRequest::GetInterface => {
                    format!("Get Interface: {}", w_index)
                },
                _ => format!("{:?} ({})", std_request, b_request),
            }
        } else {
            // For non-standard requests, just provide the basics
            match request_type {
                UsbControlRequestType::Class => {
                    format!("Class Request: 0x{:02X}, Value: 0x{:04X}, Index: 0x{:04X}", 
                            b_request, w_value, w_index)
                },
                UsbControlRequestType::Vendor => {
                    format!("Vendor Request: 0x{:02X}, Value: 0x{:04X}, Index: 0x{:04X}", 
                            b_request, w_value, w_index)
                },
                _ => {
                    format!("Request: 0x{:02X}, Type: {:?}, Value: 0x{:04X}, Index: 0x{:04X}", 
                            b_request, request_type, w_value, w_index)
                },
            }
        }
    }
}

// Data transfer stage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDataPacket {
    pub data: Vec<u8>,
    pub direction: UsbDirection,
    pub endpoint: u8,
    pub data_summary: String,
}

impl UsbDataPacket {
    // Helper method to get a reference to the data
    pub fn get_data(&self) -> &[u8] {
        &self.data
    }
    
    // Helper method to get the data length
    pub fn get_data_len(&self) -> usize {
        self.data.len()
    }
    
    // Takes ownership of data Vec<u8>
    pub fn new(data: Vec<u8>, direction: UsbDirection, endpoint: u8) -> Self {
        // Create a summary of the data based on length and contents
        let data_len = data.len();
        let data_summary = if data_len == 0 {
            "No data".to_string()
        } else {
            let is_binary = data.iter().any(|&b| b < 32 || b > 126);
            if is_binary {
                format!("{} bytes of binary data", data_len)
            } else {
                // It's probably ASCII/text, show a preview
                let preview: String = data.iter()
                    .take(32)
                    .map(|&b| b as char)
                    .collect();
                    
                if data_len > 32 {
                    format!("\"{}...\" ({} bytes)", preview, data_len)
                } else {
                    format!("\"{}\" ({} bytes)", preview, data_len)
                }
            }
        };
        
        UsbDataPacket {
            data,
            direction,
            endpoint,
            data_summary,
        }
    }
}

// Status packet information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbStatusPacket {
    pub status: UsbTransferStatus,
    pub endpoint: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsbTransferStatus {
    ACK,
    NAK,
    STALL,
    NYET,
    Unknown,
}

impl fmt::Display for UsbTransferStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsbTransferStatus::ACK => write!(f, "ACK"),
            UsbTransferStatus::NAK => write!(f, "NAK"),
            UsbTransferStatus::STALL => write!(f, "STALL"),
            UsbTransferStatus::NYET => write!(f, "NYET"),
            UsbTransferStatus::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

// A single USB transaction (Setup, Data, Status)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbTransaction {
    pub id: u64,
    pub transfer_type: UsbTransferType,
    pub setup_packet: Option<UsbSetupPacket>,
    pub data_packet: Option<UsbDataPacket>,
    pub status_packet: Option<UsbStatusPacket>,
    pub timestamp: f64,
    pub device_address: u8,
    pub endpoint: u8,
    pub fields: HashMap<String, String>,
}

impl UsbTransaction {
    pub fn new(id: u64, timestamp: f64) -> Self {
        UsbTransaction {
            id,
            transfer_type: UsbTransferType::Unknown,
            setup_packet: None,
            data_packet: None,
            status_packet: None,
            timestamp,
            device_address: 0,
            endpoint: 0,
            fields: HashMap::new(),
        }
    }
    
    pub fn get_summary(&self) -> String {
        match self.transfer_type {
            UsbTransferType::Control => {
                if let Some(setup) = &self.setup_packet {
                    let dir_str = match setup.direction {
                        UsbDirection::HostToDevice => "H→D",
                        UsbDirection::DeviceToHost => "D→H",
                        UsbDirection::Unknown => "?",
                    };
                    
                    let data_info = if let Some(data) = &self.data_packet {
                        if data.get_data().is_empty() {
                            "No data".to_string()
                        } else {
                            format!("{} bytes", data.get_data_len())
                        }
                    } else {
                        "No data".to_string()
                    };
                    
                    format!("[{}] {} - {}", dir_str, setup.request_description, data_info)
                } else {
                    "Incomplete control transfer".to_string()
                }
            },
            UsbTransferType::Bulk => {
                if let Some(data) = &self.data_packet {
                    let dir_str = match data.direction {
                        UsbDirection::HostToDevice => "H→D",
                        UsbDirection::DeviceToHost => "D→H",
                        UsbDirection::Unknown => "?",
                    };
                    
                    format!("Bulk Transfer [{}] EP{:02X} - {}", 
                            dir_str, self.endpoint, data.data_summary)
                } else {
                    format!("Bulk Transfer EP{:02X} - No data", self.endpoint)
                }
            },
            UsbTransferType::Interrupt => {
                if let Some(data) = &self.data_packet {
                    let dir_str = match data.direction {
                        UsbDirection::HostToDevice => "H→D",
                        UsbDirection::DeviceToHost => "D→H",
                        UsbDirection::Unknown => "?",
                    };
                    
                    format!("Interrupt Transfer [{}] EP{:02X} - {}", 
                            dir_str, self.endpoint, data.data_summary)
                } else {
                    format!("Interrupt Transfer EP{:02X} - No data", self.endpoint)
                }
            },
            UsbTransferType::Isochronous => {
                if let Some(data) = &self.data_packet {
                    let dir_str = match data.direction {
                        UsbDirection::HostToDevice => "H→D",
                        UsbDirection::DeviceToHost => "D→H",
                        UsbDirection::Unknown => "?",
                    };
                    
                    format!("Isochronous Transfer [{}] EP{:02X} - {}", 
                            dir_str, self.endpoint, data.data_summary)
                } else {
                    format!("Isochronous Transfer EP{:02X} - No data", self.endpoint)
                }
            },
            UsbTransferType::Unknown => {
                "Unknown Transfer".to_string()
            }
        }
    }
}

// Container for MitM traffic data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitmTrafficData {
    pub transactions: Vec<UsbTransaction>,
    pub descriptors: Vec<USBDescriptor>,
    pub fields: HashMap<String, String>,
}

impl MitmTrafficData {
    #[allow(dead_code)]
    pub fn new() -> Self {
        MitmTrafficData {
            transactions: Vec::new(),
            descriptors: Vec::new(),
            fields: HashMap::new(),
        }
    }
    
    // Add a transaction to the traffic data
    #[allow(dead_code)]
    pub fn add_transaction(&mut self, transaction: UsbTransaction) {
        self.transactions.push(transaction);
    }
    
    // Get transactions organized by device and endpoint for hierarchical display
    #[allow(dead_code)]
    pub fn get_hierarchical_transactions(&self) -> HashMap<u8, HashMap<u8, Vec<&UsbTransaction>>> {
        let mut hierarchy: HashMap<u8, HashMap<u8, Vec<&UsbTransaction>>> = HashMap::new();
        
        for transaction in &self.transactions {
            let device_addr = transaction.device_address;
            let endpoint = transaction.endpoint;
            
            // Get or create the device entry
            let device_map = hierarchy.entry(device_addr).or_insert_with(HashMap::new);
            
            // Get or create the endpoint entry for this device
            let endpoint_list = device_map.entry(endpoint).or_insert_with(Vec::new);
            
            // Add the transaction to this endpoint
            endpoint_list.push(transaction);
        }
        
        hierarchy
    }
    
    // Analyze transactions to identify USB descriptors
    #[allow(dead_code)]
    pub fn extract_descriptors(&mut self) {
        for transaction in &self.transactions {
            if transaction.transfer_type == UsbTransferType::Control {
                // Look for setup packets that might contain descriptors
                if let Some(setup) = &transaction.setup_packet {
                    // Check if this is a GET_DESCRIPTOR request
                    if setup.request_type == UsbControlRequestType::Standard &&
                       setup.standard_request == Some(UsbStandardRequest::GetDescriptor) {
                        
                        // Now check if there's a corresponding data packet with the descriptor
                        if let Some(data) = &transaction.data_packet {
                            if !data.get_data().is_empty() {
                                // This might be a descriptor - try to parse it based on the type
                                // We'll need to try parsing it as different descriptor types
                                let descriptor_type = (setup.wValue >> 8) as u8;
                                
                                // Try to parse based on descriptor type
                                match descriptor_type {
                                    1 => { // Device Descriptor
                                        if let Ok(device_desc) = DeviceDescriptor::parse(data.get_data()) {
                                            self.descriptors.push(USBDescriptor::Device(device_desc));
                                        }
                                    },
                                    2 => { // Configuration Descriptor
                                        if let Ok(config_desc) = ConfigurationDescriptor::parse(data.get_data()) {
                                            self.descriptors.push(USBDescriptor::Configuration(config_desc));
                                        }
                                    },
                                    3 => { // String Descriptor
                                        if let Ok(string_desc) = StringDescriptor::parse(data.get_data(), (setup.wValue & 0xFF) as u8) {
                                            self.descriptors.push(USBDescriptor::String(string_desc));
                                        }
                                    },
                                    4 => { // Interface Descriptor
                                        if let Ok(interface_desc) = InterfaceDescriptor::parse(data.get_data()) {
                                            self.descriptors.push(USBDescriptor::Interface(interface_desc));
                                        }
                                    },
                                    5 => { // Endpoint Descriptor
                                        if let Ok(endpoint_desc) = EndpointDescriptor::parse(data.get_data()) {
                                            self.descriptors.push(USBDescriptor::Endpoint(endpoint_desc));
                                        }
                                    },
                                    _ => {
                                        // Other descriptor types not handled yet
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Get transaction details suitable for display
    #[allow(dead_code)]
    pub fn get_transaction_details(&self, transaction_id: u64) -> HashMap<String, String> {
        let mut details = HashMap::new();
        
        // Find the transaction by ID
        if let Some(transaction) = self.transactions.iter().find(|t| t.id == transaction_id) {
            // Add basic information
            details.insert("Type".to_string(), format!("{}", transaction.transfer_type));
            details.insert("Device".to_string(), format!("Address {}", transaction.device_address));
            details.insert("Endpoint".to_string(), format!("0x{:02X}", transaction.endpoint));
            
            // Add detailed information based on transaction type
            match transaction.transfer_type {
                UsbTransferType::Control => {
                    if let Some(setup) = &transaction.setup_packet {
                        details.insert("Request".to_string(), format!("{}", setup.bRequest));
                        details.insert("Request Type".to_string(), format!("{:?}", setup.request_type));
                        details.insert("Direction".to_string(), format!("{}", setup.direction));
                        details.insert("Value".to_string(), format!("0x{:04X}", setup.wValue));
                        details.insert("Index".to_string(), format!("0x{:04X}", setup.wIndex));
                        details.insert("Length".to_string(), format!("{}", setup.wLength));
                    }
                    
                    if let Some(data) = &transaction.data_packet {
                        let data_len = data.get_data_len();
                        details.insert("Data Length".to_string(), format!("{} bytes", data_len));
                        if !data.get_data().is_empty() {
                            // Add hex dump of first few bytes
                            let max_bytes = std::cmp::min(16, data_len);
                            let hex_dump = data.get_data()[0..max_bytes]
                                .iter()
                                .map(|b| format!("{:02X}", b))
                                .collect::<Vec<String>>()
                                .join(" ");
                            details.insert("Data".to_string(), format!("{}{}", 
                                hex_dump, 
                                if data_len > max_bytes { " ..." } else { "" }
                            ));
                        }
                    }
                    
                    if let Some(status) = &transaction.status_packet {
                        details.insert("Status".to_string(), format!("{:?}", status.status));
                    }
                },
                
                UsbTransferType::Bulk | UsbTransferType::Interrupt | UsbTransferType::Isochronous => {
                    if let Some(data) = &transaction.data_packet {
                        details.insert("Direction".to_string(), format!("{}", data.direction));
                        let data_len = data.get_data_len();
                        details.insert("Data Length".to_string(), format!("{} bytes", data_len));
                        if !data.get_data().is_empty() {
                            // Add hex dump of first few bytes
                            let max_bytes = std::cmp::min(16, data_len);
                            let hex_dump = data.get_data()[0..max_bytes]
                                .iter()
                                .map(|b| format!("{:02X}", b))
                                .collect::<Vec<String>>()
                                .join(" ");
                            details.insert("Data".to_string(), format!("{}{}", 
                                hex_dump, 
                                if data_len > max_bytes { " ..." } else { "" }
                            ));
                        }
                    }
                },
                
                UsbTransferType::Unknown => {
                    // Add any fields we have
                    for (key, value) in &transaction.fields {
                        details.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        
        details
    }
}

// Generate simulated MitM traffic for testing and fallback
pub fn generate_simulated_mitm_traffic() -> Vec<u8> {
    // Create a realistic set of USB packets for a typical USB enumeration sequence
    // This simulates what might be captured during a real MitM session
    
    // Start with a standard device enumeration sequence
    let mut packets = Vec::new();
    
    // Device address is 0 during initial setup
    let device_addr: u8 = 0;
    
    // 1. GET_DESCRIPTOR - Device descriptor (control setup)
    packets.extend_from_slice(&[
        0x80, device_addr,                          // Header + address
        0x80, 0x06, 0x00, 0x01, 0x00, 0x00, 0x12, 0x00  // Setup packet: bmRequestType, bRequest, wValue, wIndex, wLength
    ]);
    
    // 2. Device descriptor data (control data)
    packets.extend_from_slice(&[
        0x81, 0x80,                                // Header + address (with direction bit)
        // Standard device descriptor - 18 bytes 
        0x12, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x40,  // bLength, bDescriptorType, bcdUSB, bDeviceClass, bDeviceSubClass, bDeviceProtocol, bMaxPacketSize0
        0x50, 0x1d, 0x5c, 0x61, 0x01, 0x00, 0x01, 0x02,  // idVendor (0x1d50), idProduct (0x615c), bcdDevice, iManufacturer, iProduct, iSerialNumber
        0x01                                              // bNumConfigurations
    ]);
    
    // 3. Status (ACK)
    packets.extend_from_slice(&[
        0x82, device_addr, 0x00  // Header + address + status (ACK)
    ]);
    
    // 4. GET_DESCRIPTOR - Configuration descriptor
    packets.extend_from_slice(&[
        0x80, device_addr,                         // Header + address
        0x80, 0x06, 0x00, 0x02, 0x00, 0x00, 0x09, 0x00  // Setup packet for config descriptor
    ]);
    
    // 5. Configuration descriptor (partial)
    packets.extend_from_slice(&[
        0x81, 0x80,                                // Header + address
        // Configuration descriptor (9 bytes)
        0x09, 0x02, 0x19, 0x00, 0x01, 0x01, 0x00, 0x80, 0xfa  // bLength, bDescriptorType, wTotalLength, bNumInterfaces, bConfigValue, iConfiguration, bmAttributes, bMaxPower
    ]);
    
    // 6. Status (ACK)
    packets.extend_from_slice(&[
        0x82, device_addr, 0x00  // ACK
    ]);
    
    // 7. GET_DESCRIPTOR - String descriptor (manufacturer)
    packets.extend_from_slice(&[
        0x80, device_addr,                         // Header + address
        0x80, 0x06, 0x01, 0x03, 0x09, 0x04, 0x20, 0x00  // Setup packet for string descriptor
    ]);
    
    // 8. String descriptor data
    packets.extend_from_slice(&[
        0x81, 0x80,                                // Header + address
        // String descriptor (manufacturer - "Great Scott Gadgets")
        0x20, 0x03, 0x47, 0x00, 0x72, 0x00, 0x65, 0x00, 0x61, 0x00, 0x74, 0x00, 0x20, 0x00,
        0x53, 0x00, 0x63, 0x00, 0x6f, 0x00, 0x74, 0x00, 0x74, 0x00, 0x20, 0x00, 0x47, 0x00,
        0x61, 0x00, 0x64, 0x00, 0x67, 0x00, 0x65, 0x00, 0x74, 0x00, 0x73, 0x00
    ]);
    
    // 9. Status (ACK)
    packets.extend_from_slice(&[
        0x82, device_addr, 0x00  // ACK
    ]);
    
    // 10. SET_ADDRESS - Set device address to 1
    let new_address: u8 = 1;
    packets.extend_from_slice(&[
        0x80, device_addr,                         // Header + address
        0x00, 0x05, new_address, 0x00, 0x00, 0x00, 0x00, 0x00  // Setup packet for SET_ADDRESS
    ]);
    
    // 11. Status (ACK)
    packets.extend_from_slice(&[
        0x82, device_addr, 0x00  // ACK
    ]);
    
    // Now device address is 1 for subsequent requests
    let device_addr = new_address;
    
    // 12. GET_DESCRIPTOR - Full configuration with interfaces and endpoints
    packets.extend_from_slice(&[
        0x80, device_addr,                         // Header + address
        0x80, 0x06, 0x00, 0x02, 0x00, 0x00, 0xFF, 0x00  // Setup packet for full config
    ]);
    
    // 13. Full configuration data with interface and endpoint descriptors
    packets.extend_from_slice(&[
        0x81, 0x80 | device_addr,                 // Header + address
        // Configuration descriptor
        0x09, 0x02, 0x29, 0x00, 0x01, 0x01, 0x00, 0x80, 0xfa,
        // Interface descriptor 
        0x09, 0x04, 0x00, 0x00, 0x02, 0xFF, 0xFF, 0xFF, 0x00,
        // Endpoint descriptor 1
        0x07, 0x05, 0x81, 0x02, 0x40, 0x00, 0x00,
        // Endpoint descriptor 2
        0x07, 0x05, 0x01, 0x02, 0x40, 0x00, 0x00
    ]);
    
    // 14. Status (ACK)
    packets.extend_from_slice(&[
        0x82, device_addr, 0x00  // ACK
    ]);
    
    // 15. SET_CONFIGURATION - Activate configuration
    packets.extend_from_slice(&[
        0x80, device_addr,                         // Header + address
        0x00, 0x09, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00  // Setup packet for SET_CONFIGURATION
    ]);
    
    // 16. Status (ACK)
    packets.extend_from_slice(&[
        0x82, device_addr, 0x00  // ACK
    ]);
    
    // 17. Some bulk data transfers (OUT - host to device) - Endpoint 0x01
    packets.extend_from_slice(&[
        0x83, device_addr | 0x01,   // Header + address + endpoint 0x01 (OUT endpoint)
        0x01, 0x02, 0x03, 0x04      // Some bulk data
    ]);
    
    // 18. Some bulk data transfer (IN - device to host) - Endpoint 0x81
    packets.extend_from_slice(&[
        0x83, device_addr | 0x08 | 0x01,   // Header + address + endpoint 0x01 + IN direction bit
        0xF1, 0xF2, 0xF3, 0xF4, 0xF5 // Response data
    ]);
    
    packets
}

// Function to decode a MitM USB packet
#[allow(dead_code)]
#[allow(dead_code)]
pub fn decode_mitm_packet(raw_data: &[u8], timestamp: f64, counter: u64) -> Option<UsbTransaction> {
    // Need at least 2 bytes for packet header
    if raw_data.len() < 2 {
        return None;
    }
    
    let mut transaction = UsbTransaction::new(counter, timestamp);
    
    // Get packet type from first byte
    let packet_type = raw_data[0];
    
    // Parse packet based on type
    match packet_type {
        // Control transfer setup packet
        0x80 => {
            transaction.transfer_type = UsbTransferType::Control;
            
            // Need at least 10 bytes (1 header + 1 address + 8 setup)
            if raw_data.len() < 10 {
                return None;
            }
            
            transaction.device_address = raw_data[1];
            
            // Parse setup packet (starts at offset 2)
            if let Some(setup_packet) = UsbSetupPacket::new(&raw_data[2..10]) {
                transaction.setup_packet = Some(setup_packet);
                
                // Store additional fields for display
                transaction.fields.insert("Device".to_string(), 
                                        format!("Address {}", transaction.device_address));
                transaction.fields.insert("Type".to_string(), 
                                        format!("Control - Setup"));
                
                return Some(transaction);
            }
        },
        
        // Control transfer data packet
        0x81 => {
            transaction.transfer_type = UsbTransferType::Control;
            
            // Need at least 3 bytes (1 header + 1 address + 1 data minimum)
            if raw_data.len() < 3 {
                return None;
            }
            
            transaction.device_address = raw_data[1];
            
            // For control transfers, direction is determined by bit 7 of header second byte
            // This is different from other transfer types!
            use log::{debug, trace};
            
            let raw_byte = raw_data[1];
            trace!("Processing control data transfer packet with byte 0x{:02X}", raw_byte);
            
            let direction_bit = raw_byte & 0x80;
            let direction = if direction_bit != 0 {
                debug!("Detected IN direction (device to host) for control data");
                UsbDirection::DeviceToHost
            } else {
                debug!("Detected OUT direction (host to device) for control data");
                UsbDirection::HostToDevice
            };
            
            // Extract endpoint
            transaction.endpoint = raw_data[1] & 0x0F;
            
            // Create data packet (starts at offset 2)
            let data_packet = UsbDataPacket::new(
                raw_data[2..].to_vec(),
                direction,
                transaction.endpoint
            );
            
            transaction.data_packet = Some(data_packet);
            
            // Store additional fields for display
            transaction.fields.insert("Device".to_string(), 
                                    format!("Address {}", transaction.device_address));
            transaction.fields.insert("Type".to_string(), 
                                    format!("Control - Data"));
            transaction.fields.insert("Direction".to_string(), 
                                    format!("{}", direction));
            
            return Some(transaction);
        },
        
        // Control transfer status packet
        0x82 => {
            transaction.transfer_type = UsbTransferType::Control;
            
            // Need at least 3 bytes (1 header + 1 address + 1 status)
            if raw_data.len() < 3 {
                return None;
            }
            
            transaction.device_address = raw_data[1];
            transaction.endpoint = raw_data[1] & 0x0F;
            
            // Parse status code
            let status = match raw_data[2] {
                0x00 => UsbTransferStatus::ACK,
                0x01 => UsbTransferStatus::NAK,
                0x02 => UsbTransferStatus::STALL,
                0x03 => UsbTransferStatus::NYET,
                _ => UsbTransferStatus::Unknown,
            };
            
            transaction.status_packet = Some(UsbStatusPacket {
                status,
                endpoint: transaction.endpoint,
            });
            
            // Store additional fields for display
            transaction.fields.insert("Device".to_string(), 
                                    format!("Address {}", transaction.device_address));
            transaction.fields.insert("Type".to_string(), 
                                    format!("Control - Status"));
            transaction.fields.insert("Status".to_string(), 
                                    format!("{:?}", status));
            
            return Some(transaction);
        },
        
        // Bulk transfer
        0x83 => {
            use log::{debug, info};
            transaction.transfer_type = UsbTransferType::Bulk;
            
            // Need at least 3 bytes (1 header + 1 endpoint + 1 data minimum)
            if raw_data.len() < 3 {
                debug!("Bulk transfer packet too small: {} bytes", raw_data.len());
                return None;
            }
            
            let raw_byte = raw_data[1];
            debug!("Processing bulk transfer packet with byte 0x{:02X}", raw_byte);
            
            // Address is in the high nibble, endpoint in the low nibble
            transaction.device_address = raw_byte >> 4;
            transaction.endpoint = raw_byte & 0x0F;
            
            // Direction is determined by bit 3 of the endpoint byte (after masking)
            let direction_bit = raw_byte & 0x08;
            let direction = if direction_bit != 0 {
                info!("Detected IN direction (device to host) for bulk transfer: ep=0x{:X}, bit=0x{:X}", 
                      transaction.endpoint, direction_bit);
                UsbDirection::DeviceToHost
            } else {
                info!("Detected OUT direction (host to device) for bulk transfer: ep=0x{:X}, bit=0x{:X}", 
                      transaction.endpoint, direction_bit);
                UsbDirection::HostToDevice
            };
            
            // Create data packet (starts at offset 2)
            let data = raw_data[2..].to_vec();
            debug!("Bulk transfer data: {} bytes", data.len());
            let data_packet = UsbDataPacket::new(
                data,
                direction,
                transaction.endpoint
            );
            
            transaction.data_packet = Some(data_packet);
            
            // Store additional fields for display
            transaction.fields.insert("Device".to_string(), 
                                    format!("Address {}", transaction.device_address));
            transaction.fields.insert("Type".to_string(), 
                                    format!("Bulk"));
            transaction.fields.insert("Endpoint".to_string(), 
                                    format!("0x{:02X}", transaction.endpoint));
            transaction.fields.insert("Direction".to_string(), 
                                    format!("{}", direction));
            transaction.fields.insert("Data Length".to_string(),
                                    format!("{} bytes", transaction.data_packet.as_ref().unwrap().data.len()));
            
            return Some(transaction);
        },
        
        // Interrupt transfer
        0x84 => {
            use log::{debug, info};
            transaction.transfer_type = UsbTransferType::Interrupt;
            
            // Need at least 3 bytes (1 header + 1 endpoint + 1 data minimum)
            if raw_data.len() < 3 {
                debug!("Interrupt transfer packet too small: {} bytes", raw_data.len());
                return None;
            }
            
            let raw_byte = raw_data[1];
            debug!("Processing interrupt transfer packet with byte 0x{:02X}", raw_byte);
            
            // Address is in the high nibble, endpoint in the low nibble
            transaction.device_address = raw_byte >> 4;
            transaction.endpoint = raw_byte & 0x0F;
            
            // Direction is determined by bit 3 of the endpoint byte (after masking)
            let direction_bit = raw_byte & 0x08;
            let direction = if direction_bit != 0 {
                info!("Detected IN direction (device to host) for interrupt transfer: ep=0x{:X}, bit=0x{:X}", 
                      transaction.endpoint, direction_bit);
                UsbDirection::DeviceToHost
            } else {
                info!("Detected OUT direction (host to device) for interrupt transfer: ep=0x{:X}, bit=0x{:X}", 
                      transaction.endpoint, direction_bit);
                UsbDirection::HostToDevice
            };
            
            // Create data packet (starts at offset 2)
            let data = raw_data[2..].to_vec();
            debug!("Interrupt transfer data: {} bytes", data.len());
            let data_packet = UsbDataPacket::new(
                data,
                direction,
                transaction.endpoint
            );
            
            transaction.data_packet = Some(data_packet);
            
            // Store additional fields for display
            transaction.fields.insert("Device".to_string(), 
                                    format!("Address {}", transaction.device_address));
            transaction.fields.insert("Type".to_string(), 
                                    format!("Interrupt"));
            transaction.fields.insert("Endpoint".to_string(), 
                                    format!("0x{:02X}", transaction.endpoint));
            transaction.fields.insert("Direction".to_string(), 
                                    format!("{}", direction));
            transaction.fields.insert("Data Length".to_string(),
                                    format!("{} bytes", transaction.data_packet.as_ref().unwrap().data.len()));
            
            return Some(transaction);
        },
        
        // Isochronous transfer
        0x85 => {
            use log::{debug, info};
            transaction.transfer_type = UsbTransferType::Isochronous;
            
            // Need at least 3 bytes (1 header + 1 endpoint + 1 data minimum)
            if raw_data.len() < 3 {
                debug!("Isochronous transfer packet too small: {} bytes", raw_data.len());
                return None;
            }
            
            let raw_byte = raw_data[1];
            debug!("Processing isochronous transfer packet with byte 0x{:02X}", raw_byte);
            
            // Address is in the high nibble, endpoint in the low nibble
            transaction.device_address = raw_byte >> 4;
            transaction.endpoint = raw_byte & 0x0F;
            
            // Direction is determined by bit 3 of the endpoint byte (after masking)
            let direction_bit = raw_byte & 0x08;
            let direction = if direction_bit != 0 {
                info!("Detected IN direction (device to host) for isochronous transfer: ep=0x{:X}, bit=0x{:X}", 
                      transaction.endpoint, direction_bit);
                UsbDirection::DeviceToHost
            } else {
                info!("Detected OUT direction (host to device) for isochronous transfer: ep=0x{:X}, bit=0x{:X}", 
                      transaction.endpoint, direction_bit);
                UsbDirection::HostToDevice
            };
            
            // Create data packet (starts at offset 2)
            let data = raw_data[2..].to_vec();
            debug!("Isochronous transfer data: {} bytes", data.len());
            let data_packet = UsbDataPacket::new(
                data,
                direction,
                transaction.endpoint
            );
            
            transaction.data_packet = Some(data_packet);
            
            // Store additional fields for display
            transaction.fields.insert("Device".to_string(), 
                                    format!("Address {}", transaction.device_address));
            transaction.fields.insert("Type".to_string(), 
                                    format!("Isochronous"));
            transaction.fields.insert("Endpoint".to_string(), 
                                    format!("0x{:02X}", transaction.endpoint));
            transaction.fields.insert("Direction".to_string(), 
                                    format!("{}", direction));
            transaction.fields.insert("Data Length".to_string(),
                                    format!("{} bytes", transaction.data_packet.as_ref().unwrap().data.len()));
            
            return Some(transaction);
        },
        
        // Unknown packet type
        _ => {
            // If we don't recognize the packet type, store raw data for debugging
            transaction.transfer_type = UsbTransferType::Unknown;
            transaction.fields.insert("Unknown Type".to_string(), 
                                    format!("0x{:02X}", packet_type));
            transaction.fields.insert("Raw Data".to_string(), 
                                    format!("{:02X?}", raw_data));
            
            return Some(transaction);
        }
    }
    
    None
}
