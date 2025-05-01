// Import the new nusb-based connection types
use crate::cynthion::{CynthionDevice, CynthionHandle};
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
    // New connections use CynthionHandle from our nusb implementation
    cynthion_handle: Option<Arc<Mutex<CynthionHandle>>>,
    connection: Option<Arc<Mutex<CynthionHandle>>>, // Active connection instance
    available_devices: Vec<CynthionDevice>,
    usb_decoder: UsbDecoder,
    active_tab: Tab,
    device_view: DeviceView,
    traffic_view: TrafficView,
    descriptor_view: DescriptorView,
    connected: bool, // Flag to track connection state
    error_message: Option<String>,
    status_message: Option<String>, // For displaying status messages to users
    dark_mode: bool,
    current_speed: crate::usb::Speed, // Current USB speed setting used for synchronization
}

#[derive(Debug, Clone)]
pub enum Message {
    Connect,
    Disconnect,
    DisconnectCompleted,  // Added message for when device is successfully released
    DevicesFound(Vec<CynthionDevice>), // New message for device scan results
    ConnectionEstablished(Arc<Mutex<CynthionHandle>>),
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
    ProcessingCapture(Arc<Mutex<CynthionHandle>>), // Process capture data from real device
    CaptureStopped,         // Notification that capture has stopped successfully
    CaptureError(String),   // Error message from capture operation
    // Dynamic speed change functionality
    ChangeUsbSpeed(crate::usb::Speed), // Change USB speed while connected
    ReconnectWithSpeed(crate::usb::Speed), // Reconnect with a new speed setting
    UpdateStatusMessage(String), // Update status message for UI feedback
}

impl Application for USBflyApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        // Use the DeviceView with auto-refresh feature
        let (device_view, device_command) = DeviceView::new().with_initial_scan();
        
        // Default to High speed for most reliable device detection
        let default_speed = crate::usb::Speed::High;
        
        // Create a decoder with the default speed
        let mut decoder = UsbDecoder::new();
        decoder.set_speed(default_speed);
        
        let app = Self {
            cynthion_handle: None,
            connection: None,
            available_devices: Vec::new(),
            usb_decoder: decoder,
            active_tab: Tab::Devices,
            device_view,
            traffic_view: TrafficView::new(),
            descriptor_view: DescriptorView::new(),
            connected: false,
            error_message: None,
            status_message: None,
            dark_mode: true, // Default to dark mode for a hacker-friendly UI
            current_speed: default_speed, // Initialize with the default speed
        };
        
        // Map the device command to our application's message type
        let init_command = device_command.map(Message::DeviceViewMessage);

        // Also initiate a scan for Cynthion devices right away using our new connection method
        let scan_command = Command::perform(
            async move {
                // Find all Cynthion devices using our new implementation
                match CynthionDevice::find_all() {
                    Ok(devices) => {
                        info!("Found {} Cynthion-compatible devices", devices.len());
                        Message::DevicesFound(devices)
                    },
                    Err(e) => {
                        error!("Error scanning for devices: {}", e);
                        Message::DevicesFound(Vec::new())
                    }
                }
            },
            |msg| msg
        );

        // Combine the initial commands
        let combined_command = Command::batch(vec![
            init_command,
            scan_command
        ]);

        (app, combined_command)
    }

    fn title(&self) -> String {
        String::from("USBfly - USB Analysis for Cynthion")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::DevicesFound(devices) => {
                info!("Device scan found {} Cynthion-compatible devices", devices.len());
                self.available_devices = devices;
                Command::none()
            },
            Message::Connect => {
                // Check if we have any devices to connect to
                if self.available_devices.is_empty() {
                    // No devices available, trigger a scan first
                    info!("No devices available, scanning for Cynthion devices...");
                    Command::perform(
                        async move {
                            match CynthionDevice::find_all() {
                                Ok(devices) => {
                                    info!("Found {} Cynthion-compatible devices", devices.len());
                                    Message::DevicesFound(devices)
                                },
                                Err(e) => {
                                    error!("Error scanning for devices: {}", e);
                                    Message::ConnectionFailed(format!("Device scan failed: {}", e))
                                }
                            }
                        },
                        |msg| msg
                    )
                } else {
                    // We have devices, select the first one
                    let device = &self.available_devices[0];
                    info!("Connecting to Cynthion device: {}", device.get_description());
                    
                    // Attempt to connect using the new implementation with retry capability
                    // Get the selected speed from DeviceView before the async block
                    let selected_speed = self.device_view.get_selected_speed();
                    info!("Using connection speed: {:?}", selected_speed);
                    
                    Command::perform(
                        {
                            // Clone the device since we need to move it into the async block
                            let device = device.clone();
                            let speed = selected_speed; // Use the speed value we got before the async block
                            
                            async move {
                                // Short delay to allow USB subsystem to fully initialize the device
                                // This helps with the first-click connection issue
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                
                                // First attempt to open the device
                                match device.open() {
                                    Ok(mut handle) => {
                                        info!("Successfully opened Cynthion device");
                                        info!("Setting connection speed to: {:?}", speed);
                                        
                                        // Configure the device with the selected speed (Auto is no longer an option)
                                        match handle.set_speed(speed) {
                                            Ok(_) => info!("✓ Successfully set USB speed to: {:?}", speed),
                                            Err(e) => warn!("Failed to set USB speed: {} (continuing anyway)", e)
                                        }
                                        
                                        let handle = Arc::new(Mutex::new(handle));
                                        Message::ConnectionEstablished(handle)
                                    },
                                    Err(e) => {
                                        // If first attempt fails, wait a bit longer and try once more
                                        // This addresses the first-time connection issue on macOS
                                        error!("First connection attempt failed: {}, trying again...", e);
                                        
                                        // Longer delay before retry attempt
                                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                                        
                                        // Second attempt to open the device
                                        match device.open() {
                                            Ok(mut handle) => {
                                                info!("Second attempt successful - opened Cynthion device");
                                                info!("Setting connection speed to: {:?}", speed);
                                                
                                                // Configure the device with the selected speed (Auto is no longer an option)
                                                match handle.set_speed(speed) {
                                                    Ok(_) => info!("✓ Successfully set USB speed to: {:?}", speed),
                                                    Err(e) => warn!("Failed to set USB speed: {} (continuing anyway)", e)
                                                }
                                                
                                                let handle = Arc::new(Mutex::new(handle));
                                                Message::ConnectionEstablished(handle)
                                            },
                                            Err(e) => {
                                                error!("Failed to open device after retry: {}", e);
                                                Message::ConnectionFailed(e.to_string())
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        |msg| msg
                    )
                }
            }
            Message::Disconnect => {
                info!("Disconnecting from Cynthion device");
                debug!("Stopping any active capture and releasing device...");
                
                // First check if we have a valid connection
                if let Some(connection) = &self.connection {
                    let conn_clone = Arc::clone(connection);
                    
                    // First try to properly stop capture if it's running
                    if self.traffic_view.is_capture_active() {
                        info!("Stopping active capture before disconnecting");
                        
                        // Make a best effort to stop capture first
                        if let Ok(mut conn) = conn_clone.lock() {
                            // Try to stop but don't worry if it fails
                            let _ = conn.stop_capture();
                            debug!("Attempted to stop capture before disconnect");
                        }
                        
                        // Update UI state to reflect stopped capture
                        self.traffic_view.set_capture_active(false);
                    }
                    
                    // Return a command to perform the actual device release
                    return Command::perform(
                        async move {
                            info!("Releasing Cynthion device properly...");
                            
                            // Attempt to properly release the device
                            let release_result = if let Ok(mut conn) = conn_clone.lock() {
                                conn.release_device()
                            } else {
                                Err(anyhow::anyhow!("Failed to acquire lock for device release"))
                            };
                            
                            // Log the result
                            match &release_result {
                                Ok(_) => info!("Successfully released Cynthion device"),
                                Err(e) => warn!("Failed to properly release device: {}", e),
                            }
                            
                            // Return disconnect completed message
                            Message::DisconnectCompleted
                        },
                        |msg| msg
                    );
                } else {
                    // No active connection, just clear state
                    info!("No active connection to disconnect");
                    self.cynthion_handle = None;
                    self.connection = None;
                    self.connected = false;
                    Command::none()
                }
            }
            
            Message::DisconnectCompleted => {
                // Now that device is properly released, clear the handles
                info!("Disconnect complete, clearing connection handles");
                self.cynthion_handle = None;
                self.connection = None;
                self.connected = false;
                Command::none()
            }
            Message::ConnectionEstablished(handle) => {
                // Set both handles to the same value to ensure consistent usage
                info!("Connection established with Cynthion device");
                debug!("Synchronizing both connection handles");
                self.cynthion_handle = Some(handle.clone());
                self.connection = Some(handle);
                self.connected = true;
                self.error_message = None;
                
                // Get the current speed from device_view and synchronize it
                let selected_speed = self.device_view.get_selected_speed();
                self.current_speed = selected_speed;
                
                // Ensure the decoder also uses this speed
                self.usb_decoder.set_speed(selected_speed);
                
                info!("✓ Synchronized speed settings: {:?}", selected_speed);
                Command::none()
            }
            Message::ConnectionFailed(error) => {
                self.error_message = Some(error);
                Command::none()
            }
            Message::ConnectionPossiblyFailed => {
                // After too many consecutive errors, we attempt automatic recovery
                log::warn!("Detected possible connection failure from persistent USB read errors");
                
                // Check if either handle is valid - if so, synchronize them
                let is_connected = self.cynthion_handle.is_some() || self.connection.is_some();
                
                // Ensure handle synchronization if one is missing
                if self.connection.is_some() && self.cynthion_handle.is_none() {
                    debug!("Synchronizing handles during possible failure: copying from connection to cynthion_handle");
                    self.cynthion_handle = self.connection.clone();
                } else if self.cynthion_handle.is_some() && self.connection.is_none() {
                    debug!("Synchronizing handles during possible failure: copying from cynthion_handle to connection");
                    self.connection = self.cynthion_handle.clone();
                }
                
                if !is_connected {
                    // If we don't have a handle, show an error
                    log::error!("No device handle available, please reconnect");
                    self.connected = false;
                    self.error_message = Some("Connection lost. Please reconnect.".to_string());
                } else {
                    // We have a handle, but we've encountered errors
                    log::info!("Device handle still available despite USB read errors");
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
                // Special handling for speed change message
                match msg {
                    crate::gui::views::traffic_view::Message::ChangeSpeed(speed) => {
                        // Forward the speed change message to our main app
                        info!("Received speed change request: {:?}", speed);
                        
                        // Update traffic view state
                        let _ = self.traffic_view.update(msg); // Ignore the command since we're handling it differently
                        
                        // Return appropriate command to change speed
                        if self.connected {
                            Command::perform(
                                async move { speed },
                                Message::ChangeUsbSpeed
                            )
                        } else {
                            Command::none()
                        }
                    },
                    _ => {
                        // Forward other messages to traffic view and map the result back to our message type
                        self.traffic_view.update(msg)
                            .map(Message::TrafficViewMessage)
                    }
                }
            }
            Message::DescriptorViewMessage(msg) => {
                // Forward message to descriptor view and map the result back to our message type
                self.descriptor_view.update(msg)
                    .map(Message::DescriptorViewMessage)
            }
            Message::USBDataReceived(data) => {
                use log::{debug, info, warn};
                
                // Enhanced logging for USB data reception
                info!("Received USB data packet: {} bytes", data.len());
                if data.len() > 0 {
                    if data.len() >= 4 {
                        debug!("Data starts with: {:02X?}", &data[0..4]);
                        
                        // Log first 16 bytes for better diagnostic information
                        let display_len = std::cmp::min(16, data.len());
                        let hex_string = data[0..display_len].iter()
                            .map(|b| format!("{:02X}", b))
                            .collect::<Vec<String>>()
                            .join(" ");
                        info!("First {} bytes: {}", display_len, hex_string);
                    } else {
                        debug!("Short data packet: {:02X?}", &data);
                    }
                } else {
                    warn!("Received empty data packet (0 bytes)");
                }
                
                // Process received USB data
                match self.usb_decoder.decode(&data) {
                    Some(decoded) => {
                        // First add the raw packet for traditional view
                        debug!("🔍 Successfully decoded packet - adding to traffic view");
                        info!("Decoded USB packet of type: {}", decoded.data_type);
                        self.traffic_view.add_packet(data.clone(), decoded.clone());
                        
                        // Ensure handle synchronization for data processing too
                        if self.connection.is_some() && self.cynthion_handle.is_none() {
                            debug!("Synchronizing handles for data processing: copying from connection to cynthion_handle");
                            self.cynthion_handle = self.connection.clone();
                        } else if self.cynthion_handle.is_some() && self.connection.is_none() {
                            debug!("Synchronizing handles for data processing: copying from cynthion_handle to connection");
                            self.connection = self.cynthion_handle.clone();
                        }
                        
                        // Process the data with our enhanced decoder using the connection handle
                        if let Some(handle) = &self.connection {
                            if let Ok(mut cynthion_handle) = handle.lock() {
                                // Check if this data contains evidence of USB devices connected to Cynthion
                                // This helps optimize packet processing for connected devices
                                use crate::cynthion::device_detector::UsbDeviceConnectionDetector;
                                
                                // Process the data for device connection detection
                                UsbDeviceConnectionDetector::check_for_usb_device_connection(&data);
                                
                                // Enhanced processing with device connection awareness
                                debug!("Processing USB traffic data through enhanced decoder...");
                                
                                // Get the transactions from the implementation with improved connected device support
                                let transactions = cynthion_handle.process_transactions(&data);
                                
                                if !transactions.is_empty() {
                                    info!("Successfully decoded {} USB transactions", transactions.len());
                                    debug!("Transaction types: {:?}", transactions.iter()
                                            .map(|t| t.transfer_type)
                                            .collect::<Vec<_>>());
                                    
                                    // Add each transaction to the traffic view with improved metadata
                                    for transaction in transactions {
                                        debug!("Adding transaction ID {} to traffic view", transaction.id);
                                        
                                        // Check if this is a transaction from a connected device
                                        let is_device_connected = UsbDeviceConnectionDetector::is_device_connected();
                                        if is_device_connected {
                                            debug!("Transaction from connected USB device detected");
                                        }
                                        
                                        self.traffic_view.add_transaction(transaction);
                                    }
                                } else {
                                    debug!("No transactions decoded from packet");
                                }
                            } else {
                                debug!("Could not acquire handle lock for MitM processing");
                            }
                        } else {
                            debug!("No device handle available for MitM processing");
                        }
                        
                        // Continue with descriptor view update
                        self.descriptor_view.update_descriptors(decoded);
                    },
                    None => {
                        debug!("Failed to decode USB data");
                    }
                }
                Command::none()
            },
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
                    async move {
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
                debug!("Starting capture: cynthion_handle={}, connection={}", 
                       self.cynthion_handle.is_some(), self.connection.is_some());
                       
                // Make sure both handles are in sync - there should be a single source of truth
                // Copy from connection to cynthion_handle if needed
                if self.connection.is_some() && self.cynthion_handle.is_none() {
                    debug!("Synchronizing handles: copying from connection to cynthion_handle");
                    self.cynthion_handle = self.connection.clone();
                }
                // Copy from cynthion_handle to connection if needed
                else if self.cynthion_handle.is_some() && self.connection.is_none() {
                    debug!("Synchronizing handles: copying from cynthion_handle to connection");
                    self.connection = self.cynthion_handle.clone();
                }
                
                if let Some(handle) = &self.connection {
                    let handle_clone = Arc::clone(handle);
                    
                    // Get the user-selected speed from device view
                    let selected_speed = self.device_view.get_selected_speed();
                    info!("Starting USB traffic capture with speed: {:?}", selected_speed);
                    
                    // Update current speed in app state to ensure consistency
                    self.current_speed = selected_speed;
                    
                    // Set the decoder's speed to match the selected speed
                    self.usb_decoder.set_speed(selected_speed);
                    info!("✓ Synchronized USB decoder to use speed: {:?}", selected_speed);
                    
                    // Additional debugging for capture start
                    debug!("Preparing to start capture with Cynthion handle...");
                    
                    // Update UI state to show capture is active
                    self.traffic_view.set_capture_active(true);
                    
                    // First get the info we need under a short-lived lock
                    let (is_simulation, start_result) = {
                        match handle_clone.lock() {
                            Ok(mut cynthion_handle) => {
                                let is_sim = cynthion_handle.is_simulation_mode();
                                
                                // Try to start capture while we have the lock
                                let result = if !is_sim {
                                    // Only try to actually start capture for real devices with selected speed
                                    cynthion_handle.start_capture_with_speed(selected_speed)
                                } else {
                                    // For simulation mode, just pretend it succeeded
                                    Ok(())
                                };
                                
                                // Return values that we need after dropping lock
                                (is_sim, result)
                            },
                            Err(_) => {
                                // Lock failed
                                error!("Failed to lock device handle");
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
                                        info!("USB traffic capture started successfully with nusb implementation");
                                        info!("Capture started with speed: {:?}", selected_speed);
                                        Message::CaptureStarted
                                    },
                                    Err(e) => {
                                        error!("Failed to start capture: {}", e);
                                        error!("Capture start failure with speed: {:?}", selected_speed);
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
                debug!("Stopping capture: cynthion_handle={}, connection={}", 
                       self.cynthion_handle.is_some(), self.connection.is_some());
                       
                // Make sure both handles are in sync before stopping
                // Copy from connection to cynthion_handle if needed
                if self.connection.is_some() && self.cynthion_handle.is_none() {
                    debug!("Synchronizing handles for stop: copying from connection to cynthion_handle");
                    self.cynthion_handle = self.connection.clone();
                }
                // Copy from cynthion_handle to connection if needed
                else if self.cynthion_handle.is_some() && self.connection.is_none() {
                    debug!("Synchronizing handles for stop: copying from cynthion_handle to connection");
                    self.connection = self.cynthion_handle.clone();
                }
                
                if let Some(connection) = &self.connection {
                    let conn_clone: Arc<Mutex<CynthionHandle>> = Arc::clone(connection);
                    
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
                debug!("Clearing buffer: cynthion_handle={}, connection={}", 
                       self.cynthion_handle.is_some(), self.connection.is_some());
                       
                // Make sure both handles are in sync before clearing buffer
                // Copy from connection to cynthion_handle if needed
                if self.connection.is_some() && self.cynthion_handle.is_none() {
                    debug!("Synchronizing handles for clear: copying from connection to cynthion_handle");
                    self.cynthion_handle = self.connection.clone();
                }
                // Copy from cynthion_handle to connection if needed
                else if self.cynthion_handle.is_some() && self.connection.is_none() {
                    debug!("Synchronizing handles for clear: copying from cynthion_handle to connection");
                    self.connection = self.cynthion_handle.clone();
                }

                if let Some(connection) = &self.connection {
                    let conn_clone: Arc<Mutex<CynthionHandle>> = Arc::clone(connection);
                    
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
            },
            
            Message::UpdateStatusMessage(message) => {
                debug!("Updating status message: {}", message);
                self.status_message = Some(message);
                Command::none()
            },
            
            // Handle USB speed change requests
            Message::ChangeUsbSpeed(speed) => {
                info!("Changing USB speed to: {:?}", speed);
                
                // Show a notification to the user that speed is being changed
                self.status_message = Some(format!("Changing device speed to {:?}...", speed));
                
                // Update the current speed tracking in app state
                self.current_speed = speed;
                
                // Also update the decoder speed setting immediately - critical for proper packet interpretation
                self.usb_decoder.set_speed(speed);
                info!("✓ Updated decoder speed to: {:?}", speed);
                
                // We need to stop capture, change speed, and restart capture
                if let Some(connection) = &self.connection {
                    let conn_clone = Arc::clone(connection);
                    
                    // Store the currently active state to restore it after reconnect
                    let was_capture_active = self.traffic_view.is_capture_active();
                    
                    // First stop any active capture with multiple attempts
                    if was_capture_active {
                        info!("Stopping active capture before changing speed");
                        
                        // Update UI state first to provide immediate feedback
                        self.traffic_view.set_capture_active(false);
                        
                        // Try to stop the capture with multiple attempts
                        let max_attempts = 3;
                        let mut success = false;
                        
                        if let Ok(mut conn) = conn_clone.lock() {
                            for attempt in 1..=max_attempts {
                                info!("Attempt {}/{} to stop capture for speed change", attempt, max_attempts);
                                
                                match conn.stop_capture() {
                                    Ok(_) => {
                                        info!("Successfully stopped capture for speed change");
                                        success = true;
                                        break;
                                    },
                                    Err(e) => {
                                        warn!("Attempt {}/{} to stop capture failed: {}", attempt, max_attempts, e);
                                        if attempt < max_attempts {
                                            std::thread::sleep(std::time::Duration::from_millis(300 * attempt as u64));
                                        }
                                    }
                                }
                            }
                        }
                        
                        if !success {
                            warn!("Failed to stop capture after {} attempts. Continuing with speed change anyway.", max_attempts);
                        }
                    }
                    
                    // Now initiate a reconnect with the new speed
                    Command::perform(
                        async move { speed },
                        Message::ReconnectWithSpeed
                    )
                } else {
                    warn!("Cannot change USB speed: No active connection");
                    self.status_message = Some("Cannot change device speed: No active connection".to_string());
                    Command::none()
                }
            },
            
            // Perform the reconnect with new speed
            Message::ReconnectWithSpeed(speed) => {
                info!("Reconnecting with new USB speed: {:?}", speed);
                
                // Update the status message to show progress
                self.status_message = Some(format!("Reconnecting to apply {:?} device speed setting...", speed));
                
                // We need to:  
                // 1. Disconnect properly
                // 2. Update the speed in the decoder
                // 3. Connect again
                // 4. Restart capture if it was active
                
                // Update the decoder speed setting first
                self.usb_decoder.set_speed(speed);
                info!("Updated USB decoder speed to: {:?}", speed);
                
                // Update device view selected speed
                self.device_view.set_selected_speed(speed);
                
                // Store the original device selection and capture state to restore later
                let was_capture_active = self.traffic_view.is_capture_active();
                
                // Start a disconnection sequence followed by reconnection
                info!("Starting disconnect/reconnect sequence for speed change");
                
                // Create a command that will disconnect first
                let disconnect_command = Command::perform(
                    async move { Message::Disconnect },
                    |msg| msg
                );
                
                // Then schedule a reconnect after the disconnect completes with a longer delay
                // for better reliability with USB device detection
                let reconnect_command = Command::perform(
                    async move {
                        // Add a delay to ensure the disconnect completes first
                        // Increased from 500ms to 1000ms for better reliability
                        info!("Waiting for device to disconnect completely before reconnecting");
                        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                        Message::Connect
                    },
                    |msg| msg
                );
                
                // Create a status update command to show when reconnection is in progress
                let status_update_command = Command::perform(
                    async move {
                        // This will run after the disconnect but before the reconnect
                        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                        Message::UpdateStatusMessage(format!("Reconnecting with {:?} device speed...", speed))
                    },
                    |msg| msg
                );
                
                // And finally, if capture was active, restart it after reconnection with longer delay
                let restart_capture_command = if was_capture_active {
                    Command::perform(
                        async move {
                            // Add a longer delay to ensure device is connected before starting capture
                            // Increased from 800ms to 1500ms for better reliability
                            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                            Message::StartCapture
                        },
                        |msg| msg
                    )
                } else {
                    Command::none()
                };
                
                // Create a final status update command to show when process is complete
                let final_status_command = Command::perform(
                    async move {
                        // This will run after everything else completes
                        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
                        Message::UpdateStatusMessage(format!("Device speed changed to {:?}", speed))
                    },
                    |msg| msg
                );
                
                // Batch all commands together
                Command::batch(vec![
                    disconnect_command,
                    status_update_command,
                    reconnect_command,
                    restart_capture_command,
                    final_status_command
                ])
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
                debug!("Fetching data: cynthion_handle={}, connection={}", 
                       self.cynthion_handle.is_some(), self.connection.is_some());
                       
                // Make sure both handles are in sync before fetching data
                // Copy from connection to cynthion_handle if needed
                if self.connection.is_some() && self.cynthion_handle.is_none() {
                    debug!("Synchronizing handles for fetch: copying from connection to cynthion_handle");
                    self.cynthion_handle = self.connection.clone();
                }
                // Copy from cynthion_handle to connection if needed
                else if self.cynthion_handle.is_some() && self.connection.is_none() {
                    debug!("Synchronizing handles for fetch: copying from cynthion_handle to connection");
                    self.connection = self.cynthion_handle.clone();
                }
                
                if let Some(connection) = &self.connection {
                    let conn_clone: Arc<Mutex<CynthionHandle>> = Arc::clone(connection);
                    
                    // Log the selected speed used for this capture
                    let selected_speed = self.device_view.get_selected_speed();
                    info!("Fetching capture data (connection speed: {:?})", selected_speed);
                    
                    // Use a simulated capture data approach for safety
                    // This avoids the Send/Sync issues with MutexGuard across await points
                    // First get the info we need under a short-lived lock
                    let (is_simulation, maybe_data, connection_ref) = {
                        match conn_clone.lock() {
                            Ok(conn) => {
                                let is_sim = conn.is_simulation_mode();
                                
                                // In hardware-only mode, we don't use simulation
                                // But we need to handle the simulation flag gracefully
                                let data = if is_sim {
                                    // We should never reach here in hardware-only mode,
                                    // but provide a clean fallback just in case
                                    warn!("Simulation mode requested but hardware-only mode is enforced");
                                    
                                    // Return empty data - we're in hardware-only mode
                                    // This ensures we don't break the app if simulation mode
                                    // is somehow enabled
                                    Some(Vec::new())
                                    
                                    // Note: Removed simulation method calls to enforce hardware-only mode
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
                                    if let Ok(mut conn) = connection_ref.lock() {
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
                                // In hardware-only mode, we don't use simulation
                                // But we need to handle simulation mode gracefully
                                warn!("Hardware-only mode is enforced - no simulation data available");
                                
                                // Return empty data for simulation mode in hardware-only mode
                                // This ensures the application continues to function even if
                                // simulation mode is somehow enabled
                                let sim_data = Vec::new();
                                
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
                let conn: Arc<Mutex<CynthionHandle>> = Arc::clone(connection);
                
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
