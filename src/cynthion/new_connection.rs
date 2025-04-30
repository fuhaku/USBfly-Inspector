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

// Import the connection module's Speed enum
use crate::cynthion::connection::Speed as ConnectionSpeed;

// Bitfield structures for device control
// We'll implement these manually since we're transitioning away from the bitfield crate

/// State structure for controlling Cynthion capture
struct State(u8);

impl State {
    fn new(enable: bool, speed: ConnectionSpeed) -> State {
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
    fn new(speed: Option<ConnectionSpeed>) -> TestConfig {
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
    supported_speeds: Vec<ConnectionSpeed>,
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
        // Check if we're in forced hardware mode
        let force_hardware = std::env::var("USBFLY_FORCE_HARDWARE")
            .unwrap_or_else(|_| "0".to_string()) == "1";
            
        if force_hardware {
            info!("Listing devices in FORCE HARDWARE mode");
        }
        
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
    
    // Find all devices with force hardware mode enabled
    pub fn find_all_force_hardware() -> Result<Vec<CynthionDevice>> {
        // Set environment variable to indicate force hardware mode
        std::env::set_var("USBFLY_FORCE_HARDWARE", "1");
        // Call regular find_all
        Self::find_all()
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
            transfer_queue: None,
        })
    }
    
    // Get device information
    pub fn vendor_id(&self) -> u16 {
        self.device_info.vendor_id()
    }
    
    pub fn product_id(&self) -> u16 {
        self.device_info.product_id()
    }

    pub fn manufacturer(&self) -> &str {
        self.device_info.manufacturer_string()
            .unwrap_or("Unknown Manufacturer")
    }
    
    pub fn product(&self) -> &str {
        self.device_info.product_string()
            .unwrap_or("Unknown Device")
    }
    
    pub fn serial_number(&self) -> &str {
        self.device_info.serial_number()
            .unwrap_or("N/A")
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
    transfer_queue: Option<TransferQueue>,
}

// Manual implementation of Debug since Interface doesn't implement Debug
impl std::fmt::Debug for CynthionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CynthionHandle")
            .field("device_info", &format!("VID:{:04x} PID:{:04x}", 
                    self.device_info.vendor_id(), 
                    self.device_info.product_id()))
            .field("interface_number", &self.interface.interface_number())
            .field("transfer_queue", &self.transfer_queue)
            .finish()
    }
}

impl CynthionHandle {
    // Get the supported speeds from the device
    #[allow(dead_code)]
    fn speeds(&self) -> Result<Vec<crate::usb::Speed>> {
        // Import the Speed enum only within this method
        use crate::usb::Speed::*;
        
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
        // Each speed corresponds to a bit in the response
        // Auto = bit 0, High = bit 1, Full = bit 2, Low = bit 3
        if buf[0] & 0x01 != 0 { speeds.push(Auto); }
        if buf[0] & 0x02 != 0 { speeds.push(High); }
        if buf[0] & 0x04 != 0 { speeds.push(Full); }
        if buf[0] & 0x08 != 0 { speeds.push(Low); }
        
        Ok(speeds)
    }
    
    // Start capturing USB traffic with specified speed
    pub fn start_capture(&mut self) -> Result<()> {
        // We'll use High speed by default for simplicity
        self.write_request(1, State::new(true, ConnectionSpeed::High).0)
    }
    
    // Stop capturing USB traffic
    pub fn stop_capture(&mut self) -> Result<()> {
        self.write_request(1, State::new(false, ConnectionSpeed::High).0)
    }
    
    // Configure the built-in test device (if available)
    pub fn configure_test_device(&mut self, speed: Option<ConnectionSpeed>) -> Result<()> {
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
        data_tx: mpsc::Sender<Vec<u8>>
    ) -> Result<TransferQueue> {
        // Default to High speed for now
        self.start_capture()?;
        
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
    
    pub fn manufacturer(&self) -> &str {
        self.device_info.manufacturer_string()
            .unwrap_or("Unknown Manufacturer")
    }
    
    pub fn product(&self) -> &str {
        self.device_info.product_string()
            .unwrap_or("Unknown Device")
    }
    
    pub fn serial_number(&self) -> &str {
        self.device_info.serial_number()
            .unwrap_or("N/A")
    }
    
    // The old direct read implementation is replaced by the version below
    // that uses TransferQueue for better performance and reliability
    fn _deprecated_read_mitm_direct(&mut self) -> Result<Vec<u8>> {
        // This is just a placeholder stub to avoid duplicate method definitions
        Ok(Vec::new())
    }
    
    // Set read timeout for bulk operations
    pub fn set_read_timeout(&mut self, _duration: Option<Duration>) -> Result<()> {
        // nusb doesn't have a direct timeout setting, we'll just store it for future use
        // This is a stub method to maintain API compatibility
        Ok(())
    }
    
    // Check if this is a simulation
    pub fn is_simulation_mode(&self) -> bool {
        // Check if we're simulating a device
        std::env::var("USBFLY_SIMULATION_MODE").unwrap_or_else(|_| "0".to_string()) == "1"
    }
    
    // Check if device is connected
    pub fn is_connected(&self) -> bool {
        // For now, if we have an interface, we're considered connected
        true
    }
    
    // Clear capture buffer (simulation only)
    pub fn clear_capture_buffer(&mut self) -> Result<()> {
        // Real hardware doesn't need to clear buffer as it streams constantly
        Ok(())
    }
    
    // Read MitM traffic using a safe clone-based approach for thread safety
    pub fn read_mitm_traffic_clone(&mut self) -> Result<Vec<u8>> {
        // If we're in simulation mode, return simulated data
        if self.is_simulation_mode() {
            return Ok(self.get_simulated_mitm_traffic());
        }
        
        // Check if we have an active transfer queue
        if self.transfer_queue.is_none() {
            // Create a channel for data transfer
            let (tx, _rx) = mpsc::channel();
            
            // Create a new transfer queue with the transmitter
            let transfer_queue = TransferQueue::new(&self.interface, tx,
                ENDPOINT, NUM_TRANSFERS, READ_LEN);
                
            // Store the transfer queue
            self.transfer_queue = Some(transfer_queue);
            
            // Start the capture
            self.start_capture()?;
            
            // Return empty data for this first call
            return Ok(Vec::new());
        }
        
        // Process any completed transfers to keep the queue moving
        if let Some(queue) = &mut self.transfer_queue {
            queue.process_completed_transfers()?;
        }
        
        // For nusb, we don't directly read from the device
        // Instead, we check if any data has been received from the transfers
        if let Some(queue) = &self.transfer_queue {
            // Check if there's data available in the transfer queue's channel
            if let Some(receiver) = queue.get_receiver() {
                match receiver.try_recv() {
                    Ok(data) => {
                        // Successfully received data from the queue
                        return Ok(data);
                    },
                    Err(mpsc::TryRecvError::Empty) => {
                        // No data available yet, return empty vector
                        return Ok(Vec::new());
                    },
                    Err(mpsc::TryRecvError::Disconnected) => {
                        // Channel is disconnected, something went wrong
                        return Err(anyhow::anyhow!("Transfer queue channel disconnected"));
                    }
                }
            } else {
                // No receiver available (this should not happen for the original connection)
                // but might happen for cloned connections
                return Ok(Vec::new());
            }
        }
        
        // Fallback if no queue is available
        Ok(Vec::new())
    }
    
    // Process raw data into USB transactions (for nusb implementation)
    pub fn process_transactions(&mut self, _data: &[u8]) -> Vec<crate::usb::mitm_traffic::UsbTransaction> {
        // For now, return an empty vector
        // This will be implemented to parse the raw data into USB transactions
        Vec::new()
    }
    
    // Get simulated MitM traffic for testing
    pub fn get_simulated_mitm_traffic(&mut self) -> Vec<u8> {
        // Create a realistic simulated USB packet
        // Format: [packet_type, endpoint, device_addr, data_len, data...]
        
        // Generate random packet type (control, bulk, interrupt)
        let packet_type = match rand::random::<u8>() % 3 {
            0 => 0xD0, // SETUP token (control transfer)
            1 => 0x90, // IN token (bulk or interrupt IN)
            _ => 0x10, // OUT token (bulk or interrupt OUT)
        };
        
        // Generate random endpoint (1-15) with direction bit
        let mut endpoint = (rand::random::<u8>() % 15) + 1;
        if packet_type == 0x90 {
            endpoint |= 0x80; // Set direction bit for IN transfers
        }
        
        // Use device address 1 or 2 for simulated devices
        let device_addr = (rand::random::<u8>() % 2) + 1;
        
        // Randomize data size (4-64 bytes)
        let data_len = (rand::random::<u8>() % 60) + 4;
        
        // Create data buffer
        let mut data = Vec::with_capacity(data_len as usize + 4);
        
        // Add header
        data.push(packet_type);
        data.push(endpoint);
        data.push(device_addr);
        data.push(data_len);
        
        // Add simulated USB data
        if packet_type == 0xD0 {
            // For SETUP packets, use standard device request format
            // bmRequestType
            data.push(0x80); // Device-to-host
            // bRequest
            data.push(0x06); // GET_DESCRIPTOR
            // wValue
            data.push(0x01); // Descriptor index
            data.push(0x00); // Descriptor type (device)
            // wIndex
            data.push(0x00);
            data.push(0x00);
            // wLength
            data.push(0x12); // 18 bytes (standard device descriptor length)
            data.push(0x00);
        } else {
            // For other packets, generate random data
            for _ in 0..(data_len as usize) {
                data.push(rand::random::<u8>());
            }
        }
        
        // Return the packet
        data
    }
    
    // Process MitM traffic into transactions
    pub fn process_mitm_traffic(&mut self, data: &[u8]) -> Vec<crate::usb::mitm_traffic::UsbTransaction> {
        use crate::usb::mitm_traffic::{UsbTransaction, UsbTransferType, UsbDirection};
        
        // If data is empty, return empty vector
        if data.is_empty() {
            return Vec::new();
        }
        
        // For our nusb implementation, properly process the data
        let mut transactions = Vec::new();
        
        // Ensure we have enough data for at least one transaction (minimum 8 bytes)
        // Real implementation would use a more robust parsing approach
        if data.len() < 8 {
            return Vec::new();
        }
        
        // Process data into packets first
        let mut offset = 0;
        while offset + 8 <= data.len() {
            // Read packet header (simplified for this implementation)
            let packet_type = data[offset];
            let endpoint = data[offset + 1];
            let direction = if endpoint & 0x80 != 0 { 
                UsbDirection::DeviceToHost 
            } else { 
                UsbDirection::HostToDevice 
            };
            
            // Extract device address
            let device_addr = data[offset + 2];
            
            // Identify transfer type based on endpoint number (simplified)
            let transfer_type = match endpoint & 0x03 {
                0 => UsbTransferType::Control,
                1 => UsbTransferType::Isochronous,
                2 => UsbTransferType::Bulk,
                3 => UsbTransferType::Interrupt,
                _ => UsbTransferType::Control, // Default fallback
            };
            
            // Calculate data length - this would be more complex in real parsing
            let mut data_len = 4; // Default to a minimum length
            if offset + 3 < data.len() {
                data_len = data[offset + 3] as usize;
                // Ensure we don't exceed buffer boundaries
                if offset + 4 + data_len > data.len() {
                    data_len = data.len() - offset - 4;
                }
            }
            
            // Extract data payload
            let payload = if offset + 4 + data_len <= data.len() {
                data[offset+4..offset+4+data_len].to_vec()
            } else {
                Vec::new()
            };
            
            // Generate a unique ID for this transaction
            // In a real implementation, this would come from the device
            let id = transactions.len() as u32 + 1;
            
            // Create a data packet for this transaction
            let data_packet = crate::usb::mitm_traffic::UsbDataPacket::new(
                payload, 
                direction,
                endpoint & 0x7F // Remove direction bit
            );
            
            // Create a status packet (assume success)
            let status_packet = crate::usb::mitm_traffic::UsbStatusPacket {
                status: crate::usb::mitm_traffic::UsbTransferStatus::ACK,
                endpoint: endpoint & 0x7F,
            };
            
            // Create the transaction
            let transaction = UsbTransaction::new(id.into(), std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64());
                
            // Update the transaction with our data
            let mut transaction = UsbTransaction {
                id: id.into(), // Convert to u64
                transfer_type,
                setup_packet: None, // We'd parse this from the data for control transfers
                data_packet: Some(data_packet),
                status_packet: Some(status_packet),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64(),
                device_address: device_addr,
                endpoint: endpoint & 0x7F, // Remove direction bit
                fields: {
                    let mut fields = std::collections::HashMap::new();
                    fields.insert("speed".to_string(), "High".to_string());
                    fields.insert("packet_type".to_string(), format!("0x{:02X}", packet_type));
                    fields
                },
            };
            
            // If this is a setup packet, create a setup packet structure
            if packet_type == 0xD0 {
                // Add a basic setup packet - in a real implementation we'd parse the data
                // Create a minimal setup packet with just the required fields
                transaction.setup_packet = Some(crate::usb::mitm_traffic::UsbSetupPacket {
                    bmRequestType: 0,  // Default
                    bRequest: 0,       // Default
                    wValue: 0,         // Default
                    wIndex: 0,         // Default
                    wLength: 0,        // Default
                    direction: direction,
                    request_type: crate::usb::mitm_traffic::UsbControlRequestType::Standard,
                    recipient: crate::usb::mitm_traffic::UsbControlRecipient::Device,
                    standard_request: None,
                    request_description: "Unknown Request".to_string(),
                });
            }
            
            // Fields are already added during initialization
            
            transactions.push(transaction);
            
            // Move to next packet
            offset += 4 + data_len;
        }
        
        transactions
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