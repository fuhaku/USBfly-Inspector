use iced::widget::{button, column, container, row, scrollable, text, text_input, Column, Row};
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
                
                let hex_data = format!("Raw Data: {:02X?}", item.raw_data);
                
                let descriptors = Column::with_children(
                    item.decoded_data.descriptors
                        .iter()
                        .map(|desc| {
                            text(format!("{}", desc))
                                .style(iced::theme::Text::Default)
                                .into()
                        })
                        .collect()
                );
                
                container(
                    column![
                        text("Selected Item").size(18),
                        text(hex_data),
                        container(descriptors)
                            .style(iced::theme::Container::Box)
                            .padding(10)
                            .width(Length::Fill)
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
