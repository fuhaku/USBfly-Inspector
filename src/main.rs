mod app;
mod cynthion;
mod data;
mod gui;
mod usb;

use app::USBflyApp;
use iced::{Application, Settings};
use log::{info, warn, LevelFilter};
use rusb::UsbContext; // Import UsbContext trait for devices method
use std::env;

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
    
    // Try to create a USB context to check for USB access - this will safely fail
    // with a warning if USB is not available (for example, in the Replit environment)
    match rusb::Context::new() {
        Ok(ctx) => {
            // If we can create a context, try to enumerate devices
            match ctx.devices() {
                Ok(devices) => {
                    let device_count = devices.iter().count();
                    info!("USB subsystem initialized successfully. Found {} devices", device_count);
                },
                Err(e) => {
                    warn!("USB device enumeration error: {}. USB device detection may not work correctly.", e);
                    warn!("On Linux, try running with sudo or add udev rules for USB device access");
                    info!("Application will use simulation mode for USB devices");
                }
            }
        },
        Err(e) => {
            warn!("USB context initialization error: {}. Environment doesn't support USB access.", e);
            info!("Application will use simulation mode for USB devices");
        }
    }
    
    // Only set simulation mode if we couldn't access USB at all
    // This way, if real devices are available, we won't use simulation
    let simulation_required = match rusb::Context::new() {
        Err(_) => true, // No USB access at all - use simulation
        Ok(ctx) => {
            match ctx.devices() {
                Err(_) => true, // Couldn't enumerate devices - use simulation
                Ok(devices) => {
                    // Check if we have any real devices
                    let device_count = devices.iter().count();
                    if device_count == 0 {
                        info!("No USB devices found. Will provide simulated devices as fallback.");
                        true // No devices - use simulation
                    } else {
                        info!("Found {} physical USB devices. Using real device mode.", device_count);
                        false // We have real devices - don't use simulation
                    }
                }
            }
        }
    };
    
    if simulation_required {
        // Set environment variable to indicate simulation mode is needed
        env::set_var("USBFLY_SIMULATION_MODE", "1");
        info!("Simulation mode enabled - will use simulated devices");
    } else {
        // Make sure simulation mode is disabled
        env::remove_var("USBFLY_SIMULATION_MODE");
        info!("Using real USB devices - simulation mode disabled");
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
