pub mod descriptors;
pub mod descriptor_types;
pub mod decoder;
pub mod hints;

// Re-export commonly used types for easier access
pub use self::descriptor_types::{
    UsbDescriptorType,
    UsbDeviceClass,
    UsbEndpointType,
    UsbIsoSyncType,
    UsbIsoUsageType,
};

pub use self::descriptors::{
    DeviceDescriptor,
    ConfigurationDescriptor,
    InterfaceDescriptor,
    EndpointDescriptor,
    UsbDevice,
    USBDescriptor,
    StringDescriptor,
    DeviceQualifierDescriptor,
};

pub use self::decoder::{DecodedUSBData, UsbDecoder};
