//! PTY (Pseudo-Terminal) Subsystem
//!
//! This module implements pseudo-terminal pairs for terminal emulation.
//! A PTY consists of a master side (used by terminal emulator) and a slave side
//! (used by shell/applications).

/// PTY pair number type
pub type PtyNumber = u32;

/// Maximum number of PTY pairs
pub const MAX_PTY_PAIRS: usize = 256;

/// Ring buffer size for PTY data (4KB)
const PTY_BUFFER_SIZE: usize = 4096;

/// Termios input flags (c_iflag)
pub mod iflag {
    /// Map CR to NL on input
    pub const ICRNL: u32 = 0x0000_0100;
    /// Map NL to CR on input
    pub const INLCR: u32 = 0x0000_0040;
    /// Enable XON/XOFF flow control on output
    pub const IXON: u32 = 0x0000_0400;
    /// Enable XON/XOFF flow control on input
    pub const IXOFF: u32 = 0x0000_1000;
}

/// Termios output flags (c_oflag)
pub mod oflag {
    /// Enable output processing
    pub const OPOST: u32 = 0x0000_0001;
    /// Map NL to CR-NL on output
    pub const ONLCR: u32 = 0x0000_0004;
}

/// Termios control flags (c_cflag)
pub mod cflag {
    // Placeholder for future use (baud rate, character size, etc.)
}

/// Termios local flags (c_lflag)
pub mod lflag {
    /// Enable canonical mode (line buffering)
    pub const ICANON: u32 = 0x0000_0002;
    /// Echo input characters
    pub const ECHO: u32 = 0x0000_0008;
    /// Enable signals (SIGINT, SIGTSTP, SIGQUIT)
    pub const ISIG: u32 = 0x0000_0001;
}

/// Control character indices in c_cc array
pub mod cc {
    /// Interrupt character (Ctrl-C)
    pub const VINTR: usize = 0;
    /// Suspend character (Ctrl-Z)
    pub const VSUSP: usize = 10;
    /// End-of-file character (Ctrl-D)
    pub const VEOF: usize = 4;
    /// Erase character (Backspace)
    pub const VERASE: usize = 2;
    /// Minimum characters for non-canonical read
    pub const VMIN: usize = 6;
    /// Timeout for non-canonical read (deciseconds)
    pub const VTIME: usize = 5;
    /// Quit character (Ctrl-\)
    pub const VQUIT: usize = 1;
}

/// Terminal I/O settings structure
///
/// Controls terminal behavior including input/output processing,
/// canonical vs raw mode, echo, and signal generation.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Termios {
    /// Input mode flags
    pub c_iflag: u32,
    /// Output mode flags
    pub c_oflag: u32,
    /// Control mode flags
    pub c_cflag: u32,
    /// Local mode flags
    pub c_lflag: u32,
    /// Control characters
    pub c_cc: [u8; 32],
}

impl Termios {
    /// Create a new Termios with default settings
    ///
    /// Default settings:
    /// - Canonical mode (ICANON)
    /// - Echo enabled (ECHO)
    /// - Signals enabled (ISIG)
    /// - CR to NL mapping (ICRNL)
    /// - NL to CR-NL mapping (ONLCR)
    /// - Output processing (OPOST)
    pub const fn default() -> Self {
        let mut termios = Self {
            c_iflag: iflag::ICRNL | iflag::IXON,
            c_oflag: oflag::OPOST | oflag::ONLCR,
            c_cflag: 0,
            c_lflag: lflag::ICANON | lflag::ECHO | lflag::ISIG,
            c_cc: [0; 32],
        };

        // Set default control characters
        termios.c_cc[cc::VINTR] = 3;   // Ctrl-C
        termios.c_cc[cc::VSUSP] = 26;  // Ctrl-Z
        termios.c_cc[cc::VEOF] = 4;    // Ctrl-D
        termios.c_cc[cc::VERASE] = 127; // Backspace
        termios.c_cc[cc::VQUIT] = 28;  // Ctrl-\
        termios.c_cc[cc::VMIN] = 1;
        termios.c_cc[cc::VTIME] = 0;

        termios
    }
}

/// Window size structure
///
/// Describes the dimensions of the terminal window in rows and columns.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Winsize {
    /// Number of rows (lines)
    pub ws_row: u16,
    /// Number of columns (characters per line)
    pub ws_col: u16,
    /// Width in pixels (usually 0)
    pub ws_xpixel: u16,
    /// Height in pixels (usually 0)
    pub ws_ypixel: u16,
}

impl Winsize {
    /// Create a new Winsize with default dimensions (24x80)
    pub const fn default() -> Self {
        Self {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }
    }
}

/// Ring buffer for PTY data flow
///
/// Implements a circular buffer for efficient data transfer between
/// master and slave sides of the PTY.
#[derive(Debug)]
pub struct RingBuffer {
    /// Buffer data (fixed size)
    data: [u8; PTY_BUFFER_SIZE],
    /// Read position
    read_pos: usize,
    /// Write position
    write_pos: usize,
    /// Number of bytes currently in buffer
    count: usize,
}

impl RingBuffer {
    /// Create a new ring buffer
    pub const fn new() -> Self {
        Self {
            data: [0u8; PTY_BUFFER_SIZE],
            read_pos: 0,
            write_pos: 0,
            count: 0,
        }
    }

    /// Get the number of bytes available to read
    pub fn available(&self) -> usize {
        self.count
    }

    /// Get the number of bytes available to write
    pub fn space(&self) -> usize {
        self.data.len() - self.count
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Check if the buffer is full
    pub fn is_full(&self) -> bool {
        self.count == self.data.len()
    }

    /// Write data to the buffer
    ///
    /// Returns the number of bytes actually written (may be less than requested
    /// if buffer is full).
    pub fn write(&mut self, data: &[u8]) -> usize {
        let space = self.space();
        let to_write = data.len().min(space);

        for i in 0..to_write {
            self.data[self.write_pos] = data[i];
            self.write_pos = (self.write_pos + 1) % self.data.len();
        }

        self.count += to_write;
        to_write
    }

    /// Read data from the buffer
    ///
    /// Returns the number of bytes actually read (may be less than requested
    /// if buffer doesn't have enough data).
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let available = self.available();
        let to_read = buf.len().min(available);

        for i in 0..to_read {
            buf[i] = self.data[self.read_pos];
            self.read_pos = (self.read_pos + 1) % self.data.len();
        }

        self.count -= to_read;
        to_read
    }

    /// Peek at data without consuming it
    pub fn peek(&self, buf: &mut [u8]) -> usize {
        let available = self.available();
        let to_peek = buf.len().min(available);
        let mut pos = self.read_pos;

        for i in 0..to_peek {
            buf[i] = self.data[pos];
            pos = (pos + 1) % self.data.len();
        }

        to_peek
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.read_pos = 0;
        self.write_pos = 0;
        self.count = 0;
    }
}

/// PTY master side
///
/// The master side is typically used by the terminal emulator.
/// It reads output from the slave and writes input to the slave.
#[derive(Debug)]
pub struct PtyMaster {
    /// Output buffer (slave → master)
    pub output_buffer: RingBuffer,
    /// Terminal settings
    pub termios: Termios,
    /// Window size
    pub winsize: Winsize,
    /// Whether the slave side is open
    pub slave_open: bool,
}

impl PtyMaster {
    /// Create a new PTY master
    pub const fn new() -> Self {
        Self {
            output_buffer: RingBuffer::new(),
            termios: Termios::default(),
            winsize: Winsize::default(),
            slave_open: false,
        }
    }
}

/// PTY slave side
///
/// The slave side is typically used by the shell or application.
/// It reads input from the master and writes output to the master.
#[derive(Debug)]
pub struct PtySlave {
    /// Input buffer (master → slave)
    pub input_buffer: RingBuffer,
    /// Session ID (if this is a controlling terminal)
    pub session: Option<usize>,
    /// Foreground process group ID
    pub foreground_pgid: Option<usize>,
}

impl PtySlave {
    /// Create a new PTY slave
    pub const fn new() -> Self {
        Self {
            input_buffer: RingBuffer::new(),
            session: None,
            foreground_pgid: None,
        }
    }
}

/// PTY pair structure
///
/// Represents a complete pseudo-terminal with both master and slave sides.
#[derive(Debug)]
pub struct PtyPair {
    /// PTY number (used for /dev/pts/<n>)
    pub number: PtyNumber,
    /// Master side
    pub master: PtyMaster,
    /// Slave side
    pub slave: PtySlave,
    /// Whether this pair is allocated
    pub allocated: bool,
}

impl PtyPair {
    /// Create a new PTY pair
    pub fn new(number: PtyNumber) -> Self {
        Self {
            number,
            master: PtyMaster::new(),
            slave: PtySlave::new(),
            allocated: false,
        }
    }

    /// Allocate this PTY pair
    pub fn allocate(&mut self) {
        self.allocated = true;
        self.master.slave_open = false;
        self.slave.session = None;
        self.slave.foreground_pgid = None;
    }

    /// Deallocate this PTY pair
    pub fn deallocate(&mut self) {
        self.allocated = false;
        self.master.output_buffer.clear();
        self.slave.input_buffer.clear();
    }
}
