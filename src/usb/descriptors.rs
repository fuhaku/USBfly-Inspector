use serde::{Deserialize, Serialize};
use std::fmt;
use crate::data::vendor_ids::lookup_vendor;
use crate::data::class_codes::get_class_description;
use crate::data::descriptor_types::get_descriptor_type_name;
use std::collections::HashMap;
use byteorder::{ByteOrder, LittleEndian};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum USBDescriptor {
    Device(DeviceDescriptor),
    Configuration(ConfigurationDescriptor),
    Interface(InterfaceDescriptor),
    Endpoint(EndpointDescriptor),
    String(StringDescriptor),
    HID(HIDDescriptor),
    Unknown {
        descriptor_type: u8,
        data: Vec<u8>,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bcdUSB: u16,
    pub bDeviceClass: u8,
    pub bDeviceSubClass: u8,
    pub bDeviceProtocol: u8,
    pub bMaxPacketSize0: u8,
    pub idVendor: u16,
    pub idProduct: u16,
    pub bcdDevice: u16,
    pub iManufacturer: u8,
    pub iProduct: u8,
    pub iSerialNumber: u8,
    pub bNumConfigurations: u8,
}

impl DeviceDescriptor {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 18 {
            return None;
        }
        
        Some(DeviceDescriptor {
            bLength: data[0],
            bDescriptorType: data[1],
            bcdUSB: LittleEndian::read_u16(&data[2..4]),
            bDeviceClass: data[4],
            bDeviceSubClass: data[5],
            bDeviceProtocol: data[6],
            bMaxPacketSize0: data[7],
            idVendor: LittleEndian::read_u16(&data[8..10]),
            idProduct: LittleEndian::read_u16(&data[10..12]),
            bcdDevice: LittleEndian::read_u16(&data[12..14]),
            iManufacturer: data[14],
            iProduct: data[15],
            iSerialNumber: data[16],
            bNumConfigurations: data[17],
        })
    }
    
    pub fn vendor_name(&self) -> String {
        lookup_vendor(self.idVendor).unwrap_or_else(|| format!("Unknown (0x{:04X})", self.idVendor))
    }
    
    pub fn device_class_description(&self) -> String {
        get_class_description(self.bDeviceClass).unwrap_or_else(|| format!("Unknown (0x{:02X})", self.bDeviceClass))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub wTotalLength: u16,
    pub bNumInterfaces: u8,
    pub bConfigurationValue: u8,
    pub iConfiguration: u8,
    pub bmAttributes: u8,
    pub bMaxPower: u8,
}

impl ConfigurationDescriptor {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 9 {
            return None;
        }
        
        Some(ConfigurationDescriptor {
            bLength: data[0],
            bDescriptorType: data[1],
            wTotalLength: LittleEndian::read_u16(&data[2..4]),
            bNumInterfaces: data[4],
            bConfigurationValue: data[5],
            iConfiguration: data[6],
            bmAttributes: data[7],
            bMaxPower: data[8],
        })
    }
    
    pub fn is_self_powered(&self) -> bool {
        (self.bmAttributes & 0x40) != 0
    }
    
    pub fn supports_remote_wakeup(&self) -> bool {
        (self.bmAttributes & 0x20) != 0
    }
    
    pub fn max_power_ma(&self) -> u16 {
        u16::from(self.bMaxPower) * 2
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bInterfaceNumber: u8,
    pub bAlternateSetting: u8,
    pub bNumEndpoints: u8,
    pub bInterfaceClass: u8,
    pub bInterfaceSubClass: u8,
    pub bInterfaceProtocol: u8,
    pub iInterface: u8,
}

impl InterfaceDescriptor {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 9 {
            return None;
        }
        
        Some(InterfaceDescriptor {
            bLength: data[0],
            bDescriptorType: data[1],
            bInterfaceNumber: data[2],
            bAlternateSetting: data[3],
            bNumEndpoints: data[4],
            bInterfaceClass: data[5],
            bInterfaceSubClass: data[6],
            bInterfaceProtocol: data[7],
            iInterface: data[8],
        })
    }
    
    pub fn interface_class_description(&self) -> String {
        get_class_description(self.bInterfaceClass).unwrap_or_else(|| format!("Unknown (0x{:02X})", self.bInterfaceClass))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bEndpointAddress: u8,
    pub bmAttributes: u8,
    pub wMaxPacketSize: u16,
    pub bInterval: u8,
}

impl EndpointDescriptor {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 7 {
            return None;
        }
        
        Some(EndpointDescriptor {
            bLength: data[0],
            bDescriptorType: data[1],
            bEndpointAddress: data[2],
            bmAttributes: data[3],
            wMaxPacketSize: LittleEndian::read_u16(&data[4..6]),
            bInterval: data[6],
        })
    }
    
    pub fn endpoint_number(&self) -> u8 {
        self.bEndpointAddress & 0x0F
    }
    
    pub fn is_in(&self) -> bool {
        (self.bEndpointAddress & 0x80) != 0
    }
    
    pub fn transfer_type(&self) -> &'static str {
        match self.bmAttributes & 0x03 {
            0 => "Control",
            1 => "Isochronous",
            2 => "Bulk",
            3 => "Interrupt",
            _ => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub wLANGID: Option<Vec<u16>>, // Only for string descriptor 0
    pub string: Option<String>,    // For all other string descriptors
}

impl StringDescriptor {
    pub fn parse(data: &[u8], index: u8) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }
        
        if index == 0 {
            // String descriptor 0 contains supported language IDs
            let mut lang_ids = Vec::new();
            for i in (2..data[0] as usize).step_by(2) {
                if i + 1 < data.len() {
                    lang_ids.push(LittleEndian::read_u16(&data[i..i+2]));
                }
            }
            
            Some(StringDescriptor {
                bLength: data[0],
                bDescriptorType: data[1],
                wLANGID: Some(lang_ids),
                string: None,
            })
        } else {
            // Other string descriptors contain actual UTF-16LE strings
            if data.len() < data[0] as usize {
                return None;
            }
            
            let mut utf16_chars = Vec::new();
            for i in (2..data[0] as usize).step_by(2) {
                if i + 1 < data.len() {
                    utf16_chars.push(LittleEndian::read_u16(&data[i..i+2]));
                }
            }
            
            let string = String::from_utf16_lossy(&utf16_chars);
            
            Some(StringDescriptor {
                bLength: data[0],
                bDescriptorType: data[1],
                wLANGID: None,
                string: Some(string),
            })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HIDDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bcdHID: u16,
    pub bCountryCode: u8,
    pub bNumDescriptors: u8,
    pub bDescriptorType2: u8,
    pub wDescriptorLength: u16,
}

impl HIDDescriptor {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 9 {
            return None;
        }
        
        Some(HIDDescriptor {
            bLength: data[0],
            bDescriptorType: data[1],
            bcdHID: LittleEndian::read_u16(&data[2..4]),
            bCountryCode: data[4],
            bNumDescriptors: data[5],
            bDescriptorType2: data[6],
            wDescriptorLength: LittleEndian::read_u16(&data[7..9]),
        })
    }
    
    pub fn country_code_description(&self) -> &'static str {
        match self.bCountryCode {
            0 => "Not localized",
            1 => "Arabic",
            2 => "Belgian",
            3 => "Canadian-Bilingual",
            4 => "Canadian-French",
            5 => "Czech Republic",
            6 => "Danish",
            7 => "Finnish",
            8 => "French",
            9 => "German",
            10 => "Greek",
            11 => "Hebrew",
            12 => "Hungary",
            13 => "International (ISO)",
            14 => "Italian",
            15 => "Japan (Katakana)",
            16 => "Korean",
            17 => "Latin American",
            18 => "Netherlands/Dutch",
            19 => "Norwegian",
            20 => "Persian (Farsi)",
            21 => "Poland",
            22 => "Portuguese",
            23 => "Russia",
            24 => "Slovakia",
            25 => "Spanish",
            26 => "Swedish",
            27 => "Swiss/French",
            28 => "Swiss/German",
            29 => "Switzerland",
            30 => "Taiwan",
            31 => "Turkish-Q",
            32 => "UK",
            33 => "US",
            34 => "Yugoslavia",
            35 => "Turkish-F",
            _ => "Unknown",
        }
    }
}

impl fmt::Display for USBDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            USBDescriptor::Device(desc) => {
                write!(f, "Device Descriptor:\n")?;
                write!(f, "  bLength: {}\n", desc.bLength)?;
                write!(f, "  bDescriptorType: {} ({})\n", desc.bDescriptorType, get_descriptor_type_name(desc.bDescriptorType).unwrap_or("Unknown"))?;
                write!(f, "  bcdUSB: {:04X} (USB {})\n", desc.bcdUSB, format!("{}.{}", desc.bcdUSB >> 8, (desc.bcdUSB & 0xFF) / 10))?;
                write!(f, "  bDeviceClass: 0x{:02X} ({})\n", desc.bDeviceClass, desc.device_class_description())?;
                write!(f, "  bDeviceSubClass: 0x{:02X}\n", desc.bDeviceSubClass)?;
                write!(f, "  bDeviceProtocol: 0x{:02X}\n", desc.bDeviceProtocol)?;
                write!(f, "  bMaxPacketSize0: {}\n", desc.bMaxPacketSize0)?;
                write!(f, "  idVendor: 0x{:04X} ({})\n", desc.idVendor, desc.vendor_name())?;
                write!(f, "  idProduct: 0x{:04X}\n", desc.idProduct)?;
                write!(f, "  bcdDevice: {:04X}\n", desc.bcdDevice)?;
                write!(f, "  iManufacturer: {}\n", desc.iManufacturer)?;
                write!(f, "  iProduct: {}\n", desc.iProduct)?;
                write!(f, "  iSerialNumber: {}\n", desc.iSerialNumber)?;
                write!(f, "  bNumConfigurations: {}", desc.bNumConfigurations)
            },
            USBDescriptor::Configuration(desc) => {
                write!(f, "Configuration Descriptor:\n")?;
                write!(f, "  bLength: {}\n", desc.bLength)?;
                write!(f, "  bDescriptorType: {} ({})\n", desc.bDescriptorType, get_descriptor_type_name(desc.bDescriptorType).unwrap_or("Unknown"))?;
                write!(f, "  wTotalLength: {}\n", desc.wTotalLength)?;
                write!(f, "  bNumInterfaces: {}\n", desc.bNumInterfaces)?;
                write!(f, "  bConfigurationValue: {}\n", desc.bConfigurationValue)?;
                write!(f, "  iConfiguration: {}\n", desc.iConfiguration)?;
                write!(f, "  bmAttributes: 0x{:02X} ({}{})\n", 
                   desc.bmAttributes, 
                   if desc.is_self_powered() { "Self-powered " } else { "" },
                   if desc.supports_remote_wakeup() { "Remote Wakeup " } else { "" })?;
                write!(f, "  bMaxPower: {} ({}mA)", desc.bMaxPower, desc.max_power_ma())
            },
            USBDescriptor::Interface(desc) => {
                write!(f, "Interface Descriptor:\n")?;
                write!(f, "  bLength: {}\n", desc.bLength)?;
                write!(f, "  bDescriptorType: {} ({})\n", desc.bDescriptorType, get_descriptor_type_name(desc.bDescriptorType).unwrap_or("Unknown"))?;
                write!(f, "  bInterfaceNumber: {}\n", desc.bInterfaceNumber)?;
                write!(f, "  bAlternateSetting: {}\n", desc.bAlternateSetting)?;
                write!(f, "  bNumEndpoints: {}\n", desc.bNumEndpoints)?;
                write!(f, "  bInterfaceClass: 0x{:02X} ({})\n", desc.bInterfaceClass, desc.interface_class_description())?;
                write!(f, "  bInterfaceSubClass: 0x{:02X}\n", desc.bInterfaceSubClass)?;
                write!(f, "  bInterfaceProtocol: 0x{:02X}\n", desc.bInterfaceProtocol)?;
                write!(f, "  iInterface: {}", desc.iInterface)
            },
            USBDescriptor::Endpoint(desc) => {
                write!(f, "Endpoint Descriptor:\n")?;
                write!(f, "  bLength: {}\n", desc.bLength)?;
                write!(f, "  bDescriptorType: {} ({})\n", desc.bDescriptorType, get_descriptor_type_name(desc.bDescriptorType).unwrap_or("Unknown"))?;
                write!(f, "  bEndpointAddress: 0x{:02X} (EP {} {})\n", 
                   desc.bEndpointAddress, 
                   desc.endpoint_number(),
                   if desc.is_in() { "IN" } else { "OUT" })?;
                write!(f, "  bmAttributes: 0x{:02X} ({})\n", desc.bmAttributes, desc.transfer_type())?;
                write!(f, "  wMaxPacketSize: {}\n", desc.wMaxPacketSize)?;
                write!(f, "  bInterval: {}", desc.bInterval)
            },
            USBDescriptor::String(desc) => {
                write!(f, "String Descriptor:\n")?;
                write!(f, "  bLength: {}\n", desc.bLength)?;
                write!(f, "  bDescriptorType: {} ({})\n", desc.bDescriptorType, get_descriptor_type_name(desc.bDescriptorType).unwrap_or("Unknown"))?;
                
                if let Some(lang_ids) = &desc.wLANGID {
                    write!(f, "  Language IDs:")?;
                    for lang_id in lang_ids {
                        write!(f, " 0x{:04X}", lang_id)?;
                    }
                } else if let Some(string) = &desc.string {
                    write!(f, "  String: \"{}\"", string)?;
                }
                
                Ok(())
            },
            USBDescriptor::HID(desc) => {
                write!(f, "HID Descriptor:\n")?;
                write!(f, "  bLength: {}\n", desc.bLength)?;
                write!(f, "  bDescriptorType: {} ({})\n", desc.bDescriptorType, get_descriptor_type_name(desc.bDescriptorType).unwrap_or("Unknown"))?;
                write!(f, "  bcdHID: {:04X} (HID {})\n", desc.bcdHID, format!("{}.{}", desc.bcdHID >> 8, desc.bcdHID & 0xFF))?;
                write!(f, "  bCountryCode: {} ({})\n", desc.bCountryCode, desc.country_code_description())?;
                write!(f, "  bNumDescriptors: {}\n", desc.bNumDescriptors)?;
                write!(f, "  bDescriptorType2: {} ({})\n", desc.bDescriptorType2, get_descriptor_type_name(desc.bDescriptorType2).unwrap_or("Unknown"))?;
                write!(f, "  wDescriptorLength: {}", desc.wDescriptorLength)
            },
            USBDescriptor::Unknown { descriptor_type, data } => {
                write!(f, "Unknown Descriptor:\n")?;
                write!(f, "  Type: 0x{:02X}\n", descriptor_type)?;
                write!(f, "  Data: {:02X?}", data)
            }
        }
    }
}
