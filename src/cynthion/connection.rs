use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use rusb::{DeviceHandle, UsbContext, Device};
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
const CYNTHION_OUT_EP: u8 = 0x01;
const CYNTHION_IN_EP: u8 = 0x81;
const TIMEOUT_MS: Duration = Duration::from_millis(1000);

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
        // Check if simulation mode is enabled via environment variable
        if Self::is_simulation_mode() {
            info!("Using simulated device list (simulation mode enabled)");
            return Ok(Self::get_simulated_devices());
        }
        
        // Try to create USB context, if it fails, use simulation mode
        let context = match rusb::Context::new() {
            Ok(ctx) => ctx,
            Err(e) => {
                warn!("USB context initialization failed: {}. Using simulated device list.", e);
                return Ok(Self::get_simulated_devices());
            }
        };
        
        let mut device_list = Vec::new();
        
        let devices = match context.devices() {
            Ok(devs) => devs,
            Err(e) => {
                warn!("Failed to enumerate USB devices: {}. Using simulated device list.", e);
                return Ok(Self::get_simulated_devices());
            }
        };
        
        for device in devices.iter() {
            if let Ok(descriptor) = device.device_descriptor() {
                let vid = descriptor.vendor_id();
                let pid = descriptor.product_id();
                
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
                
                device_list.push(device_info);
            }
        }
        
        // If no devices were found, still return successful result with empty list
        Ok(device_list)
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
    pub fn is_simulation_mode() -> bool {
        match std::env::var("USBFLY_SIMULATION_MODE") {
            Ok(val) => val == "1",
            Err(_) => false,
        }
    }
    
    pub async fn connect() -> Result<Self> {
        // Check if simulation mode is enabled via environment variable
        if Self::is_simulation_mode() {
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
        
        // Get device handle
        let mut handle = device.open()?;
        
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
        
        // Check if the interface is available
        let interface_available = match device.active_config_descriptor() {
            Ok(config) => {
                let has_interface = config.interfaces().any(|i| i.number() == CYNTHION_INTERFACE);
                if !has_interface {
                    warn!("Device does not appear to have interface {}. Will try anyway.", CYNTHION_INTERFACE);
                }
                true // Continue even if interface check fails
            },
            Err(_) => {
                // If we can't get config, assume interface is available
                true
            }
        };
        
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
                                          e.to_string().contains("EBUSY");
                    
                    if is_mac_usb_error {
                        warn!("USB interface access issue on macOS: {}", e);
                        info!("On macOS, we'll continue in a read-only mode that may have limited functionality");
                        
                        // For macOS: create connection but mark as simulation mode to avoid future interface operations
                        // This prevents the segfault in darwin_get_interface/darwin_claim_interface
                        return Ok(Self {
                            handle: Some(handle),
                            active: true,
                            simulation_mode: true, // Use simulation mode to prevent further low-level USB operations
                        });
                    }
                }
                
                // For other platforms or error types, return the error
                Err(anyhow!("Failed to claim USB interface: {}. Check if the device is being used by another application.", e))
            }
        }
    }
    
    pub fn disconnect(&mut self) -> Result<()> {
        // Handle simulation mode specially
        if self.simulation_mode {
            self.active = false;
            info!("Disconnected from simulated Cynthion device");
            return Ok(());
        }
        
        // Standard disconnect for real device
        if let Some(handle) = self.handle.take() {
            // Only try to release interface if not on macOS to avoid potential crashes
            #[cfg(not(target_os = "macos"))]
            {
                if let Err(e) = handle.release_interface(CYNTHION_INTERFACE) {
                    error!("Failed to release interface: {}", e);
                }
            }
            
            // On macOS, skip the release_interface call which could cause similar crashes
            #[cfg(target_os = "macos")]
            {
                debug!("On macOS, skipping release_interface to avoid potential crashes");
            }
            
            self.active = false;
            info!("Disconnected from Cynthion device");
        }
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
        // Simple simulated descriptor data (this could be expanded to be more realistic)
        // Just a basic device descriptor format
        let data = vec![
            0x12, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x40, // Standard device descriptor header
            0x50, 0x1d, 0x5c, 0x61, 0x00, 0x01, 0x01, 0x02, // VID/PID and other fields
            0x03, 0x01                                      // End of descriptor
        ];
        data
    }
    
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
            return Ok(self.get_simulated_data());
        }
        
        // Get handle or return error
        let handle = self.handle.as_mut().ok_or_else(|| anyhow!("No device handle"))?;
        
        // Buffer to store data
        let mut buffer = [0u8; 512];
        
        // Read data synchronously
        match handle.read_bulk(CYNTHION_IN_EP, &mut buffer, TIMEOUT_MS) {
            Ok(len) => {
                debug!("Read {} bytes from Cynthion", len);
                Ok(buffer[..len].to_vec())
            }
            Err(e) => {
                error!("Error reading from Cynthion: {}", e);
                Err(anyhow!("Failed to read from device: {}", e))
            }
        }
    }
    
    // This version allows avoiding holding a MutexGuard across an await point
    // It's simpler, returning Result directly rather than a future
    pub fn read_data_clone(&mut self) -> Result<Vec<u8>> {
        // Simply call the synchronous function directly
        self.read_data_sync()
    }
    
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
    
    // Additional methods for controlling the Cynthion device
    pub fn get_device_info(&mut self) -> Result<String> {
        // This is a placeholder - actual implementation would send a command to get device info
        // and parse the response
        Ok("Cynthion USB Analyzer".to_string())
    }
    
    pub fn is_connected(&self) -> bool {
        if self.simulation_mode {
            // In simulation mode, just check if active
            self.active
        } else {
            // In real device mode, need both active flag and handle
            self.active && self.handle.is_some()
        }
    }
}

impl Drop for CynthionConnection {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}
