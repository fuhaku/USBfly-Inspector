use crate::cynthion::connection::CynthionConnection;
use crate::gui::views::{DeviceView, TrafficView, DescriptorView};
use crate::usb::UsbDecoder;
use iced::widget::{button, column, container, row, text};
use iced::{executor, Application, Background, Color, Command, Element, Length, Subscription, Theme};
use std::sync::{Arc, Mutex};
// Use the log macros for consistent error handling
use log::{info, error};

// Custom tab styles
struct ActiveTabStyle;
struct InactiveTabStyle;
struct DarkModeActiveTabStyle;
struct DarkModeInactiveTabStyle;

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

impl iced::widget::button::StyleSheet for DarkModeActiveTabStyle {
    type Style = Theme;
    
    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            text_color: Color::from_rgb(0.9, 0.9, 1.0),
            background: Some(Background::Color(Color::from_rgb(0.1, 0.4, 0.6))),
            border_radius: 4.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            shadow_offset: iced::Vector::default(),
        }
    }
}

impl iced::widget::button::StyleSheet for DarkModeInactiveTabStyle {
    type Style = Theme;
    
    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            text_color: Color::from_rgb(0.7, 0.7, 0.7),
            background: Some(Background::Color(Color::from_rgb(0.2, 0.2, 0.2))),
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
    dark_mode: bool,
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
    ToggleDarkMode(bool),
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
            dark_mode: true, // Default to dark mode for a hacker-friendly UI
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
                        // Connect to device and handle result directly
                        match CynthionConnection::connect().await {
                            Ok(conn) => {
                                // Extra safety check - verify connection is actually valid
                                if conn.is_connected() {
                                    // Special handling for macOS to avoid crashes
                                    if cfg!(target_os = "macos") {
                                        info!("On macOS, using extra safety protection");
                                        
                                        // Mark as simulation mode but keep handle for device info
                                        // This ensures a safer experience on macOS
                                        let mut safe_conn = conn;
                                        safe_conn.set_simulation_mode(true);
                                        
                                        let connection = Arc::new(Mutex::new(safe_conn));
                                        Message::ConnectionEstablished(connection)
                                    } else {
                                        // For other platforms, continue with normal operation
                                        let connection = Arc::new(Mutex::new(conn));
                                        Message::ConnectionEstablished(connection)
                                    }
                                } else {
                                    Message::ConnectionFailed("Connection state invalid".to_string())
                                }
                            }
                            Err(e) => {
                                error!("Connection error: {}", e);
                                Message::ConnectionFailed(e.to_string())
                            }
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
            },
            Message::ToggleDarkMode(enabled) => {
                self.dark_mode = enabled;
                // Sync dark mode with child views
                let mut commands = Vec::new();
                
                // Update traffic view's dark mode
                commands.push(
                    self.traffic_view.update(crate::gui::views::traffic_view::Message::ToggleDarkMode(enabled))
                        .map(Message::TrafficViewMessage)
                );
                
                // Update descriptor view's dark mode
                commands.push(
                    self.descriptor_view.update(crate::gui::views::descriptor_view::Message::ToggleDarkMode(enabled))
                        .map(Message::DescriptorViewMessage)
                );
                
                Command::batch(commands)
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        // Create a batch of subscriptions for different features
        let mut subscriptions = Vec::new();
        
        // Subscribe to USB data from connection if connected
        if let Some(connection) = &self.connection {
            // CRITICAL SAFETY: Use a try_clone mechanism to avoid segfaults
            // Only clone if we're sure the connection is in a consistent state
            if self.connected {
                let conn = Arc::clone(connection);
                
                // Add a wrapper to catch any panics at the task level
                // This prevents a thread crash from bringing down the whole application
                subscriptions.push(
                    iced::subscription::unfold(
                        "usb-data-subscription",
                        conn,
                        move |conn| async move {
                            // Double-check the connection is still valid before proceeding
                            // This catches race conditions between when we create the subscription
                            // and when the async task actually runs
                            
                            // Use std::panic::catch_unwind around the entire operation
                            // to prevent thread crashes that propagate to the application
                            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                // Use a clone + drop approach to avoid holding MutexGuard across an await point
                                match conn.lock() {
                                    Ok(mut connection) => {
                                        // Extra safety check that connection is active
                                        if !connection.is_connected() {
                                            return Err("Device not connected".to_string());
                                        }
                                        
                                        // Clone the required fields or prepare the async call
                                        match connection.read_data_clone() {
                                            Ok(data) => Ok(data),
                                            Err(e) => Err(e.to_string())
                                        }
                                    },
                                    Err(e) => {
                                        // Handle poisoned mutex (another thread panicked while holding the lock)
                                        log::error!("Mutex poisoned: {}", e);
                                        Err("Connection error: mutex poisoned".to_string())
                                    }
                                }
                            }));
                            
                            // First level catch_unwind handler
                            match result {
                                Ok(inner_result) => {
                                    // Second level actual data result
                                    match inner_result {
                                        Ok(data) => (Message::USBDataReceived(data), conn),
                                        Err(err_msg) => {
                                            // Log connection errors but avoid overly verbose logs for routine disconnects
                                            if !err_msg.contains("not active") && 
                                               !err_msg.contains("disconnected") &&
                                               !err_msg.contains("not connected") {
                                                log::warn!("Connection error: {}", err_msg);
                                            }
                                            
                                            // Short delay to prevent error message flooding
                                            std::thread::sleep(std::time::Duration::from_millis(100));
                                            
                                            // Don't disconnect immediately on first error
                                            // Return a non-fatal message so we'll try again
                                            (Message::USBDataReceived(Vec::new()), conn)
                                        }
                                    }
                                },
                                Err(_) => {
                                    // Panic occurred in the task - recover gracefully
                                    log::error!("Recovered from subscription thread panic");
                                    std::thread::sleep(std::time::Duration::from_millis(500));
                                    
                                    // Return empty data and continue
                                    (Message::USBDataReceived(Vec::new()), conn)
                                }
                            }
                        },
                    )
                );
            }
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

        // Dark mode toggle
        let dark_mode_button = if self.dark_mode {
            button("Light Mode")
                .on_press(Message::ToggleDarkMode(false))
                .style(if self.dark_mode {
                    iced::theme::Button::Custom(Box::new(crate::gui::styles::DarkModePrimaryButton))
                } else {
                    iced::theme::Button::Primary
                })
        } else {
            button("Dark Mode")
                .on_press(Message::ToggleDarkMode(true))
                .style(iced::theme::Button::Primary)
        };
        
        let connect_button = if self.connected {
            button("Disconnect")
                .on_press(Message::Disconnect)
                .style(if self.dark_mode {
                    iced::theme::Button::Custom(Box::new(crate::gui::styles::DarkModeDestructiveButton))
                } else {
                    iced::theme::Button::Destructive
                })
        } else {
            button("Connect to Cynthion")
                .on_press(Message::Connect)
                .style(if self.dark_mode {
                    iced::theme::Button::Custom(Box::new(crate::gui::styles::DarkModePrimaryButton))
                } else {
                    iced::theme::Button::Primary
                })
        };

        let save_button = button("Save Capture")
            .on_press(Message::SaveCapture)
            .style(if self.dark_mode {
                iced::theme::Button::Custom(Box::new(crate::gui::styles::DarkModeSecondaryButton))
            } else {
                iced::theme::Button::Secondary
            });

        let load_button = button("Load Capture")
            .on_press(Message::LoadCapture)
            .style(if self.dark_mode {
                iced::theme::Button::Custom(Box::new(crate::gui::styles::DarkModeSecondaryButton))
            } else {
                iced::theme::Button::Secondary
            });

        let clear_button = button("Clear")
            .on_press(Message::ClearCapture)
            .style(if self.dark_mode {
                iced::theme::Button::Custom(Box::new(crate::gui::styles::DarkModeDestructiveButton))
            } else {
                iced::theme::Button::Destructive
            });

        let header = row![
            title,
            row![connect_button, save_button, load_button, clear_button, dark_mode_button]
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
                if self.dark_mode {
                    iced::theme::Button::Custom(Box::new(DarkModeActiveTabStyle))
                } else {
                    iced::theme::Button::Custom(Box::new(ActiveTabStyle))
                }
            } else {
                if self.dark_mode {
                    iced::theme::Button::Custom(Box::new(DarkModeInactiveTabStyle))
                } else {
                    iced::theme::Button::Custom(Box::new(InactiveTabStyle))
                }
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
                if self.dark_mode {
                    iced::theme::Button::Custom(Box::new(DarkModeActiveTabStyle))
                } else {
                    iced::theme::Button::Custom(Box::new(ActiveTabStyle))
                }
            } else {
                if self.dark_mode {
                    iced::theme::Button::Custom(Box::new(DarkModeInactiveTabStyle))
                } else {
                    iced::theme::Button::Custom(Box::new(InactiveTabStyle))
                }
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
                if self.dark_mode {
                    iced::theme::Button::Custom(Box::new(DarkModeActiveTabStyle))
                } else {
                    iced::theme::Button::Custom(Box::new(ActiveTabStyle))
                }
            } else {
                if self.dark_mode {
                    iced::theme::Button::Custom(Box::new(DarkModeInactiveTabStyle))
                } else {
                    iced::theme::Button::Custom(Box::new(InactiveTabStyle))
                }
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

        let main_content = column![header, error_banner, tab_buttons, content]
            .spacing(20)
            .padding(20);
            
        // Apply dark mode container if needed
        if self.dark_mode {
            container(main_content)
                .style(iced::theme::Container::Custom(Box::new(crate::gui::styles::DarkModeContainer)))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            main_content.into()
        }
    }
}
