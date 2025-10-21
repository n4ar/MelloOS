/// Serial port driver for debugging output
/// Provides simple serial communication for kernel debugging
use core::fmt;
use spin::Mutex;
use x86_64::instructions::port::Port;

/// COM1 serial port base address
const SERIAL_PORT: u16 = 0x3F8;

/// Global serial port instance
pub static SERIAL: Mutex<SerialPort> = Mutex::new(SerialPort::new(SERIAL_PORT));

/// Serial port structure
pub struct SerialPort {
    base: u16,
}

impl SerialPort {
    /// Create a new serial port instance
    pub const fn new(port: u16) -> Self {
        Self { base: port }
    }

    /// Initialize the serial port
    pub fn init(&mut self) {
        unsafe {
            // Disable interrupts
            Port::new(self.base + 1).write(0x00u8);
            // Enable DLAB
            Port::new(self.base + 3).write(0x80u8);
            // Set divisor to 3 (38400 baud)
            Port::new(self.base + 0).write(0x03u8);
            Port::new(self.base + 1).write(0x00u8);
            // 8 bits, no parity, one stop bit
            Port::new(self.base + 3).write(0x03u8);
            // Enable FIFO
            Port::new(self.base + 2).write(0xC7u8);
            // Mark data terminal ready
            Port::new(self.base + 4).write(0x0Bu8);
        }
    }

    /// Write a byte to the serial port
    pub fn write_byte(&mut self, byte: u8) {
        unsafe {
            // Wait for transmit buffer to be empty
            let mut line_status = Port::<u8>::new(self.base + 5);
            while line_status.read() & 0x20 == 0 {}

            Port::new(self.base).write(byte);
        }
    }

    /// Write a string to the serial port
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

/// Print to serial port (for debugging)
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*))
    };
}

/// Print to serial port with newline (for debugging)
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    SERIAL.lock().write_fmt(args).unwrap();
}
