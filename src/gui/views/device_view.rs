use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Command, Element, Length, Color, Background};
use crate::cynthion::connection::{CynthionConnection, USBDeviceInfo};
use log::{debug, info};

// Constants for compatible USB device VIDs and PIDs
const CYNTHION_VID: u16 = 0x1d50;
const CYNTHION_PID: u16 = 0x615c;
const TEST_VID: u16 = 0x1d50;
const TEST_PID: u16 = 0x60e6;
const GADGETCAP_VID: u16 = 0x1d50;
const GADGETCAP_PID: u16 = 0x6018;

pub struct DeviceView {
    connected_devices: Vec<USBDeviceInfo>,
    selected_device: Option<USBDeviceInfo>,
    last_error: Option<String>,
    // Auto-refresh timer
    last_refresh_time: std::time::Instant,
    auto_refresh_interval: std::time::Duration,
}

// Custom styles for compatible device rows
struct CompatibleDeviceStyle;
impl iced::widget::container::StyleSheet for CompatibleDeviceStyle {
    type Style = iced::Theme;
    
    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(Color::BLACK),
            background: Some(Background::Color(Color::from_rgb(0.7, 0.9, 0.7))),
            border_radius: 4.0.into(),
            border_width: 1.0,
            border_color: Color::from_rgb(0.0, 0.6, 0.0),
        }
    }
}

// Regular device style
struct RegularDeviceStyle;
impl iced::widget::container::StyleSheet for RegularDeviceStyle {
    type Style = iced::Theme;
    
    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(Color::BLACK),
            background: Some(Background::Color(Color::from_rgb(0.95, 0.95, 0.95))),
            border_radius: 4.0.into(),
            border_width: 1.0,
            border_color: Color::from_rgb(0.8, 0.8, 0.8),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    RefreshDevices,
    DeviceSelected(USBDeviceInfo),
    DevicesLoaded(Result<Vec<USBDeviceInfo>, String>),
    CheckAutoRefresh,
    NoOp,
}

impl DeviceView {
    pub fn new() -> Self {
        // Initialize with an empty device list and start the auto-refresh timer
        Self {
            connected_devices: Vec::new(),
            selected_device: None,
            last_error: None,
            last_refresh_time: std::time::Instant::now(),
            // Auto-refresh every 2 seconds by default - this can be tuned for better experience
            auto_refresh_interval: std::time::Duration::from_secs(2),
        }
    }
    
    // Call this method after creating a new instance to start the initial device scan
    pub fn with_initial_scan(self) -> (Self, Command<Message>) {
        let command = Command::perform(
            async {
                // This will run in a separate thread
                match CynthionConnection::list_devices() {
                    Ok(devices) => Ok(devices),
                    Err(e) => Err(format!("Failed to list USB devices: {}", e)),
                }
            },
            Message::DevicesLoaded
        );
        
        (self, command)
    }
    
    // Return a command that will be executed after a short delay to check for device changes
    pub fn subscription(&self) -> iced::Subscription<Message> {
        // Create a periodic timer subscription for device auto-refresh
        iced::time::every(std::time::Duration::from_millis(500))
            .map(|_| Message::CheckAutoRefresh)
    }
    
    // Helper function to determine if a device is compatible
    fn is_compatible_device(vid: u16, pid: u16) -> bool {
        // Use the central compatibility check from CynthionConnection
        CynthionConnection::is_supported_device(vid, pid)
    }
    
    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::RefreshDevices => {
                info!("Manually refreshing connected USB devices");
                // Update last refresh time
                self.last_refresh_time = std::time::Instant::now();
                // Query connected devices asynchronously
                Command::perform(
                    async {
                        // This will run in a separate thread
                        match CynthionConnection::list_devices() {
                            Ok(devices) => Ok(devices),
                            Err(e) => Err(format!("Failed to list USB devices: {}", e)),
                        }
                    },
                    Message::DevicesLoaded
                )
            },
            Message::CheckAutoRefresh => {
                // Check if it's time to auto-refresh
                let now = std::time::Instant::now();
                let elapsed = now.duration_since(self.last_refresh_time);
                
                if elapsed >= self.auto_refresh_interval {
                    debug!("Auto-refreshing USB device list");
                    self.last_refresh_time = now;
                    
                    // Perform the refresh asynchronously
                    Command::perform(
                        async {
                            match CynthionConnection::list_devices() {
                                Ok(devices) => Ok(devices),
                                Err(e) => Err(format!("Failed to list USB devices: {}", e)),
                            }
                        },
                        Message::DevicesLoaded
                    )
                } else {
                    // Not time to refresh yet
                    Command::none()
                }
            },
            Message::DeviceSelected(device) => {
                info!("Selected device: {:04x}:{:04x}", device.vendor_id, device.product_id);
                self.selected_device = Some(device);
                Command::none()
            },
            Message::DevicesLoaded(result) => {
                match result {
                    Ok(devices) => {
                        // Check if the device list has changed
                        let has_changed = if self.connected_devices.len() != devices.len() {
                            true
                        } else {
                            // Check if any device info has changed
                            self.connected_devices.iter().zip(devices.iter())
                                .any(|(old, new)| old.vendor_id != new.vendor_id || old.product_id != new.product_id)
                        };
                        
                        if has_changed {
                            info!("USB device list updated: {} devices", devices.len());
                        } else {
                            debug!("USB device list refreshed (no changes)");
                        }
                        
                        // In case selected device was disconnected, update selection
                        if let Some(selected) = &self.selected_device {
                            // If current selected device is not in the new list, clear selection
                            let still_connected = devices.iter().any(|dev| {
                                dev.vendor_id == selected.vendor_id && 
                                dev.product_id == selected.product_id &&
                                dev.serial_number == selected.serial_number
                            });
                            
                            if !still_connected {
                                info!("Selected device was disconnected");
                                self.selected_device = None;
                            }
                        }
                        
                        self.connected_devices = devices;
                        self.last_error = None;
                    },
                    Err(error) => {
                        debug!("Error loading USB devices: {}", error);
                        // Only show the error if it's a user-initiated refresh
                        if error.contains("manually") {
                            self.last_error = Some(error);
                        }
                    }
                }
                Command::none()
            },
            Message::NoOp => Command::none(),
        }
    }
    
    pub fn view(&self) -> Element<Message> {
        let title = text("Connected USB Devices")
            .size(24)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.5, 0.8)));
            
        let refresh_button = button("Refresh Devices")
            .on_press(Message::RefreshDevices)
            .style(iced::theme::Button::Primary);
            
        let header = row![title, refresh_button]
            .spacing(20)
            .align_items(iced::Alignment::Center)
            .width(Length::Fill);
            
        // Error message display if there's an error
        let error_display = if let Some(error) = &self.last_error {
            container(
                text(format!("Error: {}", error))
                    .style(iced::theme::Text::Color(Color::from_rgb(0.8, 0.0, 0.0)))
            )
            .padding(10)
            .width(Length::Fill)
            .style(iced::theme::Container::Box)
        } else {
            container(text("")).width(Length::Fill)
        };
        
        // Device list display
        let device_list = if self.connected_devices.is_empty() {
            column![
                text("No USB devices detected. Click 'Refresh Devices' to scan.")
                    .width(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
            ]
            .width(Length::Fill)
            .padding(20)
        } else {
            let devices: Vec<Element<_>> = self.connected_devices
                .iter()
                .map(|device| {
                    let is_compatible = Self::is_compatible_device(device.vendor_id, device.product_id);
                    
                    let device_row = column![
                        row![
                            text(format!("{} {}", 
                                device.manufacturer.as_deref().unwrap_or("Unknown"), 
                                device.product.as_deref().unwrap_or("Device")))
                                .width(Length::Fill)
                                .size(16),
                            if is_compatible {
                                text("✓ Compatible")
                                    .style(iced::theme::Text::Color(Color::from_rgb(0.0, 0.6, 0.0)))
                            } else {
                                text("Incompatible")
                                    .style(iced::theme::Text::Color(Color::from_rgb(0.6, 0.6, 0.6)))
                            }
                        ].spacing(10),
                        text(format!("VID:{:04x} PID:{:04x} SN:{}", 
                            device.vendor_id, 
                            device.product_id,
                            device.serial_number.as_deref().unwrap_or("N/A")))
                            .size(14)
                            .style(iced::theme::Text::Color(Color::from_rgb(0.3, 0.3, 0.3)))
                    ]
                    .padding(10)
                    .spacing(5)
                    .width(Length::Fill);
                    
                    let device_clone = device.clone();
                    container(
                        button(device_row)
                            .width(Length::Fill)
                            .style(iced::theme::Button::Text)
                            .on_press(Message::DeviceSelected(device_clone))
                    )
                    .width(Length::Fill)
                    .style(if is_compatible {
                        iced::theme::Container::Custom(Box::new(CompatibleDeviceStyle))
                    } else {
                        iced::theme::Container::Custom(Box::new(RegularDeviceStyle))
                    })
                    .into()
                })
                .collect();
                
            column(devices)
                .spacing(10)
                .width(Length::Fill)
        };
        
        // Selected device detail view
        let device_details = if let Some(device) = &self.selected_device {
            let is_compatible = Self::is_compatible_device(device.vendor_id, device.product_id);
            
            column![
                text("Selected Device Details")
                    .size(18)
                    .style(iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.4, 0.7))),
                column![
                    row![
                        text("Vendor ID:").width(Length::FillPortion(1)),
                        text(format!("0x{:04x}", device.vendor_id)).width(Length::FillPortion(2))
                    ].padding(5).spacing(10),
                    row![
                        text("Product ID:").width(Length::FillPortion(1)),
                        text(format!("0x{:04x}", device.product_id)).width(Length::FillPortion(2))
                    ].padding(5).spacing(10),
                    row![
                        text("Manufacturer:").width(Length::FillPortion(1)),
                        text(device.manufacturer.as_deref().unwrap_or("Unknown")).width(Length::FillPortion(2))
                    ].padding(5).spacing(10),
                    row![
                        text("Product:").width(Length::FillPortion(1)),
                        text(device.product.as_deref().unwrap_or("Unknown")).width(Length::FillPortion(2))
                    ].padding(5).spacing(10),
                    row![
                        text("Serial Number:").width(Length::FillPortion(1)),
                        text(device.serial_number.as_deref().unwrap_or("N/A")).width(Length::FillPortion(2))
                    ].padding(5).spacing(10),
                    row![
                        text("Compatibility:").width(Length::FillPortion(1)),
                        text(if is_compatible { "Compatible with USBfly" } else { "Not compatible" })
                            .style(if is_compatible {
                                iced::theme::Text::Color(Color::from_rgb(0.0, 0.6, 0.0))
                            } else {
                                iced::theme::Text::Color(Color::from_rgb(0.7, 0.0, 0.0))
                            })
                            .width(Length::FillPortion(2))
                    ].padding(5).spacing(10)
                ]
                .spacing(5)
                .padding(10)
                .width(Length::Fill)
            ]
            .spacing(10)
            .width(Length::Fill)
        } else {
            column![
                text("No device selected")
                    .width(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
                    .vertical_alignment(iced::alignment::Vertical::Center)
            ]
            .width(Length::Fill)
            .height(Length::Fill)
        };
        
        let content = column![
            header,
            error_display,
            container(
                scrollable(device_list)
                    .height(Length::Fill)
            )
                .style(iced::theme::Container::Box)
                .padding(10)
                .width(Length::Fill)
                .height(Length::Fixed(200.0)),
            container(device_details)
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
