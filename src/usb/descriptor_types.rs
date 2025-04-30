// USB Descriptor Types
// Based on USB 2.0 and 3.0 specifications

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum UsbDescriptorType {
    // Standard descriptor types (USB 2.0)
    Device = 0x01,
    Configuration = 0x02,
    String = 0x03,
    Interface = 0x04,
    Endpoint = 0x05,
    DeviceQualifier = 0x06,
    OtherSpeedConfiguration = 0x07,
    InterfacePower = 0x08,
    
    // Class-specific descriptor types
    Hid = 0x21,
    Report = 0x22,
    PhysicalDescriptor = 0x23,
    Hub = 0x29,
    
    // USB 3.0 descriptor types
    SuperspeedUsbEndpointCompanion = 0x30,
    SuperspeedPlusIsochronousEndpointCompanion = 0x31,
    
    // Other common types
    Bos = 0x0F,
    DeviceCapability = 0x10,
    
    // Unknown type
    Unknown(u8),
}

impl From<u8> for UsbDescriptorType {
    fn from(value: u8) -> Self {
        match value {
            0x01 => UsbDescriptorType::Device,
            0x02 => UsbDescriptorType::Configuration,
            0x03 => UsbDescriptorType::String,
            0x04 => UsbDescriptorType::Interface,
            0x05 => UsbDescriptorType::Endpoint,
            0x06 => UsbDescriptorType::DeviceQualifier,
            0x07 => UsbDescriptorType::OtherSpeedConfiguration,
            0x08 => UsbDescriptorType::InterfacePower,
            0x21 => UsbDescriptorType::Hid,
            0x22 => UsbDescriptorType::Report,
            0x23 => UsbDescriptorType::PhysicalDescriptor,
            0x29 => UsbDescriptorType::Hub,
            0x0F => UsbDescriptorType::Bos,
            0x10 => UsbDescriptorType::DeviceCapability,
            0x30 => UsbDescriptorType::SuperspeedUsbEndpointCompanion,
            0x31 => UsbDescriptorType::SuperspeedPlusIsochronousEndpointCompanion,
            _ => UsbDescriptorType::Unknown(value),
        }
    }
}

// Implement UpperHex trait for UsbDescriptorType
impl fmt::UpperHex for UsbDescriptorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02X}", self.get_value())
    }
}

impl UsbDescriptorType {
    pub fn get_value(&self) -> u8 {
        match self {
            UsbDescriptorType::Device => 0x01,
            UsbDescriptorType::Configuration => 0x02,
            UsbDescriptorType::String => 0x03,
            UsbDescriptorType::Interface => 0x04,
            UsbDescriptorType::Endpoint => 0x05,
            UsbDescriptorType::DeviceQualifier => 0x06,
            UsbDescriptorType::OtherSpeedConfiguration => 0x07,
            UsbDescriptorType::InterfacePower => 0x08,
            UsbDescriptorType::Hid => 0x21,
            UsbDescriptorType::Report => 0x22,
            UsbDescriptorType::PhysicalDescriptor => 0x23,
            UsbDescriptorType::Hub => 0x29,
            UsbDescriptorType::Bos => 0x0F,
            UsbDescriptorType::DeviceCapability => 0x10,
            UsbDescriptorType::SuperspeedUsbEndpointCompanion => 0x30,
            UsbDescriptorType::SuperspeedPlusIsochronousEndpointCompanion => 0x31,
            UsbDescriptorType::Unknown(value) => *value,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            UsbDescriptorType::Device => "Device Descriptor",
            UsbDescriptorType::Configuration => "Configuration Descriptor",
            UsbDescriptorType::String => "String Descriptor",
            UsbDescriptorType::Interface => "Interface Descriptor",
            UsbDescriptorType::Endpoint => "Endpoint Descriptor",
            UsbDescriptorType::DeviceQualifier => "Device Qualifier Descriptor",
            UsbDescriptorType::OtherSpeedConfiguration => "Other Speed Configuration Descriptor",
            UsbDescriptorType::InterfacePower => "Interface Power Descriptor",
            UsbDescriptorType::Hid => "HID Descriptor",
            UsbDescriptorType::Report => "Report Descriptor",
            UsbDescriptorType::PhysicalDescriptor => "Physical Descriptor",
            UsbDescriptorType::Hub => "Hub Descriptor",
            UsbDescriptorType::Bos => "BOS Descriptor",
            UsbDescriptorType::DeviceCapability => "Device Capability Descriptor",
            UsbDescriptorType::SuperspeedUsbEndpointCompanion => "SuperSpeed USB Endpoint Companion Descriptor",
            UsbDescriptorType::SuperspeedPlusIsochronousEndpointCompanion => "SuperSpeed Plus Isochronous Endpoint Companion Descriptor",
            UsbDescriptorType::Unknown(_) => "Unknown Descriptor Type",
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            UsbDescriptorType::Device => "Provides basic information about the device such as USB version, vendor/product IDs, and configuration count.",
            UsbDescriptorType::Configuration => "Describes a specific device configuration including power requirements and interfaces.",
            UsbDescriptorType::String => "Contains human-readable text like manufacturer name, product name, or serial number.",
            UsbDescriptorType::Interface => "Describes a specific interface within a configuration, defining its class, subclass, and protocol.",
            UsbDescriptorType::Endpoint => "Describes an endpoint's characteristics like direction, transfer type, and maximum packet size.",
            UsbDescriptorType::DeviceQualifier => "Describes device information for an alternate USB speed (high/full speed compatibility).",
            UsbDescriptorType::OtherSpeedConfiguration => "Describes a configuration for an alternate speed operation.",
            UsbDescriptorType::InterfacePower => "Provides information about interface power management capabilities.",
            UsbDescriptorType::Hid => "Describes a Human Interface Device class and its properties.",
            UsbDescriptorType::Report => "Provides detailed information about a HID device's data format.",
            UsbDescriptorType::PhysicalDescriptor => "Describes the physical aspects of a human input device.",
            UsbDescriptorType::Hub => "Describes a USB hub and its characteristics.",
            UsbDescriptorType::Bos => "USB 3.0 Binary Device Object Store descriptor that provides device-level capabilities.",
            UsbDescriptorType::DeviceCapability => "Describes specific device capabilities (USB 3.0).",
            UsbDescriptorType::SuperspeedUsbEndpointCompanion => "Additional information for SuperSpeed USB endpoints.",
            UsbDescriptorType::SuperspeedPlusIsochronousEndpointCompanion => "Additional information for SuperSpeed Plus isochronous endpoints.",
            UsbDescriptorType::Unknown(_) => "Unknown or vendor-specific descriptor type.",
        }
    }
}

// USB Device Classes based on USB-IF specifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UsbDeviceClass {
    UseInterfaceDescriptor,
    Audio,
    Communications,
    HumanInterfaceDevice,
    Physical,
    Image,
    Printer,
    MassStorage,
    Hub,
    CdcData,
    SmartCard,
    ContentSecurity,
    Video,
    PersonalHealthcare,
    AudioVideo,
    Billboard,
    UsbTypeCBridge,
    Diagnostic,
    WirelessController,
    Miscellaneous,
    ApplicationSpecific,
    VendorSpecific,
    // Store the raw value separately
    #[serde(skip)]
    Unknown(#[serde(skip_serializing)] u8)
}

impl From<u8> for UsbDeviceClass {
    fn from(value: u8) -> Self {
        match value {
            0x00 => UsbDeviceClass::UseInterfaceDescriptor,
            0x01 => UsbDeviceClass::Audio,
            0x02 => UsbDeviceClass::Communications,
            0x03 => UsbDeviceClass::HumanInterfaceDevice,
            0x05 => UsbDeviceClass::Physical,
            0x06 => UsbDeviceClass::Image,
            0x07 => UsbDeviceClass::Printer,
            0x08 => UsbDeviceClass::MassStorage,
            0x09 => UsbDeviceClass::Hub,
            0x0A => UsbDeviceClass::CdcData,
            0x0B => UsbDeviceClass::SmartCard,
            0x0D => UsbDeviceClass::ContentSecurity,
            0x0E => UsbDeviceClass::Video,
            0x0F => UsbDeviceClass::PersonalHealthcare,
            0x10 => UsbDeviceClass::AudioVideo,
            0x11 => UsbDeviceClass::Billboard,
            0x12 => UsbDeviceClass::UsbTypeCBridge,
            0xDC => UsbDeviceClass::Diagnostic,
            0xE0 => UsbDeviceClass::WirelessController,
            0xEF => UsbDeviceClass::Miscellaneous,
            0xFE => UsbDeviceClass::ApplicationSpecific,
            0xFF => UsbDeviceClass::VendorSpecific,
            _ => UsbDeviceClass::Unknown(value),
        }
    }
}

impl UsbDeviceClass {
    pub fn get_value(&self) -> u8 {
        match self {
            UsbDeviceClass::UseInterfaceDescriptor => 0x00,
            UsbDeviceClass::Audio => 0x01,
            UsbDeviceClass::Communications => 0x02,
            UsbDeviceClass::HumanInterfaceDevice => 0x03,
            UsbDeviceClass::Physical => 0x05,
            UsbDeviceClass::Image => 0x06,
            UsbDeviceClass::Printer => 0x07,
            UsbDeviceClass::MassStorage => 0x08,
            UsbDeviceClass::Hub => 0x09,
            UsbDeviceClass::CdcData => 0x0A,
            UsbDeviceClass::SmartCard => 0x0B,
            UsbDeviceClass::ContentSecurity => 0x0D,
            UsbDeviceClass::Video => 0x0E,
            UsbDeviceClass::PersonalHealthcare => 0x0F,
            UsbDeviceClass::AudioVideo => 0x10,
            UsbDeviceClass::Billboard => 0x11,
            UsbDeviceClass::UsbTypeCBridge => 0x12,
            UsbDeviceClass::Diagnostic => 0xDC,
            UsbDeviceClass::WirelessController => 0xE0,
            UsbDeviceClass::Miscellaneous => 0xEF,
            UsbDeviceClass::ApplicationSpecific => 0xFE,
            UsbDeviceClass::VendorSpecific => 0xFF,
            UsbDeviceClass::Unknown(value) => *value,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            UsbDeviceClass::UseInterfaceDescriptor => "Use Interface Descriptor",
            UsbDeviceClass::Audio => "Audio",
            UsbDeviceClass::Communications => "Communications and CDC Control",
            UsbDeviceClass::HumanInterfaceDevice => "Human Interface Device (HID)",
            UsbDeviceClass::Physical => "Physical",
            UsbDeviceClass::Image => "Image",
            UsbDeviceClass::Printer => "Printer",
            UsbDeviceClass::MassStorage => "Mass Storage",
            UsbDeviceClass::Hub => "Hub",
            UsbDeviceClass::CdcData => "CDC-Data",
            UsbDeviceClass::SmartCard => "Smart Card",
            UsbDeviceClass::ContentSecurity => "Content Security",
            UsbDeviceClass::Video => "Video",
            UsbDeviceClass::PersonalHealthcare => "Personal Healthcare",
            UsbDeviceClass::AudioVideo => "Audio/Video Devices",
            UsbDeviceClass::Billboard => "Billboard Device",
            UsbDeviceClass::UsbTypeCBridge => "USB Type-C Bridge",
            UsbDeviceClass::Diagnostic => "Diagnostic Device",
            UsbDeviceClass::WirelessController => "Wireless Controller",
            UsbDeviceClass::Miscellaneous => "Miscellaneous",
            UsbDeviceClass::ApplicationSpecific => "Application Specific",
            UsbDeviceClass::VendorSpecific => "Vendor Specific",
            UsbDeviceClass::Unknown(_) => "Unknown",
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            UsbDeviceClass::UseInterfaceDescriptor => "Class information is provided at the interface level instead of the device level.",
            UsbDeviceClass::Audio => "Audio devices like speakers, microphones, and audio processing devices.",
            UsbDeviceClass::Communications => "Communication devices like modems, network adapters, and ethernet adapters.",
            UsbDeviceClass::HumanInterfaceDevice => "Input devices like keyboards, mice, game controllers, and other human interface devices.",
            UsbDeviceClass::Physical => "Devices related to physical activity and measurements.",
            UsbDeviceClass::Image => "Imaging devices like scanners and cameras.",
            UsbDeviceClass::Printer => "Printers and printing-related devices.",
            UsbDeviceClass::MassStorage => "Mass storage devices like USB flash drives, external hard drives, and card readers.",
            UsbDeviceClass::Hub => "USB hub devices for connecting multiple USB devices.",
            UsbDeviceClass::CdcData => "Communication Device Class (CDC) data interfaces.",
            UsbDeviceClass::SmartCard => "Smart card readers and related devices.",
            UsbDeviceClass::ContentSecurity => "Content security devices for copyright protection.",
            UsbDeviceClass::Video => "Video devices like webcams and video capture cards.",
            UsbDeviceClass::PersonalHealthcare => "Healthcare devices like blood pressure monitors and glucose meters.",
            UsbDeviceClass::AudioVideo => "Combined audio and video devices.",
            UsbDeviceClass::Billboard => "Device used to display information to the user.",
            UsbDeviceClass::UsbTypeCBridge => "USB Type-C Bridge devices for alternate modes.",
            UsbDeviceClass::Diagnostic => "Diagnostic and programming devices.",
            UsbDeviceClass::WirelessController => "Wireless controllers like Bluetooth adapters and RF controllers.",
            UsbDeviceClass::Miscellaneous => "Devices that don't fit into other classes but aren't vendor-specific.",
            UsbDeviceClass::ApplicationSpecific => "Devices with functionality specific to particular applications.",
            UsbDeviceClass::VendorSpecific => "Devices with custom, vendor-defined functionality.",
            UsbDeviceClass::Unknown(_) => "Device class not recognized or not defined in standards.",
        }
    }
}

// USB Endpoint Types
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UsbEndpointType {
    Control,
    Isochronous,
    Bulk,
    Interrupt,
    #[serde(skip)]
    Unknown(#[serde(skip_serializing)] u8),
}

impl From<u8> for UsbEndpointType {
    fn from(value: u8) -> Self {
        match value & 0x03 {
            0 => UsbEndpointType::Control,
            1 => UsbEndpointType::Isochronous,
            2 => UsbEndpointType::Bulk,
            3 => UsbEndpointType::Interrupt,
            _ => UsbEndpointType::Unknown(value),
        }
    }
}

impl UsbEndpointType {
    pub fn get_value(&self) -> u8 {
        match self {
            UsbEndpointType::Control => 0,
            UsbEndpointType::Isochronous => 1,
            UsbEndpointType::Bulk => 2,
            UsbEndpointType::Interrupt => 3,
            UsbEndpointType::Unknown(value) => *value,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            UsbEndpointType::Control => "Control",
            UsbEndpointType::Isochronous => "Isochronous",
            UsbEndpointType::Bulk => "Bulk",
            UsbEndpointType::Interrupt => "Interrupt",
            UsbEndpointType::Unknown(_) => "Unknown",
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            UsbEndpointType::Control => "Used for device control and status. Guaranteed delivery with error checking.",
            UsbEndpointType::Isochronous => "Guaranteed timing but not data integrity. Used for real-time data like audio/video.",
            UsbEndpointType::Bulk => "Large data transfers with error checking but no guaranteed timing. Used for printers, storage.",
            UsbEndpointType::Interrupt => "Small data transfers with guaranteed latency. Used for time-sensitive devices like mice/keyboards.",
            UsbEndpointType::Unknown(_) => "Endpoint type not recognized in USB specification.",
        }
    }
}

// USB Endpoint Directions
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UsbEndpointDirection {
    Out = 0, // Host to device
    In = 1,  // Device to host
    // No Unknown variant needed as this is binary
}

impl From<u8> for UsbEndpointDirection {
    fn from(value: u8) -> Self {
        if (value & 0x80) == 0x80 {
            UsbEndpointDirection::In
        } else {
            UsbEndpointDirection::Out
        }
    }
}

impl UsbEndpointDirection {
    pub fn get_value(&self) -> u8 {
        match self {
            UsbEndpointDirection::Out => 0,
            UsbEndpointDirection::In => 0x80,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            UsbEndpointDirection::Out => "OUT (Host to Device)",
            UsbEndpointDirection::In => "IN (Device to Host)",
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            UsbEndpointDirection::Out => "Data flows from host to device (outbound).",
            UsbEndpointDirection::In => "Data flows from device to host (inbound).",
        }
    }
}

// Isochronous Synchronization Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum UsbIsoSyncType {
    NoSync = 0,
    Asynchronous = 1,
    Adaptive = 2,
    Synchronous = 3,
    #[serde(skip)]
    Unknown(#[serde(skip_serializing)] u8) = 4,
}

impl From<u8> for UsbIsoSyncType {
    fn from(value: u8) -> Self {
        match (value >> 2) & 0x03 {
            0 => UsbIsoSyncType::NoSync,
            1 => UsbIsoSyncType::Asynchronous,
            2 => UsbIsoSyncType::Adaptive,
            3 => UsbIsoSyncType::Synchronous,
            _ => UsbIsoSyncType::Unknown(value),
        }
    }
}

impl UsbIsoSyncType {
    pub fn get_value(&self) -> u8 {
        match self {
            UsbIsoSyncType::NoSync => 0,
            UsbIsoSyncType::Asynchronous => 1,
            UsbIsoSyncType::Adaptive => 2,
            UsbIsoSyncType::Synchronous => 3,
            UsbIsoSyncType::Unknown(value) => *value,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            UsbIsoSyncType::NoSync => "No Synchronization",
            UsbIsoSyncType::Asynchronous => "Asynchronous",
            UsbIsoSyncType::Adaptive => "Adaptive",
            UsbIsoSyncType::Synchronous => "Synchronous",
            UsbIsoSyncType::Unknown(_) => "Unknown",
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            UsbIsoSyncType::NoSync => "No synchronization supported.",
            UsbIsoSyncType::Asynchronous => "Data rate is synchronized to its own clock.",
            UsbIsoSyncType::Adaptive => "Data rate adapts to the host's data rate.",
            UsbIsoSyncType::Synchronous => "Data rate is synchronized to the USB's Start of Frame.",
            UsbIsoSyncType::Unknown(_) => "Synchronization type not recognized in USB specification.",
        }
    }
}

// Isochronous Usage Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum UsbIsoUsageType {
    Data = 0,
    Feedback = 1,
    ImplicitFeedback = 2,
    Reserved = 3,
    #[serde(skip)]
    Unknown(#[serde(skip_serializing)] u8) = 4,
}

impl From<u8> for UsbIsoUsageType {
    fn from(value: u8) -> Self {
        match (value >> 4) & 0x03 {
            0 => UsbIsoUsageType::Data,
            1 => UsbIsoUsageType::Feedback,
            2 => UsbIsoUsageType::ImplicitFeedback,
            3 => UsbIsoUsageType::Reserved,
            _ => UsbIsoUsageType::Unknown(value),
        }
    }
}

impl UsbIsoUsageType {
    pub fn get_value(&self) -> u8 {
        match self {
            UsbIsoUsageType::Data => 0,
            UsbIsoUsageType::Feedback => 1,
            UsbIsoUsageType::ImplicitFeedback => 2,
            UsbIsoUsageType::Reserved => 3,
            UsbIsoUsageType::Unknown(value) => *value,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            UsbIsoUsageType::Data => "Data",
            UsbIsoUsageType::Feedback => "Feedback",
            UsbIsoUsageType::ImplicitFeedback => "Implicit Feedback Data",
            UsbIsoUsageType::Reserved => "Reserved",
            UsbIsoUsageType::Unknown(_) => "Unknown",
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            UsbIsoUsageType::Data => "Standard data endpoint.",
            UsbIsoUsageType::Feedback => "Provides feedback about the data rate or timing.",
            UsbIsoUsageType::ImplicitFeedback => "Data endpoint that also contains feedback information.",
            UsbIsoUsageType::Reserved => "Reserved for future use in USB specification.",
            UsbIsoUsageType::Unknown(_) => "Usage type not recognized in USB specification.",
        }
    }
}