use crate::usb::descriptors::*;
use crate::usb::hints::get_descriptor_hints;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// Constants for USB descriptor types
const USB_DESCRIPTOR_DEVICE: u8 = 0x01;
const USB_DESCRIPTOR_CONFIGURATION: u8 = 0x02;
const USB_DESCRIPTOR_STRING: u8 = 0x03;
const USB_DESCRIPTOR_INTERFACE: u8 = 0x04;
const USB_DESCRIPTOR_ENDPOINT: u8 = 0x05;
const USB_DESCRIPTOR_HID: u8 = 0x21;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedUSBData {
    pub descriptors: Vec<USBDescriptor>,
    pub timestamp: f64,
    pub device_address: Option<u8>,
    pub string_descriptors: HashMap<u8, String>,
}

pub struct USBDecoder {
    string_descriptors: HashMap<u8, String>,
}

impl USBDecoder {
    pub fn new() -> Self {
        USBDecoder {
            string_descriptors: HashMap::new(),
        }
    }
    
    pub fn decode(&mut self, data: &[u8]) -> Option<DecodedUSBData> {
        // For simplicity, assume the data contains a USB descriptor
        // In a real implementation, you would need to parse the USB protocol packets
        
        if data.is_empty() {
            return None;
        }
        
        // Check if this is a descriptor response
        if data.len() < 2 {
            return None;
        }
        
        let mut descriptors = Vec::new();
        let mut offset = 0;
        
        // Process all descriptors in the data
        while offset + 2 <= data.len() {
            let length = data[offset] as usize;
            if length == 0 || offset + length > data.len() {
                break;
            }
            
            let descriptor_type = data[offset + 1];
            let descriptor_data = &data[offset..offset + length];
            
            // Parse based on descriptor type
            match descriptor_type {
                USB_DESCRIPTOR_DEVICE => {
                    if let Some(device_desc) = DeviceDescriptor::parse(descriptor_data) {
                        descriptors.push(USBDescriptor::Device(device_desc));
                    }
                },
                USB_DESCRIPTOR_CONFIGURATION => {
                    if let Some(config_desc) = ConfigurationDescriptor::parse(descriptor_data) {
                        descriptors.push(USBDescriptor::Configuration(config_desc));
                    }
                },
                USB_DESCRIPTOR_INTERFACE => {
                    if let Some(interface_desc) = InterfaceDescriptor::parse(descriptor_data) {
                        descriptors.push(USBDescriptor::Interface(interface_desc));
                    }
                },
                USB_DESCRIPTOR_ENDPOINT => {
                    if let Some(endpoint_desc) = EndpointDescriptor::parse(descriptor_data) {
                        descriptors.push(USBDescriptor::Endpoint(endpoint_desc));
                    }
                },
                USB_DESCRIPTOR_STRING => {
                    // For string descriptors, we need to know which index it is
                    // This is usually determined by the setup packet, but for simplicity
                    // let's use a heuristic - if it's just a list of language IDs, it's index 0
                    let index = if length > 2 && (length - 2) % 2 == 0 && (length - 2) / 2 >= 1 {
                        // This is likely a string descriptor with a valid string
                        // Try to determine the index by looking at previous descriptors
                        let next_index = (self.string_descriptors.keys().max().unwrap_or(&0) + 1) as u8;
                        next_index
                    } else {
                        0 // Language IDs
                    };
                    
                    if let Some(string_desc) = StringDescriptor::parse(descriptor_data, index) {
                        if let Some(string) = &string_desc.string {
                            self.string_descriptors.insert(index, string.clone());
                        }
                        descriptors.push(USBDescriptor::String(string_desc));
                    }
                },
                USB_DESCRIPTOR_HID => {
                    if let Some(hid_desc) = HIDDescriptor::parse(descriptor_data) {
                        descriptors.push(USBDescriptor::HID(hid_desc));
                    }
                },
                _ => {
                    // Unknown descriptor type
                    descriptors.push(USBDescriptor::Unknown {
                        descriptor_type,
                        data: descriptor_data.to_vec(),
                    });
                }
            }
            
            offset += length;
        }
        
        if descriptors.is_empty() {
            None
        } else {
            Some(DecodedUSBData {
                descriptors,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64(),
                device_address: None, // Would be determined from the USB transaction
                string_descriptors: self.string_descriptors.clone(),
            })
        }
    }
    
    #[allow(dead_code)]
    pub fn get_hints(&self, descriptor: &USBDescriptor) -> Vec<String> {
        get_descriptor_hints(descriptor)
    }
    
    #[allow(dead_code)]
    pub fn resolve_string_descriptor(&self, index: u8) -> Option<&String> {
        self.string_descriptors.get(&index)
    }
}
