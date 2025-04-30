use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{Command, Element, Length};
use crate::usb::descriptors::USBDescriptor;
use crate::usb::hints::get_descriptor_hints;
use crate::gui::styles;

pub struct DescriptorView {
    descriptors: Vec<USBDescriptor>,
    selected_descriptor: Option<usize>,
    decoded_data: Vec<crate::usb::decoder::DecodedUSBData>,
}

#[derive(Debug, Clone)]
pub enum Message {
    DescriptorSelected(usize),
    ClearDescriptors,
}

impl DescriptorView {
    pub fn new() -> Self {
        Self {
            descriptors: Vec::new(),
            selected_descriptor: None,
            decoded_data: Vec::new(),
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
        }
    }
    
    pub fn update_descriptors(&mut self, decoded_data: crate::usb::decoder::DecodedUSBData) {
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
            .style(iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.5, 0.8)));
            
        let clear_button = button("Clear Descriptors")
            .on_press(Message::ClearDescriptors)
            .style(iced::theme::Button::Destructive);
            
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
                                .style(iced::theme::Container::Custom(Box::new(styles::SelectedContainer)))
                                .width(Length::Fill)
                                .padding(10)
                                .into()
                        } else {
                            // Use a button instead of container with on_press
                            button(
                                container(row)
                                    .style(iced::theme::Container::Box)
                                    .width(Length::Fill)
                                    .padding(5)
                            )
                            .width(Length::Fill)
                            .style(iced::theme::Button::Text)
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
                    crate::usb::descriptors::USBDescriptor::Device(desc) => &desc.descriptor_type,
                    crate::usb::descriptors::USBDescriptor::Configuration(desc) => &desc.descriptor_type,
                    crate::usb::descriptors::USBDescriptor::Interface(desc) => &desc.descriptor_type,
                    crate::usb::descriptors::USBDescriptor::Endpoint(desc) => &desc.descriptor_type,
                    crate::usb::descriptors::USBDescriptor::String(desc) => &desc.descriptor_type,
                    crate::usb::descriptors::USBDescriptor::DeviceQualifier(desc) => &desc.descriptor_type,
                    crate::usb::descriptors::USBDescriptor::Unknown { descriptor_type, .. } => descriptor_type,
                    _ => &crate::usb::descriptor_types::UsbDescriptorType::Unknown(0),
                };
                
                // Get contextual hints
                let mut all_hints = Vec::new();
                
                // Get basic descriptor type hint
                let type_hint = get_descriptor_hints(descriptor_type);
                if !type_hint.is_empty() {
                    all_hints.push(type_hint);
                }
                
                // If this is a device descriptor, get more detailed hints
                if let crate::usb::descriptors::USBDescriptor::Device(_) = descriptor {
                    // Check if we have access to the whole device
                    for decoded_data in &self.decoded_data {
                        if let Some(device) = &decoded_data.device {
                            // Use our new device hints system
                            all_hints.extend(device.get_device_hints());
                            break;
                        }
                    }
                }
                
                let hints_view = if !all_hints.is_empty() {
                    let hint_items = all_hints.iter().map(|hint| {
                        container(
                            text(hint)
                                .style(iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.5, 0.0)))
                        )
                        .padding(5)
                        .width(Length::Fill)
                    }).collect();
                    
                    column(hint_items)
                        .spacing(5)
                        .width(Length::Fill)
                        .style(iced::theme::Container::Custom(Box::new(styles::HintContainer)))
                } else {
                    column![
                        text("No hints available for this descriptor")
                            .style(iced::theme::Text::Color(iced::Color::from_rgb(0.5, 0.5, 0.5)))
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
                        .style(iced::theme::Container::Box)
                        .width(Length::Fill),
                        text("Hints").size(18),
                        container(
                            scrollable(hints_view)
                        )
                        .padding(10)
                        .height(Length::FillPortion(2))
                        .style(iced::theme::Container::Box)
                        .width(Length::Fill)
                    ]
                    .spacing(10)
                    .padding(10)
                    .height(Length::Fill)
                )
                .style(iced::theme::Container::Box)
                .width(Length::Fill)
                .height(Length::Fill)
            } else {
                container(text("Invalid selection"))
                    .style(iced::theme::Container::Box)
                    .width(Length::Fill)
                    .height(Length::Fill)
            }
        } else {
            container(text("No descriptor selected"))
                .style(iced::theme::Container::Box)
                .width(Length::Fill)
                .height(Length::Fill)
        };
        
        let content = column![
            header,
            row![
                container(descriptor_list)
                    .style(iced::theme::Container::Box)
                    .width(Length::FillPortion(1))
                    .height(Length::Fill),
                container(selected_descriptor_view)
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
