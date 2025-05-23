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
use std::io::Write;
use std::net::TcpListener;
use std::thread;

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
    
    // Check if USBFLY_FORCE_HARDWARE is explicitly set to 1 by the user
    let force_hardware = env::var("USBFLY_FORCE_HARDWARE")
        .map(|val| val == "1")
        .unwrap_or(false);
        
    // On macOS, set the force hardware flag to prioritize real device connections
    // but only if the user hasn't explicitly disabled it
    if cfg!(target_os = "macos") {
        if force_hardware {
            info!("🔥 HARDWARE MODE EXPLICITLY FORCED on macOS - ALL SAFETY CHECKS DISABLED 🔥");
            info!("⚠️ This may cause instability with incompatible devices ⚠️");
            env::set_var("USBFLY_FORCE_HARDWARE", "1");
            env::set_var("USBFLY_SIMULATION_MODE", "0");
        } else if env::var("USBFLY_FORCE_HARDWARE").is_err() {
            // Only set the default if the env var doesn't exist already
            info!("MacOS detected - initializing with hardware mode preference");
            env::set_var("USBFLY_FORCE_HARDWARE", "1");
            env::set_var("USBFLY_SIMULATION_MODE", "0");
        }
    }
    
    // Actively check for USB devices at startup to force hardware mode if possible
    match rusb::Context::new() {
        Ok(context) => {
            match context.devices() {
                Ok(devices) => {
                    let mut found_device = false;
                    
                    // Print all USB devices for debugging
                    for device in devices.iter() {
                        if let Ok(desc) = device.device_descriptor() {
                            let vid = desc.vendor_id();
                            let pid = desc.product_id();
                            info!("Found USB device: VID={:04x} PID={:04x}", vid, pid);
                            
                            // Check if this is a Cynthion device
                            if (vid == 0x1d50 && (pid == 0x615c || pid == 0x60e6 || pid == 0x615b)) ||
                               (vid == 0x16d0 && pid == 0x0f3b) {
                                info!("🔥 Cynthion device detected at startup! 🔥");
                                info!("◉ Setting environment for hardware access mode");
                                env::set_var("USBFLY_FORCE_HARDWARE", "1");
                                env::set_var("USBFLY_SIMULATION_MODE", "0");
                                found_device = true;
                            }
                            
                            // Special handling for macOS - any USB device could potentially be a Cynthion device
                            // due to ID confusion or driver issues on macOS
                            if cfg!(target_os = "macos") && !found_device {
                                info!("◉ macOS device detection: VID:{:04x} PID:{:04x}", vid, pid);
                                
                                // Only vendors we know are compatible with our device (GoodFet, GreatFET, etc.)
                                let known_vendors = [0x1d50, 0x16d0, 0x1fc9, 0x0403];
                                if known_vendors.contains(&vid) {
                                    info!("Compatible vendor detected! Setting hardware mode");
                                    env::set_var("USBFLY_FORCE_HARDWARE", "1");
                                    env::set_var("USBFLY_SIMULATION_MODE", "0");
                                    found_device = true;
                                }
                            }
                        }
                    }
                    
                    if !found_device {
                        info!("No Cynthion device found at startup");
                    }
                },
                Err(e) => {
                    warn!("USB context works but can't list devices: {}. Will try hardware mode anyway.", e);
                }
            }
        },
        Err(e) => {
            warn!("USB context initialization error: {}. Environment doesn't support USB access.", e);
            info!("Hardware mode enforced despite environment limitations");
            env::set_var("USBFLY_FORCE_HARDWARE", "1");
            env::remove_var("USBFLY_SIMULATION_MODE");
        }
    }
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let mut port: Option<u16> = None;
    
    // Simple argument parser
    for i in 1..args.len() {
        if args[i] == "--port" && i + 1 < args.len() {
            if let Ok(port_num) = args[i + 1].parse::<u16>() {
                port = Some(port_num);
                info!("HTTP server port specified: {}", port_num);
            }
        }
    }
    
    // Start HTTP server for Replit if port is specified
    if let Some(port_num) = port {
        // Try to bind to the port first to make sure it's available
        match TcpListener::bind(format!("0.0.0.0:{}", port_num)) {
            Ok(listener) => {
                info!("Successfully bound to port {}", port_num);
                
                // Make the listener non-blocking
                listener.set_nonblocking(true).expect("Cannot set non-blocking");
                
                // Start server thread
                thread::spawn(move || {
                    info!("HTTP server thread started on port {}", port_num);
                    
                    // Signal that our server is ready (for Replit workflow detection)
                    println!("HTTP server ready on port {}", port_num);
                    
                    for stream in listener.incoming() {
                        match stream {
                            Ok(mut stream) => {
                                // Send a simple response indicating the app is running
                                let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                                    <html><body>\
                                    <h1>USBfly Application Running</h1>\
                                    <p>The USBfly application is running in the background.</p>\
                                    <p>This is a native application with a graphical interface.</p>\
                                    </body></html>";
                                
                                if let Err(e) = stream.write_all(response.as_bytes()) {
                                    warn!("Failed to send HTTP response: {}", e);
                                }
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                // No connection available yet, just continue
                                thread::sleep(std::time::Duration::from_millis(50));
                                continue;
                            }
                            Err(e) => warn!("Connection failed: {}", e),
                        }
                    }
                });
            }
            Err(e) => {
                warn!("Could not bind to port {}: {}", port_num, e);
                // Use a Box<dyn Error> as required by iced::Error::WindowCreationFailed
                let error_message = format!("Could not start HTTP server on port {}: {}", port_num, e);
                return Err(iced::Error::WindowCreationFailed(Box::new(
                    std::io::Error::new(std::io::ErrorKind::Other, error_message)
                )));
            }
        }
    }
    
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
                    info!("Hardware mode enforced despite USB device access limitations");
                    env::set_var("USBFLY_FORCE_HARDWARE", "1");
                    env::remove_var("USBFLY_SIMULATION_MODE");
                }
            }
        },
        Err(e) => {
            warn!("USB context initialization error: {}. Environment doesn't support USB access.", e);
            info!("Hardware mode enforced despite environment limitations");
            env::set_var("USBFLY_FORCE_HARDWARE", "1");
            env::remove_var("USBFLY_SIMULATION_MODE");
        }
    }
    
    // Always force hardware mode and disable simulation
    env::set_var("USBFLY_FORCE_HARDWARE", "1");
    env::remove_var("USBFLY_SIMULATION_MODE");
    info!("Hardware mode enforced - simulation mode disabled");
    
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
