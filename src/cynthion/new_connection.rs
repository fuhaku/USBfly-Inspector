//! Cynthion device connection handler using nusb
//! This is a clean reimplementation based on Packetry's approach

use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Result, bail, Context as AnyhowContext, Error};
use log::info;
use nusb::{
    self,
    transfer::{
        Control,
        ControlType,
        Recipient,
    },
    DeviceInfo,
    Interface,
};

use crate::cynthion::connection::Speed;

// Bitfield structures for device control
// We'll implement these manually since we're transitioning away from the bitfield crate

/// State structure for controlling Cynthion capture
struct State(u8);

impl State {
    fn new(enable: bool, speed: Speed) -> State {
        let mut value = 0u8;
        // Set enable bit (bit 0)
        if enable {
            value |= 0x01;
        }
        // Set speed bits (bits 1-2)
        value |= (speed as u8) << 1;
        State(value)
    }
}

/// TestConfig for Cynthion test device simulation
struct TestConfig(u8);

impl TestConfig {
    fn new(speed: Option<Speed>) -> TestConfig {
        let mut value = 0u8;
        if let Some(speed) = speed {
            // Set connect bit (bit 0)
            value |= 0x01;
            // Set speed bits (bits 1-2)
            value |= (speed as u8) << 1;
        }
        TestConfig(value)
    }
}
use crate::cynthion::transfer_queue::TransferQueue;

// Constants for Cynthion device connection
pub const CYNTHION_VID: u16 = 0x1d50;
pub const CYNTHION_PID: u16 = 0x615b;    // Original Cynthion firmware VID/PID
const CLASS: u8 = 0xff;                  // Vendor-specific class
const SUBCLASS: u8 = 0x10;               // USB analysis subclass
const PROTOCOL: u8 = 0x01;               // Cynthion protocol version
const ENDPOINT: u8 = 0x81;               // Bulk in endpoint for receiving data
const READ_LEN: usize = 0x4000;          // 16K buffer size for transfers
const NUM_TRANSFERS: usize = 4;          // Number of concurrent transfers
const TIMEOUT: Duration = Duration::from_millis(1000);

// Additional compatible devices
const ALT_CYNTHION_PID_1: u16 = 0x615c;
const ALT_CYNTHION_PID_2: u16 = 0x615d;
const GREATFET_ONE_PID: u16 = 0x60e6;

// Bitfield macros already defined in connection.rs

/// A Cynthion device that can be used for USB capture.
#[derive(Debug, Clone)]
pub struct CynthionDevice {
    device_info: DeviceInfo,
    interface_number: u8,
    alt_setting_number: u8,
    supported_speeds: Vec<Speed>,
}

impl CynthionDevice {
    // Check if the device is a supported Cynthion device
    pub fn is_supported(vid: u16, pid: u16) -> bool {
        (vid == CYNTHION_VID && pid == CYNTHION_PID) || // Primary Cynthion
        (vid == CYNTHION_VID && pid == ALT_CYNTHION_PID_1) || // Alt firmware version
        (vid == CYNTHION_VID && pid == ALT_CYNTHION_PID_2) || // Alt firmware version
        (vid == CYNTHION_VID && pid == GREATFET_ONE_PID)   // GreatFET One
    }

    // Find all compatible devices on the system
    pub fn find_all() -> Result<Vec<CynthionDevice>> {
        let devices = nusb::list_devices()?;
        
        let mut result = Vec::new();
        for device_info in devices {
            let device = match Self::from_device_info(device_info.clone()) {
                Some(device) => device,
                None => continue,
            };
            result.push(device);
        }
        
        Ok(result)
    }
    
    // Create from device info if compatible
    fn from_device_info(device_info: DeviceInfo) -> Option<CynthionDevice> {
        // Check if this is a supported Cynthion device
        if !Self::is_supported(device_info.vendor_id(), device_info.product_id()) {
            return None;
        }
        
        // nusb has a different API than rusb - we need to open the device to inspect interfaces
        // For now we'll use interface 0, alt 0 for Cynthion devices - we'll check this when opening
        let selected_if = 0;
        let selected_alt = 0;
        
        info!("Found potential Cynthion device: VID:{:04x} PID:{:04x}",
             device_info.vendor_id(), device_info.product_id());
        
        // Create device with empty speed list - will be populated when opened
        Some(CynthionDevice {
            device_info,
            interface_number: selected_if,
            alt_setting_number: selected_alt,
            supported_speeds: Vec::new(),
        })
    }
    
    // Open the device for communication
    pub fn open(&self) -> Result<CynthionHandle> {
        let device = self.device_info.open()?;
        
        // Attempt to claim the interface
        // In nusb, this creates an Interface object we can use
        let interface = match device.claim_interface(self.interface_number) {
            Ok(interface) => interface,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to claim interface {}: {}", 
                          self.interface_number, e));
            }
        };
        
        // Create the connection handle
        Ok(CynthionHandle {
            interface,
            device_info: self.device_info.clone(),
        })
    }
    
    // Get device information
    pub fn vendor_id(&self) -> u16 {
        self.device_info.vendor_id()
    }
    
    pub fn product_id(&self) -> u16 {
        self.device_info.product_id()
    }
    
    pub fn get_description(&self) -> String {
        let product = self.device_info.product_string()
            .unwrap_or("Unknown Device");
        
        format!("{} {:04x}:{:04x} (Interface {})", 
               product, 
               self.device_info.vendor_id(), 
               self.device_info.product_id(),
               self.interface_number)
    }
}

/// A handle to an open Cynthion device.
#[derive(Clone)]
pub struct CynthionHandle {
    interface: Interface,
    device_info: DeviceInfo,
}

impl CynthionHandle {
    // Get the supported speeds from the device
    fn speeds(&self) -> Result<Vec<Speed>> {
        use Speed::*;
        let control = Control {
            control_type: ControlType::Vendor,
            recipient: Recipient::Interface,
            request: 2,
            value: 0,
            index: self.interface.interface_number() as u16,
        };
        
        let mut buf = [0; 64];
        let timeout = Duration::from_secs(1);
        
        let size = self.interface
            .control_in_blocking(control, &mut buf, timeout)
            .context("Failed retrieving supported speeds from device")?;
            
        if size != 1 {
            bail!("Expected 1-byte response to speed request, got {size}");
        }
        
        let mut speeds = Vec::new();
        for speed in [Auto, High, Full, Low] {
            if buf[0] & speed.mask() != 0 {
                speeds.push(speed);
            }
        }
        
        Ok(speeds)
    }
    
    // Start capturing USB traffic with specified speed
    fn start_capture(&mut self, speed: Speed) -> Result<()> {
        self.write_request(1, State::new(true, speed).0)
    }
    
    // Stop capturing USB traffic
    fn stop_capture(&mut self) -> Result<()> {
        self.write_request(1, State::new(false, Speed::High).0)
    }
    
    // Configure the built-in test device (if available)
    pub fn configure_test_device(&mut self, speed: Option<Speed>) -> Result<()> {
        let test_config = TestConfig::new(speed);
        self.write_request(3, test_config.0)
            .context("Failed to set test device configuration")
    }
    
    // Helper method to send vendor requests to the device
    fn write_request(&mut self, request: u8, value: u8) -> Result<()> {
        let control = Control {
            control_type: ControlType::Vendor,
            recipient: Recipient::Interface,
            request,
            value: u16::from(value),
            index: self.interface.interface_number() as u16,
        };
        
        let timeout = Duration::from_secs(1);
        // nusb returns the number of bytes sent, but we just need to know it succeeded
        self.interface
            .control_out_blocking(control, &[], timeout)
            .map(|_| ()) // Convert Result<usize, Error> to Result<(), Error>
            .map_err(Error::from)
    }
    
    // Begin capture and return a queue for processing transfers
    pub fn begin_capture(
        &mut self,
        speed: Speed,
        data_tx: mpsc::Sender<Vec<u8>>
    ) -> Result<TransferQueue> {
        self.start_capture(speed)?;
        
        Ok(TransferQueue::new(&self.interface, data_tx,
            ENDPOINT, NUM_TRANSFERS, READ_LEN))
    }
    
    // End capture
    pub fn end_capture(&mut self) -> Result<()> {
        self.stop_capture()
    }
    
    // Get device information
    pub fn vendor_id(&self) -> u16 {
        self.device_info.vendor_id()
    }
    
    pub fn product_id(&self) -> u16 {
        self.device_info.product_id()
    }
}

// Processing stream for converting USB capture data into packets
pub struct CynthionStream {
    receiver: mpsc::Receiver<Vec<u8>>,
    buffer: VecDeque<u8>,
    padding_due: bool,
}

impl CynthionStream {
    // Create a new processing stream
    pub fn new(receiver: mpsc::Receiver<Vec<u8>>) -> CynthionStream {
        CynthionStream {
            receiver,
            buffer: VecDeque::new(),
            padding_due: false,
        }
    }
    
    // Process captured data into a formatted packet
    pub fn next_packet(&mut self) -> Option<Vec<u8>> {
        loop {
            // First check if we have a complete packet in the buffer
            if self.buffer.len() >= 4 {
                // Check if we have enough bytes for the packet including its length
                let packet_len = self.peek_packet_len();
                if self.buffer.len() >= packet_len + 4 {
                    // Extract the packet
                    return Some(self.extract_packet(packet_len));
                }
            }
            
            // We don't have a complete packet, try to get more data
            match self.receiver.try_recv() {
                Ok(data) => {
                    // Add new data to our buffer
                    self.buffer.extend(data);
                },
                Err(_) => {
                    // No more data available right now
                    return None;
                }
            }
        }
    }
    
    // Helper to peek at the packet length
    fn peek_packet_len(&self) -> usize {
        let mut len_bytes = [0u8; 4];
        for (i, &byte) in self.buffer.iter().take(4).enumerate() {
            len_bytes[i] = byte;
        }
        
        // Convert to u32 (little endian)
        u32::from_le_bytes(len_bytes) as usize
    }
    
    // Extract a complete packet from the buffer
    fn extract_packet(&mut self, packet_len: usize) -> Vec<u8> {
        // Remove the length bytes
        for _ in 0..4 {
            self.buffer.pop_front();
        }
        
        // Now extract the actual packet data
        let mut packet = Vec::with_capacity(packet_len);
        for _ in 0..packet_len {
            packet.push(self.buffer.pop_front().unwrap());
        }
        
        packet
    }
}