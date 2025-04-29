use iced::widget::{button, column, container, row, text, text_input, Canvas, Column, Row};
use iced::{Command, Element, Length};
use crate::gui::styles;

pub struct DeviceView {
    device_info: Option<DeviceInfo>,
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    vendor_id: String,
    product_id: String,
    manufacturer: String,
    product: String,
    serial_number: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    RefreshDevices,
    DeviceSelected(String),
    NoOp,
}

impl DeviceView {
    pub fn new() -> Self {
        Self {
            device_info: None,
        }
    }
    
    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::RefreshDevices => {
                // In a real implementation, this would query connected devices
                // For now, just set a sample device for UI demonstration
                self.device_info = Some(DeviceInfo {
                    vendor_id: "0x1d50".to_string(),
                    product_id: "0x615c".to_string(),
                    manufacturer: "Great Scott Gadgets".to_string(),
                    product: "Cynthion".to_string(),
                    serial_number: "12345678".to_string(),
                });
                Command::none()
            },
            Message::DeviceSelected(_device_id) => {
                // In a real implementation, this would select the device
                // and update the device_info accordingly
                Command::none()
            },
            Message::NoOp => Command::none(),
        }
    }
    
    pub fn view(&self) -> Element<Message> {
        let title = text("Connected Devices")
            .size(24)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.5, 0.8)));
            
        let refresh_button = button("Refresh Devices")
            .on_press(Message::RefreshDevices)
            .style(iced::theme::Button::Primary);
            
        let header = row![title, refresh_button]
            .spacing(20)
            .align_items(iced::Alignment::Center)
            .width(Length::Fill);
            
        let device_info = if let Some(info) = &self.device_info {
            column![
                row![
                    text("Vendor ID:").width(Length::FillPortion(1)),
                    text(&info.vendor_id).width(Length::FillPortion(2))
                ].padding(5).spacing(10),
                row![
                    text("Product ID:").width(Length::FillPortion(1)),
                    text(&info.product_id).width(Length::FillPortion(2))
                ].padding(5).spacing(10),
                row![
                    text("Manufacturer:").width(Length::FillPortion(1)),
                    text(&info.manufacturer).width(Length::FillPortion(2))
                ].padding(5).spacing(10),
                row![
                    text("Product:").width(Length::FillPortion(1)),
                    text(&info.product).width(Length::FillPortion(2))
                ].padding(5).spacing(10),
                row![
                    text("Serial Number:").width(Length::FillPortion(1)),
                    text(&info.serial_number).width(Length::FillPortion(2))
                ].padding(5).spacing(10)
            ]
            .spacing(5)
            .padding(10)
            .width(Length::Fill)
        } else {
            column![
                text("No device connected")
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
                    .vertical_alignment(iced::alignment::Vertical::Center)
            ]
            .width(Length::Fill)
            .height(Length::Fill)
        };
        
        let content = column![
            header,
            container(device_info)
                .style(iced::theme::Container::Box)
                .padding(10)
                .width(Length::Fill)
        ]
        .spacing(20)
        .padding(20)
        .width(Length::Fill);
        
        content.into()
    }
}
