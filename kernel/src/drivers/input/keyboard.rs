// PS/2 Keyboard Driver
//
// This driver implements support for PS/2 keyboards, handling scancode
// translation and providing a buffered interface for keyboard input.

use crate::drivers::{Device, Driver, DriverError};
use crate::io::irq::register_irq_handler;
use crate::io::port::{inb, outb};
use crate::sync::SpinLock;

/// PS/2 keyboard data port (read scancodes)
const KBD_DATA_PORT: u16 = 0x60;

/// PS/2 keyboard status port (read status)
const KBD_STATUS_PORT: u16 = 0x64;

/// PS/2 keyboard command port (write commands)
const KBD_COMMAND_PORT: u16 = 0x64;

/// Keyboard buffer size (circular buffer)
const BUFFER_SIZE: usize = 256;

/// Circular buffer for keyboard input
static KEYBOARD_BUFFER: SpinLock<[u8; BUFFER_SIZE]> = SpinLock::new([0; BUFFER_SIZE]);

/// Buffer head index (write position)
static BUFFER_HEAD: SpinLock<usize> = SpinLock::new(0);

/// Buffer tail index (read position)
static BUFFER_TAIL: SpinLock<usize> = SpinLock::new(0);

/// Scancode to ASCII translation table for US keyboard layout
/// Index is the scancode, value is the ASCII character
/// 0 means no translation (special key, modifier, etc.)
static SCANCODE_TO_ASCII: [u8; 128] = [
    // 0x00-0x0F
    0, 27, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'=', 8, b'\t',
    // 0x10-0x1F: QWERTY row
    b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'[', b']', b'\n', 0, b'a', b's',
    // 0x20-0x2F: ASDF row continued + ZXCV row
    b'd', b'f', b'g', b'h', b'j', b'k', b'l', b';', b'\'', b'`', 0, b'\\', b'z', b'x', b'c', b'v',
    // 0x30-0x3F: ZXCV row continued + space
    b'b', b'n', b'm', b',', b'.', b'/', 0, b'*', 0, b' ', 0, 0, 0, 0, 0, 0,
    // 0x40-0x4F: Function keys
    0, 0, 0, 0, 0, 0, 0, b'7', b'8', b'9', b'-', b'4', b'5', b'6', b'+', b'1',
    // 0x50-0x5F: Keypad continued
    b'2', b'3', b'0', b'.', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0x60-0x6F: Extended keys
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0x70-0x7F: Extended keys
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// Probe function to match ps2-keyboard devices
pub fn keyboard_probe(device: &Device) -> bool {
    device.name == "ps2-keyboard"
}

/// Initialize the PS/2 keyboard driver
pub fn keyboard_init(_device: &Device) -> Result<(), DriverError> {
    crate::log_info!("KEYBOARD", "Initializing PS/2 keyboard driver");

    // Register IRQ handler for keyboard interrupt (IRQ 1)
    register_irq_handler(1, keyboard_irq_handler).map_err(|e| {
        crate::log_error!("KEYBOARD", "Failed to register keyboard IRQ handler: {}", e);
        DriverError::InitFailure
    })?;

    // Enable the first PS/2 port (keyboard)
    unsafe {
        outb(KBD_COMMAND_PORT, 0xAE);
    }

    crate::log_info!("KEYBOARD", "PS/2 keyboard initialized successfully");
    Ok(())
}

/// Shutdown the PS/2 keyboard driver
pub fn keyboard_shutdown(_device: &Device) -> Result<(), DriverError> {
    crate::log_info!("KEYBOARD", "Shutting down PS/2 keyboard driver");

    // Unregister IRQ handler
    crate::io::irq::unregister_irq_handler(1);

    // Clear the buffer
    let mut head = BUFFER_HEAD.lock();
    let mut tail = BUFFER_TAIL.lock();
    *head = 0;
    *tail = 0;

    crate::log_info!("KEYBOARD", "PS/2 keyboard shutdown complete");
    Ok(())
}

/// IRQ handler for keyboard interrupts
/// Called when a key is pressed or released
fn keyboard_irq_handler() {
    unsafe {
        // Read scancode from keyboard data port
        let scancode = inb(KBD_DATA_PORT);

        // Ignore key release events (high bit set)
        if scancode & 0x80 != 0 {
            return;
        }

        // Translate scancode to ASCII
        if let Some(&ascii) = SCANCODE_TO_ASCII.get(scancode as usize) {
            if ascii != 0 {
                // Add to circular buffer
                let mut head = BUFFER_HEAD.lock();
                let tail = BUFFER_TAIL.lock();
                let mut buffer = KEYBOARD_BUFFER.lock();

                let next_head = (*head + 1) % BUFFER_SIZE;

                // Check if buffer is full
                if next_head != *tail {
                    buffer[*head] = ascii;
                    *head = next_head;
                }
                // If buffer is full, silently drop the character
            }
        }
    }
}

/// Read a character from the keyboard buffer (non-blocking)
/// Returns Some(char) if a character is available, None otherwise
pub fn keyboard_read() -> Option<u8> {
    let head = BUFFER_HEAD.lock();
    let mut tail = BUFFER_TAIL.lock();
    let buffer = KEYBOARD_BUFFER.lock();

    if *head == *tail {
        // Buffer is empty
        None
    } else {
        // Read character from tail
        let ch = buffer[*tail];
        *tail = (*tail + 1) % BUFFER_SIZE;
        Some(ch)
    }
}

/// Keyboard driver constant for registration
pub const KEYBOARD_DRIVER: Driver = Driver {
    name: "ps2-keyboard",
    probe: keyboard_probe,
    init: keyboard_init,
    shutdown: keyboard_shutdown,
};
