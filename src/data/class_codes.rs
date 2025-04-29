use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref CLASS_MAP: HashMap<u8, &'static str> = {
        let mut m = HashMap::new();
        
        // USB-IF assigned class codes
        m.insert(0x00, "Interface Association");
        m.insert(0x01, "Audio");
        m.insert(0x02, "Communications and CDC Control");
        m.insert(0x03, "Human Interface Device (HID)");
        m.insert(0x05, "Physical");
        m.insert(0x06, "Image");
        m.insert(0x07, "Printer");
        m.insert(0x08, "Mass Storage");
        m.insert(0x09, "Hub");
        m.insert(0x0A, "CDC-Data");
        m.insert(0x0B, "Smart Card");
        m.insert(0x0D, "Content Security");
        m.insert(0x0E, "Video");
        m.insert(0x0F, "Personal Healthcare");
        m.insert(0x10, "Audio/Video Devices");
        m.insert(0x11, "Billboard Device");
        m.insert(0x12, "USB Type-C Bridge");
        m.insert(0xDC, "Diagnostic Device");
        m.insert(0xE0, "Wireless Controller");
        m.insert(0xEF, "Miscellaneous");
        m.insert(0xFE, "Application Specific");
        m.insert(0xFF, "Vendor Specific");
        
        m
    };
    
    static ref SUBCLASS_MAP: HashMap<(u8, u8), &'static str> = {
        let mut m = HashMap::new();
        
        // Audio subclasses
        m.insert((0x01, 0x01), "Audio Control");
        m.insert((0x01, 0x02), "Audio Streaming");
        m.insert((0x01, 0x03), "MIDI Streaming");
        
        // CDC subclasses
        m.insert((0x02, 0x01), "Direct Line Control Model");
        m.insert((0x02, 0x02), "Abstract Control Model");
        m.insert((0x02, 0x03), "Telephone Control Model");
        m.insert((0x02, 0x04), "Multi-Channel Control Model");
        m.insert((0x02, 0x05), "CAPI Control Model");
        m.insert((0x02, 0x06), "Ethernet Networking Control Model");
        m.insert((0x02, 0x07), "ATM Networking Control Model");
        m.insert((0x02, 0x08), "Wireless Handset Control Model");
        m.insert((0x02, 0x09), "Device Management");
        m.insert((0x02, 0x0A), "Mobile Direct Line Model");
        m.insert((0x02, 0x0B), "OBEX");
        m.insert((0x02, 0x0C), "Ethernet Emulation Model");
        m.insert((0x02, 0x0D), "Network Control Model");
        
        // HID subclasses
        m.insert((0x03, 0x00), "No Subclass");
        m.insert((0x03, 0x01), "Boot Interface Subclass");
        
        // Mass Storage subclasses
        m.insert((0x08, 0x01), "Reduced Block Commands (RBC)");
        m.insert((0x08, 0x02), "CD/DVD Devices (MMC-2)");
        m.insert((0x08, 0x03), "Tape Device");
        m.insert((0x08, 0x04), "Floppy Disk Drive (UFI)");
        m.insert((0x08, 0x05), "Removable Media (ATAPI)");
        m.insert((0x08, 0x06), "SCSI Transparent Command Set");
        
        // Video subclasses
        m.insert((0x0E, 0x01), "Video Control");
        m.insert((0x0E, 0x02), "Video Streaming");
        m.insert((0x0E, 0x03), "Video Interface Collection");
        
        m
    };
    
    static ref PROTOCOL_MAP: HashMap<(u8, u8, u8), &'static str> = {
        let mut m = HashMap::new();
        
        // HID protocols
        m.insert((0x03, 0x01, 0x01), "Keyboard");
        m.insert((0x03, 0x01, 0x02), "Mouse");
        
        // Mass Storage protocols
        m.insert((0x08, 0x06, 0x50), "Bulk-Only Transport");
        
        m
    };
}

pub fn get_class_description(class_code: u8) -> Option<String> {
    CLASS_MAP.get(&class_code).map(|s| s.to_string())
}

pub fn get_subclass_description(class_code: u8, subclass_code: u8) -> Option<String> {
    SUBCLASS_MAP.get(&(class_code, subclass_code)).map(|s| s.to_string())
}

pub fn get_protocol_description(class_code: u8, subclass_code: u8, protocol_code: u8) -> Option<String> {
    PROTOCOL_MAP.get(&(class_code, subclass_code, protocol_code)).map(|s| s.to_string())
}

pub fn get_class_codes() -> Vec<(u8, String)> {
    CLASS_MAP
        .iter()
        .map(|(&code, &desc)| (code, desc.to_string()))
        .collect()
}
