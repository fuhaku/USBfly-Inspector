use iced::widget::{button, column, container, row, scrollable, text, text_input, Column};
use iced::{Command, Element, Length};
use crate::usb::decoder::DecodedUSBData;
use crate::gui::styles;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficItem {
    pub timestamp: f64,
    pub raw_data: Vec<u8>,
    pub decoded_data: DecodedUSBData,
}

pub struct TrafficView {
    traffic_data: Vec<TrafficItem>,
    selected_item: Option<usize>,
    filter_text: String,
    auto_scroll: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    ItemSelected(usize),
    FilterChanged(String),
    ToggleAutoScroll(bool),
    ClearTraffic,
    LoadData(Vec<TrafficItem>),
    NoOp,
}

impl TrafficView {
    pub fn new() -> Self {
        Self {
            traffic_data: Vec::new(),
            selected_item: None,
            filter_text: String::new(),
            auto_scroll: true,
        }
    }
    
    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ItemSelected(index) => {
                self.selected_item = Some(index);
                Command::none()
            },
            Message::FilterChanged(filter) => {
                self.filter_text = filter;
                Command::none()
            },
            Message::ToggleAutoScroll(auto_scroll) => {
                self.auto_scroll = auto_scroll;
                Command::none()
            },
            Message::ClearTraffic => {
                self.traffic_data.clear();
                self.selected_item = None;
                Command::none()
            },
            Message::LoadData(data) => {
                self.traffic_data = data;
                self.selected_item = if self.traffic_data.is_empty() {
                    None
                } else {
                    Some(0)
                };
                Command::none()
            },
            Message::NoOp => Command::none(),
        }
    }
    
    pub fn add_packet(&mut self, raw_data: Vec<u8>, decoded_data: DecodedUSBData) {
        let traffic_item = TrafficItem {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
            raw_data,
            decoded_data,
        };
        
        self.traffic_data.push(traffic_item);
        
        if self.auto_scroll {
            self.selected_item = Some(self.traffic_data.len() - 1);
        }
    }
    
    pub fn clear(&mut self) {
        self.traffic_data.clear();
        self.selected_item = None;
    }
    
    pub fn get_traffic_data(&self) -> Option<Vec<TrafficItem>> {
        if self.traffic_data.is_empty() {
            None
        } else {
            Some(self.traffic_data.clone())
        }
    }
    
    pub fn view(&self) -> Element<Message> {
        let title = text("USB Traffic")
            .size(24)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.5, 0.8)));
            
        let filter_label = text("Filter:");
        let filter_input = text_input("Enter filter", &self.filter_text)
            .on_input(Message::FilterChanged)
            .padding(5)
            .width(Length::FillPortion(3));
            
        let auto_scroll_button = if self.auto_scroll {
            button("Auto-scroll: ON")
                .on_press(Message::ToggleAutoScroll(false))
                .style(iced::theme::Button::Primary)
        } else {
            button("Auto-scroll: OFF")
                .on_press(Message::ToggleAutoScroll(true))
                .style(iced::theme::Button::Secondary)
        };
        
        let clear_button = button("Clear")
            .on_press(Message::ClearTraffic)
            .style(iced::theme::Button::Destructive);
            
        let header = row![
            title,
            row![
                filter_label,
                filter_input,
                auto_scroll_button,
                clear_button
            ]
            .spacing(10)
            .align_items(iced::Alignment::Center)
        ]
        .spacing(20)
        .align_items(iced::Alignment::Center)
        .width(Length::Fill);
        
        // Define traffic_list as Element to handle different return types
        let traffic_list: Element<Message> = if self.traffic_data.is_empty() {
            // Empty state
            container(
                text("No traffic captured yet")
                    .width(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
                    .vertical_alignment(iced::alignment::Vertical::Center)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_y()
            .into()
        } else {
            // Filtered list of traffic items
            let filtered_items: Vec<(usize, &TrafficItem)> = self.traffic_data
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    if self.filter_text.is_empty() {
                        return true;
                    }
                    
                    let filter = self.filter_text.to_lowercase();
                    
                    // Filter by hex representation of raw data
                    let hex_data = format!("{:02X?}", item.raw_data);
                    if hex_data.to_lowercase().contains(&filter) {
                        return true;
                    }
                    
                    // Filter by descriptor type or other attributes
                    for descriptor in &item.decoded_data.descriptors {
                        let desc_str = format!("{:?}", descriptor);
                        if desc_str.to_lowercase().contains(&filter) {
                            return true;
                        }
                    }
                    
                    false
                })
                .collect();
            
            let items = Column::with_children(
                filtered_items
                    .iter()
                    .map(|(index, item)| {
                        let formatted_time = format_timestamp(item.timestamp);
                        let data_preview = if item.raw_data.len() > 8 {
                            format!("{:02X?}...", &item.raw_data[..8])
                        } else {
                            format!("{:02X?}", item.raw_data)
                        };
                        
                        let descriptor_type = if let Some(first) = item.decoded_data.descriptors.first() {
                            format!("{:?}", first)
                        } else {
                            "Unknown".to_string()
                        };
                        
                        let row = row![
                            text(formatted_time).width(Length::FillPortion(1)),
                            text(&data_preview).width(Length::FillPortion(2)),
                            text(&descriptor_type).width(Length::FillPortion(3))
                        ]
                        .spacing(10)
                        .padding(5)
                        .width(Length::Fill);
                        
                        if Some(*index) == self.selected_item {
                            container(row)
                                .style(iced::theme::Container::Custom(Box::new(styles::SelectedContainer)))
                                .width(Length::Fill)
                                .into()
                        } else {
                            button(
                                container(row)
                                    .style(iced::theme::Container::Box)
                                    .width(Length::Fill)
                            )
                            .width(Length::Fill)
                            .style(iced::theme::Button::Text)
                            .on_press(Message::ItemSelected(*index))
                            .into()
                        }
                    })
                    .collect()
            );
            
            scrollable(items).height(Length::Fill).into()
        };
        
        let selected_item_view = if let Some(index) = self.selected_item {
            if index < self.traffic_data.len() {
                let item = &self.traffic_data[index];
                
                // Format raw data with byte offset display
                let mut hex_data_lines = Vec::new();
                for chunk in item.raw_data.chunks(16) {
                    let offset = hex_data_lines.len() * 16;
                    let hex_values: Vec<String> = chunk.iter().map(|b| format!("{:02X}", b)).collect();
                    let ascii_values: String = chunk.iter()
                        .map(|&b| if b >= 32 && b <= 126 { b as char } else { '.' })
                        .collect();
                    
                    let hex_line = format!("{:04X}: {} | {}", 
                                          offset, 
                                          hex_values.join(" "), 
                                          ascii_values);
                    hex_data_lines.push(hex_line);
                }
                
                let hex_data_view = Column::with_children(
                    hex_data_lines.into_iter()
                        .map(|line| text(line).font(iced::Font::MONOSPACE).into())
                        .collect()
                );
                
                // Build a tree-like view of descriptors with indentation
                let descriptors_tree = Column::with_children(
                    build_descriptor_tree(&item.decoded_data.descriptors)
                        .into_iter()
                        .map(|e| e.into())
                        .collect()
                );
                
                // Create a timestamp display
                let timestamp = format_timestamp(item.timestamp);
                
                container(
                    column![
                        // Header with timestamp and packet info
                        row![
                            text("Packet Details").size(18).width(Length::Fill),
                            text(format!("Timestamp: {}", timestamp)).size(14)
                        ],
                        
                        // Tabs for different views (Hex, Descriptors, etc)
                        container(
                            column![
                                text("Raw Hex Data").size(16).style(iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.4, 0.8))),
                                container(hex_data_view)
                                    .style(iced::theme::Container::Box)
                                    .padding(10)
                                    .width(Length::Fill)
                            ]
                        )
                        .style(iced::theme::Container::Box)
                        .padding(10)
                        .width(Length::Fill),
                        
                        // Descriptor tree view
                        container(
                            column![
                                text("USB Descriptors").size(16).style(iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.4, 0.8))),
                                container(descriptors_tree)
                                    .style(iced::theme::Container::Box)
                                    .padding(10)
                                    .width(Length::Fill)
                            ]
                        )
                        .style(iced::theme::Container::Box)
                        .padding(10)
                        .width(Length::Fill),
                    ]
                    .spacing(10)
                    .padding(10)
                )
                .style(iced::theme::Container::Box)
                .width(Length::Fill)
            } else {
                container(text("Invalid selection"))
                    .style(iced::theme::Container::Box)
                    .width(Length::Fill)
            }
        } else {
            container(text("No item selected"))
                .style(iced::theme::Container::Box)
                .width(Length::Fill)
        };
        
        let content = column![
            header,
            row![
                container(traffic_list)
                    .style(iced::theme::Container::Box)
                    .width(Length::FillPortion(2))
                    .height(Length::Fill),
                container(selected_item_view)
                    .width(Length::FillPortion(3))
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

fn format_timestamp(timestamp: f64) -> String {
    let secs = timestamp as u64;
    let nanos = ((timestamp - secs as f64) * 1_000_000_000.0) as u32;
    
    let time = time::OffsetDateTime::from_unix_timestamp(secs as i64)
        .unwrap_or_else(|_| time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc()))
        .replace_nanosecond(nanos)
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    
    time.format(&time::format_description::parse("[hour]:[minute]:[second].[subsecond digits:3]").unwrap())
        .unwrap_or_else(|_| format!("{:.3}", timestamp))
}

// Creates a hierarchical tree view of USB descriptors
fn build_descriptor_tree(descriptors: &[crate::usb::descriptors::USBDescriptor]) -> Vec<Element<Message>> {
    let mut elements = Vec::new();
    
    for (i, descriptor) in descriptors.iter().enumerate() {
        match descriptor {
            crate::usb::descriptors::USBDescriptor::Device(dev) => {
                // Device descriptor is a top-level item
                elements.push(
                    column![
                        text(format!("Device Descriptor (VID:{:04X} PID:{:04X})", 
                            dev.id_vendor, dev.id_product))
                            .size(14)
                            .style(iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.4, 0.7))),
                        row![
                            text("").width(Length::Fixed(20.0)), // Indentation
                            text(format!("USB Version: {}.{}", 
                                (dev.bcd_usb >> 8) & 0xFF, 
                                dev.bcd_usb & 0xFF))
                        ],
                        row![
                            text("").width(Length::Fixed(20.0)),
                            text(format!("Device Class: 0x{:02X}", dev.b_device_class))
                        ],
                        row![
                            text("").width(Length::Fixed(20.0)),
                            text(format!("Max Packet Size: {}", dev.b_max_packet_size0))
                        ],
                        row![
                            text("").width(Length::Fixed(20.0)),
                            text(format!("Manufacturer: Index {}", dev.i_manufacturer))
                        ],
                        row![
                            text("").width(Length::Fixed(20.0)),
                            text(format!("Product: Index {}", dev.i_product))
                        ],
                        row![
                            text("").width(Length::Fixed(20.0)),
                            text(format!("Serial Number: Index {}", dev.i_serial_number))
                        ],
                        row![
                            text("").width(Length::Fixed(20.0)),
                            text(format!("Configurations: {}", dev.b_num_configurations))
                        ]
                    ].into()
                );
            },
            crate::usb::descriptors::USBDescriptor::Configuration(cfg) => {
                // Configuration descriptor
                elements.push(
                    column![
                        text(format!("Configuration Descriptor #{}", i+1))
                            .size(14)
                            .style(iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.5, 0.5))),
                        row![
                            text("").width(Length::Fixed(20.0)),
                            text(format!("Total Length: {} bytes", cfg.w_total_length))
                        ],
                        row![
                            text("").width(Length::Fixed(20.0)),
                            text(format!("Interfaces: {}", cfg.b_num_interfaces))
                        ],
                        row![
                            text("").width(Length::Fixed(20.0)),
                            text(format!("Configuration Value: {}", cfg.b_configuration_value))
                        ],
                        row![
                            text("").width(Length::Fixed(20.0)),
                            text(format!("Attributes: 0x{:02X}", cfg.bm_attributes))
                        ],
                        row![
                            text("").width(Length::Fixed(20.0)),
                            text(format!("Max Power: {} mA", cfg.b_max_power * 2))
                        ]
                    ].into()
                );
            },
            crate::usb::descriptors::USBDescriptor::Interface(iface) => {
                // Interface descriptor (indented under configuration)
                elements.push(
                    column![
                        row![
                            text("").width(Length::Fixed(30.0)), // More indentation
                            text(format!("Interface #{} (Class: 0x{:02X})", 
                                iface.b_interface_number, iface.b_interface_class))
                                .style(iced::theme::Text::Color(iced::Color::from_rgb(0.3, 0.5, 0.7)))
                        ],
                        row![
                            text("").width(Length::Fixed(40.0)),
                            text(format!("Alt Setting: {}", iface.b_alternate_setting))
                        ],
                        row![
                            text("").width(Length::Fixed(40.0)),
                            text(format!("Endpoints: {}", iface.b_num_endpoints))
                        ],
                        row![
                            text("").width(Length::Fixed(40.0)),
                            text(format!("Subclass: 0x{:02X}", iface.b_interface_sub_class))
                        ],
                        row![
                            text("").width(Length::Fixed(40.0)),
                            text(format!("Protocol: 0x{:02X}", iface.b_interface_protocol))
                        ]
                    ].into()
                );
            },
            crate::usb::descriptors::USBDescriptor::Endpoint(ep) => {
                // Endpoint descriptor (indented under interface)
                let direction = if ep.b_endpoint_address & 0x80 == 0x80 {
                    "IN (Device to Host)"
                } else {
                    "OUT (Host to Device)"
                };
                
                let transfer_type = match ep.bm_attributes & 0x03 {
                    0 => "Control",
                    1 => "Isochronous",
                    2 => "Bulk",
                    3 => "Interrupt",
                    _ => "Unknown"
                };
                
                elements.push(
                    column![
                        row![
                            text("").width(Length::Fixed(50.0)), // More indentation
                            text(format!("Endpoint 0x{:02X} - {}", 
                                ep.b_endpoint_address, direction))
                                .style(iced::theme::Text::Color(iced::Color::from_rgb(0.5, 0.5, 0.7)))
                        ],
                        row![
                            text("").width(Length::Fixed(60.0)),
                            text(format!("Type: {}", transfer_type))
                        ],
                        row![
                            text("").width(Length::Fixed(60.0)),
                            text(format!("Max Packet Size: {} bytes", ep.w_max_packet_size))
                        ],
                        row![
                            text("").width(Length::Fixed(60.0)),
                            text(format!("Interval: {}", ep.b_interval))
                        ]
                    ].into()
                );
            },
            crate::usb::descriptors::USBDescriptor::String(str_desc) => {
                // String descriptor
                let string_content = if let Some(string) = &str_desc.string {
                    string.clone()
                } else if let Some(langids) = &str_desc.w_langid {
                    format!("Language IDs: {:?}", langids)
                } else {
                    "Empty String".to_string()
                };
                
                elements.push(
                    column![
                        text(format!("String Descriptor #{}", i+1))
                            .style(iced::theme::Text::Color(iced::Color::from_rgb(0.5, 0.3, 0.5))),
                        row![
                            text("").width(Length::Fixed(20.0)),
                            text(format!("Content: \"{}\"", string_content))
                        ]
                    ].into()
                );
            },
            crate::usb::descriptors::USBDescriptor::HID(hid) => {
                // HID descriptor
                elements.push(
                    column![
                        row![
                            text("").width(Length::Fixed(40.0)), // Indented under interface
                            text(format!("HID Descriptor v{}.{}", 
                                (hid.bcd_hid >> 8) & 0xFF, 
                                hid.bcd_hid & 0xFF))
                                .style(iced::theme::Text::Color(iced::Color::from_rgb(0.7, 0.5, 0.0)))
                        ],
                        row![
                            text("").width(Length::Fixed(50.0)),
                            text(format!("Country Code: {}", hid.b_country_code))
                        ],
                        row![
                            text("").width(Length::Fixed(50.0)),
                            text(format!("Descriptors: {}", hid.b_num_descriptors))
                        ],
                        row![
                            text("").width(Length::Fixed(50.0)),
                            text(format!("Report Descriptor: {} bytes", hid.w_descriptor_length))
                        ]
                    ].into()
                );
            },
            crate::usb::descriptors::USBDescriptor::Unknown { descriptor_type, data } => {
                // Unknown descriptor type
                elements.push(
                    column![
                        text(format!("Unknown Descriptor (Type: 0x{:02X})", descriptor_type))
                            .style(iced::theme::Text::Color(iced::Color::from_rgb(0.5, 0.5, 0.5))),
                        row![
                            text("").width(Length::Fixed(20.0)),
                            text(format!("Data: {:02X?}", data))
                        ]
                    ].into()
                );
            },
        }
    }
    
    elements
}
