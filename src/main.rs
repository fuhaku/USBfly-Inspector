mod app;
mod cynthion;
mod data;
mod gui;
mod usb;

use app::USBflyApp;
use iced::{Application, Settings};
use log::info;

fn main() -> iced::Result {
    // Initialize logger
    pretty_env_logger::init();
    info!("Starting USBfly application");

    // Run the application
    USBflyApp::run(Settings {
        id: Some(String::from("com.usbfly.app")),
        window: iced::window::Settings {
            size: (1024, 768),
            min_size: Some((800, 600)),
            max_size: None,
            resizable: true,
            decorations: true,
            transparent: false,
            position: iced::window::Position::Centered,
            // always_on_top is not available in this version of iced
            ..Default::default()
        },
        default_font: iced::Font::DEFAULT, // Use the default font instead of None
        default_text_size: 16.0,
        antialiasing: true,
        exit_on_close_request: true,
        ..Default::default()
    })
}
