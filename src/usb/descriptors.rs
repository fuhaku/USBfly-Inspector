use std::fmt;
use super::descriptor_types::*;

// USB Standard Device Descriptor
#[derive(Debug, Clone)]
pub struct DeviceDescriptor {
    pub length: u8,                // Descriptor size in bytes (18)
    pub descriptor_type: UsbDescriptorType, // DEVICE descriptor type (1)
    pub usb_version: u16,          // USB specification release number (BCD)
    pub device_class: UsbDeviceClass, // USB device class code
    pub device_subclass: u8,       // USB device subclass code
    pub device_protocol: u8,       // USB device protocol code
    pub max_packet_size0: u8,      // Maximum packet size for endpoint 0
    pub vendor_id: u16,            // Vendor ID (assigned by USB-IF)
    pub product_id: u16,           // Product ID (assigned by manufacturer)
    pub device_version: u16,       // Device release number (BCD)
    pub manufacturer_index: u8,    // Index of manufacturer string descriptor
    pub product_index: u8,         // Index of product string descriptor
    pub serial_number_index: u8,   // Index of serial number string descriptor
    pub num_configurations: u8,    // Number of possible configurations
    
    // Derived data - not in the descriptor itself
    pub manufacturer_string: Option<String>,
    pub product_string: Option<String>,
    pub serial_number_string: Option<String>,
}

impl DeviceDescriptor {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 18 {
            return Err(format!("Invalid device descriptor length: {}", data.len()));
        }
        
        let length = data[0];
        let descriptor_type = UsbDescriptorType::from(data[1]);
        
        match descriptor_type {
            UsbDescriptorType::Device => {
                let usb_version = (data[3] as u16) << 8 | (data[2] as u16);
                let device_class = UsbDeviceClass::from(data[4]);
                let device_subclass = data[5];
                let device_protocol = data[6];
                let max_packet_size0 = data[7];
                let vendor_id = (data[9] as u16) << 8 | (data[8] as u16);
                let product_id = (data[11] as u16) << 8 | (data[10] as u16);
                let device_version = (data[13] as u16) << 8 | (data[12] as u16);
                let manufacturer_index = data[14];
                let product_index = data[15];
                let serial_number_index = data[16];
                let num_configurations = data[17];
                
                Ok(DeviceDescriptor {
                    length,
                    descriptor_type,
                    usb_version,
                    device_class,
                    device_subclass,
                    device_protocol,
                    max_packet_size0,
                    vendor_id,
                    product_id,
                    device_version,
                    manufacturer_index,
                    product_index,
                    serial_number_index,
                    num_configurations,
                    
                    // Strings will be filled in later
                    manufacturer_string: None,
                    product_string: None,
                    serial_number_string: None,
                })
            },
            _ => Err(format!("Invalid descriptor type: {:?}", descriptor_type)),
        }
    }
    
    pub fn usb_version_string(&self) -> String {
        let major = (self.usb_version >> 8) & 0xFF;
        let minor = (self.usb_version >> 4) & 0xF;
        let subminor = self.usb_version & 0xF;
        
        format!("{}.{}.{}", major, minor, subminor)
    }
    
    pub fn device_version_string(&self) -> String {
        let major = (self.device_version >> 8) & 0xFF;
        let minor = (self.device_version >> 4) & 0xF;
        let subminor = self.device_version & 0xF;
        
        format!("{}.{}.{}", major, minor, subminor)
    }
}

impl fmt::Display for DeviceDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Device Descriptor:")?;
        writeln!(f, "  bLength: {} bytes", self.length)?;
        writeln!(f, "  bDescriptorType: {} ({})", self.descriptor_type.name(), self.descriptor_type.get_value())?;
        writeln!(f, "  bcdUSB: {} (USB {})", self.usb_version, self.usb_version_string())?;
        writeln!(f, "  bDeviceClass: 0x{:02x} ({})", self.device_class.get_value(), self.device_class.name())?;
        writeln!(f, "  bDeviceSubClass: 0x{:02x}", self.device_subclass)?;
        writeln!(f, "  bDeviceProtocol: 0x{:02x}", self.device_protocol)?;
        writeln!(f, "  bMaxPacketSize0: {} bytes", self.max_packet_size0)?;
        writeln!(f, "  idVendor: 0x{:04x}", self.vendor_id)?;
        writeln!(f, "  idProduct: 0x{:04x}", self.product_id)?;
        writeln!(f, "  bcdDevice: 0x{:04x} ({})", self.device_version, self.device_version_string())?;
        writeln!(f, "  iManufacturer: {}", self.manufacturer_index)?;
        if let Some(ref s) = self.manufacturer_string {
            writeln!(f, "    Manufacturer: {}", s)?;
        }
        writeln!(f, "  iProduct: {}", self.product_index)?;
        if let Some(ref s) = self.product_string {
            writeln!(f, "    Product: {}", s)?;
        }
        writeln!(f, "  iSerialNumber: {}", self.serial_number_index)?;
        if let Some(ref s) = self.serial_number_string {
            writeln!(f, "    Serial Number: {}", s)?;
        }
        writeln!(f, "  bNumConfigurations: {}", self.num_configurations)
    }
}

// USB Configuration Descriptor
#[derive(Debug, Clone)]
pub struct ConfigurationDescriptor {
    pub length: u8,                    // Descriptor size in bytes (9)
    pub descriptor_type: UsbDescriptorType, // CONFIGURATION descriptor type (2)
    pub total_length: u16,             // Total length in bytes of data returned
    pub num_interfaces: u8,            // Number of interfaces supported
    pub configuration_value: u8,       // Value to use as an argument to select this configuration
    pub configuration_index: u8,       // Index of string descriptor describing this configuration
    pub attributes: u8,                // Configuration characteristics
    pub max_power: u8,                 // Maximum power consumption in 2mA units
    
    // Derived data
    pub configuration_string: Option<String>,
    pub self_powered: bool,
    pub remote_wakeup: bool,
    
    // Child descriptors
    pub interfaces: Vec<InterfaceDescriptor>,
}

impl ConfigurationDescriptor {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 9 {
            return Err(format!("Invalid configuration descriptor length: {}", data.len()));
        }
        
        let length = data[0];
        let descriptor_type = UsbDescriptorType::from(data[1]);
        
        match descriptor_type {
            UsbDescriptorType::Configuration => {
                let total_length = (data[3] as u16) << 8 | (data[2] as u16);
                let num_interfaces = data[4];
                let configuration_value = data[5];
                let configuration_index = data[6];
                let attributes = data[7];
                let max_power = data[8];
                
                // Derived fields
                let self_powered = (attributes & 0x40) != 0;
                let remote_wakeup = (attributes & 0x20) != 0;
                
                Ok(ConfigurationDescriptor {
                    length,
                    descriptor_type,
                    total_length,
                    num_interfaces,
                    configuration_value,
                    configuration_index,
                    attributes,
                    max_power,
                    configuration_string: None,
                    self_powered,
                    remote_wakeup,
                    interfaces: Vec::new(), // Will be filled later
                })
            },
            _ => Err(format!("Invalid descriptor type: {:?}", descriptor_type)),
        }
    }
    
    pub fn power_consumption_ma(&self) -> u16 {
        self.max_power as u16 * 2
    }
}

impl fmt::Display for ConfigurationDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Configuration Descriptor:")?;
        writeln!(f, "  bLength: {} bytes", self.length)?;
        writeln!(f, "  bDescriptorType: {} ({})", self.descriptor_type.name(), self.descriptor_type.get_value())?;
        writeln!(f, "  wTotalLength: {} bytes", self.total_length)?;
        writeln!(f, "  bNumInterfaces: {}", self.num_interfaces)?;
        writeln!(f, "  bConfigurationValue: {}", self.configuration_value)?;
        writeln!(f, "  iConfiguration: {}", self.configuration_index)?;
        if let Some(ref s) = self.configuration_string {
            writeln!(f, "    Configuration: {}", s)?;
        }
        writeln!(f, "  bmAttributes: 0x{:02x}", self.attributes)?;
        writeln!(f, "    Self Powered: {}", if self.self_powered { "Yes" } else { "No" })?;
        writeln!(f, "    Remote Wakeup: {}", if self.remote_wakeup { "Yes" } else { "No" })?;
        writeln!(f, "  bMaxPower: {}mA", self.power_consumption_ma())?;
        
        // Display interfaces
        for interface in &self.interfaces {
            write!(f, "{}", interface)?;
        }
        
        Ok(())
    }
}

// USB Interface Descriptor
#[derive(Debug, Clone)]
pub struct InterfaceDescriptor {
    pub length: u8,                    // Descriptor size in bytes (9)
    pub descriptor_type: UsbDescriptorType, // INTERFACE descriptor type (4)
    pub interface_number: u8,          // Number of this interface
    pub alternate_setting: u8,         // Value used to select this alternate setting
    pub num_endpoints: u8,             // Number of endpoints used by this interface
    pub interface_class: UsbDeviceClass, // Class code
    pub interface_subclass: u8,        // Subclass code
    pub interface_protocol: u8,        // Protocol code
    pub interface_index: u8,           // Index of string descriptor describing this interface
    
    // Derived data
    pub interface_string: Option<String>,
    
    // Child descriptors
    pub endpoints: Vec<EndpointDescriptor>,
    pub class_specific: Vec<Vec<u8>>,  // Raw class-specific descriptors
}

impl InterfaceDescriptor {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 9 {
            return Err(format!("Invalid interface descriptor length: {}", data.len()));
        }
        
        let length = data[0];
        let descriptor_type = UsbDescriptorType::from(data[1]);
        
        match descriptor_type {
            UsbDescriptorType::Interface => {
                let interface_number = data[2];
                let alternate_setting = data[3];
                let num_endpoints = data[4];
                let interface_class = UsbDeviceClass::from(data[5]);
                let interface_subclass = data[6];
                let interface_protocol = data[7];
                let interface_index = data[8];
                
                Ok(InterfaceDescriptor {
                    length,
                    descriptor_type,
                    interface_number,
                    alternate_setting,
                    num_endpoints,
                    interface_class,
                    interface_subclass,
                    interface_protocol,
                    interface_index,
                    interface_string: None,
                    endpoints: Vec::new(),
                    class_specific: Vec::new(),
                })
            },
            _ => Err(format!("Invalid descriptor type: {:?}", descriptor_type)),
        }
    }
}

impl fmt::Display for InterfaceDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  Interface Descriptor:")?;
        writeln!(f, "    bLength: {} bytes", self.length)?;
        writeln!(f, "    bDescriptorType: {} ({})", self.descriptor_type.name(), self.descriptor_type.get_value())?;
        writeln!(f, "    bInterfaceNumber: {}", self.interface_number)?;
        writeln!(f, "    bAlternateSetting: {}", self.alternate_setting)?;
        writeln!(f, "    bNumEndpoints: {}", self.num_endpoints)?;
        writeln!(f, "    bInterfaceClass: 0x{:02x} ({})", self.interface_class.get_value(), self.interface_class.name())?;
        writeln!(f, "    bInterfaceSubClass: 0x{:02x}", self.interface_subclass)?;
        writeln!(f, "    bInterfaceProtocol: 0x{:02x}", self.interface_protocol)?;
        writeln!(f, "    iInterface: {}", self.interface_index)?;
        if let Some(ref s) = self.interface_string {
            writeln!(f, "      Interface: {}", s)?;
        }
        
        // Display class-specific descriptors if any
        if !self.class_specific.is_empty() {
            writeln!(f, "    Class-Specific Descriptors:")?;
            for (i, descriptor) in self.class_specific.iter().enumerate() {
                writeln!(f, "      Descriptor {}: {} bytes", i, descriptor.len())?;
            }
        }
        
        // Display endpoints
        for endpoint in &self.endpoints {
            write!(f, "{}", endpoint)?;
        }
        
        Ok(())
    }
}

// USB Endpoint Descriptor
#[derive(Debug, Clone)]
pub struct EndpointDescriptor {
    pub length: u8,                    // Descriptor size in bytes (7)
    pub descriptor_type: UsbDescriptorType, // ENDPOINT descriptor type (5)
    pub endpoint_address: u8,          // Endpoint address (includes direction bit)
    pub attributes: u8,                // Endpoint attributes
    pub max_packet_size: u16,          // Maximum packet size
    pub interval: u8,                  // Interval for polling endpoint (frames)
    
    // Derived fields
    pub endpoint_number: u8,           // Endpoint number (0-15)
    pub direction: UsbEndpointDirection, // Direction (IN or OUT)
    pub transfer_type: UsbEndpointType, // Transfer type (Control, Isochronous, Bulk, Interrupt)
    pub sync_type: Option<UsbIsoSyncType>, // Synchronization type (only for isochronous)
    pub usage_type: Option<UsbIsoUsageType>, // Usage type (only for isochronous)
}

impl EndpointDescriptor {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 7 {
            return Err(format!("Invalid endpoint descriptor length: {}", data.len()));
        }
        
        let length = data[0];
        let descriptor_type = UsbDescriptorType::from(data[1]);
        
        match descriptor_type {
            UsbDescriptorType::Endpoint => {
                let endpoint_address = data[2];
                let attributes = data[3];
                
                // Handle 2-byte or 3-byte max packet size
                let max_packet_size = if data.len() >= 7 {
                    (data[5] as u16) << 8 | (data[4] as u16)
                } else {
                    data[4] as u16
                };
                
                let interval = if data.len() >= 7 { data[6] } else { 0 };
                
                // Derived fields
                let endpoint_number = endpoint_address & 0x0F;
                let direction = UsbEndpointDirection::from(endpoint_address);
                let transfer_type = UsbEndpointType::from(attributes);
                
                // Synchronization and usage types are only valid for isochronous endpoints
                let (sync_type, usage_type) = if transfer_type == UsbEndpointType::Isochronous {
                    (Some(UsbIsoSyncType::from(attributes)), 
                     Some(UsbIsoUsageType::from(attributes)))
                } else {
                    (None, None)
                };
                
                Ok(EndpointDescriptor {
                    length,
                    descriptor_type,
                    endpoint_address,
                    attributes,
                    max_packet_size,
                    interval,
                    endpoint_number,
                    direction,
                    transfer_type,
                    sync_type,
                    usage_type,
                })
            },
            _ => Err(format!("Invalid descriptor type: {:?}", descriptor_type)),
        }
    }
}

impl fmt::Display for EndpointDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "    Endpoint Descriptor:")?;
        writeln!(f, "      bLength: {} bytes", self.length)?;
        writeln!(f, "      bDescriptorType: {} ({})", self.descriptor_type.name(), self.descriptor_type.get_value())?;
        writeln!(f, "      bEndpointAddress: 0x{:02x} (EP{} {})", 
            self.endpoint_address, 
            self.endpoint_number,
            self.direction.name())?;
        writeln!(f, "      bmAttributes: 0x{:02x}", self.attributes)?;
        writeln!(f, "        Transfer Type: {}", self.transfer_type.name())?;
        
        // Display synchronization and usage types for isochronous endpoints
        if self.transfer_type == UsbEndpointType::Isochronous {
            if let Some(sync_type) = self.sync_type {
                writeln!(f, "        Synchronization Type: {}", sync_type.name())?;
            }
            if let Some(usage_type) = self.usage_type {
                writeln!(f, "        Usage Type: {}", usage_type.name())?;
            }
        }
        
        writeln!(f, "      wMaxPacketSize: {} bytes", self.max_packet_size)?;
        writeln!(f, "      bInterval: {} {}", self.interval, 
            if self.transfer_type == UsbEndpointType::Isochronous || 
               self.transfer_type == UsbEndpointType::Interrupt {
                "frames"
            } else {
                "ms"
            })?;
        
        Ok(())
    }
}

// USB String Descriptor
#[derive(Debug, Clone)]
pub struct StringDescriptor {
    pub length: u8,                    // Descriptor size in bytes
    pub descriptor_type: UsbDescriptorType, // STRING descriptor type (3)
    pub string: String,                // Unicode string
    pub string_index: u8,              // Index of this string descriptor
}

impl StringDescriptor {
    pub fn parse(data: &[u8], index: u8) -> Result<Self, String> {
        if data.len() < 2 {
            return Err(format!("Invalid string descriptor length: {}", data.len()));
        }
        
        let length = data[0];
        let descriptor_type = UsbDescriptorType::from(data[1]);
        
        match descriptor_type {
            UsbDescriptorType::String => {
                if data.len() < length as usize {
                    return Err(format!("String descriptor truncated: {} < {}", data.len(), length));
                }
                
                // Convert UTF-16LE to Rust String
                let mut string = String::new();
                let str_data = &data[2..length as usize];
                
                for i in (0..str_data.len()).step_by(2) {
                    if i + 1 < str_data.len() {
                        let c = u16::from_le_bytes([str_data[i], str_data[i + 1]]);
                        if let Some(ch) = std::char::from_u32(c as u32) {
                            string.push(ch);
                        }
                    }
                }
                
                Ok(StringDescriptor {
                    length,
                    descriptor_type,
                    string,
                    string_index: index,
                })
            },
            _ => Err(format!("Invalid descriptor type: {:?}", descriptor_type)),
        }
    }
}

impl fmt::Display for StringDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "String Descriptor:")?;
        writeln!(f, "  Index: {}", self.string_index)?;
        writeln!(f, "  bLength: {} bytes", self.length)?;
        writeln!(f, "  bDescriptorType: {} ({})", self.descriptor_type.name(), self.descriptor_type.get_value())?;
        writeln!(f, "  String: \"{}\"", self.string)
    }
}

// USB Device Qualifier Descriptor
#[derive(Debug, Clone)]
pub struct DeviceQualifierDescriptor {
    pub length: u8,                // Descriptor size in bytes (10)
    pub descriptor_type: UsbDescriptorType, // DEVICE_QUALIFIER descriptor type (6)
    pub usb_version: u16,          // USB specification release number (BCD)
    pub device_class: UsbDeviceClass, // USB device class code
    pub device_subclass: u8,       // USB device subclass code
    pub device_protocol: u8,       // USB device protocol code
    pub max_packet_size0: u8,      // Maximum packet size for endpoint 0
    pub num_configurations: u8,    // Number of possible configurations
    pub reserved: u8,              // Reserved for future use, must be zero
}

impl DeviceQualifierDescriptor {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 10 {
            return Err(format!("Invalid device qualifier descriptor length: {}", data.len()));
        }
        
        let length = data[0];
        let descriptor_type = UsbDescriptorType::from(data[1]);
        
        match descriptor_type {
            UsbDescriptorType::DeviceQualifier => {
                let usb_version = (data[3] as u16) << 8 | (data[2] as u16);
                let device_class = UsbDeviceClass::from(data[4]);
                let device_subclass = data[5];
                let device_protocol = data[6];
                let max_packet_size0 = data[7];
                let num_configurations = data[8];
                let reserved = data[9];
                
                Ok(DeviceQualifierDescriptor {
                    length,
                    descriptor_type,
                    usb_version,
                    device_class,
                    device_subclass,
                    device_protocol,
                    max_packet_size0,
                    num_configurations,
                    reserved,
                })
            },
            _ => Err(format!("Invalid descriptor type: {:?}", descriptor_type)),
        }
    }
    
    pub fn usb_version_string(&self) -> String {
        let major = (self.usb_version >> 8) & 0xFF;
        let minor = (self.usb_version >> 4) & 0xF;
        let subminor = self.usb_version & 0xF;
        
        format!("{}.{}.{}", major, minor, subminor)
    }
}

impl fmt::Display for DeviceQualifierDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Device Qualifier Descriptor:")?;
        writeln!(f, "  bLength: {} bytes", self.length)?;
        writeln!(f, "  bDescriptorType: {} ({})", self.descriptor_type.name(), self.descriptor_type.get_value())?;
        writeln!(f, "  bcdUSB: {} (USB {})", self.usb_version, self.usb_version_string())?;
        writeln!(f, "  bDeviceClass: 0x{:02x} ({})", self.device_class.get_value(), self.device_class.name())?;
        writeln!(f, "  bDeviceSubClass: 0x{:02x}", self.device_subclass)?;
        writeln!(f, "  bDeviceProtocol: 0x{:02x}", self.device_protocol)?;
        writeln!(f, "  bMaxPacketSize0: {} bytes", self.max_packet_size0)?;
        writeln!(f, "  bNumConfigurations: {}", self.num_configurations)?;
        writeln!(f, "  bReserved: 0x{:02x}", self.reserved)
    }
}

// Main structure to hold all parsed USB descriptors
#[derive(Debug, Clone)]
pub struct UsbDevice {
    pub device: Option<DeviceDescriptor>,
    pub configurations: Vec<ConfigurationDescriptor>,
    pub strings: Vec<StringDescriptor>,
    pub device_qualifier: Option<DeviceQualifierDescriptor>,
    
    // Raw descriptor data
    pub raw_descriptors: Vec<Vec<u8>>,
}

impl UsbDevice {
    pub fn new() -> Self {
        UsbDevice {
            device: None,
            configurations: Vec::new(),
            strings: Vec::new(),
            device_qualifier: None,
            raw_descriptors: Vec::new(),
        }
    }
    
    pub fn parse_descriptors(&mut self, data: &[u8]) -> Result<(), String> {
        let mut offset = 0;
        
        while offset < data.len() {
            if offset + 2 > data.len() {
                break; // Not enough data for length and type
            }
            
            let length = data[offset];
            if length == 0 {
                // Invalid descriptor, skip
                offset += 1;
                continue;
            }
            
            let end = offset + length as usize;
            if end > data.len() {
                break; // Not enough data for complete descriptor
            }
            
            let descriptor_data = &data[offset..end];
            self.raw_descriptors.push(descriptor_data.to_vec());
            
            let descriptor_type = UsbDescriptorType::from(data[offset + 1]);
            
            match descriptor_type {
                UsbDescriptorType::Device => {
                    if let Ok(device) = DeviceDescriptor::parse(descriptor_data) {
                        self.device = Some(device);
                    }
                },
                UsbDescriptorType::Configuration => {
                    if let Ok(config) = ConfigurationDescriptor::parse(descriptor_data) {
                        self.configurations.push(config);
                    }
                },
                UsbDescriptorType::String => {
                    // For string descriptors, we need to keep track of their index
                    let index = self.strings.len() as u8;
                    if let Ok(string) = StringDescriptor::parse(descriptor_data, index) {
                        self.strings.push(string);
                    }
                },
                UsbDescriptorType::DeviceQualifier => {
                    if let Ok(qualifier) = DeviceQualifierDescriptor::parse(descriptor_data) {
                        self.device_qualifier = Some(qualifier);
                    }
                },
                _ => {
                    // We'll process Interface and Endpoint descriptors when linking everything
                }
            }
            
            offset = end;
        }
        
        self.link_descriptors();
        
        Ok(())
    }
    
    // Link descriptors together (configurations -> interfaces -> endpoints)
    // and fill in string descriptors
    fn link_descriptors(&mut self) {
        // Process each configuration descriptor
        for i in 0..self.configurations.len() {
            let mut interfaces = Vec::new();
            let mut current_interface: Option<InterfaceDescriptor> = None;
            
            // Find all interface and endpoint descriptors for this configuration
            for raw_desc in &self.raw_descriptors {
                if raw_desc.len() < 2 {
                    continue;
                }
                
                let desc_type = UsbDescriptorType::from(raw_desc[1]);
                
                match desc_type {
                    UsbDescriptorType::Interface => {
                        // If we have a current interface, add it to our list
                        if let Some(iface) = current_interface.take() {
                            interfaces.push(iface);
                        }
                        
                        // Parse new interface
                        if let Ok(iface) = InterfaceDescriptor::parse(raw_desc) {
                            current_interface = Some(iface);
                        }
                    },
                    UsbDescriptorType::Endpoint => {
                        // If we have a current interface, add this endpoint to it
                        if let Some(ref mut iface) = current_interface {
                            if let Ok(endpoint) = EndpointDescriptor::parse(raw_desc) {
                                iface.endpoints.push(endpoint);
                            }
                        }
                    },
                    _ => {
                        // For other descriptor types within a configuration,
                        // check if it might be a class-specific descriptor
                        if let Some(ref mut iface) = current_interface {
                            if desc_type.get_value() >= 0x21 && desc_type.get_value() <= 0x2F {
                                // This is likely a class-specific descriptor
                                iface.class_specific.push(raw_desc.clone());
                            }
                        }
                    }
                }
            }
            
            // Add the last interface if there is one
            if let Some(iface) = current_interface {
                interfaces.push(iface);
            }
            
            // Add interfaces to the configuration
            if let Some(config) = self.configurations.get_mut(i) {
                config.interfaces = interfaces;
            }
        }
        
        // Fill in string descriptors
        if let Some(ref mut device) = self.device {
            // Manufacturer string
            if device.manufacturer_index > 0 && (device.manufacturer_index as usize) < self.strings.len() {
                let string = &self.strings[device.manufacturer_index as usize];
                device.manufacturer_string = Some(string.string.clone());
            }
            
            // Product string
            if device.product_index > 0 && (device.product_index as usize) < self.strings.len() {
                let string = &self.strings[device.product_index as usize];
                device.product_string = Some(string.string.clone());
            }
            
            // Serial number string
            if device.serial_number_index > 0 && (device.serial_number_index as usize) < self.strings.len() {
                let string = &self.strings[device.serial_number_index as usize];
                device.serial_number_string = Some(string.string.clone());
            }
        }
        
        // Configuration strings
        for config in &mut self.configurations {
            if config.configuration_index > 0 && (config.configuration_index as usize) < self.strings.len() {
                let string = &self.strings[config.configuration_index as usize];
                config.configuration_string = Some(string.string.clone());
            }
            
            // Interface strings
            for iface in &mut config.interfaces {
                if iface.interface_index > 0 && (iface.interface_index as usize) < self.strings.len() {
                    let string = &self.strings[iface.interface_index as usize];
                    iface.interface_string = Some(string.string.clone());
                }
            }
        }
    }
}

impl fmt::Display for UsbDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "USB Device Descriptors:")?;
        
        if let Some(ref device) = self.device {
            writeln!(f, "{}", device)?;
        }
        
        if let Some(ref qualifier) = self.device_qualifier {
            writeln!(f, "{}", qualifier)?;
        }
        
        for config in &self.configurations {
            writeln!(f, "{}", config)?;
        }
        
        writeln!(f, "String Descriptors:")?;
        for string in &self.strings {
            writeln!(f, "  [{}]: \"{}\"", string.string_index, string.string)?;
        }
        
        Ok(())
    }
}