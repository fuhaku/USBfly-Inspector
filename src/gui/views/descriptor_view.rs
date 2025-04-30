use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{Command, Element, Length};
use crate::usb::USBDescriptor;
use crate::usb::hints::{get_descriptor_hints, UsbStandardReferences};
use crate::usb::UsbDescriptorType;
use crate::usb::UsbEndpointType;
use crate::gui::styles;

pub struct DescriptorView {
    descriptors: Vec<USBDescriptor>,
    selected_descriptor: Option<usize>,
    decoded_data: Vec<crate::usb::DecodedUSBData>,
    dark_mode: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    DescriptorSelected(usize),
    ClearDescriptors,
    ToggleDarkMode(bool),
}

impl DescriptorView {
    pub fn new() -> Self {
        Self {
            descriptors: Vec::new(),
            selected_descriptor: None,
            decoded_data: Vec::new(),
            dark_mode: true, // Default to dark mode for hacker-friendly UI
        }
    }
    
    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::DescriptorSelected(index) => {
                self.selected_descriptor = Some(index);
                Command::none()
            },
            Message::ClearDescriptors => {
                self.descriptors.clear();
                self.decoded_data.clear();
                self.selected_descriptor = None;
                Command::none()
            },
            Message::ToggleDarkMode(enabled) => {
                self.dark_mode = enabled;
                Command::none()
            },
        }
    }
    
    pub fn update_descriptors(&mut self, decoded_data: crate::usb::DecodedUSBData) {
        // Store the complete decoded data for context and hints
        self.decoded_data.push(decoded_data.clone());
        
        // Add new descriptors to our list
        for descriptor in decoded_data.descriptors {
            if !self.descriptors.iter().any(|d| format!("{:?}", d) == format!("{:?}", descriptor)) {
                self.descriptors.push(descriptor);
            }
        }
        
        // If no descriptor is selected, select the first one
        if self.selected_descriptor.is_none() && !self.descriptors.is_empty() {
            self.selected_descriptor = Some(0);
        }
    }
    
    pub fn clear(&mut self) {
        self.descriptors.clear();
        self.decoded_data.clear();
        self.selected_descriptor = None;
    }
    
    pub fn view(&self) -> Element<Message> {
        let title = text("USB Descriptors")
            .size(24)
            .style(if self.dark_mode {
                iced::theme::Text::Color(styles::color::dark::PRIMARY)
            } else {
                iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.5, 0.8))
            });
            
        let clear_button = button("Clear Descriptors")
            .on_press(Message::ClearDescriptors)
            .style(if self.dark_mode {
                iced::theme::Button::Custom(Box::new(styles::DarkModeDestructiveButton))
            } else {
                iced::theme::Button::Destructive
            });
            
        let header = row![title, clear_button]
            .spacing(20)
            .align_items(iced::Alignment::Center)
            .width(Length::Fill);
            
        // Define descriptor_list as Element to handle different return types
        let descriptor_list: Element<Message> = if self.descriptors.is_empty() {
            container(
                text("No descriptors decoded yet")
                    .width(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
                    .vertical_alignment(iced::alignment::Vertical::Center)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_y()
            .into()
        } else {
            let items = Column::with_children(
                self.descriptors
                    .iter()
                    .enumerate()
                    .map(|(index, descriptor)| {
                        let descriptor_type = match descriptor {
                            USBDescriptor::Device(_) => "Device Descriptor",
                            USBDescriptor::Configuration(_) => "Configuration Descriptor",
                            USBDescriptor::Interface(_) => "Interface Descriptor",
                            USBDescriptor::Endpoint(_) => "Endpoint Descriptor",
                            USBDescriptor::String(_) => "String Descriptor",
                            USBDescriptor::HID(_) => "HID Descriptor",
                            USBDescriptor::DeviceQualifier(_) => "Device Qualifier Descriptor",
                            USBDescriptor::Unknown { descriptor_type, .. } => 
                                return text(format!("Unknown Descriptor (0x{:02X})", descriptor_type))
                                    .width(Length::Fill)
                                    .into(),
                        };
                        
                        let row = text(descriptor_type)
                            .width(Length::Fill);
                        
                        if Some(index) == self.selected_descriptor {
                            container(row)
                                .style(if self.dark_mode {
                                    iced::theme::Container::Custom(Box::new(styles::DarkModeSelectedContainer))
                                } else {
                                    iced::theme::Container::Custom(Box::new(styles::SelectedContainer))
                                })
                                .width(Length::Fill)
                                .padding(10)
                                .into()
                        } else {
                            // Use a button instead of container with on_press
                            button(
                                container(row)
                                    .style(if self.dark_mode {
                                        iced::theme::Container::Custom(Box::new(styles::DarkModeContainer)) 
                                    } else {
                                        iced::theme::Container::Box
                                    })
                                    .width(Length::Fill)
                                    .padding(5)
                            )
                            .width(Length::Fill)
                            .style(if self.dark_mode {
                                iced::theme::Button::Custom(Box::new(styles::DarkModeTreeNodeButton))
                            } else {
                                iced::theme::Button::Text
                            })
                            .on_press(Message::DescriptorSelected(index))
                            .into()
                        }
                    })
                    .collect()
            );
            
            scrollable(
                container(items)
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
            .height(Length::Fill)
            .into()
        };
        
        let selected_descriptor_view = if let Some(index) = self.selected_descriptor {
            if index < self.descriptors.len() {
                let descriptor = &self.descriptors[index];
                let descriptor_str = format!("{}", descriptor);
                
                // Get hints for this descriptor type
                let descriptor_type = match descriptor {
                    USBDescriptor::Device(desc) => &desc.descriptor_type,
                    USBDescriptor::Configuration(desc) => &desc.descriptor_type,
                    USBDescriptor::Interface(desc) => &desc.descriptor_type,
                    USBDescriptor::Endpoint(desc) => &desc.descriptor_type,
                    USBDescriptor::String(desc) => &desc.descriptor_type,
                    USBDescriptor::DeviceQualifier(desc) => &desc.descriptor_type,
                    USBDescriptor::Unknown { descriptor_type, .. } => descriptor_type,
                    _ => &UsbDescriptorType::Unknown(0),
                };
                
                // Create structured hints with categories
                let mut general_hints = Vec::new();
                let mut usage_hints = Vec::new();
                let mut details_hints = Vec::new();
                let mut specs_hints = Vec::new();
                
                // Get basic descriptor type hint
                let type_hint = get_descriptor_hints(descriptor_type);
                if !type_hint.is_empty() {
                    specs_hints.push(type_hint);
                }
                
                match descriptor {
                    // Device descriptor special handling
                    USBDescriptor::Device(device_desc) => {
                        // Basic device information for general category
                        let usb_version = format!("USB {}.{}", 
                            (device_desc.usb_version >> 8) & 0xFF, 
                            (device_desc.usb_version >> 4) & 0xF);
                        general_hints.push(format!("USB Version: {}", usb_version));
                        
                        if device_desc.device_class.get_value() == 0 {
                            general_hints.push("Interface Association: Each interface specifies its own class".to_string());
                        } else {
                            general_hints.push(format!("Device Class: {}", device_desc.device_class.name()));
                        }
                        
                        // Add endpoint 0 info for details category
                        details_hints.push(format!("Maximum packet size for endpoint 0: {} bytes", device_desc.max_packet_size0));
                        
                        // Add vendor info for details category
                        details_hints.push(format!("Vendor ID: 0x{:04X}", device_desc.vendor_id));
                        details_hints.push(format!("Product ID: 0x{:04X}", device_desc.product_id));
                        
                        // Add standard references for key fields
                        if let Some(vendor_ref) = UsbStandardReferences::for_field("idVendor") {
                            specs_hints.push(format!("idVendor: {}", vendor_ref));
                        }
                        
                        if let Some(product_ref) = UsbStandardReferences::for_field("idProduct") {
                            specs_hints.push(format!("idProduct: {}", product_ref));
                        }
                        
                        if let Some(device_class_ref) = UsbStandardReferences::for_field_value(
                            "bDeviceClass", device_desc.device_class.get_value()) {
                            specs_hints.push(format!("Class 0x{:02X}: {}", 
                                device_desc.device_class.get_value(), device_class_ref));
                        }
                        
                        // Add more device-specific hints directly
                        if let Some(product_str) = &device_desc.product_string {
                            general_hints.push(format!("Product: {}", product_str));
                        }
                        
                        if let Some(manufacturer_str) = &device_desc.manufacturer_string {
                            general_hints.push(format!("Manufacturer: {}", manufacturer_str));
                        }
                        
                        if let Some(serial_str) = &device_desc.serial_number_string {
                            general_hints.push(format!("Serial: {}", serial_str));
                        }
                        
                        // Add version hints
                        let device_version_major = (device_desc.device_version >> 8) & 0xFF;
                        let device_version_minor = (device_desc.device_version >> 4) & 0xF;
                        details_hints.push(format!("Device version: {}.{}", device_version_major, device_version_minor));
                        
                        // Add configuration count
                        details_hints.push(format!("Configurations: {}", device_desc.num_configurations));
                        
                        // Add specifications for USB versions
                        match (device_desc.usb_version >> 8) & 0xFF {
                            1 => {
                                let minor = (device_desc.usb_version >> 4) & 0xF;
                                if minor == 0 {
                                    specs_hints.push("USB 1.0: Original USB specification (1996)".to_string());
                                } else if minor == 1 {
                                    specs_hints.push("USB 1.1: Full-Speed USB at 12 Mbps (1998)".to_string());
                                }
                            },
                            2 => {
                                specs_hints.push("USB 2.0: High-Speed USB at 480 Mbps (2000)".to_string());
                                specs_hints.push("Supports backward compatibility with USB 1.1 devices".to_string());
                            },
                            3 => {
                                specs_hints.push("USB 3.0/3.1 Gen 1: SuperSpeed USB at 5 Gbps".to_string());
                                specs_hints.push("Supports backward compatibility with USB 2.0 devices".to_string());
                            },
                            _ => {}
                        }
                    },
                    // Configuration descriptor special handling  
                    USBDescriptor::Configuration(config_desc) => {
                        general_hints.push(format!("Interfaces: {}", config_desc.num_interfaces));
                        usage_hints.push(format!("Power consumption: {}mA", config_desc.max_power as u16 * 2));
                        
                        if (config_desc.attributes & 0x40) != 0 {
                            usage_hints.push("Device is self-powered".to_string());
                            
                            // Add attribute details from standard reference
                            if let Some(attr_ref) = UsbStandardReferences::for_field_value("bmConfigAttributes", config_desc.attributes) {
                                specs_hints.push(attr_ref);
                            }
                        } else {
                            usage_hints.push("Device is bus-powered".to_string());
                        }
                        
                        if (config_desc.attributes & 0x20) != 0 {
                            usage_hints.push("Remote wakeup supported".to_string());
                        }
                        
                        // Add standard reference information
                        if let Some(config_ref) = UsbStandardReferences::for_field("bConfigurationValue") {
                            specs_hints.push(format!("Configuration Value: {} ({})", 
                                config_desc.configuration_value, config_ref));
                        }
                        
                        if let Some(max_power_ref) = UsbStandardReferences::for_field("bMaxPower") {
                            specs_hints.push(format!("bMaxPower: {}", max_power_ref));
                        }
                        
                        if let Some(total_length_ref) = UsbStandardReferences::for_field("wTotalLength") {
                            specs_hints.push(format!("Total Length: {} bytes ({})", 
                                config_desc.total_length, total_length_ref));
                        }
                    },
                    // Interface descriptor special handling
                    USBDescriptor::Interface(iface_desc) => {
                        general_hints.push(format!("Interface Number: {}", iface_desc.interface_number));
                        general_hints.push(format!("Class: {}", iface_desc.interface_class.name()));
                        
                        if iface_desc.alternate_setting > 0 {
                            general_hints.push(format!("Alternate Setting: {}", iface_desc.alternate_setting));
                        }
                        
                        // Add protocol info if available
                        if iface_desc.interface_protocol > 0 {
                            details_hints.push(format!("Protocol: 0x{:02X}", iface_desc.interface_protocol));
                        }
                        
                        // Add standard reference information for interface fields
                        if let Some(iface_ref) = UsbStandardReferences::for_field("bInterfaceNumber") {
                            specs_hints.push(format!("Interface Number: {}", iface_ref));
                        }
                        
                        if let Some(alt_ref) = UsbStandardReferences::for_field("bAlternateSetting") {
                            specs_hints.push(format!("Alternate Setting: {}", alt_ref));
                        }
                        
                        if let Some(class_ref) = UsbStandardReferences::for_field("bInterfaceClass") {
                            specs_hints.push(format!("Interface Class: {}", class_ref));
                        }
                        
                        // Add information about class value
                        if let Some(class_value_ref) = UsbStandardReferences::for_field_value(
                            "bDeviceClass", iface_desc.interface_class.get_value()) {
                            specs_hints.push(format!("Class 0x{:02X}: {}", 
                                iface_desc.interface_class.get_value(), class_value_ref));
                        }
                        
                        if !iface_desc.endpoints.is_empty() {
                            details_hints.push(format!("Endpoints: {}", iface_desc.endpoints.len()));
                            for (i, ep) in iface_desc.endpoints.iter().enumerate() {
                                details_hints.push(format!("  Endpoint {}: 0x{:02X} ({})", 
                                    i+1, 
                                    ep.endpoint_address,
                                    if ep.endpoint_address & 0x80 != 0 { "IN" } else { "OUT" }
                                ));
                            }
                        }
                        
                        // Based on class, provide specialized information
                        match iface_desc.interface_class.get_value() {
                            0x03 => { // HID class
                                specs_hints.push("Human Interface Device (HID) class interfaces are used for input devices like keyboards, mice, and game controllers.".to_string());
                                specs_hints.push("HID devices use standardized report formats to describe their capabilities and controls.".to_string());
                            },
                            0x08 => { // Mass Storage class
                                specs_hints.push("Mass Storage class interfaces implement protocols like SCSI, allowing access to storage media.".to_string());
                                specs_hints.push("These devices typically use bulk transfers for data and control transfers for commands.".to_string());
                            },
                            0x0E => { // Video class
                                specs_hints.push("Video class interfaces handle video streaming from devices like webcams.".to_string());
                                specs_hints.push("They often use isochronous transfers for continuous video data.".to_string());
                            },
                            _ => {}
                        }
                    },
                    // Endpoint descriptor special handling
                    USBDescriptor::Endpoint(ep_desc) => {
                        // Use the fields available in the endpoint descriptor
                        general_hints.push(format!("Address: 0x{:02X}", ep_desc.endpoint_address));
                        general_hints.push(format!("Direction: {}", ep_desc.direction.name()));
                        general_hints.push(format!("Type: {}", ep_desc.transfer_type.name()));
                        details_hints.push(format!("Max Packet Size: {} bytes", ep_desc.max_packet_size));
                        details_hints.push(format!("Interval: {} ms", ep_desc.interval));
                        
                        // Add endpoint number info
                        details_hints.push(format!("Endpoint Number: {}", ep_desc.endpoint_number));
                        
                        // Add standard reference information
                        if let Some(addr_ref) = UsbStandardReferences::for_field("bEndpointAddress") {
                            specs_hints.push(format!("Endpoint Address: {}", addr_ref));
                        }
                        
                        if let Some(max_packet_ref) = UsbStandardReferences::for_field("wMaxPacketSize") {
                            specs_hints.push(format!("Max Packet Size: {}", max_packet_ref));
                        }
                        
                        if let Some(interval_ref) = UsbStandardReferences::for_field("bInterval") {
                            specs_hints.push(format!("Interval: {}", interval_ref));
                        }
                        
                        // Add endpoint attributes reference
                        let attr_value = match ep_desc.transfer_type {
                            UsbEndpointType::Control => 0x00,
                            UsbEndpointType::Isochronous => 0x01,
                            UsbEndpointType::Bulk => 0x02,
                            UsbEndpointType::Interrupt => 0x03,
                            _ => 0xFF,
                        };
                        
                        if let Some(attr_ref) = UsbStandardReferences::for_field_value("bmEndpointAttributes", attr_value) {
                            specs_hints.push(attr_ref);
                        }
                        
                        // Additional endpoint type info based on the transfer type
                        match ep_desc.transfer_type {
                            UsbEndpointType::Isochronous => {
                                specs_hints.push("Isochronous endpoints are used for time-critical data like audio/video".to_string());
                                
                                // Add sync and usage type if available
                                if let Some(sync) = &ep_desc.sync_type {
                                    details_hints.push(format!("Sync Type: {}", sync.name()));
                                }
                                
                                if let Some(usage) = &ep_desc.usage_type {
                                    details_hints.push(format!("Usage Type: {}", usage.name()));
                                }
                            },
                            UsbEndpointType::Bulk => {
                                specs_hints.push("Bulk endpoints are used for large non-time-critical data transfers".to_string());
                                
                                // Get standard reference for bulk endpoint
                                if let Some(bulk_ref) = UsbStandardReferences::for_field("Bulk") {
                                    specs_hints.push(bulk_ref);
                                }
                            },
                            UsbEndpointType::Interrupt => {
                                specs_hints.push("Interrupt endpoints are used for small, time-sensitive data like keyboard/mouse input".to_string());
                                
                                // Get standard reference for interrupt endpoint
                                if let Some(int_ref) = UsbStandardReferences::for_field("Interrupt") {
                                    specs_hints.push(int_ref);
                                }
                            },
                            UsbEndpointType::Control => {
                                specs_hints.push("Control endpoints are used for device configuration and control".to_string());
                                
                                // Get standard reference for control endpoint
                                if let Some(ctrl_ref) = UsbStandardReferences::for_field("Control") {
                                    specs_hints.push(ctrl_ref);
                                }
                            },
                            _ => {}
                        }
                    },
                    // String descriptor handling
                    USBDescriptor::String(string_desc) => {
                        general_hints.push(format!("String Index: {}", string_desc.string_index));
                        general_hints.push(format!("String: \"{}\"", string_desc.string));
                        specs_hints.push("String descriptors provide human-readable information for the device".to_string());
                        
                        // Add language info for string descriptor 0 (special case)
                        if string_desc.string_index == 0 {
                            specs_hints.push("String descriptor 0 contains the language IDs supported by the device".to_string());
                            // Since we don't have langids field in StringDescriptor, just add general info
                            details_hints.push("Note: Language IDs are encoded in the raw descriptor data".to_string());
                        }
                    },
                    // Device qualifier descriptor handling
                    USBDescriptor::DeviceQualifier(qual_desc) => {
                        let usb_version = format!("USB {}.{}", 
                            (qual_desc.usb_version >> 8) & 0xFF, 
                            (qual_desc.usb_version >> 4) & 0xF);
                        general_hints.push(format!("USB Version: {}", usb_version));
                        
                        specs_hints.push("A device qualifier descriptor describes how a high-speed capable device would operate in full-speed mode".to_string());
                        specs_hints.push("This indicates the device is dual-speed capable (high-speed and full-speed)".to_string());
                        
                        if qual_desc.device_class.get_value() == 0 {
                            general_hints.push("Interface Association: Each interface specifies its own class".to_string());
                        } else {
                            general_hints.push(format!("Device Class: {}", qual_desc.device_class.name()));
                        }
                        
                        details_hints.push(format!("Max Packet Size (EP0): {} bytes", qual_desc.max_packet_size0));
                        details_hints.push(format!("Configurations: {}", qual_desc.num_configurations));
                    },
                    // HID descriptor handling
                    USBDescriptor::HID(hid_desc) => {
                        // Since HID is stored as raw Vec<u8>, we provide general information about HID
                        general_hints.push("HID Descriptor Raw Data".to_string());
                        
                        specs_hints.push("HID (Human Interface Device) is used for input devices like keyboards, mice, game controllers, etc.".to_string());
                        specs_hints.push("HID descriptors define how the device communicates its capabilities and controls".to_string());
                        
                        // Add info about raw data size
                        details_hints.push(format!("Raw Data Size: {} bytes", hid_desc.len()));
                        
                        // Extract basic HID information if possible
                        if hid_desc.len() >= 6 {
                            // Standard HID descriptor format:
                            // Offset 0-1: bcdHID (HID version)
                            // Offset 2: bCountryCode
                            // Offset 3: bNumDescriptors
                            let version_major = hid_desc[0];
                            let version_minor = hid_desc[1];
                            let country_code = hid_desc[2];
                            let num_descriptors = hid_desc[3];
                            
                            general_hints.push(format!("HID Version: {}.{}", version_major, version_minor));
                            general_hints.push(format!("Country Code: {}", country_code));
                            details_hints.push(format!("Number of descriptors: {}", num_descriptors));
                        }
                    },
                    // Unknown descriptor type
                    USBDescriptor::Unknown { descriptor_type, data } => {
                        general_hints.push(format!("Type: 0x{:02X}", descriptor_type.get_value()));
                        details_hints.push(format!("Data Length: {} bytes", data.len()));
                        
                        specs_hints.push("This is a vendor-specific or class-specific descriptor not recognized by standard USB specifications".to_string());
                        specs_hints.push("It may contain specialized functionality for this particular device".to_string());
                    }
                }
                
                // Build the complete hints view with categories
                let mut hint_sections: Vec<Element<'_, Message>> = Vec::new();
                
                // General information section
                if !general_hints.is_empty() {
                    hint_sections.push(
                        container(text("General Information").size(16))
                            .width(Length::Fill)
                            .padding(5)
                            .style(if self.dark_mode {
                                iced::theme::Container::Custom(Box::new(styles::DarkModeHintCategoryContainer))
                            } else {
                                iced::theme::Container::Custom(Box::new(styles::HintCategoryContainer))
                            })
                            .into()
                    );
                    
                    let general_items: Vec<Element<'_, Message>> = general_hints.iter().map(|hint| {
                        container(
                            text(hint)
                                .style(if self.dark_mode {
                                    iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.9, 0.7))
                                } else {
                                    iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.5, 0.0))
                                })
                        )
                        .padding(5)
                        .width(Length::Fill)
                        .into()
                    }).collect();
                    
                    hint_sections.push(
                        column(general_items)
                            .spacing(2)
                            .width(Length::Fill)
                            .into()
                    );
                }
                
                // Usage information section
                if !usage_hints.is_empty() {
                    hint_sections.push(
                        container(text("Power & Usage").size(16))
                            .width(Length::Fill)
                            .padding(5)
                            .style(if self.dark_mode {
                                iced::theme::Container::Custom(Box::new(styles::DarkModeHintCategoryContainer))
                            } else {
                                iced::theme::Container::Custom(Box::new(styles::HintCategoryContainer))
                            })
                            .into()
                    );
                    
                    let usage_items: Vec<Element<'_, Message>> = usage_hints.iter().map(|hint| {
                        container(
                            text(hint)
                                .style(if self.dark_mode {
                                    iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.9, 0.7))
                                } else {
                                    iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.5, 0.0))
                                })
                        )
                        .padding(5)
                        .width(Length::Fill)
                        .into()
                    }).collect();
                    
                    hint_sections.push(
                        column(usage_items)
                            .spacing(2)
                            .width(Length::Fill)
                            .into()
                    );
                }
                
                // Technical details section
                if !details_hints.is_empty() {
                    hint_sections.push(
                        container(text("Technical Details").size(16))
                            .width(Length::Fill)
                            .padding(5)
                            .style(if self.dark_mode {
                                iced::theme::Container::Custom(Box::new(styles::DarkModeHintCategoryContainer))
                            } else {
                                iced::theme::Container::Custom(Box::new(styles::HintCategoryContainer))
                            })
                            .into()
                    );
                    
                    let details_items: Vec<Element<'_, Message>> = details_hints.iter().map(|hint| {
                        container(
                            text(hint)
                                .style(if self.dark_mode {
                                    iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.9, 0.7))
                                } else {
                                    iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.5, 0.0))
                                })
                        )
                        .padding(5)
                        .width(Length::Fill)
                        .into()
                    }).collect();
                    
                    hint_sections.push(
                        column(details_items)
                            .spacing(2)
                            .width(Length::Fill)
                            .into()
                    );
                }
                
                // USB specifications section
                if !specs_hints.is_empty() {
                    hint_sections.push(
                        container(text("USB Specification").size(16))
                            .width(Length::Fill)
                            .padding(5)
                            .style(if self.dark_mode {
                                iced::theme::Container::Custom(Box::new(styles::DarkModeHintCategoryContainer))
                            } else {
                                iced::theme::Container::Custom(Box::new(styles::HintCategoryContainer))
                            })
                            .into()
                    );
                    
                    let specs_items: Vec<Element<'_, Message>> = specs_hints.iter().map(|hint| {
                        container(
                            text(hint)
                                .style(if self.dark_mode {
                                    iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.9, 0.7))
                                } else {
                                    iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.5, 0.0))
                                })
                        )
                        .padding(5)
                        .width(Length::Fill)
                        .into()
                    }).collect();
                    
                    hint_sections.push(
                        column(specs_items)
                            .spacing(2)
                            .width(Length::Fill)
                            .into()
                    );
                }
                
                // Create the final hints view
                let hints_view = if !hint_sections.is_empty() {
                    column(hint_sections)
                        .spacing(10)
                        .width(Length::Fill)
                } else {
                    column![
                        text("No hints available for this descriptor")
                            .style(if self.dark_mode {
                                iced::theme::Text::Color(iced::Color::from_rgb(0.6, 0.6, 0.6))
                            } else {
                                iced::theme::Text::Color(iced::Color::from_rgb(0.5, 0.5, 0.5))
                            })
                    ]
                };
                
                container(
                    column![
                        text("Descriptor Details").size(18),
                        container(
                            scrollable(
                                text(&descriptor_str)
                                    .style(iced::theme::Text::Default)
                            )
                        )
                        .padding(10)
                        .height(Length::Fill)
                        .style(if self.dark_mode {
                            iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                        } else {
                            iced::theme::Container::Box
                        })
                        .width(Length::Fill),
                        text("Hints").size(18),
                        container(
                            scrollable(hints_view)
                        )
                        .padding(10)
                        .height(Length::FillPortion(2))
                        .style(if self.dark_mode {
                            iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                        } else {
                            iced::theme::Container::Box
                        })
                        .width(Length::Fill)
                    ]
                    .spacing(10)
                    .padding(10)
                    .height(Length::Fill)
                )
                .style(if self.dark_mode {
                    iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                } else {
                    iced::theme::Container::Box
                })
                .width(Length::Fill)
                .height(Length::Fill)
            } else {
                container(text("Invalid selection"))
                    .style(if self.dark_mode {
                        iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                    } else {
                        iced::theme::Container::Box
                    })
                    .width(Length::Fill)
                    .height(Length::Fill)
            }
        } else {
            container(text("No descriptor selected"))
                .style(if self.dark_mode {
                    iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                } else {
                    iced::theme::Container::Box
                })
                .width(Length::Fill)
                .height(Length::Fill)
        };
        
        let content = column![
            header,
            row![
                container(descriptor_list)
                    .style(if self.dark_mode {
                        iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                    } else {
                        iced::theme::Container::Box
                    })
                    .width(Length::FillPortion(1))
                    .height(Length::Fill),
                container(selected_descriptor_view)
                    .style(if self.dark_mode {
                        iced::theme::Container::Custom(Box::new(styles::DarkModeContainer))
                    } else {
                        iced::theme::Container::Box
                    })
                    .width(Length::FillPortion(4))
                    .height(Length::Fill)
            ]
            .spacing(10)
            .height(Length::Fill)
        ]
        .spacing(20)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);
        
        content.into()
    }
}
