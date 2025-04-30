use crate::cynthion::connection::CynthionConnection;
use crate::gui::views::{DeviceView, TrafficView, DescriptorView};
use crate::usb::UsbDecoder;
use iced::widget::{button, column, container, row, text};
use iced::{executor, Application, Background, Color, Command, Element, Length, Subscription, Theme};
use std::sync::{Arc, Mutex};
// Use the log macros for consistent error handling
use log::{info, error, debug};

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
    // New message types for MitM capture functionality
    StartCapture,           // Start USB traffic capture (MitM mode)
    StopCapture,            // Stop USB traffic capture
    FetchCaptureData,       // Fetch captured USB data from device
    ClearCaptureBuffer,     // Clear capture buffer on device
    CaptureStarted,         // Notification that capture has started successfully
    CaptureStopped,         // Notification that capture has stopped successfully
    CaptureError(String),   // Error message from capture operation
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
                    // First add the raw packet for traditional view
                    self.traffic_view.add_packet(data.clone(), decoded.clone());
                    
                    // Now also process the data with our enhanced MitM module
                    if let Some(connection) = &self.connection {
                        if let Ok(conn) = connection.lock() {
                            // Process the raw data into USB transactions
                            let transactions = conn.process_mitm_traffic(&data);
                            
                            if !transactions.is_empty() {
                                debug!("Adding {} transactions to traffic view", transactions.len());
                                
                                // Add each transaction to the traffic view
                                for transaction in transactions {
                                    self.traffic_view.add_transaction(transaction);
                                }
                            }
                        }
                    }
                    
                    // Continue with descriptor view update
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
            Message::StartCapture => {
                if let Some(connection) = &self.connection {
                    let conn_clone = Arc::clone(connection);
                    
                    info!("Starting USB traffic capture...");
                    
                    // Update UI state to show capture is active
                    self.traffic_view.set_capture_active(true);
                    
                    // First get the info we need under a short-lived lock
                    let (is_simulation, start_result) = {
                        match conn_clone.lock() {
                            Ok(mut conn) => {
                                let is_sim = conn.is_simulation_mode();
                                
                                // Try to start capture while we have the lock
                                let result = if !is_sim {
                                    // Only try to actually start capture for real devices
                                    conn.start_capture()
                                } else {
                                    // For simulation mode, just pretend it succeeded
                                    Ok(())
                                };
                                
                                // Return values that we need after dropping lock
                                (is_sim, result)
                            },
                            Err(_) => {
                                // Lock failed
                                error!("Failed to lock connection");
                                self.error_message = Some("Failed to access USB device".to_string());
                                return Command::none();
                            }
                        }
                    }; // MutexGuard is dropped here
                    
                    // Now we can perform async operations without holding a lock
                    Command::perform(
                        async move {
                            if is_simulation {
                                // In simulation mode
                                info!("Starting simulated USB traffic capture");
                                
                                // Simulate a delay for starting capture
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                Message::CaptureStarted
                            } else {
                                // In real device mode, check the result of start_capture
                                match start_result {
                                    Ok(_) => {
                                        info!("USB traffic capture started successfully");
                                        Message::CaptureStarted
                                    },
                                    Err(e) => {
                                        error!("Failed to start capture: {}", e);
                                        Message::CaptureError(format!("Failed to start capture: {}", e))
                                    }
                                }
                            }
                        },
                        |msg| msg
                    )
                } else {
                    self.error_message = Some("No device connected".to_string());
                    Command::none()
                }
            }
            Message::StopCapture => {
                if let Some(connection) = &self.connection {
                    let conn_clone = Arc::clone(connection);
                    
                    info!("Stopping USB traffic capture...");
                    
                    // Update UI state to show capture is not active
                    self.traffic_view.set_capture_active(false);
                    
                    // First get the info we need under a short-lived lock
                    let (is_simulation, stop_result) = {
                        match conn_clone.lock() {
                            Ok(mut conn) => {
                                let is_sim = conn.is_simulation_mode();
                                
                                // Try to stop capture while we have the lock
                                let result = if !is_sim {
                                    // Only try to actually stop capture for real devices
                                    conn.stop_capture()
                                } else {
                                    // For simulation mode, just pretend it succeeded
                                    Ok(())
                                };
                                
                                // Return values that we need after dropping lock
                                (is_sim, result)
                            },
                            Err(_) => {
                                // Lock failed
                                error!("Failed to lock connection");
                                self.error_message = Some("Failed to access USB device".to_string());
                                return Command::none();
                            }
                        }
                    }; // MutexGuard is dropped here
                    
                    // Now we can perform async operations without holding a lock
                    Command::perform(
                        async move {
                            if is_simulation {
                                // In simulation mode
                                info!("Stopping simulated USB traffic capture");
                                
                                // Simulate a delay for stopping capture
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                Message::CaptureStopped
                            } else {
                                // In real device mode, check the result of stop_capture
                                match stop_result {
                                    Ok(_) => {
                                        info!("USB traffic capture stopped successfully");
                                        Message::CaptureStopped
                                    },
                                    Err(e) => {
                                        error!("Failed to stop capture: {}", e);
                                        Message::CaptureError(format!("Failed to stop capture: {}", e))
                                    }
                                }
                            }
                        },
                        |msg| msg
                    )
                } else {
                    self.error_message = Some("No device connected".to_string());
                    Command::none()
                }
            }
            Message::ClearCaptureBuffer => {
                if let Some(connection) = &self.connection {
                    let conn_clone = Arc::clone(connection);
                    
                    info!("Clearing capture buffer...");
                    
                    // First get the info we need under a short-lived lock
                    let (is_simulation, clear_result) = {
                        match conn_clone.lock() {
                            Ok(mut conn) => {
                                let is_sim = conn.is_simulation_mode();
                                
                                // Try to clear buffer while we have the lock
                                let result = conn.clear_capture_buffer();
                                
                                // Return values that we need after dropping lock
                                (is_sim, result)
                            },
                            Err(_) => {
                                // Lock failed
                                error!("Failed to lock connection");
                                self.error_message = Some("Failed to access USB device".to_string());
                                return Command::none();
                            }
                        }
                    }; // MutexGuard is dropped here
                    
                    // Now we can perform async operations without holding a lock
                    Command::perform(
                        async move {
                            // Process the result
                            match clear_result {
                                Ok(_) => {
                                    info!("Capture buffer cleared successfully");
                                    
                                    // If we're in simulation mode, add a small delay for realism
                                    if is_simulation {
                                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                                    }
                                    
                                    Message::TrafficViewMessage(crate::gui::views::traffic_view::Message::ClearTraffic)
                                },
                                Err(e) => {
                                    error!("Failed to clear capture buffer: {}", e);
                                    Message::CaptureError(format!("Failed to clear buffer: {}", e))
                                }
                            }
                        },
                        |msg| msg
                    )
                } else {
                    // No device connected, just clear the UI
                    self.traffic_view.clear_traffic();
                    Command::none()
                }
            }
            Message::CaptureStarted => {
                info!("USB traffic capture started successfully");
                self.error_message = None;
                Command::none()
            }
            Message::CaptureStopped => {
                info!("USB traffic capture stopped successfully");
                self.error_message = None;
                Command::none()
            }
            Message::CaptureError(error) => {
                error!("USB traffic capture error: {}", error);
                self.error_message = Some(format!("Capture error: {}", error));
                // Reset capture state in UI
                self.traffic_view.set_capture_active(false);
                Command::none()
            }
            Message::FetchCaptureData => {
                if let Some(connection) = &self.connection {
                    let conn_clone = Arc::clone(connection);
                    
                    // Use a simulated capture data approach for safety
                    // This avoids the Send/Sync issues with MutexGuard across await points
                    // First get the info we need under a short-lived lock
                    let (is_simulation, maybe_data, connection_ref) = {
                        match conn_clone.lock() {
                            Ok(conn) => {
                                let is_sim = conn.is_simulation_mode();
                                
                                // Get simulated data while we have the lock if in simulation mode
                                let data = if is_sim {
                                    // Use enhanced simulation with our new module
                                    let raw_data = conn.get_simulated_mitm_traffic();
                                    
                                    // Process the raw data into USB transactions using our new module
                                    let transactions = conn.process_mitm_traffic(&raw_data);
                                    
                                    // For simulated data, always provide something to show
                                    info!("Generated {} simulated USB transactions", transactions.len());
                                    
                                    // Still need to return raw data for compatibility with current code
                                    Some(raw_data)
                                } else {
                                    // For real device, we'll get data after dropping the lock
                                    None
                                };
                                
                                (is_sim, data, Arc::clone(&conn_clone))
                            },
                            Err(_) => {
                                error!("Failed to lock connection");
                                return Command::none();
                            }
                        }
                    }; // MutexGuard is dropped here
                    
                    // Now we can perform async operations without holding a lock
                    Command::perform(
                        async move {
                            // Generate a timestamp for the capture
                            let _timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs_f64(); // Using _timestamp to indicate it may be used in future
                            
                            // If we have simulated data from the simulation mode, use it
                            if let Some(data) = maybe_data {
                                if !data.is_empty() {
                                    info!("Received {} bytes of MitM USB traffic", data.len());
                                    
                                    // Process the data using our new method (if we can get a lock)
                                    if let Ok(conn) = connection_ref.lock() {
                                        let transactions = conn.process_mitm_traffic(&data);
                                        info!("Processed into {} USB transactions", transactions.len());
                                        
                                        // Log details of the first few transactions for debugging
                                        for (i, tx) in transactions.iter().take(3).enumerate() {
                                            info!("Transaction {}: Type={:?}, Endpoint={}, Addr={}", 
                                                i, tx.transfer_type, tx.endpoint, tx.device_address);
                                        }
                                    }
                                    
                                    return Message::USBDataReceived(data);
                                }
                            }
                            
                            // For real devices or when simulation didn't provide data, 
                            // we use a polling approach with a delay
                            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                            
                            if is_simulation {
                                // Use our enhanced simulation module for more realistic data
                                let sim_data = if let Ok(conn) = connection_ref.lock() {
                                    conn.get_simulated_mitm_traffic()
                                } else {
                                    // Fallback if we couldn't get a lock for some reason
                                    crate::usb::generate_simulated_mitm_traffic()
                                };
                                
                                info!("Generated {} bytes of MitM USB traffic", sim_data.len());
                                return Message::USBDataReceived(sim_data);
                            } else {
                                // For real device, we would attempt to get actual data here
                                // But since we need to implement proper async handling for real devices,
                                // we'll return an empty result for now
                                info!("No MitM traffic data received from device");
                                return Message::TrafficViewMessage(crate::gui::views::traffic_view::Message::NoOp);
                            }
                        },
                        |msg| msg
                    )
                } else {
                    Command::none()
                }
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
            Tab::Traffic => {
                // Add capture control buttons when in Traffic tab and connected
                let capture_controls = if self.connected {
                    // Visibility based on connection status
                    row![
                        button("Start Capture")
                            .on_press(Message::StartCapture)
                            .style(if self.dark_mode {
                                iced::theme::Button::Custom(Box::new(crate::gui::styles::DarkModePrimaryButton))
                            } else {
                                iced::theme::Button::Primary
                            }),
                        button("Stop Capture")
                            .on_press(Message::StopCapture)
                            .style(if self.dark_mode {
                                iced::theme::Button::Custom(Box::new(crate::gui::styles::DarkModeSecondaryButton))
                            } else {
                                iced::theme::Button::Secondary
                            }),
                        button("Fetch Data")
                            .on_press(Message::FetchCaptureData)
                            .style(if self.dark_mode {
                                iced::theme::Button::Custom(Box::new(crate::gui::styles::DarkModePrimaryButton))
                            } else {
                                iced::theme::Button::Primary
                            }),
                        button("Clear Buffer")
                            .on_press(Message::ClearCaptureBuffer)
                            .style(if self.dark_mode {
                                iced::theme::Button::Custom(Box::new(crate::gui::styles::DarkModeDestructiveButton))
                            } else {
                                iced::theme::Button::Destructive
                            })
                    ]
                    .spacing(10)
                    .padding(5)
                } else {
                    row![
                        text("Connect to a device to enable USB traffic capture").style(iced::theme::Text::Color(Color::from_rgb(0.7, 0.7, 0.7)))
                    ]
                    .padding(5)
                };
                
                column![
                    capture_controls,
                    self.traffic_view.view().map(Message::TrafficViewMessage)
                ]
                .spacing(10)
                .into()
            },
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
