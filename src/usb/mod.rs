pub mod descriptors;
pub mod descriptor_types;
pub mod decoder;
pub mod hints;
pub mod mitm_traffic;

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
    UsbDevice,
    USBDescriptor,
    // StringDescriptor and DeviceQualifierDescriptor are still available directly from descriptors module
};

pub use self::decoder::{DecodedUSBData, UsbDecoder};

// Re-export the essential MitM traffic types
pub use self::mitm_traffic::{
    generate_simulated_mitm_traffic,
    decode_mitm_packet,
    UsbTransaction
};
