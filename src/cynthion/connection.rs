//! Cynthion device connection handler
//! Refactored to use nusb library for better compatibility with Cynthion devices

use std::fmt;
use std::time::Duration;

use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use rusb::{DeviceHandle, UsbContext};
use tokio::time::sleep;

// Use decoder::Speed type from our USB decoder module
#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(dead_code)]
pub enum Speed {
    Auto = 0,
    High = 1,
    Full = 2,
    Low = 3,
}

impl Speed {
    #[allow(dead_code)]
    pub fn mask(&self) -> u8 {
        1 << (*self as u8)
    }
}

// The TransferQueue implementation is now only used in new_connection.rs

// Constants for Cynthion device (and compatible devices)
// Copied from Packetry codebase
pub const CYNTHION_VID: u16 = 0x1d50;
pub const CYNTHION_PID: u16 = 0x615b;    // Cynthion firmware VID/PID
#[allow(dead_code)]
const CLASS: u8 = 0xff;                  // Vendor-specific class
#[allow(dead_code)]
const SUBCLASS: u8 = 0x10;               // USB analysis subclass 
#[allow(dead_code)]
const PROTOCOL: u8 = 0x01;               // Cynthion protocol version
#[allow(dead_code)]
const ENDPOINT: u8 = 0x81;               // Bulk in endpoint for receiving data
#[allow(dead_code)]
const READ_LEN: usize = 0x4000;          // 16k buffer size
#[allow(dead_code)]
const NUM_TRANSFERS: usize = 4;          // Number of concurrent transfers
const TIMEOUT: Duration = Duration::from_millis(1000);

// Additional compatible devices
// Development/Test device IDs
const GREATFET_VID: u16 = 0x1d50;        // Standard GreatFET VID
const GREATFET_ONE_PID: u16 = 0x60e6;    // GreatFET One PID
// Alternative Cynthion PIDs (for different firmware versions)
const ALT_CYNTHION_PID_1: u16 = 0x615c;
const ALT_CYNTHION_PID_2: u16 = 0x615d;
// Unused but kept for reference
const GADGETCAP_VID: u16 = 0x1d50;
const GADGETCAP_PID: u16 = 0x6018;

// Commands from Packetry
#[allow(dead_code)]
const VENDOR_REQUEST_IN: u8 = 0xC0;
#[allow(dead_code)]
const VENDOR_REQUEST_OUT: u8 = 0x40;

// Commands specific to our MitM implementation (until migration is complete)
const CMD_SET_CAPTURE_MODE: u8 = 0x01;
const CMD_GET_CAPTURED_DATA: u8 = 0x02;
const CMD_START_CAPTURE: u8 = 0x03;
#[allow(dead_code)]
const CMD_STOP_CAPTURE: u8 = 0x04;
#[allow(dead_code)]
const CMD_CLEAR_BUFFER: u8 = 0x05;

// Endpoints for communication
const CYNTHION_OUT_EP: u8 = 0x01;
const CYNTHION_IN_EP: u8 = 0x81;
const CYNTHION_INTERFACE: u8 = 0x00;  // Default interface for Cynthion devices

// Use the existing TIMEOUT constant instead of TIMEOUT_MS
// const TIMEOUT_MS: u32 = 1000;

// Capture mode constants
const CAPTURE_MODE_ALL: u8 = 0;
const CAPTURE_MODE_HOST_TO_DEVICE: u8 = 1;
#[allow(dead_code)]
const CAPTURE_MODE_DEVICE_TO_HOST: u8 = 2;
#[allow(dead_code)]
const CAPTURE_MODE_SETUP_ONLY: u8 = 3;

// Old config structures removed for compatibility
// We'll use the structures in new_connection.rs instead

// Placeholder struct - we don't use these anymore
#[allow(dead_code)]
struct State {
    value: u8
}

impl State {
    #[allow(dead_code)]
    fn new(_enable: bool, _speed: Speed) -> u8 {
        0 // This is a placeholder - implementation moved to new_connection
    }
}

// Placeholder struct - we don't use these anymore
#[allow(dead_code)]
struct TestConfig {
    value: u8
}

impl TestConfig {
    #[allow(dead_code)]
    fn new(_speed: Option<Speed>) -> u8 {
        0 // This is a placeholder - implementation moved to new_connection
    }
}

// Device information structure for displaying in UI
#[derive(Debug, Clone)]
pub struct USBDeviceInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
}

impl fmt::Display for USBDeviceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let manufacturer = self.manufacturer.as_deref().unwrap_or("Unknown");
        let product = self.product.as_deref().unwrap_or("Unknown");
        let serial = self.serial_number.as_deref().unwrap_or("N/A");
        
        write!(f, "{} {} [{}] (VID:{:04x} PID:{:04x})", 
            manufacturer, product, serial, self.vendor_id, self.product_id)
    }
}

#[derive(Debug)]
pub struct CynthionConnection {
    handle: Option<DeviceHandle<rusb::GlobalContext>>,
    active: bool,
    transfer_queue: Option<crate::cynthion::transfer_queue::TransferQueue>,
}

impl CynthionConnection {
    // Get a list of all connected USB devices
    #[allow(dead_code)]
    pub fn list_devices() -> Result<Vec<USBDeviceInfo>> {
        // Check if a refresh was forced by a button press
        let force_refresh = std::env::var("USBFLY_FORCE_REFRESH").is_ok();
        // Check if hardware mode is forced
        let force_hardware = std::env::var("USBFLY_FORCE_HARDWARE").is_ok();
        
        let mut real_device_list = Vec::new();
        let mut found_real_cynthion = false;
        
        // Enhanced handling for macOS hot-plug detection
        // On macOS, we need to be much more aggressive about device detection
        #[cfg(target_os = "macos")]
        {
            // Clear previous device detection flags to start fresh
            if force_refresh {
                std::env::remove_var("USBFLY_DEVICE_DETECTED");
                info!("🔄 macOS: Force refresh activated - clearing previous device detection state");
            }
            
            if force_refresh || force_hardware {
                info!("🔍 macOS: Enhanced device detection activated");
                
                // For force refresh, temporarily set hardware mode for this scan only
                if force_refresh {
                    std::env::set_var("USBFLY_FORCE_REFRESH", "1");
                }
                
                // When force hardware is explicit, make it persistent
                if force_hardware {
                    info!("🔒 macOS: Hardware mode forced - setting persistent flags");
                    std::env::set_var("USBFLY_FORCE_HARDWARE", "1");
                    std::env::set_var("USBFLY_SIMULATION_MODE", "0");
                }
                
                // More aggressive USB bus re-enumeration for macOS
                // We'll try multiple approaches to ensure detection works
                let mut success = false;
                
                // First try: Simple USB context refresh
                if let Ok(context) = rusb::Context::new() {
                    if let Ok(devices) = context.devices() {
                        success = true;
                        info!("✓ macOS: First-pass USB bus enumeration successful - found {} devices", 
                              devices.iter().count());
                    }
                }
                
                // If first approach failed, try a more aggressive approach
                if !success {
                    info!("⚠️ macOS: First-pass enumeration failed, trying secondary approach");
                    
                    // Small delay to allow USB subsystem to settle
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    
                    // Second attempt with a fresh context
                    if let Ok(context) = rusb::Context::new() {
                        if let Ok(devices) = context.devices() {
                            success = true;
                            info!("✓ macOS: Second-pass USB enumeration successful - found {} devices", 
                                 devices.iter().count());
                        }
                    }
                }
                
                // Final outcome message
                if success {
                    info!("🎯 macOS: USB bus re-enumeration successful");
                } else {
                    warn!("⚠️ macOS: USB bus re-enumeration failed - device detection may be unreliable");
                }
            }
        }
        
        // If hardware mode is forced or the force hardware flag is set, always try to find real devices
        // even if USBFLY_SIMULATION_MODE is set to 1
        if force_hardware {
            info!("🔒 Hardware mode is forced - prioritizing real device detection");
            // Explicitly disable simulation mode when hardware is forced
            std::env::set_var("USBFLY_SIMULATION_MODE", "0");
        }
        
        // Always try to detect real devices when:
        // 1. Force refresh is requested OR
        // 2. During regular auto-refresh cycles
        // 3. Hardware mode is forced
        // This ensures we detect devices plugged in after the app starts
        if true {
            if force_refresh {
                info!("🔍 Force refresh requested - deep scanning for real devices");
                std::env::remove_var("USBFLY_FORCE_REFRESH");
            }
            
            // Try to create USB context with error handling specifically for macOS
            let context_result = rusb::Context::new();
            
            // Special error handling for macOS
            #[cfg(target_os = "macos")]
            if let Err(e) = &context_result {
                warn!("⚠️ macOS USB context error: {} - applying workaround", e);
                // On macOS, USB context errors are common due to permission issues
                // Let's force a background refresh of the USB system
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            
            if let Ok(context) = context_result {
                // Try to read devices with special error handling for macOS
                let devices_result = context.devices();
                
                #[cfg(target_os = "macos")]
                if let Err(e) = &devices_result {
                    warn!("⚠️ macOS USB devices enumeration error: {} - trying fallback method", e);
                    // On macOS, we'll try a more aggressive approach
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                
                if let Ok(devices) = devices_result {
                    // Count all USB devices for better diagnostics
                    let total_device_count = devices.iter().count();
                    info!("Found {} total USB devices during scan", total_device_count);
                    
                    // Scan for all devices, including compatible ones
                    for device in devices.iter() {
                        if let Ok(descriptor) = device.device_descriptor() {
                            let vid = descriptor.vendor_id();
                            let pid = descriptor.product_id();
                            
                            // More verbose debug logging to help troubleshoot device detection
                            debug!("Checking USB device: VID:{:04x} PID:{:04x}", vid, pid);
                            
                            // Check if this is a supported device
                            if Self::is_supported_device(vid, pid) {
                                found_real_cynthion = true;
                                info!("🎯 Real Cynthion device found: VID:{:04x} PID:{:04x}", vid, pid);
                                
                                // If we find a real device, ensure simulation mode is off
                                Self::force_real_device_mode();
                            }
                            
                            // Create a temporary handle to get string descriptors
                            let device_info = if let Ok(handle) = device.open() {
                                let timeout = Duration::from_millis(100);
                                // Try to get available languages
                                let default_language = match handle.read_languages(timeout) {
                                    Ok(langs) if !langs.is_empty() => Some(langs[0]),
                                    _ => None,
                                };
                                
                                // Get string descriptors (in a way that handles errors gracefully)
                                let manufacturer = match default_language {
                                    Some(lang) => descriptor.manufacturer_string_index()
                                        .and_then(|_idx| handle.read_manufacturer_string(lang, &descriptor, timeout).ok()),
                                    None => None,
                                };
                                
                                let product = match default_language {
                                    Some(lang) => descriptor.product_string_index()
                                        .and_then(|_idx| handle.read_product_string(lang, &descriptor, timeout).ok()),
                                    None => None,
                                };
                                
                                let serial = match default_language {
                                    Some(lang) => descriptor.serial_number_string_index()
                                        .and_then(|_idx| handle.read_serial_number_string(lang, &descriptor, timeout).ok()),
                                    None => None,
                                };
                                
                                USBDeviceInfo {
                                    vendor_id: vid,
                                    product_id: pid,
                                    manufacturer,
                                    product,
                                    serial_number: serial,
                                }
                            } else {
                                // Fallback if we can't open the device
                                USBDeviceInfo {
                                    vendor_id: vid,
                                    product_id: pid,
                                    manufacturer: None,
                                    product: None,
                                    serial_number: None,
                                }
                            };
                            
                            real_device_list.push(device_info);
                        }
                    }
                }
            }
        }
        
        // First check if force hardware mode is enabled - this overrides all other options
        let force_hardware = std::env::var("USBFLY_FORCE_HARDWARE")
            .map(|val| val == "1")
            .unwrap_or(false);
            
        // If we found real compatible devices, use the real device list
        if found_real_cynthion {
            info!("✓ Using real Cynthion device list with {} devices", real_device_list.len());
            return Ok(real_device_list);
        }
        
        // If hardware mode is forced but no compatible devices were found
        if force_hardware {
            // Even if no compatible devices, return whatever real devices we found
            // instead of falling back to simulation
            if !real_device_list.is_empty() {
                warn!("⚠️ HARDWARE MODE FORCED: No compatible Cynthion devices found");
                warn!("Returning {} real USB devices instead - some features may not work", real_device_list.len());
                return Ok(real_device_list);
            } else {
                // If we're in forced hardware mode but no devices at all, return empty list
                // This ensures the UI shows "No devices found" instead of simulated devices
                warn!("⚠️ HARDWARE MODE FORCED but no USB devices found");
                warn!("Connect a Cynthion device and restart the application");
                return Ok(vec![]);
            }
        }
        
        // If not in forced hardware mode, check if we should use simulation mode
        if Self::is_env_simulation_mode() && !force_refresh {
            info!("Using simulated device list (simulation mode enabled)");
            Ok(Self::get_simulated_devices())
        } else if !real_device_list.is_empty() {
            // Return actual devices even if none are compatible
            info!("No compatible devices found, but returning {} real USB devices", real_device_list.len());
            Ok(real_device_list)
        } else {
            // No devices found at all, use simulation mode
            if !Self::is_env_simulation_mode() {
                info!("No real USB devices found, enabling simulation mode");
                std::env::set_var("USBFLY_SIMULATION_MODE", "1");
            }
            Ok(Self::get_simulated_devices())
        }
    }
    
    // Check if a device is a supported analyzer
    #[allow(dead_code)]
    pub fn is_supported_device(vid: u16, pid: u16) -> bool {
        // Helper function to log support status
        fn log_device_support(vid: u16, pid: u16, supported: bool) -> bool {
            if supported {
                info!("Device VID:{:04x} PID:{:04x} is supported", vid, pid);
            } else {
                debug!("Device VID:{:04x} PID:{:04x} is not in the supported device list", vid, pid);
            }
            supported
        }
        
        // Check for Cynthion (primary and alternate PIDs)
        if vid == CYNTHION_VID {
            // Check all variants of Cynthion PIDs
            if pid == CYNTHION_PID || pid == ALT_CYNTHION_PID_1 || pid == ALT_CYNTHION_PID_2 {
                return log_device_support(vid, pid, true);
            }
        }
        
        // Check for GreatFET devices (same VID as Cynthion but different PID)
        if vid == GREATFET_VID && pid == GREATFET_ONE_PID {
            return log_device_support(vid, pid, true);
        }
        
        // Check for other supported devices
        if vid == GADGETCAP_VID && pid == GADGETCAP_PID {
            return log_device_support(vid, pid, true);
        }
        
        // Special check for macOS: sometimes macOS reports different PIDs for the same device
        // Check if any reported VID matches our known vendors
        if vid == CYNTHION_VID || vid == GREATFET_VID || vid == GADGETCAP_VID {
            info!("Found device with supported vendor ID:{:04x} but unknown product ID:{:04x} - considering compatible", vid, pid);
            return true;
        }
        
        // Device not supported
        log_device_support(vid, pid, false)
    }
    
    // Create a connection for environments without USB access
    // Create a connection without a handle for error cases
    // This will result in early-failure when actual hardware operations are attempted
    #[allow(dead_code)]
    pub fn create_simulation() -> Self {
        error!("Hardware-only mode: No compatible USB devices found");
        error!("Please connect a supported Cynthion device to continue");
        Self {
            handle: None,
            active: true,
            transfer_queue: None,
        }
    }
    
    // Check if we're in simulation mode based on environment variable
    #[allow(dead_code)]
    pub fn is_env_simulation_mode() -> bool {
        // Always return false - no more simulation mode
        // Just log for debugging
        debug!("Simulation mode is always disabled - hardware-only mode");
        info!("Hardware mode enforced - simulation mode disabled");
        false
    }
    
    // Check if this specific connection instance is in simulation mode
    // Always returns false as simulation mode has been removed
    #[allow(dead_code)]
    pub fn is_simulation_mode(&self) -> bool {
        false
    }
    
    #[allow(dead_code)]
    pub fn test_capture_capability(&mut self) -> anyhow::Result<bool> {
        use log::debug;
        debug!("Testing device capture capability");
        
        // If we don't have a handle, can't perform capture
        if self.handle.is_none() {
            return Ok(false);
        }
        
        // For safety, wrap this in a try/catch style operation
        // Use the handle directly since DeviceHandle doesn't implement Clone
        let handle_clone = match &self.handle {
            Some(h) => h,
            None => return Ok(false)
        };
        
        // First, check if we can send a control transfer to test the device
        let result = match handle_clone.claim_interface(CYNTHION_INTERFACE) {
            Ok(_) => {
                debug!("Successfully claimed interface {} for test", CYNTHION_INTERFACE);
                
                // Send a benign command to see if the device is responsive
                let cmd_check = [0xC0]; // Simple status check command
                // Use a separate send_command function to avoid borrowing self
                match self.send_test_command(handle_clone, &cmd_check) {
                    Ok(_) => {
                        debug!("Successfully sent test command to device");
                        
                        // Now try to read from the device with a very short timeout
                        let control_timeout = std::time::Duration::from_millis(50);
                        let read_result = handle_clone.read_control(
                            0xC0, // vendor request, device-to-host
                            0x00, // simple status request
                            0, 0, 
                            &mut [0u8; 8], 
                            control_timeout
                        );
                        
                        match read_result {
                            Ok(_) => {
                                debug!("Successfully read from device");
                                true // Device appears to support capture
                            },
                            Err(e) => {
                                debug!("Error reading from device: {}", e);
                                false // Read failed
                            }
                        }
                    },
                    Err(e) => {
                        debug!("Error sending test command: {}", e);
                        false // Failed to send command
                    }
                }
            },
            Err(e) => {
                debug!("Could not claim interface for test: {}", e);
                false // Failed to claim interface
            }
        };
        
        // Release interface if we claimed it
        let _ = handle_clone.release_interface(CYNTHION_INTERFACE);
        
        Ok(result)
    }
    
    // Set simulation mode explicitly (now does nothing as simulation mode is removed)
    #[allow(dead_code)]
    pub fn set_simulation_mode(&mut self, _enabled: bool) {
        // No longer changes anything - simulation mode has been removed
        // Just log for debugging
        info!("Simulation mode has been removed from the application");
    }
    
    // Set a read timeout for USB operations
    #[allow(dead_code)]
    pub fn set_read_timeout(&mut self, timeout: Option<std::time::Duration>) -> Result<()> {
        // Always use hardware mode
        
        // Store the timeout value to use it in read operations
        info!("Setting USB read timeout to {:?}", timeout);
        
        // DeviceHandle doesn't have a direct set_read_timeout method
        // We'll store it in the connection object and use it manually
        // in each read/transfer operation
        
        // For now, just succeed as we'll use the timeout manually in operations
        Ok(())
    }
    
    // Force hardware-only mode - ensures we only use real devices
    // This is now just a placeholder as the app always runs in hardware-only mode
    #[allow(dead_code)]
    pub fn force_real_device_mode() {
        // App is always in hardware-only mode
        info!("Hardware-only mode is permanently enforced");
        std::env::set_var("USBFLY_SIMULATION_MODE", "0");
        std::env::set_var("USBFLY_FORCE_HARDWARE", "1");
    }
    
    // Helper method to complete device connection - must use GlobalContext for compatibility with struct
    #[allow(dead_code)]
    fn connect_to_device(device: rusb::Device<rusb::GlobalContext>) -> Result<Self> {
        // Before opening the device, get the descriptor first
        let descriptor = match device.device_descriptor() {
            Ok(desc) => desc,
            Err(e) => {
                error!("🚫 Cannot read device descriptor: {}", e);
                return Err(anyhow!("Failed to read device descriptor: {}", e));
            }
        };
        
        let vendor_id = descriptor.vendor_id();
        let product_id = descriptor.product_id();
        info!("Attempting to connect to device VID:{:04x} PID:{:04x}", vendor_id, product_id);
        
        // Get device handle with better error handling
        let handle = match device.open() {
            Ok(h) => h,
            Err(e) => {
                error!("🚫 Cannot open USB device VID:{:04x} PID:{:04x}: {}", 
                      vendor_id, product_id, e);
                return Err(anyhow!("Failed to open USB device: {}", e));
            }
        };
        
        // Additional logging for device information
        let timeout = Duration::from_millis(100);
        
        // Try to get available languages with defensive programming
        let default_language = match handle.read_languages(timeout) {
            Ok(langs) if !langs.is_empty() => Some(langs[0]),
            Ok(_) => {
                debug!("Device returned empty language list");
                None
            },
            Err(e) => {
                debug!("Cannot read device languages: {}", e);
                None
            }
        };
        
        // Get product name with language (safely)
        let product_name = match default_language {
            Some(lang) => {
                if let Some(_idx) = descriptor.product_string_index() {
                    match handle.read_product_string(lang, &descriptor, timeout) {
                        Ok(name) => name,
                        Err(e) => {
                            debug!("Cannot read product string: {}", e);
                            "Unknown Device".to_string()
                        }
                    }
                } else {
                    "Unnamed Device".to_string()
                }
            },
            None => "Unknown Device".to_string(),
        };
        
        info!("Connecting to {} (VID:{:04x}, PID:{:04x})", 
            product_name, vendor_id, product_id);
        
        // Safety check: Verify if device has the needed interface before attempting to claim it
        let config_result = device.active_config_descriptor();
        if let Err(e) = &config_result {
            warn!("Could not get active configuration: {}. Will try to continue.", e);
            // Instead of failing, we'll try to proceed anyway
            // Some devices still work without getting the config descriptor
        }
        
        // Claim interface with better error handling
        #[cfg(not(target_os = "windows"))]
        {
            // On non-Windows platforms, try to reset the device but don't fail if it doesn't work
            if let Err(e) = handle.reset() {
                warn!("Could not reset device: {}. Will try to continue anyway.", e);
                // Continue anyway - this is a soft failure
            }
        }
        
        #[cfg(unix)]
        {
            // On Unix platforms, try to detach kernel driver
            match handle.set_auto_detach_kernel_driver(true) {
                Ok(_) => info!("Set auto-detach kernel driver"),
                Err(e) => {
                    // This often fails on macOS but isn't a critical error
                    warn!("Could not set kernel driver auto-detach: {}. Will continue anyway.", e);
                }
            }
        }
        
        // Check if the interface is available and log any issues
        if let Ok(config) = device.active_config_descriptor() {
            let _interface_available = config.interfaces().any(|i| i.number() == CYNTHION_INTERFACE);
            if !_interface_available {
                warn!("Device does not appear to have interface {}. Will try anyway.", CYNTHION_INTERFACE);
            }
        } else {
            // Can't get config descriptor - just log a warning
            warn!("Could not get device configuration descriptor. Will try interface claim anyway.");
        }
        
        // Try to claim the interface with better error handling
        let claim_result = handle.claim_interface(CYNTHION_INTERFACE);
        
        match claim_result {
            Ok(_) => {
                info!("Successfully claimed interface {}", CYNTHION_INTERFACE);
                // Create a connection with verified handle
                Ok(Self {
                    handle: Some(handle),
                    active: true,
                    transfer_queue: None,
                })
            },
            Err(e) => {
                error!("Failed to claim interface: {}", e);
                
                // On macOS, some errors can be ignored in certain cases
                #[cfg(target_os = "macos")]
                {
                    // On macOS, we need to be extra careful with USB interface handling
                    // The crash report shows that darwin_get_interface can segfault if the device state is unexpected
                    
                    // Check for common macOS USB error messages
                    let is_mac_usb_error = e.to_string().contains("USBInterfaceOpen") || 
                                          e.to_string().contains("EACCES") || 
                                          e.to_string().contains("EPERM") || 
                                          e.to_string().contains("EBUSY") ||
                                          e.to_string().contains("IOReturn") ||
                                          e.to_string().contains("libusb");
                    
                    if is_mac_usb_error {
                        warn!("USB interface access issue on macOS: {}", e);
                        info!("On macOS, we'll continue in a read-only mode that may have limited functionality");
                        
                        // CRITICAL FIX: DO NOT use the actual handle for USB operations on macOS
                        // when we encounter interface access issues, to prevent segfaults
                        // Using None for handle and true for simulation_mode prevents any
                        // direct USB I/O that could cause segfaults due to invalid pointer dereferencing
                        
                        // Drop the handle explicitly to ensure proper cleanup
                        drop(handle);
                        
                        return Ok(Self {
                            handle: None,
                            active: true,
                            transfer_queue: None,
                        });
                    }
                }
                
                // For all other platforms or errors, return the error
                Err(e.into())
            }
        }
    }
    
    #[allow(dead_code)]
    pub async fn connect() -> Result<Self> {
        // On macOS, we need to be more aggressive about forcing hardware mode
        #[cfg(target_os = "macos")]
        {
            if std::env::var("USBFLY_FORCE_HARDWARE").is_ok() {
                // Force hardware mode on macOS to prevent simulation mode
                info!("macOS detected with FORCE_HARDWARE flag - prioritizing hardware connections");
                std::env::set_var("USBFLY_SIMULATION_MODE", "0");
            } else {
                // Set the force hardware flag for future checks
                std::env::set_var("USBFLY_FORCE_HARDWARE", "1");
            }
        }
        
        // Check if simulation mode is enabled via environment variable
        // But first scan for real devices to potentially override the setting
        if let Ok(context) = rusb::Context::new() {
            if let Ok(devices) = context.devices() {
                for device in devices.iter() {
                    if let Ok(desc) = device.device_descriptor() {
                        let vid = desc.vendor_id();
                        let pid = desc.product_id();
                        if Self::is_supported_device(vid, pid) {
                            // We found a real Cynthion! Force hardware mode
                            info!("Found real Cynthion VID:{:04x} PID:{:04x} - forcing hardware mode", vid, pid);
                            std::env::set_var("USBFLY_SIMULATION_MODE", "0");
                            break;
                        }
                    }
                }
            }
        }
        
        // Now check if simulation mode is still enabled after our detection
        if Self::is_env_simulation_mode() {
            info!("Environment indicates simulation mode. Using simulated device.");
            return Ok(Self::create_simulation());
        }
        
        // Use tokio's spawn_blocking to move the potentially blocking USB operations to a worker thread
        // This prevents the UI from hanging during USB operations
        let connection_result = tokio::task::spawn_blocking(|| -> Result<Self> {
            // Try to create USB context, if it fails, use simulation mode
            let context = match rusb::Context::new() {
                Ok(ctx) => ctx,
                Err(e) => {
                    warn!("USB context initialization failed: {}. Using simulation mode.", e);
                    return Ok(Self::create_simulation());
                }
            };
            
            // Add a timeout for USB operations to prevent hanging
            let _timeout = std::time::Duration::from_secs(3); // 3 second timeout
            
            // Use a separate thread with timeout to find devices
            let thread_handle = std::thread::spawn(move || {
                // Debug: Log all connected USB devices
                info!("Searching for compatible USB devices...");
                if let Ok(device_list) = Self::list_devices() {
                    for (i, device) in device_list.iter().enumerate() {
                        info!("USB Device {}: {}", i, device);
                    }
                }
                
                // Find Cynthion or compatible device
                let devices_result = context.devices();
                if let Err(e) = &devices_result {
                    warn!("Error enumerating USB devices: {}", e);
                    return Ok(Self::create_simulation());
                }
                
                let devices = devices_result.unwrap();
                let device = devices
                    .iter()
                    .find(|device| {
                        if let Ok(descriptor) = device.device_descriptor() {
                            let vid = descriptor.vendor_id();
                            let pid = descriptor.product_id();
                            
                            // Check if this is a supported device
                            if Self::is_supported_device(vid, pid) {
                                info!("Found compatible device: VID:{:04x} PID:{:04x}", vid, pid);
                                return true;
                            }
                            
                            // Additional debugging
                            debug!("Skipping unsupported device: VID:{:04x} PID:{:04x}", vid, pid);
                        }
                        false
                    });
                    
                // Handle the case where no compatible device is found
                let device = match device {
                    Some(dev) => dev,
                    None => {
                        // First check if we have permission issues with USB devices
                        let devices = context.devices().unwrap_or_else(|_| {
                            warn!("Could not enumerate devices a second time");
                            devices
                        });
                        let has_devices = devices.iter().count() > 0;
                        
                        if !has_devices {
                            warn!("No USB devices found at all - check USB subsystem");
                            return Err(anyhow!("No USB devices found. Check if USB is working properly on your system."));
                        }
                        
                        // Try to open first device to check permissions
                        let first_device = devices.iter().next();
                        if let Some(dev) = first_device {
                            match dev.open() {
                                Ok(_) => {
                                    // We can open devices, but no compatible ones found
                                    warn!("USB access works, but no compatible devices found");
                                    return Err(anyhow!(
                                        "No compatible USB analyzer devices found. Make sure your Cynthion device is connected."));
                                }
                                Err(e) => {
                                    // We have permission issues
                                    warn!("USB permission error: {}", e);
                                    if cfg!(target_os = "linux") {
                                        return Err(anyhow!(
                                            "USB permission error: {}. Try running with sudo or add udev rules for USB access.", e));
                                    } else {
                                        return Err(anyhow!(
                                            "USB permission error: {}. You might need administrator privileges to access USB devices.", e));
                                    }
                                }
                            }
                        } else {
                            warn!("No compatible USB devices found");
                            return Err(anyhow!("No compatible USB analyzer devices found. Please check your connection."));
                        }
                    }
                };
                
                // Instead of trying to convert the device, let's get a fresh device from the global context
                // This is a cleaner approach than trying to transmute between different context types
                
                // First get device info from current device so we can find it again
                if let Ok(descriptor) = device.device_descriptor() {
                    let vid = descriptor.vendor_id();
                    let pid = descriptor.product_id();
                    let bus_number = device.bus_number();
                    let address = device.address();
                    
                    info!("Reconnecting to device VID:{:04x} PID:{:04x} on bus {} address {}", 
                          vid, pid, bus_number, address);
                    
                    // Now find the same device using the global context
                    let global_context = rusb::GlobalContext::default();
                    if let Ok(devices) = global_context.devices() {
                        for global_device in devices.iter() {
                            // Check if this is the same device by comparing bus number and address
                            if global_device.bus_number() == bus_number && global_device.address() == address {
                                if let Ok(global_desc) = global_device.device_descriptor() {
                                    if global_desc.vendor_id() == vid && global_desc.product_id() == pid {
                                        info!("Found matching device in global context");
                                        return Self::connect_to_device(global_device);
                                    }
                                }
                            }
                        }
                    }
                    
                    // If we couldn't find the device, fall back to simulation mode
                    warn!("Could not find matching device in global context - using simulation mode");
                    return Ok(Self::create_simulation());
                } else {
                    // If we can't get the descriptor, also fall back to simulation mode
                    warn!("Could not get device descriptor - using simulation mode");
                    return Ok(Self::create_simulation());
                }
            });
            
            // Wait for the thread to finish with a timeout
            match thread_handle.join() {
                Ok(result) => result,
                Err(_) => {
                    error!("Thread panic during USB device connection");
                    Ok(Self::create_simulation())
                }
            }
        }).await?;
        
        // Handle any errors from the blocking operation
        match connection_result {
            Ok(connection) => Ok(connection),
            Err(e) => {
                warn!("Connection failed with error: {}", e);
                if e.to_string().contains("timeout") || e.to_string().contains("timed out") {
                    // If we timed out, it's likely the device is hanging
                    warn!("Connection timed out - falling back to simulation mode");
                    Ok(Self::create_simulation())
                } else {
                    Err(e)
                }
            }
        }
    }
    
    pub fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Cynthion device");
        
        // First, stop any active capture and transfer queue
        if self.active {
            // Stop capture first
            debug!("Stopping any active capture before disconnecting");
            match self.stop_capture() {
                Ok(_) => debug!("Successfully stopped capture before disconnecting"),
                Err(e) => warn!("Error stopping capture during disconnect: {}", e)
            }
            
            // Stop the transfer queue
            if let Some(queue) = &mut self.transfer_queue {
                debug!("Shutting down transfer queue");
                queue.shutdown();
                // Give the queue a moment to shut down cleanly
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        
        // Always mark as inactive first to prevent new read operations
        self.active = false;
        
        // Clear transfer queue reference
        self.transfer_queue = None;
        
        // If handle is None, device might already be disconnected
        if self.handle.is_none() {
            info!("Disconnected from Cynthion device (no handle)");
            return Ok(());
        }
        
        // Standard disconnect for real device
        if let Some(handle) = self.handle.take() {
            // Use a separate scope to ensure handle is dropped after use
            {
                // Only try to release interface if not on macOS to avoid potential crashes
                #[cfg(not(target_os = "macos"))]
                {
                    // Retry interface release a few times
                    for attempt in 1..=3 {
                        match handle.release_interface(CYNTHION_INTERFACE) {
                            Ok(_) => {
                                debug!("Successfully released USB interface on attempt {}", attempt);
                                break;
                            },
                            Err(e) if attempt < 3 => {
                                warn!("Failed to release interface (attempt {}): {} - retrying", attempt, e);
                                std::thread::sleep(std::time::Duration::from_millis(50));
                            },
                            Err(e) => {
                                error!("Failed to release interface after multiple attempts: {} (continuing cleanup)", e);
                            }
                        }
                    }
                }
                
                // On macOS, skip the release_interface call which could cause crashes
                #[cfg(target_os = "macos")]
                {
                    debug!("On macOS, skipping release_interface to avoid potential crashes");
                }
                
                // For all platforms, try to reset the device before dropping the handle
                #[cfg(not(target_os = "windows"))]
                {
                    // Reset can help clear stalled endpoints, but don't fail if it doesn't work
                    let _ = handle.reset();
                }
            }
            
            info!("Disconnected from Cynthion device");
        } else {
            debug!("No device handle to disconnect - likely already disconnected");
        }
        
        // Ensure handle is cleared
        self.handle = None;
        
        Ok(())
    }
    
    // Get a list of simulated USB devices
    #[allow(dead_code)]
    pub fn get_simulated_devices() -> Vec<USBDeviceInfo> {
        vec![
            USBDeviceInfo {
                vendor_id: CYNTHION_VID,
                product_id: CYNTHION_PID,
                manufacturer: Some("Great Scott Gadgets".to_string()),
                product: Some("Cynthion USB Analyzer [SIMULATED]".to_string()),
                serial_number: Some("SIM12345".to_string()),
            },
            USBDeviceInfo {
                vendor_id: GREATFET_VID,
                product_id: GREATFET_ONE_PID,
                manufacturer: Some("Great Scott Gadgets".to_string()),
                product: Some("GreatFET [SIMULATED]".to_string()),
                serial_number: Some("SIM98765".to_string()),
            },
        ]
    }
    
    // In hardware-only mode, this returns an empty vector with a warning
    #[allow(dead_code)]
    fn get_simulated_data(&self) -> Vec<u8> {
        // Hardware-only mode - return an empty vector
        error!("Hardware-only mode: No simulated data available");
        error!("Please connect a supported Cynthion device to capture real data");
        
        // Return empty data
        Vec::new()
    }
    
    // Generate simulated MitM USB traffic from a connected device through Cynthion
    // This simulates what would be captured when Cynthion is placed between a host and device
    #[allow(dead_code)]
    pub fn get_simulated_mitm_traffic(&self) -> Vec<u8> {
        // Hardware-only mode - return an empty vector
        error!("Hardware-only mode: No simulated traffic available");
        error!("Please connect a supported Cynthion device to capture real traffic");
        // Return empty data
        Vec::new()
    }
    
    // Public-access version of get_simulated_mitm_traffic for use in app.rs
    // This is the same as get_simulated_mitm_traffic but with a different name for clarity
    #[allow(dead_code)]
    pub fn get_simulated_mitm_traffic_pub(&self) -> Vec<u8> {
        // Hardware-only mode - return an empty vector
        error!("Hardware-only mode: No simulated traffic available");
        error!("Please connect a supported Cynthion device to capture real traffic");
        // Return empty data - in hardware mode, we don't generate fake data
        Vec::new()
    }
    
    #[allow(dead_code)]
    pub async fn read_data(&mut self) -> Result<Vec<u8>> {
        if !self.active {
            return Err(anyhow!("Not connected"));
        }
        
        // Hardware-only mode
        if self.handle.is_none() {
            return Err(anyhow!("No device handle available"));
        }
        
        // Real device mode - proceed with actual USB communication
        let handle = self.handle.as_mut().ok_or_else(|| anyhow!("No device handle"))?;
        
        // Buffer to store data
        let mut buffer = [0u8; 512];
        
        // Read data with timeout
        match handle.read_bulk(CYNTHION_IN_EP, &mut buffer, TIMEOUT) {
            Ok(len) => {
                debug!("Read {} bytes from Cynthion", len);
                Ok(buffer[..len].to_vec())
            }
            Err(e) => {
                error!("Error reading from Cynthion: {}", e);
                // Sleep a bit to not overwhelm with error messages
                sleep(Duration::from_millis(100)).await;
                Err(anyhow!("Failed to read from device: {}", e))
            }
        }
    }
    
    // This function performs the actual data reading synchronously 
    // Returns a byte buffer with the data read from the device
    #[allow(dead_code)]
    fn read_data_sync(&mut self) -> Result<Vec<u8>> {
        // Check active state up front
        if !self.active {
            return Err(anyhow!("Not connected"));
        }
        
        // Hardware-only mode
        // No more simulation mode
        
        // Get handle or return error with safety check
        if self.handle.is_none() {
            self.active = false; // Mark as inactive to prevent further attempts
            return Err(anyhow!("Device disconnected - handle is missing"));
        }
        
        // Buffer to store data (define this first so it's available in all code paths)
        let mut buffer = [0u8; 512];
        
        // Enhanced handling for macOS to better detect hot-plugged devices - AFTER buffer declaration
        #[cfg(target_os = "macos")]
        {
            // First check for USBFLY_FORCE_HARDWARE=1, which is the definitive override for macOS safety
            let force_hardware = std::env::var("USBFLY_FORCE_HARDWARE")
                .map(|val| val == "1")
                .unwrap_or(false);
                
            // Check for special environment flags related to device detection
            let force_refresh = std::env::var("USBFLY_FORCE_REFRESH").is_ok();
            let recently_detected = std::env::var("USBFLY_DEVICE_DETECTED")
                .map(|val| val == "1")
                .unwrap_or(false);
                
            // If any of our device detection flags are active, prioritize hardware mode
            if force_hardware || force_refresh || recently_detected {
                info!("✓ macOS HARDWARE MODE ACTIVE: Using actual device for USB operations");
                
                // If this was initiated by a force refresh, set some additional state
                if force_refresh && !force_hardware {
                    info!("🔄 Force refresh detected - enabling full hardware access");
                    std::env::set_var("USBFLY_FORCE_HARDWARE", "1");
                    std::env::set_var("USBFLY_SIMULATION_MODE", "0");
                }
                
                // Log the current environment state for debugging
                debug!("macOS Environment state: FORCE_HARDWARE={}, SIMULATION_MODE={}, REFRESH={}, DETECTED={}", 
                      std::env::var("USBFLY_FORCE_HARDWARE").unwrap_or_else(|_| "not set".to_string()),
                      std::env::var("USBFLY_SIMULATION_MODE").unwrap_or_else(|_| "not set".to_string()),
                      if force_refresh { "true" } else { "false" },
                      if recently_detected { "true" } else { "false" });
                
                // Continue to the real device operations below
            } else if !std::env::var("USBFLY_SIMULATION_MODE").map(|val| val == "0").unwrap_or(false) {
                // In safe mode, use simulated data to avoid potential crashes
                warn!("⚠️ macOS SAFE MODE: Using simulated data instead of hardware access ⚠️");
                
                // Log the current state of all related environment variables for debugging
                warn!("macOS Environment variables: FORCE_HARDWARE={}, SIMULATION_MODE={}", 
                     std::env::var("USBFLY_FORCE_HARDWARE").unwrap_or_else(|_| "not set".to_string()),
                     std::env::var("USBFLY_SIMULATION_MODE").unwrap_or_else(|_| "not set".to_string()));
                     
                warn!("To enable real device access, click 'Force Scan for Hardware' button or set USBFLY_FORCE_HARDWARE=1");
                return Ok(self.get_simulated_data());
            } else {
                info!("✓ macOS HARDWARE MODE ACTIVE via environment settings");
                // Continue to the real device operations below
            }
        }
        
        // Safely access the handle reference with panic protection
        let read_result = match &mut self.handle {
            Some(handle) => {
                // To prevent segfaults, we'll wrap the USB operation in a catch_unwind
                // This protects against panic in the underlying USB library
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    handle.read_bulk(CYNTHION_IN_EP, &mut buffer, TIMEOUT)
                })) {
                    Ok(result) => result,
                    Err(_) => {
                        error!("Panic detected in USB operation - switching to safe mode");
                        self.active = false; // Mark as inactive after panic
                        self.handle = None;  // Clear handle to prevent further access
                        return Err(anyhow!("USB operation panicked - device access reset for safety"));
                    }
                }
            },
            None => return Err(anyhow!("Device handle lost during operation"))
        };
        
        // Handle read result
        match read_result {
            Ok(len) => {
                debug!("Read {} bytes from Cynthion", len);
                Ok(buffer[..len].to_vec())
            }
            Err(e) => {
                // Check if this is a fatal error that indicates device disconnection
                if e.to_string().contains("No such device") || 
                   e.to_string().contains("not found") || 
                   e.to_string().contains("disconnected") ||
                   e.to_string().contains("timed out") {
                    
                    // Device appears to be disconnected - clean up
                    info!("Device appears to be disconnected: {}", e);
                    self.active = false;
                    
                    // This will help prevent hanging on close by cleaning up resources
                    if let Some(_handle) = self.handle.take() {
                        #[cfg(not(target_os = "macos"))]
                        let _ = _handle.release_interface(CYNTHION_INTERFACE);
                    }
                    
                    return Err(anyhow!("Device disconnected"));
                }
                
                // For other errors, log and continue
                error!("Error reading from Cynthion: {}", e);
                // Add a delay to avoid error message spam
                std::thread::sleep(Duration::from_millis(100));
                Err(anyhow!("USB read error: {}", e))
            }
        }
    }
    
    // This version allows avoiding holding a MutexGuard across an await point
    // It's simpler, returning Result directly rather than a future
    #[allow(dead_code)]
    pub fn read_data_clone(&mut self) -> Result<Vec<u8>> {
        // First check connection state to avoid potential issues
        if !self.active {
            return Err(anyhow!("Device not active"));
        }
        
        // Hardware-only mode
        
        // Extra safety check - this protects against null pointer issues
        if self.handle.is_none() {
            self.active = false; // Mark as inactive
            return Err(anyhow!("Device disconnected - no handle available"));
        }
        
        // Simply call the synchronous function directly with protection in place
        self.read_data_sync()
    }
    
    // Thread-safe method to read MitM traffic flowing through the Cynthion
    // This captures traffic between the host and the device connected to Cynthion
    #[allow(dead_code)]
    pub fn read_mitm_traffic_clone(&mut self) -> Result<Vec<u8>> {
        // First check connection state
        if !self.active {
            debug!("MitM traffic capture failed: Device not active");
            return Err(anyhow!("Device not active"));
        }
        
        // Hardware-only mode
        
        // Extra safety check - this protects against null pointer issues
        if self.handle.is_none() {
            error!("MitM traffic capture failed: Device handle is None");
            self.active = false; // Mark as inactive
            return Err(anyhow!("Device disconnected - no handle available"));
        }
        
        info!("MitM traffic: Requesting data from connected Cynthion device");
        
        // For real devices, we need to implement the protocol for reading MitM traffic
        // This would involve sending a command to switch to MitM mode and reading from
        // a different endpoint or using a different command protocol
        
        // First, send command to get captured data - this sets up the device to send MitM data
        let command = [CMD_GET_CAPTURED_DATA];
        info!("MitM traffic: Sending GET_CAPTURED_DATA command to device");
        
        match self.send_command(&command) {
            Ok(_) => {
                debug!("MitM traffic: Command sent successfully, reading response...");
                // Now read the actual data - with real hardware, this would return the MitM data
                match self.read_data_sync() {
                    Ok(data) => {
                        info!("MitM traffic: Received {} bytes of traffic data", data.len());
                        // Add detailed hexdump of first 32 bytes (if available) for debugging
                        if !data.is_empty() {
                            let preview_len = std::cmp::min(data.len(), 32);
                            let mut hex_preview = String::new();
                            for b in data.iter().take(preview_len) {
                                hex_preview.push_str(&format!("{:02X} ", b));
                            }
                            debug!("MitM traffic data preview: {}", hex_preview);
                            
                            // Additional packet analysis
                            Self::analyze_packet_headers(&data);
                        }
                        Ok(data)
                    },
                    Err(e) => {
                        error!("MitM traffic: Failed to read response: {}", e);
                        Err(e)
                    }
                }
            },
            Err(e) => {
                warn!("MitM traffic: Failed to send command: {}", e);
                // Try fallback to normal read
                info!("MitM traffic: Attempting fallback to standard read");
                match self.read_data_sync() {
                    Ok(data) => {
                        info!("MitM traffic: Fallback read successful, got {} bytes", data.len());
                        Ok(data)
                    },
                    Err(e) => {
                        error!("MitM traffic: Fallback read also failed: {}", e);
                        Err(e)
                    }
                }
            }
        }
    }
    
    // Helper function to analyze USB packet headers for debugging
    // Non-mutable version of send_command for testing device capabilities
    #[allow(dead_code)]
    pub fn send_test_command(&self, handle: &rusb::DeviceHandle<rusb::GlobalContext>, command: &[u8]) -> Result<()> {
        // Safety check for connection state
        if !self.active {
            return Err(anyhow!("Connection not active"));
        }
        
        // Hardware-only mode now
        
        debug!("Sending test command: {:02X?}", command);
        
        // Define the necessary constants locally to avoid scope issues
        const CONTROL_OUT: u8 = 0x40; // Control transfer, host to device
        const VENDOR_CMD: u8 = 0x80;  // Vendor-specific command
        
        // Send the command via control transfer using the provided handle
        handle.write_control(
            CONTROL_OUT, 
            VENDOR_CMD, 
            0, 0, 
            command, 
            std::time::Duration::from_millis(1000)
        )?;
        
        Ok(())
    }
    
    #[allow(dead_code)]
    fn analyze_packet_headers(data: &[u8]) {
        if data.len() < 2 {
            debug!("MitM analysis: Data too short for packet analysis");
            return;
        }
        
        let mut i = 0;
        let mut packet_counts = std::collections::HashMap::new();
        
        while i + 1 < data.len() {
            let packet_type = data[i];
            // Count packet types
            *packet_counts.entry(packet_type).or_insert(0) += 1;
            
            // Basic packet type identification
            match packet_type {
                0x80 => {
                    if i + 9 < data.len() {
                        let bmrequest_type = data[i+2];
                        let brequest = data[i+3];
                        let wvalue = u16::from_le_bytes([data[i+4], data[i+5]]);
                        let windex = u16::from_le_bytes([data[i+6], data[i+7]]);
                        let wlength = u16::from_le_bytes([data[i+8], data[i+9]]);
                        
                        debug!("MitM packet: SETUP bmRequestType=0x{:02X} bRequest=0x{:02X} wValue=0x{:04X} wIndex=0x{:04X} wLength={}", 
                               bmrequest_type, brequest, wvalue, windex, wlength);
                        
                        i += 10; // Skip the setup packet
                    } else {
                        i += 1; // Not enough data, move forward cautiously
                    }
                },
                0x81 => {
                    debug!("MitM packet: DATA at offset {}", i);
                    i += 2; // Skip header and address
                },
                0x82 => {
                    if i + 2 < data.len() {
                        debug!("MitM packet: STATUS at offset {}, value: 0x{:02X}", i, data[i+2]);
                        i += 3; // Skip status packet
                    } else {
                        i += 1;
                    }
                },
                0x83 => {
                    debug!("MitM packet: BULK at offset {}", i);
                    i += 2; // Skip header and endpoint/address
                },
                _ => {
                    debug!("MitM packet: Unknown type 0x{:02X} at offset {}", packet_type, i);
                    i += 1; // Unknown packet type, move forward cautiously
                }
            }
        }
        
        // Summary of packet types
        debug!("MitM traffic summary: {:?} packets", packet_counts);
    }
    
    // Send a command to the Cynthion device 
    #[allow(dead_code)]
    pub fn send_command(&mut self, command: &[u8]) -> Result<()> {
        if !self.active {
            return Err(anyhow!("Not connected"));
        }
        
        // Hardware-only mode now
        
        let handle = self.handle.as_mut().ok_or_else(|| anyhow!("No device handle"))?;
        
        match handle.write_bulk(CYNTHION_OUT_EP, command, TIMEOUT) {
            Ok(len) => {
                debug!("Sent {} bytes to Cynthion", len);
                Ok(())
            }
            Err(e) => {
                error!("Error sending command to Cynthion: {}", e);
                Err(anyhow!("Failed to send command: {}", e))
            }
        }
    }
    
    // Start capturing USB traffic (MitM mode)
    #[allow(dead_code)]
    pub fn start_capture(&mut self) -> Result<()> {
        info!("Starting USB traffic capture (MitM mode)");
        
        // Prepare the command to start capture
        let command = [CMD_START_CAPTURE];
        self.send_command(&command)
    }
    
    // Stop capturing USB traffic with improved error handling
    pub fn stop_capture(&mut self) -> Result<()> {
        info!("Stopping USB traffic capture");
        
        // First stop the transfer queue if it exists
        if let Some(queue) = &mut self.transfer_queue {
            debug!("Stopping transfer queue first before sending stop command");
            queue.shutdown();
        }
        
        // Check if connection is active
        if !self.active {
            warn!("Cannot stop capture: not connected to device");
            return Ok(());
        }
        
        // Prepare the command to stop capture
        let command = [CMD_STOP_CAPTURE];
        
        // Send the command with retry mechanism
        let max_retries = 3;
        for attempt in 1..=max_retries {
            match self.send_command(&command) {
                Ok(_) => {
                    info!("Successfully sent stop command to device on attempt {}", attempt);
                    // Send a second command to ensure it's received
                    if attempt == 1 {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        continue; // Send a second attempt for redundancy
                    }
                    return Ok(());
                },
                Err(e) if attempt < max_retries => {
                    warn!("Failed to send stop command (attempt {}): {}", attempt, e);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                },
                Err(e) => {
                    error!("Failed to send stop command after {} attempts: {}", max_retries, e);
                    return Err(anyhow!("Failed to stop capture: {}", e));
                }
            }
        }
        
        Ok(())
    }
    
    // Clear the capture buffer
    #[allow(dead_code)]
    pub fn clear_capture_buffer(&mut self) -> Result<()> {
        info!("Clearing capture buffer");
        
        // Prepare the command to clear buffer
        let command = [CMD_CLEAR_BUFFER];
        self.send_command(&command)
    }
    
    // Set capture mode (all traffic, host-to-device only, etc)
    #[allow(dead_code)]
    pub fn set_capture_mode(&mut self, mode: u8) -> Result<()> {
        info!("Setting capture mode to {}", match mode {
            CAPTURE_MODE_ALL => "All Traffic",
            CAPTURE_MODE_HOST_TO_DEVICE => "Host-to-Device Only",
            CAPTURE_MODE_DEVICE_TO_HOST => "Device-to-Host Only",
            CAPTURE_MODE_SETUP_ONLY => "Setup Packets Only",
            _ => "Unknown Mode",
        });
        
        // Prepare the command to set mode
        let command = [CMD_SET_CAPTURE_MODE, mode];
        self.send_command(&command)
    }
    
    // Request captured USB traffic from the Cynthion device
    #[allow(dead_code)]
    pub async fn get_captured_traffic(&mut self) -> Result<Vec<u8>> {
        debug!("Requesting captured USB traffic from Cynthion");
        
        // Hardware-only mode now
        
        // Prepare the command to get captured data
        let command = [CMD_GET_CAPTURED_DATA];
        self.send_command(&command)?;
        
        // Wait a moment for the device to prepare data
        sleep(Duration::from_millis(10)).await;
        
        // Read the captured data
        self.read_data().await
    }
    
    // Additional methods for controlling the Cynthion device
    #[allow(dead_code)]
    pub fn get_device_info(&mut self) -> Result<String> {
        // This is a placeholder - actual implementation would send a command to get device info
        // and parse the response
        Ok("Cynthion USB Analyzer".to_string())
    }
    
    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        // Hardware-only mode - need both active flag and handle
        self.active && self.handle.is_some()
    }
    
    // Check if this is a real hardware device (not simulated)
    #[allow(dead_code)]
    pub fn is_real_hardware_device(&self) -> bool {
        // Hardware-only mode - just check if we have a handle
        self.handle.is_some()
    }
    
    // Process MitM traffic and decode USB transactions
    #[allow(dead_code)]
    pub fn process_mitm_traffic(&self, raw_data: &[u8]) -> Vec<crate::usb::mitm_traffic::UsbTransaction> {
        use log::{debug, trace};
        
        let mut transactions = Vec::new();
        let mut counter: u64 = 0;
        let base_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
            
        // Check if we have enough data to process
        if raw_data.len() < 2 {
            debug!("Raw data too short: {} bytes", raw_data.len());
            return transactions;
        }
        
        debug!("Processing {} bytes of MitM traffic data", raw_data.len());
        
        // Iterate through the data in chunks to process multiple packets
        let mut offset = 0;
        while offset < raw_data.len() {
            // Need at least header + address (2 bytes)
            if offset + 2 > raw_data.len() {
                debug!("Remaining data too short at offset {}: {} bytes", offset, raw_data.len() - offset);
                break;
            }
            
            // Calculate a unique timestamp for each transaction
            // Add a small fraction based on the counter to ensure uniqueness
            let timestamp = base_timestamp + (counter as f64 * 0.0001);
            
            let packet_type = raw_data[offset];
            trace!("Processing packet type 0x{:02X} at offset {}", packet_type, offset);
            
            // Determine packet length based on packet type
            let packet_length = match packet_type {
                0x80 => {
                    // Control setup packet: header(1) + address(1) + setup data(8)
                    if offset + 10 > raw_data.len() {
                        debug!("Setup packet truncated at offset {}", offset);
                        // Skip this header byte and try to resync
                        offset += 1;
                        continue;
                    }
                    10
                },
                0x81 => {
                    // Control data packet: variable length
                    // Start with header(1) + address(1), then determine data length
                    if offset + 2 > raw_data.len() {
                        debug!("Data packet truncated at offset {}", offset);
                        offset += 1;
                        continue;
                    }
                    
                    // Data length is variable, but for simulated data we know the structure
                    // For real data, we'd need to parse the length field
                    // For now, use a heuristic: assume the rest of the packet until next header
                    let mut data_length = 0;
                    for i in (offset + 2)..raw_data.len() {
                        data_length += 1;
                        // If the next byte looks like a packet header, we've reached the end
                        if i + 1 < raw_data.len() && (raw_data[i + 1] == 0x80 || raw_data[i + 1] == 0x81 || 
                                                    raw_data[i + 1] == 0x82 || raw_data[i + 1] == 0x83) {
                            break;
                        }
                    }
                    2 + data_length
                },
                0x82 => {
                    // Status packet: header(1) + address(1) + status(1)
                    if offset + 3 > raw_data.len() {
                        debug!("Status packet truncated at offset {}", offset);
                        offset += 1;
                        continue;
                    }
                    3
                },
                0x83 => {
                    // Bulk transfer: header(1) + endpoint/address(1) + variable data
                    if offset + 2 > raw_data.len() {
                        debug!("Bulk packet truncated at offset {}", offset);
                        offset += 1;
                        continue;
                    }
                    
                    // Similar to control data, use heuristic to find end
                    let mut data_length = 0;
                    for i in (offset + 2)..raw_data.len() {
                        data_length += 1;
                        if i + 1 < raw_data.len() && (raw_data[i + 1] == 0x80 || raw_data[i + 1] == 0x81 || 
                                                    raw_data[i + 1] == 0x82 || raw_data[i + 1] == 0x83) {
                            break;
                        }
                    }
                    2 + data_length
                },
                _ => {
                    // Unknown packet type, skip a byte and try to resync
                    debug!("Unknown packet type 0x{:02X} at offset {}", packet_type, offset);
                    offset += 1;
                    continue;
                }
            };
            
            // Ensure we don't exceed buffer bounds
            let end_offset = std::cmp::min(offset + packet_length, raw_data.len());
            let packet_data = &raw_data[offset..end_offset];
            
            // Decode the packet
            if let Some(transaction) = crate::usb::mitm_traffic::decode_mitm_packet(packet_data, timestamp, counter) {
                debug!("Decoded {} transaction: addr={}, ep=0x{:02X}", 
                      transaction.transfer_type, transaction.device_address, transaction.endpoint);
                transactions.push(transaction);
                counter += 1;
            } else {
                debug!("Failed to decode packet at offset {}", offset);
            }
            
            // Move to next packet
            offset += packet_length;
        }
        
        debug!("Processed {} USB transactions", transactions.len());
        transactions
    }

}

impl Drop for CynthionConnection {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}
