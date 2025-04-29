mod app;
mod cynthion;
mod data;
mod gui;
mod usb;

use app::USBflyApp;
use iced::{Application, Settings};
use log::{info, warn, debug, LevelFilter};
use std::{env, io};

fn main() -> iced::Result {
    // Set default logging environment variable if not already set
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info,usbfly=debug,rusb=warn");
    }
    
    // Initialize logger with more useful configuration
    pretty_env_logger::formatted_builder()
        .filter_level(LevelFilter::Info)
        .filter_module("usbfly", LevelFilter::Debug)
        .filter_module("rusb", LevelFilter::Warn)
        .init();
    
    info!("Starting USBfly application v{}", env!("CARGO_PKG_VERSION"));
    info!("Platform: {}", std::env::consts::OS);
    
    // Check for USB access - this is important especially on Linux
    match rusb::devices() {
        Ok(devices) => {
            let device_count = devices.iter().count();
            info!("USB subsystem initialized successfully. Found {} devices", device_count);
        },
        Err(e) => {
            warn!("USB access error: {}. USB device detection may not work correctly.", e);
            warn!("On Linux, try running with sudo or add udev rules for USB device access");
        }
    }
    
    // Log information about renderer
    info!("Using default software renderer for cross-platform compatibility");
    
    // Run the application
    USBflyApp::run(Settings {
        id: Some(String::from("com.usbfly.app")),
        window: iced::window::Settings {
            size: (1024, 768),
            min_size: Some((800, 600)),
            max_size: None,
            resizable: true,
            decorations: true,
            transparent: false,
            position: iced::window::Position::Centered,
            // always_on_top is not available in this version of iced
            ..Default::default()
        },
        default_font: iced::Font::DEFAULT, // Use the default font instead of None
        default_text_size: 16.0,
        antialiasing: true,
        exit_on_close_request: true,
        ..Default::default()
    })
}
