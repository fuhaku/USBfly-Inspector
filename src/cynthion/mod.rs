// We're now using the new connection implementation based on nusb
// The old connection module is kept for reference but marked as deprecated
#[deprecated(since = "0.1.0", note = "Use the new connection module instead")]
pub mod connection;

// New modules for our nusb implementation
pub mod transfer_queue;
pub mod new_connection;

// Re-export the new connection types as our public API
pub use crate::usb::Speed;  

// Re-export the new implementation types directly at the module level
// We'll deprecate the old connection for now but keep it available
#[deprecated(since = "0.1.0", note = "Use CynthionDevice and CynthionHandle instead")]
pub use connection::CynthionConnection;

// These are the primary types that should be used by the application
pub use new_connection::{CynthionDevice, CynthionHandle, CynthionStream};
pub use transfer_queue::TransferQueue;