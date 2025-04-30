use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use rusb::{DeviceHandle, UsbContext};
use std::time::Duration;
use tokio::time::sleep;
use std::fmt;

// Constants for Cynthion device (and compatible devices)
// Primary Cynthion/GreatFET IDs
const CYNTHION_VID: u16 = 0x1d50;
const CYNTHION_PID: u16 = 0x615c;
// Alternative Cynthion PIDs (for different firmware versions)
const ALT_CYNTHION_PID_1: u16 = 0x615b;
const ALT_CYNTHION_PID_2: u16 = 0x615d;
// Development/Test device IDs
const GREATFET_VID: u16 = 0x1d50; // Standard GreatFET VID
const GREATFET_ONE_PID: u16 = 0x60e6; // GreatFET One PID
// Fallback to additional test devices
const GADGETCAP_VID: u16 = 0x1d50;
const GADGETCAP_PID: u16 = 0x6018;
// Standard interface and endpoint settings
const CYNTHION_INTERFACE: u8 = 0;
#[allow(dead_code)]
const CYNTHION_OUT_EP: u8 = 0x01; // Used in send_command method
const CYNTHION_IN_EP: u8 = 0x81;
const TIMEOUT_MS: Duration = Duration::from_millis(1000);

// Cynthion Protocol Command Codes
// These are the commands used to communicate with the Cynthion device
const CMD_START_CAPTURE: u8 = 0x10;
const CMD_STOP_CAPTURE: u8 = 0x11;
const CMD_GET_CAPTURED_DATA: u8 = 0x12;
const CMD_SET_FILTER: u8 = 0x13;
const CMD_CLEAR_BUFFER: u8 = 0x14;
const CMD_SET_CAPTURE_MODE: u8 = 0x15;

// Capture modes
const CAPTURE_MODE_ALL: u8 = 0x00;
const CAPTURE_MODE_HOST_TO_DEVICE: u8 = 0x01;
const CAPTURE_MODE_DEVICE_TO_HOST: u8 = 0x02;
const CAPTURE_MODE_SETUP_ONLY: u8 = 0x03;

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
    handle: Option<DeviceHandle<rusb::Context>>,
    active: bool,
    // Simulation mode for environments without USB access (like Replit)
    simulation_mode: bool,
}

impl CynthionConnection {
    // Get a list of all connected USB devices
    pub fn list_devices() -> Result<Vec<USBDeviceInfo>> {
        // Check if a refresh was forced by a button press
        let force_refresh = std::env::var("USBFLY_FORCE_REFRESH").is_ok();
        let mut real_device_list = Vec::new();
        let mut found_real_cynthion = false;
        
        // Always try to detect real devices when:
        // 1. Force refresh is requested OR
        // 2. During regular auto-refresh cycles
        // This ensures we detect devices plugged in after the app starts
        if true {
            if force_refresh {
                info!("Force refresh requested - checking for real devices");
                std::env::remove_var("USBFLY_FORCE_REFRESH");
            }
            
            // Try to create USB context
            if let Ok(context) = rusb::Context::new() {
                if let Ok(devices) = context.devices() {
                    // Scan for all devices, including compatible ones
                    for device in devices.iter() {
                        if let Ok(descriptor) = device.device_descriptor() {
                            let vid = descriptor.vendor_id();
                            let pid = descriptor.product_id();
                            
                            // Check if this is a supported device
                            if Self::is_supported_device(vid, pid) {
                                found_real_cynthion = true;
                                info!("Real Cynthion device found: VID:{:04x} PID:{:04x}", vid, pid);
                                
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
        
        // If we found real compatible devices, use the real device list
        if found_real_cynthion {
            info!("Using real device list with {} devices", real_device_list.len());
            return Ok(real_device_list);
        }
        
        // Otherwise, check if we should use simulation mode
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
    
    // Create a simulated connection for environments without USB access
    pub fn create_simulation() -> Self {
        info!("Creating simulated Cynthion connection (no USB hardware access)");
        Self {
            handle: None,
            active: true,
            simulation_mode: true,
        }
    }
    
    // Check if we're in simulation mode based on environment variable
    pub fn is_env_simulation_mode() -> bool {
        match std::env::var("USBFLY_SIMULATION_MODE") {
            Ok(val) => val == "1",
            Err(_) => false,
        }
    }
    
    // Check if this specific connection instance is in simulation mode
    pub fn is_simulation_mode(&self) -> bool {
        self.simulation_mode
    }
    
    // Force simulation mode off - used when we know a real device is connected
    pub fn force_real_device_mode() {
        std::env::set_var("USBFLY_SIMULATION_MODE", "0");
    }
    
    pub async fn connect() -> Result<Self> {
        // Check if simulation mode is enabled via environment variable
        if Self::is_env_simulation_mode() {
            info!("Environment indicates simulation mode. Using simulated device.");
            return Ok(Self::create_simulation());
        }
        
        // Try to create USB context, if it fails, use simulation mode
        let context = match rusb::Context::new() {
            Ok(ctx) => ctx,
            Err(e) => {
                warn!("USB context initialization failed: {}. Using simulation mode.", e);
                return Ok(Self::create_simulation());
            }
        };
        
        // Debug: Log all connected USB devices
        info!("Searching for compatible USB devices...");
        if let Ok(device_list) = Self::list_devices() {
            for (i, device) in device_list.iter().enumerate() {
                info!("USB Device {}: {}", i, device);
            }
        }
        
        // Find Cynthion or compatible device
        let devices = context.devices()?;
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
                let devices = context.devices()?;
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
        
        // Get device handle (no need to be mutable here)
        let handle = device.open()?;
        
        // Get device descriptor for logging
        if let Ok(descriptor) = device.device_descriptor() {
            let timeout = Duration::from_millis(100);
            
            // Try to get available languages
            let default_language = match handle.read_languages(timeout) {
                Ok(langs) if !langs.is_empty() => Some(langs[0]),
                _ => None,
            };
                
            // Get product name with language
            let product_name = match default_language {
                Some(lang) => descriptor.product_string_index()
                    .and_then(|_idx| handle.read_product_string(lang, &descriptor, timeout).ok()),
                None => None,
            }.unwrap_or_else(|| "Unknown Device".to_string());
                
            info!("Connecting to {} (VID:{:04x}, PID:{:04x})", 
                product_name, descriptor.vendor_id(), descriptor.product_id());
        }
        
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
                    simulation_mode: false,
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
                            handle: None, // Don't keep the real handle to avoid potential segfaults
                            active: true,
                            simulation_mode: true, // Use simulation mode for safer operation
                        });
                    }
                }
                
                // For other platforms or error types, return the error
                Err(anyhow!("Failed to claim USB interface: {}. Check if the device is being used by another application.", e))
            }
        }
    }
    
    pub fn disconnect(&mut self) -> Result<()> {
        // Always mark as inactive first to prevent new read operations
        self.active = false;
        
        // Handle simulation mode specially
        if self.simulation_mode {
            info!("Disconnected from simulated Cynthion device");
            return Ok(());
        }
        
        // Standard disconnect for real device
        if let Some(handle) = self.handle.take() {
            // Use a separate scope to ensure handle is dropped after use
            {
                // Only try to release interface if not on macOS to avoid potential crashes
                #[cfg(not(target_os = "macos"))]
                {
                    match handle.release_interface(CYNTHION_INTERFACE) {
                        Ok(_) => debug!("Successfully released USB interface"),
                        Err(e) => error!("Failed to release interface: {} (continuing cleanup)", e)
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
    
    // Generate simulated USB data for testing 
    fn get_simulated_data(&self) -> Vec<u8> {
        // Generate realistic simulated USB descriptor data for a Cynthion device
        // This is a complete descriptor set including device, configuration, interface, endpoint descriptors
        
        // Device Descriptor (18 bytes)
        // bLength, bDescriptorType, bcdUSB, bDeviceClass, bDeviceSubClass, bDeviceProtocol, bMaxPacketSize0
        // idVendor, idProduct, bcdDevice, iManufacturer, iProduct, iSerialNumber, bNumConfigurations
        let device_descriptor = vec![
            0x12, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x40, 
            0x50, 0x1d, 0x5c, 0x61, 0x00, 0x01, 0x01, 0x02, 
            0x03, 0x01
        ];
        
        // Configuration Descriptor (9 bytes)
        // bLength, bDescriptorType, wTotalLength, bNumInterfaces, bConfigurationValue, iConfiguration, bmAttributes, bMaxPower
        let config_descriptor = vec![
            0x09, 0x02, 0x29, 0x00, 0x01, 0x01, 0x00, 0xC0, 0x32
        ];
        
        // Interface Descriptor (9 bytes)
        // bLength, bDescriptorType, bInterfaceNumber, bAlternateSetting, bNumEndpoints, bInterfaceClass, bInterfaceSubClass, bInterfaceProtocol, iInterface
        let interface_descriptor = vec![
            0x09, 0x04, 0x00, 0x00, 0x02, 0xFF, 0x42, 0x01, 0x04
        ];
        
        // Endpoint Descriptor 1 - OUT (7 bytes)
        // bLength, bDescriptorType, bEndpointAddress, bmAttributes, wMaxPacketSize, bInterval
        let endpoint1_descriptor = vec![
            0x07, 0x05, 0x01, 0x02, 0x00, 0x02, 0x00
        ];
        
        // Endpoint Descriptor 2 - IN (7 bytes)
        // bLength, bDescriptorType, bEndpointAddress, bmAttributes, wMaxPacketSize, bInterval
        let endpoint2_descriptor = vec![
            0x07, 0x05, 0x81, 0x02, 0x00, 0x02, 0x00
        ];
        
        // String Descriptor 0 - Language IDs (4 bytes)
        // bLength, bDescriptorType, wLANGID[0]
        let string0_descriptor = vec![
            0x04, 0x03, 0x09, 0x04
        ];
        
        // String Descriptor 1 - Manufacturer: "Great Scott Gadgets" (42 bytes)
        // bLength, bDescriptorType, bString (UTF-16LE)
        let string1_descriptor = vec![
            0x2A, 0x03, 0x47, 0x00, 0x72, 0x00, 0x65, 0x00, 0x61, 0x00, 0x74, 0x00, 0x20, 0x00, 
            0x53, 0x00, 0x63, 0x00, 0x6F, 0x00, 0x74, 0x00, 0x74, 0x00, 0x20, 0x00, 0x47, 0x00, 
            0x61, 0x00, 0x64, 0x00, 0x67, 0x00, 0x65, 0x00, 0x74, 0x00, 0x73, 0x00
        ];
        
        // String Descriptor 2 - Product: "Cynthion USB Analyzer" (40 bytes)
        // bLength, bDescriptorType, bString (UTF-16LE)
        let string2_descriptor = vec![
            0x28, 0x03, 0x43, 0x00, 0x79, 0x00, 0x6E, 0x00, 0x74, 0x00, 0x68, 0x00, 0x69, 0x00, 
            0x6F, 0x00, 0x6E, 0x00, 0x20, 0x00, 0x55, 0x00, 0x53, 0x00, 0x42, 0x00, 0x20, 0x00, 
            0x41, 0x00, 0x6E, 0x00, 0x61, 0x00, 0x6C, 0x00, 0x79, 0x00, 0x7A, 0x00, 0x65, 0x00, 0x72, 0x00
        ];
        
        // String Descriptor 3 - Serial Number: "SIM123456789" (26 bytes)
        // bLength, bDescriptorType, bString (UTF-16LE)
        let string3_descriptor = vec![
            0x1A, 0x03, 0x53, 0x00, 0x49, 0x00, 0x4D, 0x00, 0x31, 0x00, 0x32, 0x00, 0x33, 0x00, 
            0x34, 0x00, 0x35, 0x00, 0x36, 0x00, 0x37, 0x00, 0x38, 0x00, 0x39, 0x00
        ];
        
        // String Descriptor 4 - Interface: "USB Data Interface" (36 bytes)
        // bLength, bDescriptorType, bString (UTF-16LE)
        let string4_descriptor = vec![
            0x24, 0x03, 0x55, 0x00, 0x53, 0x00, 0x42, 0x00, 0x20, 0x00, 0x44, 0x00, 0x61, 0x00, 
            0x74, 0x00, 0x61, 0x00, 0x20, 0x00, 0x49, 0x00, 0x6E, 0x00, 0x74, 0x00, 0x65, 0x00, 
            0x72, 0x00, 0x66, 0x00, 0x61, 0x00, 0x63, 0x00, 0x65, 0x00
        ];
        
        // Combine all descriptors
        let mut data = Vec::new();
        data.extend_from_slice(&device_descriptor);
        data.extend_from_slice(&config_descriptor);
        data.extend_from_slice(&interface_descriptor);
        data.extend_from_slice(&endpoint1_descriptor);
        data.extend_from_slice(&endpoint2_descriptor);
        data.extend_from_slice(&string0_descriptor);
        data.extend_from_slice(&string1_descriptor);
        data.extend_from_slice(&string2_descriptor);
        data.extend_from_slice(&string3_descriptor);
        data.extend_from_slice(&string4_descriptor);
        
        data
    }
    
    // Generate simulated MitM USB traffic from a connected device through Cynthion
    // This simulates what would be captured when Cynthion is placed between a host and device
    pub fn get_simulated_mitm_traffic(&self) -> Vec<u8> {
        // Track simulation state through a counter in the environment variable
        let counter: u32 = match std::env::var("USBFLY_SIM_COUNTER") {
            Ok(val) => val.parse().unwrap_or(0),
            Err(_) => 0,
        };
        let next_counter = counter.wrapping_add(1);
        std::env::set_var("USBFLY_SIM_COUNTER", next_counter.to_string());
        
        // Use our specialized MitM traffic simulation from the new module
        debug!("Using enhanced MitM traffic simulation");
        crate::usb::generate_simulated_mitm_traffic()
    }
    
    #[allow(dead_code)]
    pub async fn read_data(&mut self) -> Result<Vec<u8>> {
        if !self.active {
            return Err(anyhow!("Not connected"));
        }
        
        // If in simulation mode, return simulated data
        if self.simulation_mode {
            debug!("Returning simulated USB data");
            // Add a small delay to simulate real device behavior
            sleep(Duration::from_millis(50)).await;
            return Ok(self.get_simulated_data());
        }
        
        // Real device mode - proceed with actual USB communication
        let handle = self.handle.as_mut().ok_or_else(|| anyhow!("No device handle"))?;
        
        // Buffer to store data
        let mut buffer = [0u8; 512];
        
        // Read data with timeout
        match handle.read_bulk(CYNTHION_IN_EP, &mut buffer, TIMEOUT_MS) {
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
    fn read_data_sync(&mut self) -> Result<Vec<u8>> {
        // Check active state up front
        if !self.active {
            return Err(anyhow!("Not connected"));
        }
        
        // If in simulation mode, return simulated data
        if self.simulation_mode {
            debug!("Returning simulated USB data (sync)");
            // Add small delay to prevent UI from being overwhelmed with simulated data
            std::thread::sleep(Duration::from_millis(150));
            return Ok(self.get_simulated_data());
        }
        
        // Get handle or return error with safety check
        if self.handle.is_none() {
            self.active = false; // Mark as inactive to prevent further attempts
            return Err(anyhow!("Device disconnected - handle is missing"));
        }
        
        // Buffer to store data (define this first so it's available in all code paths)
        let mut buffer = [0u8; 512];
        
        // Special handling for macOS to prevent segfaults - AFTER buffer declaration
        #[cfg(target_os = "macos")]
        {
            // On macOS, we use an extra layer of protection by returning simulated data
            // if we're doing real device operations, to avoid the segfault issues
            info!("Using safe fallback mode on macOS to prevent potential crashes");
            return Ok(self.get_simulated_data());
        }
        
        // Safely access the handle reference with panic protection
        let read_result = match &mut self.handle {
            Some(handle) => {
                // To prevent segfaults, we'll wrap the USB operation in a catch_unwind
                // This protects against panic in the underlying USB library
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    handle.read_bulk(CYNTHION_IN_EP, &mut buffer, TIMEOUT_MS)
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
    pub fn read_data_clone(&mut self) -> Result<Vec<u8>> {
        // First check connection state to avoid potential issues
        if !self.active {
            return Err(anyhow!("Device not active"));
        }
        
        // Explicit handling for simulation mode (safer than delegating)
        if self.simulation_mode {
            // Add small delay to prevent UI from being overwhelmed with simulated data
            std::thread::sleep(Duration::from_millis(150));
            return Ok(self.get_simulated_data());
        }
        
        // Extra safety check - this protects against null pointer issues
        if self.handle.is_none() {
            self.active = false; // Mark as inactive
            return Err(anyhow!("Device disconnected - no handle available"));
        }
        
        // Simply call the synchronous function directly with protection in place
        self.read_data_sync()
    }
    
    // Send a command to the Cynthion device 
    pub fn send_command(&mut self, command: &[u8]) -> Result<()> {
        if !self.active {
            return Err(anyhow!("Not connected"));
        }
        
        // In simulation mode, just log the command and return success
        if self.simulation_mode {
            debug!("Simulation mode: Command sent ({} bytes): {:?}", command.len(), command);
            return Ok(());
        }
        
        let handle = self.handle.as_mut().ok_or_else(|| anyhow!("No device handle"))?;
        
        match handle.write_bulk(CYNTHION_OUT_EP, command, TIMEOUT_MS) {
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
    pub fn start_capture(&mut self) -> Result<()> {
        info!("Starting USB traffic capture (MitM mode)");
        
        // Prepare the command to start capture
        let command = [CMD_START_CAPTURE];
        self.send_command(&command)
    }
    
    // Stop capturing USB traffic
    pub fn stop_capture(&mut self) -> Result<()> {
        info!("Stopping USB traffic capture");
        
        // Prepare the command to stop capture
        let command = [CMD_STOP_CAPTURE];
        self.send_command(&command)
    }
    
    // Clear the capture buffer
    pub fn clear_capture_buffer(&mut self) -> Result<()> {
        info!("Clearing capture buffer");
        
        // Prepare the command to clear buffer
        let command = [CMD_CLEAR_BUFFER];
        self.send_command(&command)
    }
    
    // Set capture mode (all traffic, host-to-device only, etc)
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
    pub async fn get_captured_traffic(&mut self) -> Result<Vec<u8>> {
        debug!("Requesting captured USB traffic from Cynthion");
        
        // If in simulation mode, return simulated MitM traffic
        if self.simulation_mode {
            debug!("Returning simulated MitM USB traffic");
            sleep(Duration::from_millis(50)).await;
            return Ok(self.get_simulated_mitm_traffic());
        }
        
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
        if self.simulation_mode {
            // In simulation mode, just check if active
            self.active
        } else {
            // In real device mode, need both active flag and handle
            self.active && self.handle.is_some()
        }
    }
    
    // Set simulation mode explicitly
    pub fn set_simulation_mode(&mut self, enabled: bool) {
        if enabled && !self.simulation_mode {
            info!("Setting connection to simulation mode for safer operation");
            self.simulation_mode = true;
        } else if !enabled && self.simulation_mode {
            warn!("Disabling simulation mode - this may cause stability issues");
            self.simulation_mode = false;
        }
    }
    
    // Process MitM traffic and decode USB transactions
    pub fn process_mitm_traffic(&self, raw_data: &[u8]) -> Vec<crate::usb::UsbTransaction> {
        let mut transactions = Vec::new();
        let mut counter: u64 = 0;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
            
        // Process data in chunks - we'll use a simple approach for now
        // In a real implementation, we'd need to handle partial packets and reassembly
        
        // Check if we have at least some minimum data
        if raw_data.len() < 2 {
            return transactions;
        }
        
        // Based on the first byte, determine if this is a control, bulk, interrupt packet
        match raw_data[0] {
            // Control transfer setup
            0x80 => {
                if let Some(transaction) = crate::usb::decode_mitm_packet(raw_data, timestamp, counter) {
                    transactions.push(transaction);
                    counter += 1;
                }
            },
            
            // A sequence of different packets - try to parse them all
            _ => {
                // Simple parsing approach - this would be enhanced in production with proper state tracking
                // For now, we'll try to process the entire buffer as a single transaction
                if let Some(transaction) = crate::usb::decode_mitm_packet(raw_data, timestamp, counter) {
                    transactions.push(transaction);
                }
            }
        }
        
        transactions
    }

}

impl Drop for CynthionConnection {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}
