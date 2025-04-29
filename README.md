# USBfly

USBfly is a Rust-based USB analysis application for Cynthion devices with comprehensive descriptor decoding and an intuitive Mac GUI. It provides real-time monitoring and analysis of USB traffic with a focus on usability and thorough descriptor decoding.

## Features

- Connect to and communicate with Cynthion USB analysis devices
- Capture and analyze USB traffic data streams in real-time
- Decode USB descriptors completely and accurately with contextual hints
- Display decoded information with helpful cross-references and standards information
- Comprehensive reference data for vendor IDs, class codes, and descriptor types from USB.org
- Clean, intuitive GUI optimized for macOS with tabbed interface
- Save and load capture sessions for later analysis
- Export decoded information for documentation or reporting

## Screenshots

[Screenshot images will be added in future releases]

## Requirements

- macOS 10.14 or later
- Cynthion USB analysis device from Great Scott Gadgets
- USB-C cable for device connection

## Installation

### Option 1: Download the prebuilt application

1. Download the latest release from the [releases page](https://github.com/username/usbfly/releases)
2. Mount the .dmg file
3. Drag USBfly to your Applications folder

### Option 2: Build from source

1. Make sure you have Rust and Cargo installed:
   ```
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Clone the repository:
   ```
   git clone https://github.com/username/usbfly.git
   cd usbfly
   ```

3. Build the application:
   ```
   cargo build --release
   ```

4. The compiled binary will be available at `target/release/usbfly`

5. Optionally, create a macOS app bundle:
   ```
   ./package-macos.sh
   ```
   This will create a properly formatted `.app` file in the `dist` directory.

## Usage

1. Connect your Cynthion device to your computer using a USB-C cable
2. Launch USBfly
3. Click "Connect to Cynthion" in the app interface
4. Once connected, the app will automatically begin capturing USB traffic
5. Use the tabbed interface to switch between different views:
   - Devices: Shows connected USB devices and their properties
   - Traffic: Displays captured USB traffic in real-time
   - Descriptors: Shows decoded USB descriptors with detailed information

### Saving and Loading Captures

You can save the current capture session for later analysis:
1. Click "Save Capture" in the toolbar
2. Choose a location and filename (uses .usb extension)
3. To load a previous capture, click "Load Capture" and select the file

### Clearing Captures

To clear the current capture data and start fresh, click the "Clear" button in the toolbar.

## Architecture

USBfly is built with a modular architecture:

- **Cynthion Connection Module**: Manages communication with the Cynthion device
- **USB Decoder Module**: Handles parsing and decoding of USB traffic data
- **Data Reference Module**: Contains comprehensive USB standard reference data
- **GUI Module**: Implements the user interface with specific views for different aspects of USB analysis

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

[MIT License](LICENSE)

## Acknowledgments

- [Great Scott Gadgets](https://greatscottgadgets.com/) for the Cynthion USB analysis device
- The [Packetry](https://github.com/greatscottgadgets/packetry) project for inspiration
- The [USBdecoder-app](https://github.com/fuhaku/USBdecoder-app) project for descriptor decoding inspiration
- [USB.org](https://www.usb.org/) for providing the reference data used in this application
