use anyhow::{anyhow, Result};
use log::{debug, error, info};
use rusb::{DeviceHandle, UsbContext};
use std::time::Duration;
use tokio::time::sleep;

// Constants for Cynthion device
const CYNTHION_VID: u16 = 0x1d50;
const CYNTHION_PID: u16 = 0x615c;
const CYNTHION_INTERFACE: u8 = 0;
const CYNTHION_OUT_EP: u8 = 0x01;
const CYNTHION_IN_EP: u8 = 0x81;
const TIMEOUT_MS: Duration = Duration::from_millis(1000);

#[derive(Debug)]
pub struct CynthionConnection {
    handle: Option<DeviceHandle<rusb::Context>>,
    active: bool,
}

impl CynthionConnection {
    pub async fn connect() -> Result<Self> {
        let context = rusb::Context::new()?;
        
        // Find Cynthion device
        let devices = context.devices()?;
        let device = devices
            .iter()
            .find(|device| {
                if let Ok(descriptor) = device.device_descriptor() {
                    return descriptor.vendor_id() == CYNTHION_VID && descriptor.product_id() == CYNTHION_PID;
                }
                false
            })
            .ok_or_else(|| anyhow!("Cynthion device not found"))?;
        
        // Get device handle
        let mut handle = device.open()?;
        
        // Claim interface
        #[cfg(not(target_os = "windows"))]
        {
            handle.reset()?;
        }
        
        #[cfg(unix)]
        {
            handle.set_auto_detach_kernel_driver(true)?;
        }
        
        handle.claim_interface(CYNTHION_INTERFACE)?;
        
        info!("Connected to Cynthion device");
        
        Ok(Self {
            handle: Some(handle),
            active: true,
        })
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
