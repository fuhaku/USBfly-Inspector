pub mod descriptors;
pub mod descriptor_types;
pub mod decoder;
pub mod hints;
pub mod mitm_traffic;
pub mod packet_types;

// Re-export commonly used types for easier access
pub use self::descriptor_types::{
    UsbDescriptorType,
    UsbDeviceClass,
    UsbEndpointType,
    UsbIsoSyncType,
    // UsbIsoUsageType removed as it wasn't used
};

pub use self::descriptors::{
    DeviceDescriptor,
    ConfigurationDescriptor,
    InterfaceDescriptor,
    EndpointDescriptor,
    USBDescriptor,
    // StringDescriptor and DeviceQualifierDescriptor are still available directly from descriptors module
};

pub use self::decoder::{DecodedUSBData, UsbDecoder, Speed};

// No need to re-export packet_types as it's already a public module

// Keep this import area for future MitM traffic types as needed
// Currently they're directly imported where used
