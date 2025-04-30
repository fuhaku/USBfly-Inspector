use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Command, Element, Length, Color, Background};
use crate::cynthion::connection::{CynthionConnection, USBDeviceInfo};
use log::{debug, info};

// Constants for compatible USB device VIDs and PIDs
// These are kept for reference and potential future use
#[allow(dead_code)]
const CYNTHION_VID: u16 = 0x1d50;
#[allow(dead_code)]
const CYNTHION_PID: u16 = 0x615c;
#[allow(dead_code)]
const TEST_VID: u16 = 0x1d50;
#[allow(dead_code)]
const TEST_PID: u16 = 0x60e6;
#[allow(dead_code)]
const GADGETCAP_VID: u16 = 0x1d50;
#[allow(dead_code)]
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
            text_color: Some(Color::from_rgb(0.1, 0.1, 0.1)),
            background: Some(Background::Color(Color::from_rgba(0.0, 0.7, 0.4, 0.2))),
            border_radius: crate::gui::styles::BORDER_RADIUS.into(),
            border_width: 1.0,
            border_color: Color::from_rgba(0.0, 0.7, 0.4, 0.5),
        }
    }
}

// Regular device style
struct RegularDeviceStyle;
impl iced::widget::container::StyleSheet for RegularDeviceStyle {
    type Style = iced::Theme;
    
    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(crate::gui::styles::color::dark::TEXT),
            background: Some(Background::Color(Color::from_rgba(0.2, 0.2, 0.3, 0.1))),
            border_radius: crate::gui::styles::BORDER_RADIUS.into(),
            border_width: 1.0,
            border_color: Color::from_rgba(0.4, 0.4, 0.5, 0.3),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    RefreshDevices,
    ForceRefreshDevices,
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
            Message::ForceRefreshDevices => {
                info!("Force refreshing connected USB devices (checking for real hardware)");
                // Set the force refresh flag
                std::env::set_var("USBFLY_FORCE_REFRESH", "1");
                // Update last refresh time
                self.last_refresh_time = std::time::Instant::now();
                // Query connected devices asynchronously
                Command::perform(
                    async {
                        // This will run in a separate thread and use the force refresh flag
                        match CynthionConnection::list_devices() {
                            Ok(devices) => Ok(devices),
                            Err(e) => Err(format!("Failed to force refresh USB devices: {}", e)),
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
                        
                        // Update selected device with more complete information if it's in the new list
                        if let Some(selected) = &mut self.selected_device {
                            // The selected device may have more complete information in the new list
                            for dev in &devices {
                                if dev.vendor_id == selected.vendor_id && 
                                   dev.product_id == selected.product_id {
                                   
                                    // Update the selected device with more complete information
                                    // Only if it's missing information and the new device has it
                                    if selected.manufacturer.is_none() && dev.manufacturer.is_some() {
                                        selected.manufacturer = dev.manufacturer.clone();
                                    }
                                    if selected.product.is_none() && dev.product.is_some() {
                                        selected.product = dev.product.clone();
                                    }
                                    if selected.serial_number.is_none() && dev.serial_number.is_some() {
                                        selected.serial_number = dev.serial_number.clone();
                                    }
                                    
                                    // Log the update
                                    debug!("Updated selected device information");
                                    break;
                                }
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
        // Use our title style with theme-compatible formatting
        let title = container(
            text("Connected USB Devices")
                .size(24)
                .style(iced::theme::Text::Color(crate::gui::styles::color::PRIMARY_DARK))
        )
        .padding([0, 10, 0, 10]);
        
        // Use primary and secondary button styles that work with the iced theme system
        let refresh_button = button("Refresh Devices")
            .on_press(Message::RefreshDevices)
            .style(iced::theme::Button::Primary);
            
        let force_refresh_button = button("Force Scan for Hardware")
            .on_press(Message::ForceRefreshDevices)
            .style(iced::theme::Button::Secondary);
            
        let header = row![title, refresh_button, force_refresh_button]
            .spacing(20)
            .align_items(iced::Alignment::Center)
            .width(Length::Fill);
            
        // Error message display if there's an error
        let error_display = if let Some(error) = &self.last_error {
            container(
                text(format!("Error: {}", error))
                    .style(iced::theme::Text::Color(crate::gui::styles::color::ERROR))
            )
            .padding(10)
            .width(Length::Fill)
            .style(iced::theme::Container::Box)
        } else {
            container(text("")).width(Length::Fill)
        };
        
        // Device list display with card-like appearance
        let device_list = if self.connected_devices.is_empty() {
            container(
                column![
                    text("No USB devices detected.").width(Length::Fill)
                        .horizontal_alignment(iced::alignment::Horizontal::Center)
                        .style(iced::theme::Text::Color(crate::gui::styles::color::INFO)),
                    text("Click 'Refresh Devices' for a standard scan or 'Force Scan for Hardware' to check for newly connected devices.")
                        .width(Length::Fill)
                        .horizontal_alignment(iced::alignment::Horizontal::Center)
                        .size(14)
                        .style(iced::theme::Text::Color(crate::gui::styles::color::TEXT_SECONDARY))
                ]
                .spacing(10)
            )
            .padding(15)
            .width(Length::Fill)
            .style(iced::theme::Container::Box)
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
                                    .style(iced::theme::Text::Color(crate::gui::styles::color::SUCCESS))
                            } else {
                                text("Incompatible")
                                    .style(iced::theme::Text::Color(crate::gui::styles::color::TEXT_SECONDARY))
                            }
                        ].spacing(10),
                        text(format!("VID:{:04x} PID:{:04x} SN:{}", 
                            device.vendor_id, 
                            device.product_id,
                            device.serial_number.as_deref().unwrap_or("N/A")))
                            .size(14)
                            .style(iced::theme::Text::Color(crate::gui::styles::color::TEXT_SECONDARY))
                    ]
                    .padding(10)
                    .spacing(5)
                    .width(Length::Fill);
                    
                    let device_clone = device.clone();
                    container(
                        button(device_row)
                            .width(Length::Fill)
                            .style(iced::theme::Button::Custom(Box::new(crate::gui::styles::DeviceButtonStyle)))
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
                
            container(
                column(devices)
                    .spacing(10)
                    .width(Length::Fill)
            )
            .padding(15)
            .width(Length::Fill)
            .style(iced::theme::Container::Box)
        };
        
        // Selected device detail view
        let device_details = if let Some(device) = &self.selected_device {
            let is_compatible = Self::is_compatible_device(device.vendor_id, device.product_id);
            
            column![
                row![
                    text("Selected Device Details").size(18),
                    if is_compatible {
                        text("✓ Compatible with USBfly")
                            .style(iced::theme::Text::Color(crate::gui::styles::color::SUCCESS))
                    } else {
                        text("⚠ Not compatible")
                            .style(iced::theme::Text::Color(crate::gui::styles::color::ERROR))
                    }
                ]
                .spacing(15)
                .width(Length::Fill),
                
                container(
                    column![
                        row![
                            text("Vendor ID:")
                                .width(Length::FillPortion(1))
                                .style(iced::theme::Text::Color(crate::gui::styles::color::dark::TEXT_SECONDARY)),
                            text(format!("0x{:04x}", device.vendor_id))
                                .width(Length::FillPortion(2))
                                .style(iced::theme::Text::Color(crate::gui::styles::color::dark::PRIMARY_LIGHT))
                        ].padding(5).spacing(10),
                        row![
                            text("Product ID:")
                                .width(Length::FillPortion(1))
                                .style(iced::theme::Text::Color(crate::gui::styles::color::dark::TEXT_SECONDARY)),
                            text(format!("0x{:04x}", device.product_id))
                                .width(Length::FillPortion(2))
                                .style(iced::theme::Text::Color(crate::gui::styles::color::dark::PRIMARY_LIGHT))
                        ].padding(5).spacing(10),
                        row![
                            text("Manufacturer:")
                                .width(Length::FillPortion(1))
                                .style(iced::theme::Text::Color(crate::gui::styles::color::dark::TEXT_SECONDARY)),
                            text(device.manufacturer.as_deref().unwrap_or("Unknown"))
                                .width(Length::FillPortion(2))
                        ].padding(5).spacing(10),
                        row![
                            text("Product:")
                                .width(Length::FillPortion(1))
                                .style(iced::theme::Text::Color(crate::gui::styles::color::dark::TEXT_SECONDARY)),
                            text(device.product.as_deref().unwrap_or("Unknown"))
                                .width(Length::FillPortion(2))
                        ].padding(5).spacing(10),
                        row![
                            text("Serial Number:")
                                .width(Length::FillPortion(1))
                                .style(iced::theme::Text::Color(crate::gui::styles::color::dark::TEXT_SECONDARY)),
                            text(device.serial_number.as_deref().unwrap_or("N/A"))
                                .width(Length::FillPortion(2))
                        ].padding(5).spacing(10),
                    ]
                    .spacing(5)
                    .width(Length::Fill)
                )
                .style(if is_compatible {
                    iced::theme::Container::Custom(Box::new(CompatibleDeviceStyle))
                } else {
                    iced::theme::Container::Custom(Box::new(RegularDeviceStyle))
                })
                .padding(10)
                .width(Length::Fill)
            ]
            .spacing(10)
            .width(Length::Fill)
        } else {
            column![
                text("No device selected")
                    .size(18)
                    .style(iced::theme::Text::Color(crate::gui::styles::color::dark::TEXT_SECONDARY))
                    .width(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
                text("Select a device from the list above to view details")
                    .style(iced::theme::Text::Color(crate::gui::styles::color::dark::TEXT_SECONDARY))
                    .width(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
            ]
            .spacing(10)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(iced::Alignment::Center)
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
