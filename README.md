# USBfly - USB Analyzer for Cynthion

USBfly is a comprehensive USB analysis application designed specifically for the [Cynthion](https://greatscottgadgets.com/cynthion/) USB test instrument from Great Scott Gadgets. It provides detailed USB descriptor decoding, packet analysis, and an intuitive macOS-oriented graphical interface.

## Features

- **USB Descriptor Decoding**: Complete and detailed decoding of all USB descriptor types with contextual hints
- **Device Detection**: Automatic detection and connection to Cynthion devices
- **Elegant UI**: Modern, intuitive macOS-styled interface
- **Real-time Analysis**: Monitor USB traffic in real-time
- **Export Capabilities**: Save and share your analysis results
- **Simulation Mode**: Test and explore the application without a physical Cynthion device

## Installation

### macOS

1. Download the latest release `.dmg` file from the [Releases](https://github.com/greatscottgadgets/usbfly/releases) page.
2. Open the `.dmg` file and drag USBfly to your Applications folder.
3. Launch USBfly from your Applications folder.

### Building from Source

USBfly is written in Rust. To build from source:

1. Ensure you have Rust and Cargo installed (https://www.rust-lang.org/tools/install)
2. Clone this repository: `git clone https://github.com/greatscottgadgets/usbfly.git`
3. Navigate to the repository: `cd usbfly`
4. Build the release version: `cargo build --release`
5. The binary will be available at `target/release/usbfly`

#### Building macOS App Bundle

On macOS, you can create a proper app bundle:

```bash
./package-macos.sh
```

This will create a `.app` bundle in `target/release/bundle/macos/` and a `.dmg` file if the necessary tools are available.

## Usage

1. Connect your Cynthion device to your computer via USB
2. Launch USBfly
3. The application will automatically detect your Cynthion device
4. Select your device from the device list
5. View USB descriptors and real-time data

### Simulation Mode

If you don't have a Cynthion device, you can use simulation mode:

1. Launch USBfly
2. The simulation mode is enabled by default when no device is connected
3. Explore the UI and features with simulated data

## Requirements

- macOS 10.15 (Catalina) or later
- Cynthion USB test instrument (for real device mode)

## Development

USBfly is built with:

- Rust programming language
- iced GUI library for native UI
- rusb for USB communication

### Project Structure

- `src/`: Source code
  - `cynthion/`: Cynthion device connection and communication
  - `usb/`: USB protocol parsing and analysis
  - `gui/`: User interface components
- `assets/`: Application resources
- `package-macos.sh`: macOS packaging script

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Great Scott Gadgets for the amazing Cynthion hardware
- The USB-IF for USB specifications and documentation
- The Rust community for excellent libraries and tools