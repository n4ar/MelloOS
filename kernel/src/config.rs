/// Configuration constants for MelloOS kernel

/// Scheduler tick frequency in Hz (100 Hz = 10ms per tick)
pub const SCHED_HZ: u64 = 100;

/// Maximum number of CPUs supported by the kernel
pub const MAX_CPUS: usize = 16;
