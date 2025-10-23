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
#[derive(Debug, Copy, Clone)]
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
    #[inline(always)]
    pub fn available(&self) -> usize {
        self.count
    }

    /// Get the number of bytes available to write
    #[inline(always)]
    pub fn space(&self) -> usize {
        self.data.len() - self.count
    }

    /// Check if the buffer is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Check if the buffer is full
    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.count == self.data.len()
    }

    /// Write data to the buffer
    ///
    /// Returns the number of bytes actually written (may be less than requested
    /// if buffer is full).
    ///
    /// Optimized for performance with inline hint and fast path for contiguous writes.
    #[inline]
    pub fn write(&mut self, data: &[u8]) -> usize {
        let space = self.space();
        if space == 0 {
            return 0;
        }
        
        let to_write = data.len().min(space);
        let buffer_len = self.data.len();
        
        // Fast path: contiguous write (no wrap-around)
        let contiguous = (buffer_len - self.write_pos).min(to_write);
        if contiguous > 0 {
            // Use slice copy for better performance
            self.data[self.write_pos..self.write_pos + contiguous]
                .copy_from_slice(&data[..contiguous]);
            self.write_pos = (self.write_pos + contiguous) % buffer_len;
        }
        
        // Handle wrap-around if needed
        let remaining = to_write - contiguous;
        if remaining > 0 {
            self.data[..remaining].copy_from_slice(&data[contiguous..to_write]);
            self.write_pos = remaining;
        }

        self.count += to_write;
        to_write
    }

    /// Read data from the buffer
    ///
    /// Returns the number of bytes actually read (may be less than requested
    /// if buffer doesn't have enough data).
    ///
    /// Optimized for performance with inline hint and fast path for contiguous reads.
    #[inline]
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let available = self.available();
        if available == 0 {
            return 0;
        }
        
        let to_read = buf.len().min(available);
        let buffer_len = self.data.len();
        
        // Fast path: contiguous read (no wrap-around)
        let contiguous = (buffer_len - self.read_pos).min(to_read);
        if contiguous > 0 {
            // Use slice copy for better performance
            buf[..contiguous].copy_from_slice(
                &self.data[self.read_pos..self.read_pos + contiguous]
            );
            self.read_pos = (self.read_pos + contiguous) % buffer_len;
        }
        
        // Handle wrap-around if needed
        let remaining = to_read - contiguous;
        if remaining > 0 {
            buf[contiguous..to_read].copy_from_slice(&self.data[..remaining]);
            self.read_pos = remaining;
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
#[derive(Debug, Copy, Clone)]
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
#[derive(Debug, Copy, Clone)]
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
#[derive(Debug, Copy, Clone)]
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
    pub const fn new(number: PtyNumber) -> Self {
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

use crate::sync::SpinLock;

/// Global PTY table
///
/// Manages all PTY pairs in the system. Protected by a spinlock for SMP safety.
pub struct PtyTable {
    /// Array of PTY pairs (fixed size)
    pairs: [PtyPair; MAX_PTY_PAIRS],
}

impl PtyTable {
    /// Create a new PTY table
    pub const fn new() -> Self {
        // Create array of PTY pairs using const initialization
        const INIT_PAIR: PtyPair = PtyPair::new(0);
        let mut pairs = [INIT_PAIR; MAX_PTY_PAIRS];
        
        // Initialize each pair with its correct number
        let mut i = 0;
        while i < MAX_PTY_PAIRS {
            pairs[i] = PtyPair::new(i as PtyNumber);
            i += 1;
        }
        
        Self { pairs }
    }

    /// Initialize the PTY table (no-op since we use const initialization)
    pub fn init(&mut self) {
        // Nothing to do - pairs are already initialized
    }

    /// Allocate a new PTY pair
    ///
    /// Returns the PTY number on success, or None if no pairs are available.
    pub fn allocate_pty(&mut self) -> Option<PtyNumber> {
        for pair in &mut self.pairs {
            if !pair.allocated {
                pair.allocate();
                return Some(pair.number);
            }
        }
        None
    }

    /// Deallocate a PTY pair
    ///
    /// Returns true if the pair was deallocated, false if it wasn't allocated.
    pub fn deallocate_pty(&mut self, number: PtyNumber) -> bool {
        if let Some(pair) = self.pairs.get_mut(number as usize) {
            if pair.allocated {
                pair.deallocate();
                return true;
            }
        }
        false
    }

    /// Get a reference to a PTY pair
    pub fn get_pty(&self, number: PtyNumber) -> Option<&PtyPair> {
        if (number as usize) < MAX_PTY_PAIRS {
            let pair = &self.pairs[number as usize];
            if pair.allocated {
                Some(pair)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get a mutable reference to a PTY pair
    pub fn get_pty_mut(&mut self, number: PtyNumber) -> Option<&mut PtyPair> {
        if (number as usize) < MAX_PTY_PAIRS {
            let pair = &mut self.pairs[number as usize];
            if pair.allocated {
                Some(pair)
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Global PTY table instance
static PTY_TABLE: SpinLock<PtyTable> = SpinLock::new(PtyTable::new());

/// Send a signal to the foreground process group of a PTY
///
/// This is called when special characters (Ctrl-C, Ctrl-Z, Ctrl-\) are detected
/// in the input stream and ISIG is enabled.
///
/// # Arguments
/// * `pair` - The PTY pair
/// * `signal` - The signal to send
fn send_signal_to_foreground_group(pair: &PtyPair, signal: u32) {
    use crate::signal::send_signal;

    // Get the foreground process group ID
    if let Some(pgid) = pair.slave.foreground_pgid {
        crate::serial_println!("[PTY] Sending signal {} to foreground PGID {}", signal, pgid);
        
        // TODO: Send signal to all processes in the process group
        // For now, just send to the process with ID == PGID (the group leader)
        if let Some(task) = crate::sched::get_task_mut(pgid) {
            match send_signal(task, signal) {
                Ok(()) => {
                    crate::serial_println!("[PTY] Signal {} sent to process {}", signal, pgid);
                }
                Err(()) => {
                    crate::serial_println!("[PTY] ERROR: Failed to send signal {} to process {}", 
                                          signal, pgid);
                }
            }
        } else {
            crate::serial_println!("[PTY] WARNING: Foreground process {} not found", pgid);
        }
    } else {
        crate::serial_println!("[PTY] WARNING: No foreground process group set");
    }
}

/// Send a signal to a specific process group
///
/// # Arguments
/// * `pgid` - Process group ID
/// * `signal` - Signal to send
fn send_signal_to_process_group(pgid: usize, signal: u32) {
    use crate::signal::send_signal;

    crate::serial_println!("[PTY] Sending signal {} to PGID {}", signal, pgid);
    
    // TODO: Send signal to all processes in the process group
    // For now, just send to the process with ID == PGID (the group leader)
    if let Some(task) = crate::sched::get_task_mut(pgid) {
        match send_signal(task, signal) {
            Ok(()) => {
                crate::serial_println!("[PTY] Signal {} sent to process {}", signal, pgid);
            }
            Err(()) => {
                crate::serial_println!("[PTY] ERROR: Failed to send signal {} to process {}", 
                                      signal, pgid);
            }
        }
    } else {
        crate::serial_println!("[PTY] WARNING: Process {} not found", pgid);
    }
}

/// Check if the current process is in the foreground process group
///
/// # Arguments
/// * `pair` - The PTY pair
///
/// # Returns
/// true if current process is in foreground group, false otherwise
fn is_foreground_process(pair: &PtyPair) -> bool {
    // Get current task's process group ID
    if let Some((task_id, _)) = crate::sched::get_current_task_info() {
        // TODO: Get actual PGID from task
        // For now, assume task_id == pgid
        let current_pgid = task_id;
        
        if let Some(fg_pgid) = pair.slave.foreground_pgid {
            return current_pgid == fg_pgid;
        }
    }
    
    // If no foreground group is set, allow access
    true
}

/// Initialize the PTY subsystem
///
/// Must be called once during kernel initialization.
pub fn init() {
    let mut table = PTY_TABLE.lock();
    table.init();
    crate::serial_println!("[PTY] Initialized PTY subsystem with {} pairs", MAX_PTY_PAIRS);
}

/// Allocate a new PTY pair
///
/// This is called when /dev/ptmx is opened.
/// Returns the PTY number on success, or None if no pairs are available.
pub fn allocate_pty() -> Option<PtyNumber> {
    let mut table = PTY_TABLE.lock();
    let number = table.allocate_pty();
    if let Some(n) = number {
        crate::serial_println!("[PTY] Allocated PTY pair {}", n);
    } else {
        crate::serial_println!("[PTY] ERROR: No PTY pairs available");
    }
    number
}

/// Deallocate a PTY pair
///
/// This is called when both master and slave sides are closed.
/// Sends SIGHUP to the foreground process group before deallocating.
pub fn deallocate_pty(number: PtyNumber) -> bool {
    let mut table = PTY_TABLE.lock();
    
    // Send SIGHUP to foreground process group before closing
    if let Some(pair) = table.get_pty_mut(number) {
        if pair.allocated {
            crate::serial_println!("[PTY] Sending SIGHUP to foreground group before closing PTY {}", number);
            send_signal_to_foreground_group(pair, crate::signal::signals::SIGHUP);
        }
    }
    
    let result = table.deallocate_pty(number);
    if result {
        crate::serial_println!("[PTY] Deallocated PTY pair {}", number);
    }
    result
}

/// Get the slave number for a PTY (for TIOCGPTN ioctl)
///
/// Returns the PTY number if it's allocated, or None otherwise.
pub fn get_pty_slave_number(number: PtyNumber) -> Option<PtyNumber> {
    let table = PTY_TABLE.lock();
    table.get_pty(number).map(|p| p.number)
}

/// Get termios settings for a PTY
///
/// Returns the termios structure if the PTY is allocated.
pub fn get_termios(number: PtyNumber) -> Option<Termios> {
    let table = PTY_TABLE.lock();
    table.get_pty(number).map(|p| p.master.termios)
}

/// Set termios settings for a PTY
///
/// Returns true if successful, false if PTY not allocated.
pub fn set_termios(number: PtyNumber, termios: Termios) -> bool {
    let mut table = PTY_TABLE.lock();
    if let Some(pair) = table.get_pty_mut(number) {
        pair.master.termios = termios;
        true
    } else {
        false
    }
}

/// Get window size for a PTY
///
/// Returns the window size if the PTY is allocated.
pub fn get_winsize(number: PtyNumber) -> Option<Winsize> {
    let table = PTY_TABLE.lock();
    table.get_pty(number).map(|p| p.master.winsize)
}

/// Set window size for a PTY
///
/// Returns true if successful, false if PTY not allocated.
/// Generates SIGWINCH signal to foreground process group.
pub fn set_winsize(number: PtyNumber, winsize: Winsize) -> bool {
    let mut table = PTY_TABLE.lock();
    if let Some(pair) = table.get_pty_mut(number) {
        pair.master.winsize = winsize;
        crate::serial_println!("[PTY] Window size changed for PTY {}: {}x{}", 
                              number, winsize.ws_row, winsize.ws_col);
        
        // Send SIGWINCH to foreground process group
        send_signal_to_foreground_group(pair, crate::signal::signals::SIGWINCH);
        true
    } else {
        false
    }
}

/// Read from PTY master (reads from slave output buffer)
///
/// Returns the number of bytes read.
pub fn read_master(number: PtyNumber, buf: &mut [u8]) -> usize {
    let mut table = PTY_TABLE.lock();
    if let Some(pair) = table.get_pty_mut(number) {
        let bytes_read = pair.master.output_buffer.read(buf);
        if bytes_read > 0 {
            crate::serial_println!("[PTY] Master read {} bytes from PTY {}", bytes_read, number);
        }
        bytes_read
    } else {
        0
    }
}

/// Write to PTY master (writes to slave input buffer)
///
/// Returns the number of bytes written.
/// Processes data according to termios settings (canonical mode, echo, etc.)
pub fn write_master(number: PtyNumber, data: &[u8]) -> usize {
    let mut table = PTY_TABLE.lock();
    if let Some(pair) = table.get_pty_mut(number) {
        let termios = pair.master.termios;
        let mut bytes_written = 0;
        
        for &byte in data {
            // Process input according to termios flags
            let mut processed_byte = byte;
            
            // Input processing (c_iflag)
            if termios.c_iflag & iflag::ICRNL != 0 && byte == b'\r' {
                processed_byte = b'\n'; // Map CR to NL
            } else if termios.c_iflag & iflag::INLCR != 0 && byte == b'\n' {
                processed_byte = b'\r'; // Map NL to CR
            }
            
            // Check for special characters if ISIG is enabled
            if termios.c_lflag & lflag::ISIG != 0 {
                if byte == termios.c_cc[cc::VINTR] {
                    // Ctrl-C - generate SIGINT
                    crate::serial_println!("[PTY] SIGINT triggered on PTY {}", number);
                    send_signal_to_foreground_group(pair, crate::signal::signals::SIGINT);
                    continue;
                } else if byte == termios.c_cc[cc::VSUSP] {
                    // Ctrl-Z - generate SIGTSTP
                    crate::serial_println!("[PTY] SIGTSTP triggered on PTY {}", number);
                    send_signal_to_foreground_group(pair, crate::signal::signals::SIGTSTP);
                    continue;
                } else if byte == termios.c_cc[cc::VQUIT] {
                    // Ctrl-\ - generate SIGQUIT
                    crate::serial_println!("[PTY] SIGQUIT triggered on PTY {}", number);
                    send_signal_to_foreground_group(pair, crate::signal::signals::SIGQUIT);
                    continue;
                }
            }
            
            // Write to slave input buffer
            if pair.slave.input_buffer.write(&[processed_byte]) > 0 {
                bytes_written += 1;
                
                // Echo back if ECHO is enabled
                if termios.c_lflag & lflag::ECHO != 0 {
                    // Echo to master output buffer
                    let echo_byte = processed_byte;
                    
                    // Output processing for echo
                    if termios.c_oflag & oflag::OPOST != 0 {
                        if termios.c_oflag & oflag::ONLCR != 0 && echo_byte == b'\n' {
                            // Map NL to CR-NL on output
                            pair.master.output_buffer.write(&[b'\r']);
                        }
                    }
                    
                    pair.master.output_buffer.write(&[echo_byte]);
                }
            } else {
                // Buffer full
                break;
            }
        }
        
        if bytes_written > 0 {
            crate::serial_println!("[PTY] Master wrote {} bytes to PTY {}", bytes_written, number);
        }
        bytes_written
    } else {
        0
    }
}

/// Read from PTY slave (reads from master output buffer)
///
/// Returns the number of bytes read.
/// In canonical mode, blocks until a newline is available.
/// Sends SIGTTIN if a background process tries to read.
pub fn read_slave(number: PtyNumber, buf: &mut [u8]) -> usize {
    let mut table = PTY_TABLE.lock();
    if let Some(pair) = table.get_pty_mut(number) {
        // Check if this is a background process trying to read
        if !is_foreground_process(pair) {
            crate::serial_println!("[PTY] Background process attempting to read from PTY {}", number);
            
            // Get current process group ID and send SIGTTIN
            if let Some((task_id, _)) = crate::sched::get_current_task_info() {
                // TODO: Get actual PGID from task
                let current_pgid = task_id;
                drop(table); // Release lock before sending signal
                send_signal_to_process_group(current_pgid, crate::signal::signals::SIGTTIN);
                return 0; // Return 0 bytes read (process will be stopped)
            }
        }
        
        let termios = pair.master.termios;
        
        // In canonical mode, read until newline
        if termios.c_lflag & lflag::ICANON != 0 {
            // Check if there's a newline in the buffer
            let mut temp_buf = [0u8; PTY_BUFFER_SIZE];
            let available = pair.slave.input_buffer.peek(&mut temp_buf);
            
            // Look for newline
            let mut newline_pos = None;
            for i in 0..available {
                if temp_buf[i] == b'\n' {
                    newline_pos = Some(i + 1); // Include the newline
                    break;
                }
            }
            
            if let Some(pos) = newline_pos {
                // Read up to and including the newline
                let to_read = pos.min(buf.len());
                let bytes_read = pair.slave.input_buffer.read(&mut buf[..to_read]);
                if bytes_read > 0 {
                    crate::serial_println!("[PTY] Slave read {} bytes from PTY {} (canonical)", 
                                          bytes_read, number);
                }
                bytes_read
            } else {
                // No complete line available
                0
            }
        } else {
            // Raw mode - read whatever is available
            let bytes_read = pair.slave.input_buffer.read(buf);
            if bytes_read > 0 {
                crate::serial_println!("[PTY] Slave read {} bytes from PTY {} (raw)", 
                                      bytes_read, number);
            }
            bytes_read
        }
    } else {
        0
    }
}

/// Write to PTY slave (writes to master output buffer)
///
/// Returns the number of bytes written.
/// Processes output according to termios settings.
/// Sends SIGTTOU if a background process tries to write.
pub fn write_slave(number: PtyNumber, data: &[u8]) -> usize {
    let mut table = PTY_TABLE.lock();
    if let Some(pair) = table.get_pty_mut(number) {
        // Check if this is a background process trying to write
        if !is_foreground_process(pair) {
            crate::serial_println!("[PTY] Background process attempting to write to PTY {}", number);
            
            // Get current process group ID and send SIGTTOU
            if let Some((task_id, _)) = crate::sched::get_current_task_info() {
                // TODO: Get actual PGID from task
                let current_pgid = task_id;
                drop(table); // Release lock before sending signal
                send_signal_to_process_group(current_pgid, crate::signal::signals::SIGTTOU);
                return 0; // Return 0 bytes written (process will be stopped)
            }
        }
        
        let termios = pair.master.termios;
        let mut bytes_written = 0;
        
        for &byte in data {
            let processed_byte = byte;
            
            // Output processing (c_oflag)
            if termios.c_oflag & oflag::OPOST != 0 {
                if termios.c_oflag & oflag::ONLCR != 0 && byte == b'\n' {
                    // Map NL to CR-NL on output
                    if pair.master.output_buffer.write(&[b'\r']) > 0 {
                        // Successfully wrote CR
                    }
                }
            }
            
            // Write to master output buffer
            if pair.master.output_buffer.write(&[processed_byte]) > 0 {
                bytes_written += 1;
            } else {
                // Buffer full
                break;
            }
        }
        
        if bytes_written > 0 {
            crate::serial_println!("[PTY] Slave wrote {} bytes to PTY {}", bytes_written, number);
        }
        bytes_written
    } else {
        0
    }
}

/// Set the foreground process group for a PTY
///
/// # Arguments
/// * `number` - PTY number
/// * `pgid` - Process group ID to set as foreground
///
/// # Returns
/// true if successful, false if PTY not found
pub fn set_foreground_pgid(number: PtyNumber, pgid: usize) -> bool {
    let mut table = PTY_TABLE.lock();
    if let Some(pair) = table.get_pty_mut(number) {
        pair.slave.foreground_pgid = Some(pgid);
        crate::serial_println!("[PTY] Set foreground PGID to {} for PTY {}", pgid, number);
        true
    } else {
        false
    }
}

/// Get the foreground process group for a PTY
///
/// # Arguments
/// * `number` - PTY number
///
/// # Returns
/// Some(pgid) if set, None if not set or PTY not found
pub fn get_foreground_pgid(number: PtyNumber) -> Option<usize> {
    let table = PTY_TABLE.lock();
    if let Some(pair) = table.get_pty(number) {
        pair.slave.foreground_pgid
    } else {
        None
    }
}

/// Set the session for a PTY
///
/// # Arguments
/// * `number` - PTY number
/// * `sid` - Session ID
///
/// # Returns
/// true if successful, false if PTY not found
pub fn set_session(number: PtyNumber, sid: usize) -> bool {
    let mut table = PTY_TABLE.lock();
    if let Some(pair) = table.get_pty_mut(number) {
        pair.slave.session = Some(sid);
        crate::serial_println!("[PTY] Set session to {} for PTY {}", sid, number);
        true
    } else {
        false
    }
}
