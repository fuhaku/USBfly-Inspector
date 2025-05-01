# USBfly Improvements

## Fixed Issues

1. Fixed critical issue causing disconnects when clicking Start Capture
   - Ensured consistent usage of connection handles (cynthion_handle and connection)
   - Synchronized handle usage across all connection-related operations
   - Added debug logging for handle state tracking

2. Enhanced UI Experience
   - Made device list scrollable with a fixed height (200px)
   - Made device details scrollable with a fixed height (250px)
   - Ensured USB speed selection is always visible for connected devices
   - Added better debug logging for speed selection visibility

## Key Modified Files

- **src/app.rs**: Added handle synchronization for all connection operations (Start/Stop/Clear/Fetch)
- **src/cynthion/connection.rs**: Improved connection state management
- **src/gui/views/device_view.rs**: Enhanced UI with scrollable views and always-visible speed selection
- **src/cynthion/new_connection.rs**: Fixed handle usage consistency
- **src/cynthion/device_detector.rs**: Improved device detection and error handling

## All Modified Files

- src/app.rs
- src/cynthion/connection.rs
- src/cynthion/device_detector.rs
- src/cynthion/mod.rs
- src/cynthion/new_connection.rs
- src/cynthion/transfer_queue.rs
- src/gui/styles.rs
- src/gui/views/descriptor_view.rs
- src/gui/views/device_view.rs
- src/gui/views/traffic_view.rs
- src/main.rs
- src/usb/decoder.rs
- src/usb/descriptors.rs
- src/usb/mitm_traffic.rs
- src/usb/mod.rs
- src/usb/packet_types.rs
