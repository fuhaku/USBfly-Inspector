use crate::usb::{
    UsbDescriptorType, UsbDeviceClass, UsbEndpointType, UsbIsoSyncType, UsbIsoUsageType,
    DeviceDescriptor, ConfigurationDescriptor, InterfaceDescriptor, EndpointDescriptor
};

// This module provides contextual hints and explanations for USB descriptors
// These hints help users understand the meaning and implications of different USB values

pub struct UsbHints;

// Function to get descriptor hints for a specific descriptor
pub fn get_descriptor_hints(descriptor_type: &UsbDescriptorType) -> String {
    UsbHints::for_descriptor_type(descriptor_type)
}

impl UsbHints {
    // Get a hint for a descriptor type
    pub fn for_descriptor_type(descriptor_type: &UsbDescriptorType) -> String {
        match descriptor_type {
            UsbDescriptorType::Device => 
                "The Device Descriptor provides essential information about the USB device including \
                vendor ID, product ID, device class, and power requirements.".to_string(),
            
            UsbDescriptorType::Configuration => 
                "The Configuration Descriptor defines a specific operating mode for the device, \
                including its power requirements and the interfaces it provides.".to_string(),
            
            UsbDescriptorType::String => 
                "String Descriptors contain human-readable text like manufacturer name, \
                product name, and serial number.".to_string(),
            
            UsbDescriptorType::Interface => 
                "Interface Descriptors define a specific function of the device. A device may \
                have multiple interfaces for different functions (e.g., a webcam with video and audio).".to_string(),
            
            UsbDescriptorType::Endpoint => 
                "Endpoint Descriptors define communication channels between the host and device. \
                Each endpoint has a direction (IN/OUT) and transfer type (Control/Bulk/Interrupt/Isochronous).".to_string(),
            
            UsbDescriptorType::DeviceQualifier => 
                "The Device Qualifier Descriptor defines how a high-speed device behaves when \
                operating at a different speed (e.g., when a USB 2.0 device connects to a USB 1.1 port).".to_string(),
            
            UsbDescriptorType::OtherSpeedConfiguration => 
                "This descriptor defines a configuration for when the device operates at a different speed \
                than its normal speed.".to_string(),
            
            UsbDescriptorType::InterfacePower => 
                "The Interface Power Descriptor defines power management capabilities for a specific interface.".to_string(),
            
            UsbDescriptorType::Hid => 
                "The HID Descriptor is specific to Human Interface Devices like keyboards and mice. \
                It defines report formats and other HID-specific information.".to_string(),
            
            UsbDescriptorType::Report => 
                "The Report Descriptor defines the data format for an HID device. It specifies \
                how input/output data is structured.".to_string(),
            
            UsbDescriptorType::PhysicalDescriptor => 
                "The Physical Descriptor describes the physical aspects of a human input device, \
                such as which body part (finger, hand) controls a specific input.".to_string(),
            
            UsbDescriptorType::Hub => 
                "The Hub Descriptor defines characteristics of a USB hub, including the number of ports \
                and power characteristics.".to_string(),
            
            UsbDescriptorType::Bos => 
                "The Binary Device Object Store (BOS) descriptor (USB 3.0) provides a way to access device-level \
                capabilities, like USB 2.0 extension capabilities.".to_string(),
            
            UsbDescriptorType::DeviceCapability => 
                "The Device Capability Descriptor provides information about specific capabilities \
                of the device, like USB 2.0 extensions, SuperSpeed capabilities, etc.".to_string(),
            
            UsbDescriptorType::SuperspeedUsbEndpointCompanion => 
                "This descriptor provides additional information about SuperSpeed USB endpoints, \
                complementing the standard endpoint descriptor.".to_string(),
            
            UsbDescriptorType::SuperspeedPlusIsochronousEndpointCompanion => 
                "This descriptor provides additional information specific to SuperSpeed Plus \
                isochronous endpoints.".to_string(),
            
            UsbDescriptorType::Unknown(_) => 
                "This is an unknown or vendor-specific descriptor type. It may contain proprietary \
                or device-specific information.".to_string(),
        }
    }
    
    // Get a hint for a device class
    pub fn for_device_class(device_class: &UsbDeviceClass) -> String {
        match device_class {
            UsbDeviceClass::UseInterfaceDescriptor => 
                "This device has no class at the device level. Each interface specifies its own class.\
                This is common for composite devices with multiple functions.".to_string(),
            
            UsbDeviceClass::Audio => 
                "Audio class devices handle sound input/output, like speakers, microphones, and audio interfaces.\
                They often use isochronous transfers for continuous audio data.".to_string(),
            
            UsbDeviceClass::Communications => 
                "Communication devices include modems, ethernet adapters, and other network interfaces.\
                They typically use bulk transfers for data and interrupt transfers for status notifications.".to_string(),
            
            UsbDeviceClass::HumanInterfaceDevice => 
                "Human Interface Devices (HID) include keyboards, mice, game controllers, and other input devices.\
                They use interrupt transfers for low-latency input reporting.".to_string(),
            
            UsbDeviceClass::Physical => 
                "Physical devices relate to physical activity and measurements, often used for fitness or medical applications.".to_string(),
            
            UsbDeviceClass::Image => 
                "Image class devices include scanners and cameras. They typically use bulk transfers\
                for image data and may include multiple interfaces.".to_string(),
            
            UsbDeviceClass::Printer => 
                "Printer class devices handle printing operations. They typically use bulk transfers\
                for print data.".to_string(),
            
            UsbDeviceClass::MassStorage => 
                "Mass Storage devices include USB drives, external hard drives, and card readers.\
                They use bulk transfers for data and follow specific protocols like SCSI.".to_string(),
            
            UsbDeviceClass::Hub => 
                "USB hubs allow multiple USB devices to connect through a single port.\
                They have a special status in the USB hierarchy.".to_string(),
            
            UsbDeviceClass::CdcData => 
                "CDC Data class is used for communication devices' data interfaces, often paired\
                with a Communications class control interface.".to_string(),
            
            UsbDeviceClass::SmartCard => 
                "Smart Card devices interface with smart cards and security tokens used for\
                authentication and secure operations.".to_string(),
            
            UsbDeviceClass::ContentSecurity => 
                "Content Security devices handle digital rights management and content protection,\
                often used in media applications.".to_string(),
            
            UsbDeviceClass::Video => 
                "Video class devices include webcams and video capture devices. They typically use\
                isochronous transfers for video data.".to_string(),
            
            UsbDeviceClass::PersonalHealthcare => 
                "Personal Healthcare devices include blood pressure monitors, glucose meters, and other\
                health monitoring equipment.".to_string(),
            
            UsbDeviceClass::AudioVideo => 
                "Audio/Video devices combine audio and video functionality, like webcams with microphones.\
                They typically have multiple interfaces.".to_string(),
            
            UsbDeviceClass::Billboard => 
                "Billboard devices display information to users, often about alternate modes or device capabilities.".to_string(),
            
            UsbDeviceClass::UsbTypeCBridge => 
                "USB Type-C Bridge devices facilitate alternate modes in USB Type-C connections,\
                like DisplayPort or Thunderbolt.".to_string(),
            
            UsbDeviceClass::Diagnostic => 
                "Diagnostic devices are used for debugging, testing, and measuring USB communications.".to_string(),
            
            UsbDeviceClass::WirelessController => 
                "Wireless Controller devices include Bluetooth adapters, RF controllers, and other\
                wireless communication bridges.".to_string(),
            
            UsbDeviceClass::Miscellaneous => 
                "Miscellaneous class covers devices that don't fit other categories but aren't\
                vendor-specific.".to_string(),
            
            UsbDeviceClass::ApplicationSpecific => 
                "Application Specific class devices are designed for specific applications and use\
                protocols defined for those applications.".to_string(),
            
            UsbDeviceClass::VendorSpecific => 
                "Vendor Specific class devices use proprietary protocols defined by the manufacturer\
                rather than USB-IF standard protocols.".to_string(),
            
            UsbDeviceClass::Unknown(_) => 
                "This device uses an unknown or undocumented device class code.".to_string(),
        }
    }
    
    // Get a hint for an endpoint type
    pub fn for_endpoint_type(endpoint_type: &UsbEndpointType) -> String {
        match endpoint_type {
            UsbEndpointType::Control => 
                "Control endpoints are used for device configuration, status, and control operations.\
                Every USB device must have at least one control endpoint (Endpoint 0).".to_string(),
            
            UsbEndpointType::Isochronous => 
                "Isochronous endpoints are used for time-sensitive data like audio and video.\
                They prioritize timely delivery over data integrity (no retries if data is corrupted).\
                Commonly used in webcams, microphones, and speakers.".to_string(),
            
            UsbEndpointType::Bulk => 
                "Bulk endpoints are used for large, non-time-critical data transfers with error checking.\
                They use all available bandwidth but with no guaranteed timing.\
                Commonly used in printers, scanners, and storage devices.".to_string(),
            
            UsbEndpointType::Interrupt => 
                "Interrupt endpoints are used for small, time-sensitive data that needs guaranteed latency.\
                They're typically used for user input devices like keyboards and mice,\
                or for status updates that need prompt attention.".to_string(),
            
            UsbEndpointType::Unknown(_) => 
                "This endpoint uses an unknown or non-standard transfer type.".to_string(),
        }
    }
    
    // Get hints for a device descriptor
    pub fn for_device_descriptor(desc: &DeviceDescriptor) -> Vec<String> {
        let mut hints = Vec::new();
        
        // USB version hint
        let usb_version = desc.usb_version_string();
        if usb_version.starts_with("2.") {
            hints.push("This device supports USB 2.0, with theoretical maximum speed of 480 Mbps (High Speed).".to_string());
        } else if usb_version.starts_with("1.1") {
            hints.push("This device supports USB 1.1, with theoretical maximum speed of 12 Mbps (Full Speed).".to_string());
        } else if usb_version.starts_with("3.") {
            hints.push("This device supports USB 3.0 or higher, with theoretical speeds of 5 Gbps (SuperSpeed) or more.".to_string());
        }
        
        // Max packet size hint
        match desc.max_packet_size0 {
            8 => hints.push("Max packet size of 8 bytes for Endpoint 0 indicates a Low Speed USB device.".to_string()),
            64 => hints.push("Max packet size of 64 bytes for Endpoint 0 indicates a Full Speed or High Speed USB device.".to_string()),
            _ => {}
        }
        
        // Device class hint
        if desc.device_class.get_value() == 0 {
            hints.push("This device uses interface-specific classes rather than a device-level class.".to_string());
        } else {
            hints.push(format!("Device Class: {}", Self::for_device_class(&desc.device_class)));
        }
        
        hints
    }
    
    // Get hints for a configuration descriptor
    pub fn for_configuration_descriptor(desc: &ConfigurationDescriptor) -> Vec<String> {
        let mut hints = Vec::new();
        
        // Power hints
        if desc.self_powered {
            hints.push("This configuration is self-powered, meaning it doesn't draw significant power from the USB bus.".to_string());
        } else {
            hints.push(format!(
                "This configuration draws up to {}mA from the USB bus. Standard USB 2.0 ports provide 500mA, USB 3.0 ports provide 900mA.",
                desc.power_consumption_ma()
            ));
        }
        
        // Remote wakeup hint
        if desc.remote_wakeup {
            hints.push("This device supports remote wakeup, allowing it to signal the host to wake from a suspended state.".to_string());
        }
        
        // Multiple interfaces hint
        if desc.num_interfaces > 1 {
            hints.push(format!(
                "This configuration has {} interfaces, making it a composite device that provides multiple functions.",
                desc.num_interfaces
            ));
        }
        
        hints
    }
    
    // Get hints for an interface descriptor
    pub fn for_interface_descriptor(desc: &InterfaceDescriptor) -> Vec<String> {
        let mut hints = Vec::new();
        
        // Class-specific hints
        hints.push(format!("Interface Class: {}", Self::for_device_class(&desc.interface_class)));
        
        // Alternate setting hint
        if desc.alternate_setting > 0 {
            hints.push(format!(
                "This is alternate setting {} for interface {}. Alternate settings provide different characteristics for the same interface.",
                desc.alternate_setting, desc.interface_number
            ));
        }
        
        // Add subclass/protocol specific hints
        match desc.interface_class {
            UsbDeviceClass::HumanInterfaceDevice => {
                match desc.interface_protocol {
                    1 => hints.push("Protocol 1 on HID interface indicates a keyboard.".to_string()),
                    2 => hints.push("Protocol 2 on HID interface indicates a mouse.".to_string()),
                    _ => {}
                }
            },
            UsbDeviceClass::MassStorage => {
                match desc.interface_protocol {
                    80 => hints.push("This mass storage device uses the SCSI transparent command set, typical for USB flash drives.".to_string()),
                    _ => {}
                }
            },
            _ => {}
        }
        
        hints
    }
    
    // Get hints for an endpoint descriptor
    pub fn for_endpoint_descriptor(desc: &EndpointDescriptor) -> Vec<String> {
        let mut hints = Vec::new();
        
        // Transfer type hint
        hints.push(format!("Endpoint Type: {}", Self::for_endpoint_type(&desc.transfer_type)));
        
        // Direction hint
        let direction = if desc.direction.name().contains("IN") {
            "Device-to-Host (IN)"
        } else {
            "Host-to-Device (OUT)"
        };
        hints.push(format!("Data flows from {} on this endpoint.", direction));
        
        // Max packet size hint
        hints.push(format!(
            "Maximum packet size is {} bytes. Larger data transfers will be split into multiple packets.",
            desc.max_packet_size
        ));
        
        // Interval hint
        match desc.transfer_type {
            UsbEndpointType::Interrupt => {
                hints.push(format!(
                    "Polling interval of {} frames (~{} ms) - how often the host checks this endpoint for data.",
                    desc.interval, desc.interval
                ));
            },
            UsbEndpointType::Isochronous => {
                if let Some(sync_type) = &desc.sync_type {
                    match sync_type {
                        UsbIsoSyncType::NoSync => hints.push("No synchronization for this isochronous endpoint.".to_string()),
                        UsbIsoSyncType::Asynchronous => hints.push("Asynchronous synchronization - endpoint uses its own clock source.".to_string()),
                        UsbIsoSyncType::Adaptive => hints.push("Adaptive synchronization - endpoint adjusts to USB data timing.".to_string()),
                        UsbIsoSyncType::Synchronous => hints.push("Synchronous with USB frame clock.".to_string()),
                        _ => {}
                    }
                }
            },
            _ => {}
        }
        
        hints
    }
}