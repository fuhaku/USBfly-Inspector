use crate::cynthion::connection::CynthionConnection;
use crate::gui::views::{DeviceView, TrafficView, DescriptorView};
use crate::usb::UsbDecoder;
use iced::widget::{button, column, container, row, text};
use iced::{executor, Application, Background, Color, Command, Element, Length, Subscription, Theme};
use std::sync::{Arc, Mutex};

// Custom tab styles
struct ActiveTabStyle;
struct InactiveTabStyle;

impl iced::widget::button::StyleSheet for ActiveTabStyle {
    type Style = Theme;
    
    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            text_color: Color::WHITE,
            background: Some(Background::Color(Color::from_rgb(0.2, 0.4, 0.8))),
            border_radius: 4.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            shadow_offset: iced::Vector::default(),
        }
    }
}

impl iced::widget::button::StyleSheet for InactiveTabStyle {
    type Style = Theme;
    
    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            text_color: Color::BLACK,
            background: Some(Background::Color(Color::from_rgb(0.9, 0.9, 0.9))),
            border_radius: 4.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            shadow_offset: iced::Vector::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Tab {
    Devices,
    Traffic,
    Descriptors,
}

pub struct USBflyApp {
    connection: Option<Arc<Mutex<CynthionConnection>>>,
    usb_decoder: UsbDecoder,
    active_tab: Tab,
    device_view: DeviceView,
    traffic_view: TrafficView,
    descriptor_view: DescriptorView,
    connected: bool,
    error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Connect,
    Disconnect,
    ConnectionEstablished(Arc<Mutex<CynthionConnection>>),
    ConnectionFailed(String),
    TabSelected(Tab),
    DeviceViewMessage(crate::gui::views::device_view::Message),
    TrafficViewMessage(crate::gui::views::traffic_view::Message),
    DescriptorViewMessage(crate::gui::views::descriptor_view::Message),
    USBDataReceived(Vec<u8>),
    SaveCapture,
    LoadCapture,
    ClearCapture,
}

impl Application for USBflyApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        // Use the DeviceView with auto-refresh feature
        let (device_view, device_command) = DeviceView::new().with_initial_scan();
        
        let app = Self {
            connection: None,
            usb_decoder: UsbDecoder::new(),
            active_tab: Tab::Devices,
            device_view,
            traffic_view: TrafficView::new(),
            descriptor_view: DescriptorView::new(),
            connected: false,
            error_message: None,
        };
        
        // Map the device command to our application's message type
        let init_command = device_command.map(Message::DeviceViewMessage);

        (app, init_command)
    }

    fn title(&self) -> String {
        String::from("USBfly - USB Analysis for Cynthion")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Connect => {
                // Attempt to connect to Cynthion device
                Command::perform(
                    async {
                        match CynthionConnection::connect().await {
                            Ok(conn) => {
                                let connection = Arc::new(Mutex::new(conn));
                                Message::ConnectionEstablished(connection)
                            }
                            Err(e) => Message::ConnectionFailed(e.to_string()),
                        }
                    },
                    |msg| msg,
                )
            }
            Message::Disconnect => {
                if let Some(connection) = &self.connection {
                    let _ = connection.lock().unwrap().disconnect();
                }
                self.connection = None;
                self.connected = false;
                Command::none()
            }
            Message::ConnectionEstablished(connection) => {
                self.connection = Some(connection);
                self.connected = true;
                self.error_message = None;
                Command::none()
            }
            Message::ConnectionFailed(error) => {
                self.error_message = Some(error);
                Command::none()
            }
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                Command::none()
            }
            Message::DeviceViewMessage(msg) => {
                // Forward message to device view and map the result back to our message type
                self.device_view.update(msg)
                    .map(Message::DeviceViewMessage)
            }
            Message::TrafficViewMessage(msg) => {
                // Forward message to traffic view and map the result back to our message type
                self.traffic_view.update(msg)
                    .map(Message::TrafficViewMessage)
            }
            Message::DescriptorViewMessage(msg) => {
                // Forward message to descriptor view and map the result back to our message type
                self.descriptor_view.update(msg)
                    .map(Message::DescriptorViewMessage)
            }
            Message::USBDataReceived(data) => {
                // Process received USB data
                if let Some(decoded) = self.usb_decoder.decode(&data) {
                    self.traffic_view.add_packet(data.clone(), decoded.clone());
                    self.descriptor_view.update_descriptors(decoded);
                }
                Command::none()
            }
            Message::SaveCapture => {
                // Save current capture to file
                if let Some(traffic_data) = self.traffic_view.get_traffic_data() {
                    Command::perform(
                        async move {
                            // Use rfd to show save dialog
                            let task = rfd::AsyncFileDialog::new()
                                .add_filter("USB Capture", &["usb"])
                                .set_directory("/")
                                .save_file();
                            
                            if let Some(file_handle) = task.await {
                                let path = file_handle.path().to_path_buf();
                                let _ = std::fs::write(path, serde_json::to_string(&traffic_data).unwrap());
                            }
                            Message::DeviceViewMessage(crate::gui::views::device_view::Message::NoOp)
                        },
                        |msg| msg,
                    )
                } else {
                    Command::none()
                }
            }
            Message::LoadCapture => {
                // Load capture from file
                Command::perform(
                    async {
                        let task = rfd::AsyncFileDialog::new()
                            .add_filter("USB Capture", &["usb"])
                            .set_directory("/")
                            .pick_file();
                        
                        if let Some(file_handle) = task.await {
                            let path = file_handle.path();
                            if let Ok(content) = std::fs::read_to_string(path) {
                                if let Ok(data) = serde_json::from_str(&content) {
                                    return Message::TrafficViewMessage(
                                        crate::gui::views::traffic_view::Message::LoadData(data)
                                    );
                                }
                            }
                        }
                        Message::TrafficViewMessage(crate::gui::views::traffic_view::Message::NoOp)
                    },
                    |msg| msg,
                )
            }
            Message::ClearCapture => {
                self.traffic_view.clear();
                self.descriptor_view.clear();
                Command::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        // Create a batch of subscriptions for different features
        let mut subscriptions = Vec::new();
        
        // Subscribe to USB data from connection if connected
        if let Some(connection) = &self.connection {
            let conn = Arc::clone(connection);
            subscriptions.push(
                iced::subscription::unfold(
                    "usb-data-subscription",
                    conn,
                    move |conn| async move {
                        // Use a clone + drop approach to avoid holding MutexGuard across an await point
                        let result = {
                            let mut connection = conn.lock().unwrap();
                            // Clone the required fields or prepare the async call
                            connection.read_data_clone()
                        };
                        
                        // The read_data_clone method now returns Result directly, 
                        // so we don't need to use poll or futures machinery
                        let data = result;
                        
                        match data {
                            Ok(data) => (Message::USBDataReceived(data), conn),
                            Err(_) => (Message::ConnectionFailed("Connection lost".to_string()), conn),
                        }
                    },
                )
            );
        }
        
        // Always add the device view subscription for auto-refresh of USB devices
        subscriptions.push(
            self.device_view.subscription()
                .map(Message::DeviceViewMessage)
        );
        
        // Return a batch of all subscriptions
        if subscriptions.is_empty() {
            Subscription::none()
        } else {
            Subscription::batch(subscriptions)
        }
    }

    fn view(&self) -> Element<Message> {
        let title = text("USBfly")
            .size(28)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.5, 0.8)));

        let connect_button = if self.connected {
            button("Disconnect")
                .on_press(Message::Disconnect)
                .style(iced::theme::Button::Destructive)
        } else {
            button("Connect to Cynthion")
                .on_press(Message::Connect)
                .style(iced::theme::Button::Primary)
        };

        let save_button = button("Save Capture")
            .on_press(Message::SaveCapture)
            .style(iced::theme::Button::Secondary);

        let load_button = button("Load Capture")
            .on_press(Message::LoadCapture)
            .style(iced::theme::Button::Secondary);

        let clear_button = button("Clear")
            .on_press(Message::ClearCapture)
            .style(iced::theme::Button::Destructive);

        let header = row![
            title,
            row![connect_button, save_button, load_button, clear_button]
                .spacing(10)
                .align_items(iced::Alignment::Center)
        ]
        .spacing(10)
        .padding(10)
        .width(iced::Length::Fill)
        .align_items(iced::Alignment::Center);

        // Show error message if any
        let error_banner = if let Some(error) = &self.error_message {
            container(
                text(format!("Error: {}", error)).style(iced::theme::Text::Color(iced::Color::from_rgb(0.8, 0.0, 0.0))),
            )
            .padding(10)
            .style(iced::theme::Container::Box)
            .width(iced::Length::Fill)
        } else {
            container(text("")).width(iced::Length::Fill)
        };

        // Custom tab bar using buttons instead of containers
        let tab_buttons = row![
            // Devices tab
            button(
                text("Devices")
                    .size(16)
                    .width(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
            )
            .padding(10)
            .width(Length::Fill)
            .style(if matches!(self.active_tab, Tab::Devices) {
                iced::theme::Button::Custom(Box::new(ActiveTabStyle))
            } else {
                iced::theme::Button::Custom(Box::new(InactiveTabStyle))
            })
            .on_press(Message::TabSelected(Tab::Devices)),
            
            // Traffic tab
            button(
                text("Traffic")
                    .size(16)
                    .width(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
            )
            .padding(10)
            .width(Length::Fill)
            .style(if matches!(self.active_tab, Tab::Traffic) {
                iced::theme::Button::Custom(Box::new(ActiveTabStyle))
            } else {
                iced::theme::Button::Custom(Box::new(InactiveTabStyle))
            })
            .on_press(Message::TabSelected(Tab::Traffic)),
            
            // Descriptors tab
            button(
                text("Descriptors")
                    .size(16)
                    .width(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
            )
            .padding(10)
            .width(Length::Fill)
            .style(if matches!(self.active_tab, Tab::Descriptors) {
                iced::theme::Button::Custom(Box::new(ActiveTabStyle))
            } else {
                iced::theme::Button::Custom(Box::new(InactiveTabStyle))
            })
            .on_press(Message::TabSelected(Tab::Descriptors))
        ]
        .spacing(1)
        .width(Length::Fill);

        // Tab content
        let content = match self.active_tab {
            Tab::Devices => self.device_view.view().map(Message::DeviceViewMessage),
            Tab::Traffic => self.traffic_view.view().map(Message::TrafficViewMessage),
            Tab::Descriptors => self.descriptor_view.view().map(Message::DescriptorViewMessage),
        };

        column![header, error_banner, tab_buttons, content]
            .spacing(20)
            .padding(20)
            .into()
    }
}
