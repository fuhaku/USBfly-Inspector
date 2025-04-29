use crate::usb::descriptors::*;

pub fn get_descriptor_hints(descriptor: &USBDescriptor) -> Vec<String> {
    match descriptor {
        USBDescriptor::Device(desc) => device_descriptor_hints(desc),
        USBDescriptor::Configuration(desc) => configuration_descriptor_hints(desc),
        USBDescriptor::Interface(desc) => interface_descriptor_hints(desc),
        USBDescriptor::Endpoint(desc) => endpoint_descriptor_hints(desc),
        USBDescriptor::String(desc) => string_descriptor_hints(desc),
        USBDescriptor::HID(desc) => hid_descriptor_hints(desc),
        USBDescriptor::Unknown { descriptor_type, data: _ } => {
            vec![format!("Unknown descriptor type: 0x{:02X}", descriptor_type)]
        }
    }
}

fn device_descriptor_hints(desc: &DeviceDescriptor) -> Vec<String> {
    let mut hints = Vec::new();
    
    // USB version hint
    hints.push(format!("Device supports USB {} specification", 
        format!("{}.{}", desc.bcd_usb >> 8, (desc.bcd_usb & 0xFF) / 10)));
    
    // Device class hints
    if desc.b_device_class == 0x00 {
        hints.push("Device class is defined at the interface level".to_string());
    } else if desc.b_device_class == 0xFF {
        hints.push("Device uses a vendor-specific class".to_string());
    }
    
    // Max packet size hint
    hints.push(format!("Endpoint 0 supports {} byte packets", desc.b_max_packet_size0));
    
    // Vendor ID hint
    hints.push(format!("Vendor: {}", desc.vendor_name()));
    
    // Configuration hint
    hints.push(format!("Device has {} configuration(s)", desc.b_num_configurations));
    
    // String descriptor hints
    if desc.i_manufacturer > 0 {
        hints.push(format!("Manufacturer name is in string descriptor {}", desc.i_manufacturer));
    }
    if desc.i_product > 0 {
        hints.push(format!("Product name is in string descriptor {}", desc.i_product));
    }
    if desc.i_serial_number > 0 {
        hints.push(format!("Serial number is in string descriptor {}", desc.i_serial_number));
    }
    
    hints
}

fn configuration_descriptor_hints(desc: &ConfigurationDescriptor) -> Vec<String> {
    let mut hints = Vec::new();
    
    // Power hints
    if desc.is_self_powered() {
        hints.push("Device is self-powered".to_string());
    } else {
        hints.push("Device is bus-powered".to_string());
    }
    hints.push(format!("Device uses maximum of {}mA from USB", desc.max_power_ma()));
    
    // Remote wakeup hint
    if desc.supports_remote_wakeup() {
        hints.push("Device supports remote wakeup".to_string());
    }
    
    // Interface hint
    hints.push(format!("Configuration has {} interface(s)", desc.b_num_interfaces));
    
    // Total length hint
    hints.push(format!("Total descriptor set length: {} bytes", desc.w_total_length));
    
    hints
}

fn interface_descriptor_hints(desc: &InterfaceDescriptor) -> Vec<String> {
    let mut hints = Vec::new();
    
    // Class hints
    hints.push(format!("Interface class: {}", desc.interface_class_description()));
    
    // Specific class hints
    match desc.b_interface_class {
        0x01 => { // Audio
            hints.push("Audio interface - sound or voice".to_string());
            match desc.b_interface_sub_class {
                0x01 => hints.push("Audio Control interface".to_string()),
                0x02 => hints.push("Audio Streaming interface".to_string()),
                0x03 => hints.push("MIDI Streaming interface".to_string()),
                _ => {}
            }
        },
        0x02 => { // Communications and CDC Control
            hints.push("Communications Device Class (CDC)".to_string());
        },
        0x03 => { // HID
            hints.push("Human Interface Device (HID) - keyboard, mouse, etc.".to_string());
            match desc.b_interface_protocol {
                0x01 => hints.push("Keyboard protocol".to_string()),
                0x02 => hints.push("Mouse protocol".to_string()),
                _ => {}
            }
        },
        0x08 => { // Mass Storage
            hints.push("Mass Storage Device - flash drive, external HDD, etc.".to_string());
            match desc.b_interface_sub_class {
                0x01 => hints.push("Reduced Block Commands (RBC)".to_string()),
                0x02 => hints.push("CD/DVD device (MMC-2)".to_string()),
                0x03 => hints.push("QIC-157 Tape device".to_string()),
                0x04 => hints.push("USB Floppy Interface (UFI)".to_string()),
                0x05 => hints.push("SFF-8070i (ATAPI-style) device".to_string()),
                0x06 => hints.push("SCSI transparent command set".to_string()),
                _ => {}
            }
        },
        0x0A => { // CDC-Data
            hints.push("CDC-Data - used with Communication Device Class".to_string());
        },
        0x0E => { // Video
            hints.push("Video interface - webcam, television tuner, etc.".to_string());
        },
        0xFF => {
            hints.push("Vendor-specific interface".to_string());
        },
        _ => {}
    }
    
    // Endpoint hint
    hints.push(format!("Interface has {} endpoint(s)", desc.b_num_endpoints));
    
    hints
}

fn endpoint_descriptor_hints(desc: &EndpointDescriptor) -> Vec<String> {
    let mut hints = Vec::new();
    
    // Direction hint
    if desc.is_in() {
        hints.push(format!("Endpoint {} is IN (device-to-host)", desc.endpoint_number()));
    } else {
        hints.push(format!("Endpoint {} is OUT (host-to-device)", desc.endpoint_number()));
    }
    
    // Transfer type hint
    hints.push(format!("Endpoint uses {} transfer type", desc.transfer_type()));
    
    // Max packet size hint
    hints.push(format!("Maximum packet size: {} bytes", desc.w_max_packet_size));
    
    // Interval hint
    match desc.transfer_type() {
        "Interrupt" => {
            hints.push(format!("Polling interval: {}ms", desc.b_interval));
        },
        "Isochronous" => {
            hints.push(format!("Polling interval: 2^{}-1 frames", desc.b_interval));
        },
        _ => {
            hints.push(format!("Interval field: {}", desc.b_interval));
        }
    }
    
    hints
}

fn string_descriptor_hints(desc: &StringDescriptor) -> Vec<String> {
    let mut hints = Vec::new();
    
    if let Some(lang_ids) = &desc.w_langid {
        hints.push(format!("Device supports {} language(s)", lang_ids.len()));
        
        // Add common language ID hints
        for &lang_id in lang_ids {
            match lang_id {
                0x0409 => hints.push("English (United States)".to_string()),
                0x0809 => hints.push("English (United Kingdom)".to_string()),
                0x0407 => hints.push("German (Standard)".to_string()),
                0x040C => hints.push("French (Standard)".to_string()),
                0x0410 => hints.push("Italian (Standard)".to_string()),
                0x0411 => hints.push("Japanese".to_string()),
                0x0804 => hints.push("Chinese (Simplified)".to_string()),
                0x0C04 => hints.push("Chinese (Traditional)".to_string()),
                0x040A => hints.push("Spanish (Traditional Sort)".to_string()),
                _ => hints.push(format!("Language ID: 0x{:04X}", lang_id)),
            }
        }
    } else if let Some(string) = &desc.string {
        if !string.is_empty() {
            hints.push("String descriptor contains text data".to_string());
        } else {
            hints.push("String descriptor is empty".to_string());
        }
    }
    
    hints
}

fn hid_descriptor_hints(desc: &HIDDescriptor) -> Vec<String> {
    let mut hints = Vec::new();
    
    // HID version hint
    hints.push(format!("HID version {}.{}", desc.bcd_hid >> 8, desc.bcd_hid & 0xFF));
    
    // Country code hint
    if desc.b_country_code != 0 {
        hints.push(format!("Device is localized for: {}", desc.country_code_description()));
    } else {
        hints.push("Device is not localized".to_string());
    }
    
    // Report descriptor hint
    hints.push(format!("HID has {} subordinate descriptor(s)", desc.b_num_descriptors));
    hints.push(format!("Report descriptor is {} bytes long", desc.w_descriptor_length));
    
    hints
}
