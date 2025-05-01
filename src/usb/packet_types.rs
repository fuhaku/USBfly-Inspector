// Define all known packet types used by Cynthion devices
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PacketType {
    Standard = 0x00,
    Control = 0x01,
    Status = 0x04,
    Transaction = 0x24,
    Transfer = 0x1C,
    Data = 0x5A,
    Endpoint = 0x69,
    // Additional types can be added as discovered
}

/// Recognize a packet type from its type byte
pub fn recognize_packet_type(packet_type: u8) -> Option<PacketType> {
    match packet_type {
        0x00 => Some(PacketType::Standard),
        0x01 => Some(PacketType::Control),
        0x04 => Some(PacketType::Status),
        0x24 => Some(PacketType::Transaction),
        0x1C => Some(PacketType::Transfer),
        0x5A => Some(PacketType::Data),
        0x69 => Some(PacketType::Endpoint),
        // Add other packet types as needed
        _ => None,
    }
}

/// Represents standard control requests for Cynthion devices
#[allow(dead_code)]
pub enum ControlRequest {
    StopCapture = 1,
    EnableDeviceDetection = 2,
    SetMonitoringMode = 3,
    StartCapture = 4,
    FullDeviceReset = 255,
}

/// Represents speed settings for Cynthion commands
#[allow(dead_code)]
pub enum SpeedValue {
    HighSpeed = 0x03,
    FullSpeed = 0x02,
    LowSpeed = 0x01,
    // AutoSpeed = 0x00 variant removed to require explicit speed selection
}

/// Represents device detection settings
#[allow(dead_code)]
pub enum DeviceDetectionValue {
    Enable = 0x01,
    Disable = 0x00,
}

/// Represents monitoring mode settings
#[allow(dead_code)]
pub enum MonitoringModeValue {
    Passive = 0x00,
    Active = 0x01,
    Full = 0x03,
}
