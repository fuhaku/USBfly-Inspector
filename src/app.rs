use crate::cynthion::connection::CynthionConnection;
use crate::gui::views::{DeviceView, TrafficView, DescriptorView};
use crate::usb::UsbDecoder;
use iced::widget::{button, column, container, row, text};
use iced::{executor, Application, Background, Color, Command, Element, Length, Subscription, Theme};
use std::sync::{Arc, Mutex};
// Use the log macros for consistent error handling
use log::{info, error, debug, warn};

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
    ConnectionPossiblyFailed,  // New message for persistent USB read failures
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
    ProcessingCapture(Arc<Mutex<CynthionConnection>>), // Process capture data from real device
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
                            Ok(mut conn) => {
                                // Extra safety check - verify connection is actually valid
                                if conn.is_connected() {
                                    // Check if hardware mode is forced via environment variable
                                    let force_hardware = std::env::var("USBFLY_FORCE_HARDWARE")
                                        .map(|val| val == "1")
                                        .unwrap_or(false);
                                    
                                    // Check if we have a real Cynthion device
                                    let is_real_device = conn.is_real_hardware_device();
                                    
                                    // Determine when to use simulation vs hardware mode
                                    let (use_simulation, reason) = if force_hardware {
                                        // If hardware mode is forced, always use real device
                                        (false, "Hardware mode forced via USBFLY_FORCE_HARDWARE".to_string())
                                    } else if is_real_device {
                                        // Real device detected, check features
                                        if cfg!(target_os = "macos") {
                                            // On macOS, we may need additional checks
                                            match conn.test_capture_capability() {
                                                Ok(true) => (false, "Device supports direct hardware capture".to_string()),
                                                // CRITICAL CHANGE: Don't fall back to simulation if a real device was found
                                                // Even with timeout issues, we should use the real device for all other operations
                                                Ok(false) => (false, "Using real device with limited MitM capture".to_string()),
                                                Err(e) => {
                                                    warn!("MitM test error, but using real device anyway: {}", e);
                                                    (false, "Using real device despite testing error".to_string())
                                                }
                                            }
                                        } else {
                                            // For non-macOS, use hardware by default when a real device is detected
                                            (false, "Real device detected".to_string())
                                        }
                                    } else {
                                        // No real device found, use simulation
                                        (true, "No real device detected".to_string())
                                    };
                                    
                                    if use_simulation {
                                        info!("Using simulation mode: {}", reason);
                                        
                                        // Mark as simulation mode but keep handle for device info
                                        let mut safe_conn = conn;
                                        safe_conn.set_simulation_mode(true);
                                        
                                        let connection = Arc::new(Mutex::new(safe_conn));
                                        Message::ConnectionEstablished(connection)
                                    } else {
                                        info!("Using real hardware mode: {}", reason);
                                        
                                        // Ensure simulation mode is explicitly disabled
                                        conn.set_simulation_mode(false);
                                        
                                        // Continue with normal operation for hardware access
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
            Message::ConnectionPossiblyFailed => {
                // After too many consecutive errors, we attempt automatic recovery
                log::warn!("Detected possible connection failure from persistent USB read errors");
                
                // First try to validate if the connection is still active
                let is_connected = self.connection.as_ref().map_or(false, |conn| {
                    if let Ok(connection) = conn.try_lock() {
                        connection.is_connected()
                    } else {
                        // If we can't get a lock, assume it's still connected but busy
                        true
                    }
                });
                
                if !is_connected {
                    // If the connection is definitely broken, disconnect and show an error
                    log::error!("Connection validation failed, performing automatic disconnect");
                    self.connection = None;
                    self.connected = false;
                    self.error_message = Some("Connection lost due to persistent errors. Please reconnect.".to_string());
                } else {
                    // Connection appears valid, but we should show a warning
                    log::info!("Connection appears valid despite USB read errors, continuing with caution");
                    self.error_message = Some("USB read errors detected, but connection still active.".to_string());
                }
                
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
                use log::{debug, info, trace};
                
                // Log incoming data size
                debug!("Received USB data packet: {} bytes", data.len());
                if data.len() > 4 {
                    trace!("Data starts with: {:02X?}", &data[0..4]);
                }
                
                // Process received USB data
                if let Some(decoded) = self.usb_decoder.decode(&data) {
                    // First add the raw packet for traditional view
                    self.traffic_view.add_packet(data.clone(), decoded.clone());
                    
                    // Now also process the data with our enhanced MitM module
                    if let Some(connection) = &self.connection {
                        if let Ok(conn) = connection.lock() {
                            // Process the raw data into USB transactions
                            debug!("Processing data through MitM decoder...");
                            let transactions = conn.process_mitm_traffic(&data);
                            
                            if !transactions.is_empty() {
                                info!("Successfully decoded {} USB transactions", transactions.len());
                                debug!("Transaction types: {:?}", transactions.iter()
                                        .map(|t| t.transfer_type)
                                        .collect::<Vec<_>>());
                                
                                // Add each transaction to the traffic view
                                for transaction in transactions {
                                    debug!("Adding transaction ID {} to traffic view", transaction.id);
                                    self.traffic_view.add_transaction(transaction);
                                }
                            } else {
                                debug!("No transactions decoded from packet");
                            }
                        } else {
                            debug!("Could not acquire connection lock for MitM processing");
                        }
                    } else {
                        trace!("No connection available for MitM processing");
                    }
                    
                    // Continue with descriptor view update
                    self.descriptor_view.update_descriptors(decoded);
                } else {
                    debug!("Failed to decode USB data");
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
            Message::ProcessingCapture(connection_ref) => {
                // Process the MitM traffic capture in a thread-safe way
                Command::perform(
                    async move {
                        // Use tokio's spawn_blocking to move the potentially blocking operation
                        // to a separate thread to avoid blocking the event loop
                        let traffic_data = tokio::task::spawn_blocking(move || {
                            // Try to acquire the lock for the connection
                            if let Ok(mut conn) = connection_ref.lock() {
                                // Use the synchronous function to avoid Send issues with MutexGuard
                                conn.read_mitm_traffic_clone()
                            } else {
                                Err(anyhow::anyhow!("Failed to acquire connection lock"))
                            }
                        }).await;
                        
                        // Process the result
                        match traffic_data {
                            Ok(Ok(data)) => {
                                if !data.is_empty() {
                                    info!("Received {} bytes of MitM traffic from device", data.len());
                                    Message::USBDataReceived(data)
                                } else {
                                    debug!("Device returned empty MitM traffic data");
                                    Message::TrafficViewMessage(
                                        crate::gui::views::traffic_view::Message::NoOp
                                    )
                                }
                            },
                            Ok(Err(e)) => {
                                error!("Failed to get MitM traffic: {}", e);
                                Message::CaptureError(format!("Traffic capture error: {}", e))
                            },
                            Err(e) => {
                                error!("Task error during MitM traffic capture: {}", e);
                                Message::CaptureError(format!("Thread error: {}", e))
                            }
                        }
                    },
                    |msg| msg
                )
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
                                // For real device, use the get_captured_traffic function
                                info!("Attempting to get MitM traffic from real device");
                                
                                // Clone the connection reference to avoid Send issues with MutexGuard
                                // and perform the capture outside the async block
                                let connection_ref_clone = Arc::clone(&connection_ref);
                                
                                // Process the capture on the background thread
                                return Message::ProcessingCapture(Arc::clone(&connection_ref_clone));
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
            // Only setup the USB data subscription if we're connected and the UI indicates we're connected
            // This prevents race conditions where we're in the process of connecting/disconnecting
            if self.connected {
                // Create a thread-safe clone of the connection for the subscription
                let conn = Arc::clone(connection);
                
                // Add a robust subscription with improved error handling
                subscriptions.push(
                    iced::subscription::unfold(
                        // Unique ID for this subscription
                        "usb-data-subscription",
                        // Initial state - the connection
                        (conn, 0u32), // Add a retry counter to track consecutive failures
                        move |(conn, retries)| async move {
                            // Add a small delay to avoid a tight polling loop when errors occur
                            // This is especially important in error conditions to prevent CPU hogging
                            if retries > 0 {
                                let delay_time = std::cmp::min(retries * 10, 500); // Cap at 500ms
                                tokio::time::sleep(tokio::time::Duration::from_millis(delay_time as u64)).await;
                            }
                            
                            // Use tokio's spawn_blocking to move the potentially blocking USB read operation
                            // to a separate thread to avoid blocking the event loop
                            let conn_clone = Arc::clone(&conn); // Clone the Arc for use in the closure
                            let data_result = tokio::task::spawn_blocking(move || {
                                // Wrap the entire operation in catch_unwind to prevent panics from propagating
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    // Try to acquire the lock for a limited time to avoid deadlocks
                                    match conn_clone.try_lock() {
                                        Ok(mut connection) => {
                                            // Double check that the connection is still active
                                            if !connection.is_connected() {
                                                return Err("Device not connected".to_string());
                                            }
                                            
                                            // Use a timeout for the read operation to prevent hangs
                                            let _ = connection.set_read_timeout(Some(std::time::Duration::from_millis(100)));
                                            
                                            // Attempt to read MitM traffic data instead of just descriptor data
                                            // This will capture traffic flowing through the Cynthion from attached devices
                                            match connection.read_mitm_traffic_clone() {
                                                Ok(data) => Ok(data),
                                                Err(e) => Err(e.to_string())
                                            }
                                        },
                                        Err(std::sync::TryLockError::WouldBlock) => {
                                            // Another thread is using the connection - skip this cycle
                                            Err("Connection busy - will retry".to_string())
                                        },
                                        Err(std::sync::TryLockError::Poisoned(e)) => {
                                            // The mutex is poisoned - indicate fatal error
                                            log::error!("Connection mutex poisoned: {}", e);
                                            Err("Fatal connection error: mutex poisoned".to_string())
                                        }
                                    }
                                }))
                            }).await;
                            
                            // Process the overall result
                            match data_result {
                                Ok(Ok(data_result)) => {
                                    // Process the inner data result
                                    match data_result {
                                        Ok(data) => {
                                            // Success - reset retry counter and return data
                                            (Message::USBDataReceived(data), (conn, 0))
                                        },
                                        Err(err_msg) => {
                                            // Filter out routine disconnection messages to avoid log spam
                                            if !err_msg.contains("not active") && 
                                               !err_msg.contains("disconnected") &&
                                               !err_msg.contains("not connected") &&
                                               !err_msg.contains("busy") {
                                                log::warn!("USB data read error: {}", err_msg);
                                            }
                                            
                                            // Increment retry counter for backoff
                                            let new_retries = retries + 1;
                                            
                                            // Too many consecutive failures might indicate a more serious problem
                                            if new_retries > 10 && !err_msg.contains("busy") {
                                                log::error!("Persistent USB read errors: {}", err_msg);
                                                // After many consecutive errors, signal a possible need to reconnect
                                                (Message::ConnectionPossiblyFailed, (conn, new_retries))
                                            } else {
                                                // Return empty data but keep trying
                                                (Message::USBDataReceived(Vec::new()), (conn, new_retries))
                                            }
                                        }
                                    }
                                },
                                Ok(Err(_panic)) => {
                                    // A panic occurred in the USB reading logic
                                    log::error!("Recovered from USB reader thread panic");
                                    
                                    // Increment retry counter exponentially for more serious failures
                                    let new_retries = retries + 5;
                                    
                                    // Return empty data but keep trying
                                    (Message::USBDataReceived(Vec::new()), (conn, new_retries))
                                },
                                Err(join_err) => {
                                    // The tokio task failed to join
                                    log::error!("USB reader task join error: {}", join_err);
                                    
                                    // Increment retry counter
                                    let new_retries = retries + 1;
                                    
                                    // Return empty data and continue
                                    (Message::USBDataReceived(Vec::new()), (conn, new_retries))
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
