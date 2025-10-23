/// Configuration constants for MelloOS kernel

/// Scheduler tick frequency in Hz (100 Hz = 10ms per tick)
/// Higher frequency = more responsive scheduling and faster sleep/wake
pub const SCHED_HZ: u64 = 100;

/// Maximum number of CPUs supported by the kernel
pub const MAX_CPUS: usize = 16;

/// Fast boot mode - reduces delays and test iterations for faster testing
/// Set to true for development/testing, false for full validation
pub const FAST_BOOT_MODE: bool = true;

/// Boot timeout in ticks (for fast boot mode)
/// At 100 Hz: 500 ticks = 5 seconds
pub const FAST_BOOT_TIMEOUT_TICKS: usize = 500;

/// Normal boot timeout in ticks
/// At 100 Hz: 3000 ticks = 30 seconds
pub const NORMAL_BOOT_TIMEOUT_TICKS: usize = 3000;
