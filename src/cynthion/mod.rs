// We're now using the new connection implementation based on nusb
// The old connection module is kept for reference but marked as deprecated
#[deprecated(since = "0.1.0", note = "Use the new connection module instead")]
pub mod connection;

// New modules for our nusb implementation
pub mod transfer_queue;
pub mod new_connection;

// These are the primary types that should be used by the application
pub use new_connection::{CynthionDevice, CynthionHandle};