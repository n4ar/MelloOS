// Memory Management Logging Utilities
// Provides formatted logging for memory management operations

#![allow(dead_code)]

use core::fmt::Write;

/// Format memory size in appropriate units (bytes, KB, MB)
/// 
/// # Arguments
/// * `bytes` - Size in bytes
/// 
/// # Returns
/// A tuple containing (value, unit) where value is the size in the appropriate unit
/// 
/// # Examples
/// ```
/// assert_eq!(format_size(512), (512, "bytes"));
/// assert_eq!(format_size(1024), (1, "KB"));
/// assert_eq!(format_size(1048576), (1, "MB"));
/// ```
pub fn format_size(bytes: usize) -> (usize, &'static str) {
    const KB: usize = 1024;
    const MB: usize = 1024 * 1024;
    
    if bytes >= MB {
        (bytes / MB, "MB")
    } else if bytes >= KB {
        (bytes / KB, "KB")
    } else {
        (bytes, "bytes")
    }
}

/// Simple writer for serial output (placeholder)
/// In a real implementation, this would write to the serial port
struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, _s: &str) -> core::fmt::Result {
        // TODO: Implement actual serial output
        // For now, this is a no-op placeholder
        Ok(())
    }
}

/// Write a formatted message to serial output with "[MM]" prefix
/// 
/// # Arguments
/// * `args` - Format arguments
pub fn mm_write(args: core::fmt::Arguments) {
    let mut writer = SerialWriter;
    let _ = write!(writer, "[MM] {}\n", args);
}

/// Write a formatted error message to serial output with "[MM] ERROR:" prefix
/// 
/// # Arguments
/// * `args` - Format arguments
pub fn mm_write_error(args: core::fmt::Arguments) {
    let mut writer = SerialWriter;
    let _ = write!(writer, "[MM] ERROR: {}\n", args);
}

/// Write a formatted test success message with "[MM] ✓" prefix
/// 
/// # Arguments
/// * `args` - Format arguments
pub fn mm_write_test_ok(args: core::fmt::Arguments) {
    let mut writer = SerialWriter;
    let _ = write!(writer, "[MM] ✓ {}\n", args);
}

/// Write a formatted test failure message with "[MM] ✗" prefix
/// 
/// # Arguments
/// * `args` - Format arguments
pub fn mm_write_test_fail(args: core::fmt::Arguments) {
    let mut writer = SerialWriter;
    let _ = write!(writer, "[MM] ✗ {}\n", args);
}

/// Log macro for memory management with "[MM]" prefix
/// Formats addresses in hexadecimal and sizes in appropriate units
/// 
/// # Examples
/// ```
/// mm_log!("Initializing memory management");
/// mm_log!("Total memory: {} MB", total_mb);
/// mm_log!("Allocated frame at 0x{:x}", addr);
/// ```
#[macro_export]
macro_rules! mm_log {
    ($($arg:tt)*) => {
        $crate::mm::log::mm_write(format_args!($($arg)*))
    };
}

/// Log info message with "[MM]" prefix
/// 
/// # Examples
/// ```
/// mm_info!("Physical memory manager initialized");
/// mm_info!("Free memory: {} MB", free_mb);
/// ```
#[macro_export]
macro_rules! mm_info {
    ($($arg:tt)*) => {
        $crate::mm_log!($($arg)*)
    };
}

/// Log error message with "[MM] ERROR:" prefix
/// 
/// # Examples
/// ```
/// mm_error!("Out of physical memory");
/// mm_error!("Failed to map page at 0x{:x}", virt_addr);
/// ```
#[macro_export]
macro_rules! mm_error {
    ($($arg:tt)*) => {
        $crate::mm::log::mm_write_error(format_args!($($arg)*))
    };
}

/// Log debug message with "[MM] DEBUG:" prefix
/// 
/// # Examples
/// ```
/// mm_debug!("Scanning bitmap from frame {}", start_frame);
/// ```
#[macro_export]
macro_rules! mm_debug {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::mm_log!("DEBUG: {}", format_args!($($arg)*))
    };
}

/// Log test success with "[MM] ✓" prefix
/// 
/// # Examples
/// ```
/// mm_test_ok!("PMM allocation test passed");
/// mm_test_ok!("Allocated frame at 0x{:x}", frame_addr);
/// ```
#[macro_export]
macro_rules! mm_test_ok {
    ($($arg:tt)*) => {
        $crate::mm::log::mm_write_test_ok(format_args!($($arg)*))
    };
}

/// Log test failure with "[MM] ✗" prefix
/// 
/// # Examples
/// ```
/// mm_test_fail!("PMM allocation test failed");
/// mm_test_fail!("Expected 0x{:x}, got 0x{:x}", expected, actual);
/// ```
#[macro_export]
macro_rules! mm_test_fail {
    ($($arg:tt)*) => {
        $crate::mm::log::mm_write_test_fail(format_args!($($arg)*))
    };
}

/// Format an address as hexadecimal with 0x prefix
/// This is a helper for use in format strings
/// 
/// # Examples
/// ```
/// let addr = 0x1000;
/// mm_log!("Address: 0x{:x}", addr);  // Prints: [MM] Address: 0x1000
/// ```
pub fn format_addr(addr: usize) -> usize {
    addr
}

/// Format a size with appropriate units
/// Returns a formatted string representation
/// 
/// # Examples
/// ```
/// let (value, unit) = format_size(1048576);
/// mm_log!("Heap size: {} {}", value, unit);  // Prints: [MM] Heap size: 1 MB
/// ```
pub fn format_size_str(bytes: usize) -> (usize, &'static str) {
    format_size(bytes)
}
