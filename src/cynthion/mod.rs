pub mod connection;
pub mod transfer_queue;
pub mod new_connection;

// We'll transition to the new implementation
// Old connection module will remain for backward compatibility 
// until the migration is complete
pub use connection::CynthionConnection;  // Keep old connection temporarily
pub use connection::Speed;  // Share Speed enum

// Also expose new API that we're transitioning to
pub use new_connection::{CynthionDevice, CynthionHandle, CynthionStream};