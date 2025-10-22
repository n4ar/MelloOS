// Serial driver module

pub mod uart16550;

// Re-export public API
pub use uart16550::{serial_write, serial_write_str, serial_read, SERIAL_DRIVER};
