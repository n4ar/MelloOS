/// Configuration constants for MelloOS kernel

/// Scheduler tick frequency in Hz (20 Hz = 50ms per tick)
pub const SCHED_HZ: u64 = 20;

/// Maximum number of CPUs supported by the kernel
pub const MAX_CPUS: usize = 16;
