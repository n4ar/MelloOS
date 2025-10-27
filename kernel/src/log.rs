/// Structured logging module for MelloOS kernel
/// Provides logging with format: [cpuN][pid=X][subsys] message
/// Supports log levels: ERROR, WARN, INFO, DEBUG, TRACE
use crate::arch::x86_64::smp::percpu::percpu_current;
use core::fmt;
use spin::Mutex;

/// Log levels for kernel logging
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    /// Critical errors that may cause system instability
    Error = 0,
    /// Warning conditions that should be addressed
    Warn = 1,
    /// Informational messages about important events
    Info = 2,
    /// Detailed debugging information
    Debug = 3,
    /// Very verbose tracing information
    Trace = 4,
}

impl LogLevel {
    /// Get the string representation of the log level
    pub const fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Global log level filter
/// Only messages at or below this level will be logged
static LOG_LEVEL: core::sync::atomic::AtomicU8 =
    core::sync::atomic::AtomicU8::new(LogLevel::Info as u8);

/// Set the global log level
pub fn set_log_level(level: LogLevel) {
    LOG_LEVEL.store(level as u8, core::sync::atomic::Ordering::Relaxed);
}

/// Get the current global log level
pub fn get_log_level() -> LogLevel {
    let level = LOG_LEVEL.load(core::sync::atomic::Ordering::Relaxed);
    match level {
        0 => LogLevel::Error,
        1 => LogLevel::Warn,
        2 => LogLevel::Info,
        3 => LogLevel::Debug,
        4 => LogLevel::Trace,
        _ => LogLevel::Info,
    }
}

/// Check if a log level should be logged
#[inline]
pub fn should_log(level: LogLevel) -> bool {
    level <= get_log_level()
}

/// Kernel log buffer for dmesg
/// Uses a fixed-size circular buffer to store log messages
const LOG_BUFFER_SIZE: usize = 65536; // 64KB buffer
const MAX_LOG_ENTRIES: usize = 1000;

struct LogBuffer {
    buffer: [u8; LOG_BUFFER_SIZE],
    write_pos: usize,
    entries: usize,
}

impl LogBuffer {
    const fn new() -> Self {
        Self {
            buffer: [0; LOG_BUFFER_SIZE],
            write_pos: 0,
            entries: 0,
        }
    }

    fn add_message(&mut self, message: &str) {
        let bytes = message.as_bytes();
        let len = bytes.len();

        // If message is too long, truncate it
        if len >= LOG_BUFFER_SIZE {
            return;
        }

        // If we would overflow, wrap around
        if self.write_pos + len + 1 > LOG_BUFFER_SIZE {
            self.write_pos = 0;
            self.entries = 0; // Reset on wrap
        }

        // Copy message to buffer
        self.buffer[self.write_pos..self.write_pos + len].copy_from_slice(bytes);
        self.buffer[self.write_pos + len] = b'\n';
        self.write_pos += len + 1;

        if self.entries < MAX_LOG_ENTRIES {
            self.entries += 1;
        }
    }

    fn read_all(&self) -> &[u8] {
        &self.buffer[..self.write_pos]
    }
}

static LOG_BUFFER: Mutex<LogBuffer> = Mutex::new(LogBuffer::new());

/// Add a log entry to the kernel log buffer
fn add_to_log_buffer(message: &str) {
    let mut buffer = LOG_BUFFER.lock();
    buffer.add_message(message);
}

/// Read the kernel log buffer into a provided buffer
/// Returns the number of bytes copied
pub fn read_log_buffer(dest: &mut [u8]) -> usize {
    let buffer = LOG_BUFFER.lock();
    let data = buffer.read_all();
    let to_copy = core::cmp::min(data.len(), dest.len());
    dest[..to_copy].copy_from_slice(&data[..to_copy]);
    to_copy
}

/// Internal logging function
/// Format: [cpuN][pid=X][subsys] message
#[doc(hidden)]
pub fn _log(level: LogLevel, subsys: &str, args: fmt::Arguments) {
    if !should_log(level) {
        return;
    }

    // Get current CPU ID and PID
    // Use a simple approach that's safe even during early boot
    let (cpu_id, pid) = {
        // Try to get per-CPU data, but handle the case where it's not initialized
        // During early boot or panic, we may not have valid per-CPU data
        let percpu = percpu_current();
        let cpu = percpu.id;
        let task_id = percpu.current_task.unwrap_or(0);
        (cpu, task_id)
    };

    // Print to serial with structured format
    use crate::serial_println;
    serial_println!(
        "[cpu{}][pid={}][{}][{}] {}",
        cpu_id,
        pid,
        subsys,
        level.as_str(),
        args
    );

    // Format message for log buffer
    // We need to format it into a temporary buffer
    use core::fmt::Write;
    struct LogWriter {
        buffer: [u8; 512],
        pos: usize,
    }

    impl LogWriter {
        fn new() -> Self {
            Self {
                buffer: [0u8; 512],
                pos: 0,
            }
        }

        fn as_str(&self) -> Result<&str, core::str::Utf8Error> {
            core::str::from_utf8(&self.buffer[..self.pos])
        }
    }

    impl Write for LogWriter {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let bytes = s.as_bytes();
            let remaining = self.buffer.len() - self.pos;
            let to_write = core::cmp::min(bytes.len(), remaining);
            self.buffer[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
            self.pos += to_write;
            Ok(())
        }
    }

    let mut writer = LogWriter::new();

    let _ = write!(
        writer,
        "[cpu{}][pid={}][{}][{}] {}",
        cpu_id,
        pid,
        subsys,
        level.as_str(),
        args
    );

    // Add to log buffer
    if let Ok(message) = writer.as_str() {
        add_to_log_buffer(message);
    }
}

/// Log an error message
/// Format: [cpuN][pid=X][subsys][ERROR] message
#[macro_export]
macro_rules! log_error {
    ($subsys:expr, $($arg:tt)*) => {
        $crate::log::_log(
            $crate::log::LogLevel::Error,
            $subsys,
            format_args!($($arg)*)
        )
    };
}

/// Log a warning message
/// Format: [cpuN][pid=X][subsys][WARN] message
#[macro_export]
macro_rules! log_warn {
    ($subsys:expr, $($arg:tt)*) => {
        $crate::log::_log(
            $crate::log::LogLevel::Warn,
            $subsys,
            format_args!($($arg)*)
        )
    };
}

/// Log an informational message
/// Format: [cpuN][pid=X][subsys][INFO] message
#[macro_export]
macro_rules! log_info {
    ($subsys:expr, $($arg:tt)*) => {
        $crate::log::_log(
            $crate::log::LogLevel::Info,
            $subsys,
            format_args!($($arg)*)
        )
    };
}

/// Log a debug message
/// Format: [cpuN][pid=X][subsys][DEBUG] message
#[macro_export]
macro_rules! log_debug {
    ($subsys:expr, $($arg:tt)*) => {
        $crate::log::_log(
            $crate::log::LogLevel::Debug,
            $subsys,
            format_args!($($arg)*)
        )
    };
}

/// Log a trace message
/// Format: [cpuN][pid=X][subsys][TRACE] message
#[macro_export]
macro_rules! log_trace {
    ($subsys:expr, $($arg:tt)*) => {
        $crate::log::_log(
            $crate::log::LogLevel::Trace,
            $subsys,
            format_args!($($arg)*)
        )
    };
}
