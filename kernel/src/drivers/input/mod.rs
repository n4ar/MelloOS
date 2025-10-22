// Input device drivers module

pub mod keyboard;

// Re-export keyboard driver
pub use keyboard::KEYBOARD_DRIVER;
