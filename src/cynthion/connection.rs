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
// Development/Test device IDs
const TEST_VID: u16 = 0x1d50; // Default GreatFET VID
const TEST_PID: u16 = 0x60e6; // GreatFET PID
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
}

impl CynthionConnection {
    // Get a list of all connected USB devices
    pub fn list_devices() -> Result<Vec<USBDeviceInfo>> {
        let context = rusb::Context::new()?;
        let mut device_list = Vec::new();
        
        let devices = context.devices()?;
        for device in devices.iter() {
            if let Ok(descriptor) = device.device_descriptor() {
                let vid = descriptor.vendor_id();
                let pid = descriptor.product_id();
                
                // Create a temporary handle to get string descriptors
                let device_info = if let Ok(mut handle) = device.open() {
                    let timeout = Duration::from_millis(100);
                    // Use default language (usually English/US)
                    let language = rusb::Language::get_english();
                    
                    // Get string descriptors (in a way that handles errors gracefully)
                    let manufacturer = descriptor.manufacturer_string_index()
                        .and_then(|idx| handle.read_manufacturer_string(language, &descriptor, timeout).ok());
                    
                    let product = descriptor.product_string_index()
                        .and_then(|idx| handle.read_product_string(language, &descriptor, timeout).ok());
                    
                    let serial = descriptor.serial_number_string_index()
                        .and_then(|idx| handle.read_serial_number_string(language, &descriptor, timeout).ok());
                    
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
        
        Ok(device_list)
    }
    
    // Check if a device is a supported analyzer
    fn is_supported_device(vid: u16, pid: u16) -> bool {
        // Check for Cynthion
        if vid == CYNTHION_VID && pid == CYNTHION_PID {
            return true;
        }
        
        // Check for test/development devices
        if vid == TEST_VID && pid == TEST_PID {
            return true;
        }
        
        // Check for other supported devices
        if vid == GADGETCAP_VID && pid == GADGETCAP_PID {
            return true;
        }
        
        false
    }
    
    pub async fn connect() -> Result<Self> {
        let context = rusb::Context::new()?;
        
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
            let language = rusb::Language::get_english();
            let product_name = descriptor.product_string_index()
                .and_then(|idx| handle.read_product_string(language, &descriptor, timeout).ok())
                .unwrap_or_else(|| "Unknown Device".to_string());
                
            info!("Connecting to {} (VID:{:04x}, PID:{:04x})", 
                product_name, descriptor.vendor_id(), descriptor.product_id());
        }
        
        // Claim interface
        #[cfg(not(target_os = "windows"))]
        {
            if let Err(e) = handle.reset() {
                warn!("Could not reset device: {}", e);
                // Continue anyway
            }
        }
        
        #[cfg(unix)]
        {
            if let Err(e) = handle.set_auto_detach_kernel_driver(true) {
                warn!("Could not set kernel driver auto-detach: {}", e);
                // Continue anyway, this might be expected on some systems
            }
        }
        
        match handle.claim_interface(CYNTHION_INTERFACE) {
            Ok(_) => {
                info!("Successfully claimed interface {}", CYNTHION_INTERFACE);
                Ok(Self {
                    handle: Some(handle),
                    active: true,
                })
            },
            Err(e) => {
                error!("Failed to claim interface: {}", e);
                Err(anyhow!("Failed to claim USB interface: {}. Check if the device is being used by another application.", e))
            }
        }
    }
    
    pub fn disconnect(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            if let Err(e) = handle.release_interface(CYNTHION_INTERFACE) {
                error!("Failed to release interface: {}", e);
            }
            self.active = false;
            info!("Disconnected from Cynthion device");
        }
        Ok(())
    }
    
    pub async fn read_data(&mut self) -> Result<Vec<u8>> {
        if !self.active {
            return Err(anyhow!("Not connected"));
        }
        
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
        self.active && self.handle.is_some()
    }
}

impl Drop for CynthionConnection {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}
