use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

// Track some basic statistics
static mut VISITOR_COUNT: AtomicUsize = AtomicUsize::new(0);

fn main() {
    println!("Starting USBfly HTTP server on port 5000");
    
    match TcpListener::bind("0.0.0.0:5000") {
        Ok(listener) => {
            println!("USBfly HTTP server ready on port 5000");
            
            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        // Increment visitor count
                        let count = unsafe { VISITOR_COUNT.fetch_add(1, Ordering::SeqCst) + 1 };
                        
                        // Read the request to determine the path
                        let mut buffer = [0; 512];
                        let _ = stream.read(&mut buffer);
                        let request = String::from_utf8_lossy(&buffer[..]);
                        let path = request.lines().next().unwrap_or("").split_whitespace().nth(1).unwrap_or("/");
                        
                        // Get current timestamp
                        let timestamp = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        
                        // Generate HTML response based on path
                        let response = match path {
                            "/status" => format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                                <html><body>\
                                <h1>USBfly Status</h1>\
                                <p>Server uptime: {} seconds</p>\
                                <p>Visitors: {}</p>\
                                <p><a href=\"/\">Back to home</a></p>\
                                </body></html>",
                                timestamp, count
                            ),
                            "/about" => String::from(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                                <html><body>\
                                <h1>About USBfly</h1>\
                                <p>USBfly is a Rust-based USB analysis application for Cynthion devices.</p>\
                                <p>It provides comprehensive USB descriptor decoding and an intuitive interface.</p>\
                                <p>Features:</p>\
                                <ul>\
                                  <li>Advanced USB protocol handling</li>\
                                  <li>Comprehensive descriptor decoding</li>\
                                  <li>Simulation mode for development</li>\
                                  <li>Cross-platform compatibility</li>\
                                </ul>\
                                <p><a href=\"/\">Back to home</a></p>\
                                </body></html>"
                            ),
                            _ => String::from(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                                <html><body style=\"font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px;\">\
                                <h1 style=\"color: #2a5db0;\">USBfly Application</h1>\
                                <p>The USBfly application is running in the background.</p>\
                                <p>This is a native Rust application with a GUI that's being displayed via Replit.</p>\
                                <hr style=\"margin: 20px 0;\">\
                                <h2>What is USBfly?</h2>\
                                <p>USBfly is a Rust-based USB analysis application for Cynthion devices, providing comprehensive USB descriptor decoding and an intuitive interface.</p>\
                                <div style=\"background-color: #f5f5f5; padding: 15px; border-left: 4px solid #2a5db0; margin: 15px 0;\">\
                                  <h3>Key Features:</h3>\
                                  <ul>\
                                    <li>Real-time USB traffic analysis</li>\
                                    <li>Comprehensive descriptor decoding</li>\
                                    <li>Support for Cynthion USB analyzer devices</li>\
                                    <li>Simulation mode for development environments</li>\
                                  </ul>\
                                </div>\
                                <h3>Navigation:</h3>\
                                <ul>\
                                  <li><a href=\"/about\">About USBfly</a></li>\
                                  <li><a href=\"/status\">Server Status</a></li>\
                                </ul>\
                                <hr style=\"margin: 20px 0;\">\
                                <footer style=\"font-size: 0.8em; color: #666;\">\
                                  <p>USBfly - Rust-based USB Analysis Tool</p>\
                                </footer>\
                                </body></html>"
                            ),
                        };
                        
                        if let Err(e) = stream.write_all(response.as_bytes()) {
                            eprintln!("Failed to send HTTP response: {}", e);
                        }
                    }
                    Err(e) => eprintln!("Connection failed: {}", e),
                }
            }
        }
        Err(e) => {
            eprintln!("Could not bind to port 5000: {}", e);
            std::process::exit(1);
        }
    }
}