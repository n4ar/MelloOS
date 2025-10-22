// UART16550 serial port driver for COM1
// This is the driver subsystem's serial driver, separate from the early boot serial.rs

use crate::io::port::{inb, outb};
use crate::drivers::{Driver, Device, DriverError};
use crate::sync::SpinLock;

const COM1_PORT: u16 = 0x3F8;

static SERIAL_PORT: SpinLock<Option<SerialPort>> = SpinLock::new(None);

/// Serial port structure
pub struct SerialPort {
    base: u16,
}

impl SerialPort {
    /// Create a new serial port instance
    fn new(base: u16) -> Self {
        SerialPort { base }
    }
    
    /// Initialize the UART (38400 baud, 8N1)
    fn init(&self) {
        unsafe {
            // Disable interrupts
            outb(self.base + 1, 0x00);
            
            // Enable DLAB (set baud rate divisor)
            outb(self.base + 3, 0x80);
            
            // Set divisor to 3 (38400 baud)
            outb(self.base + 0, 0x03);
            outb(self.base + 1, 0x00);
            
            // 8 bits, no parity, one stop bit (8N1)
            outb(self.base + 3, 0x03);
            
            // Enable FIFO, clear them, with 14-byte threshold
            outb(self.base + 2, 0xC7);
            
            // IRQs enabled, RTS/DSR set
            outb(self.base + 4, 0x0B);
        }
    }
    
    /// Write a byte to the serial port
    fn write_byte(&self, byte: u8) {
        unsafe {
            // Wait for transmit buffer to be empty
            while (inb(self.base + 5) & 0x20) == 0 {}
            outb(self.base, byte);
        }
    }
    
    /// Read a byte from the serial port (non-blocking)
    fn read_byte(&self) -> Option<u8> {
        unsafe {
            // Check if data is available
            if (inb(self.base + 5) & 0x01) != 0 {
                Some(inb(self.base))
            } else {
                None
            }
        }
    }
}

/// Probe function to match serial devices
pub fn serial_probe(device: &Device) -> bool {
    device.name == "serial-com1"
}

/// Initialize the serial driver
pub fn serial_init(_device: &Device) -> Result<(), DriverError> {
    crate::log_info!("SERIAL", "Initializing UART16550 serial driver");
    
    let port = SerialPort::new(COM1_PORT);
    port.init();
    
    let mut serial = SERIAL_PORT.lock();
    *serial = Some(port);
    
    crate::log_info!("SERIAL", "Serial port COM1 initialized");
    Ok(())
}

/// Shutdown the serial driver
pub fn serial_shutdown(_device: &Device) -> Result<(), DriverError> {
    crate::log_info!("SERIAL", "Shutting down serial port");
    let mut serial = SERIAL_PORT.lock();
    *serial = None;
    Ok(())
}

/// Write a byte to serial port
pub fn serial_write(byte: u8) {
    if let Some(port) = SERIAL_PORT.lock().as_ref() {
        port.write_byte(byte);
    }
}

/// Write a string to serial port
pub fn serial_write_str(s: &str) {
    for byte in s.bytes() {
        serial_write(byte);
    }
}

/// Read a byte from serial port (non-blocking)
pub fn serial_read() -> Option<u8> {
    SERIAL_PORT.lock().as_ref().and_then(|port| port.read_byte())
}

/// Serial driver constant for registration
pub const SERIAL_DRIVER: Driver = Driver {
    name: "uart16550",
    probe: serial_probe,
    init: serial_init,
    shutdown: serial_shutdown,
};
