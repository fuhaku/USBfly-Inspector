use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref DESCRIPTOR_TYPE_MAP: HashMap<u8, &'static str> = {
        let mut m = HashMap::new();
        
        // Standard descriptor types
        m.insert(0x01, "Device");
        m.insert(0x02, "Configuration");
        m.insert(0x03, "String");
        m.insert(0x04, "Interface");
        m.insert(0x05, "Endpoint");
        m.insert(0x06, "Device Qualifier");
        m.insert(0x07, "Other Speed Configuration");
        m.insert(0x08, "Interface Power");
        m.insert(0x09, "OTG");
        m.insert(0x0A, "Debug");
        m.insert(0x0B, "Interface Association");
        m.insert(0x0C, "BOS");
        m.insert(0x0D, "Device Capability");
        m.insert(0x0E, "Wireless Endpoint Companion");
        m.insert(0x0F, "SuperSpeed Endpoint Companion");
        m.insert(0x10, "SuperSpeedPlus Isochronous Endpoint Companion");
        
        // Class specific descriptor types
        m.insert(0x21, "HID");
        m.insert(0x22, "HID Report");
        m.insert(0x23, "HID Physical");
        m.insert(0x24, "Class Specific Interface");
        m.insert(0x25, "Class Specific Endpoint");
        
        m
    };
}

#[allow(dead_code)]
pub fn get_descriptor_type_name(descriptor_type: u8) -> Option<&'static str> {
    DESCRIPTOR_TYPE_MAP.get(&descriptor_type).copied()
}

#[allow(dead_code)]
pub fn get_descriptor_types() -> Vec<(u8, String)> {
    DESCRIPTOR_TYPE_MAP
        .iter()
        .map(|(&code, &name)| (code, name.to_string()))
        .collect()
}
