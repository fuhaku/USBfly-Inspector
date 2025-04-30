pub mod connection;
pub mod transfer_queue;
pub mod new_connection;

// Re-export these types if they're needed elsewhere in the application
// The #[allow(unused_imports)] is temporary during our refactoring
// and will be removed when the implementation is complete
#[allow(unused_imports)]
pub use crate::usb::Speed;  

// We'll keep the re-exports but mark them as allowed
// during our transition to the new implementation
#[allow(unused_imports)]
pub use connection::CynthionConnection;
#[allow(unused_imports)]
pub use new_connection::{CynthionDevice, CynthionHandle, CynthionStream};
#[allow(unused_imports)]
pub use transfer_queue::TransferQueue;