/// Kernel metrics collection module
/// Tracks system-wide statistics using atomic counters for SMP safety
/// Metrics are exposed via /proc/stat
use core::sync::atomic::{AtomicU64, Ordering};

/// Maximum number of syscalls to track
pub const MAX_SYSCALLS: usize = 512;

/// Global system metrics
pub struct SystemMetrics {
    /// Total context switches across all CPUs
    pub context_switches: AtomicU64,

    /// Total signals delivered
    pub signals_delivered: AtomicU64,

    /// PTY bytes read (from slave to master)
    pub pty_bytes_in: AtomicU64,

    /// PTY bytes written (from master to slave)
    pub pty_bytes_out: AtomicU64,

    /// Per-syscall counters (indexed by syscall number)
    pub syscalls: [AtomicU64; MAX_SYSCALLS],

    /// Total interrupts handled
    pub interrupts: AtomicU64,

    /// Total page faults
    pub page_faults: AtomicU64,

    /// Total IPC messages sent
    pub ipc_messages_sent: AtomicU64,

    /// Total IPC messages received
    pub ipc_messages_received: AtomicU64,
}

impl SystemMetrics {
    /// Create a new SystemMetrics instance with all counters at zero
    pub const fn new() -> Self {
        const ZERO: AtomicU64 = AtomicU64::new(0);
        Self {
            context_switches: AtomicU64::new(0),
            signals_delivered: AtomicU64::new(0),
            pty_bytes_in: AtomicU64::new(0),
            pty_bytes_out: AtomicU64::new(0),
            syscalls: [ZERO; MAX_SYSCALLS],
            interrupts: AtomicU64::new(0),
            page_faults: AtomicU64::new(0),
            ipc_messages_sent: AtomicU64::new(0),
            ipc_messages_received: AtomicU64::new(0),
        }
    }

    /// Increment context switch counter
    #[inline]
    pub fn inc_context_switches(&self) {
        self.context_switches.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment signals delivered counter
    #[inline]
    pub fn inc_signals_delivered(&self) {
        self.signals_delivered.fetch_add(1, Ordering::Relaxed);
    }

    /// Add to PTY bytes in counter
    #[inline]
    pub fn add_pty_bytes_in(&self, bytes: u64) {
        self.pty_bytes_in.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Add to PTY bytes out counter
    #[inline]
    pub fn add_pty_bytes_out(&self, bytes: u64) {
        self.pty_bytes_out.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Increment syscall counter for a specific syscall number
    #[inline]
    pub fn inc_syscall(&self, syscall_num: usize) {
        if syscall_num < MAX_SYSCALLS {
            self.syscalls[syscall_num].fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Increment interrupt counter
    #[inline]
    pub fn inc_interrupts(&self) {
        self.interrupts.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment page fault counter
    #[inline]
    pub fn inc_page_faults(&self) {
        self.page_faults.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment IPC messages sent counter
    #[inline]
    pub fn inc_ipc_sent(&self) {
        self.ipc_messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment IPC messages received counter
    #[inline]
    pub fn inc_ipc_received(&self) {
        self.ipc_messages_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Get context switches count
    pub fn get_context_switches(&self) -> u64 {
        self.context_switches.load(Ordering::Relaxed)
    }

    /// Get signals delivered count
    pub fn get_signals_delivered(&self) -> u64 {
        self.signals_delivered.load(Ordering::Relaxed)
    }

    /// Get PTY bytes in count
    pub fn get_pty_bytes_in(&self) -> u64 {
        self.pty_bytes_in.load(Ordering::Relaxed)
    }

    /// Get PTY bytes out count
    pub fn get_pty_bytes_out(&self) -> u64 {
        self.pty_bytes_out.load(Ordering::Relaxed)
    }

    /// Get syscall count for a specific syscall number
    pub fn get_syscall_count(&self, syscall_num: usize) -> u64 {
        if syscall_num < MAX_SYSCALLS {
            self.syscalls[syscall_num].load(Ordering::Relaxed)
        } else {
            0
        }
    }

    /// Get total syscall count (sum of all syscalls)
    pub fn get_total_syscalls(&self) -> u64 {
        let mut total = 0;
        for counter in &self.syscalls {
            total += counter.load(Ordering::Relaxed);
        }
        total
    }

    /// Get interrupt count
    pub fn get_interrupts(&self) -> u64 {
        self.interrupts.load(Ordering::Relaxed)
    }

    /// Get page fault count
    pub fn get_page_faults(&self) -> u64 {
        self.page_faults.load(Ordering::Relaxed)
    }

    /// Get IPC messages sent count
    pub fn get_ipc_sent(&self) -> u64 {
        self.ipc_messages_sent.load(Ordering::Relaxed)
    }

    /// Get IPC messages received count
    pub fn get_ipc_received(&self) -> u64 {
        self.ipc_messages_received.load(Ordering::Relaxed)
    }
}

/// Global system metrics instance
pub static METRICS: SystemMetrics = SystemMetrics::new();

/// Get a reference to the global metrics
#[inline]
pub fn metrics() -> &'static SystemMetrics {
    &METRICS
}

/// Performance timing utilities for benchmarking hot paths
pub mod timing {
    use core::sync::atomic::{AtomicU64, Ordering};

    /// Performance timing buckets for different operations
    pub struct TimingStats {
        /// Syscall entry/exit timing (nanoseconds)
        pub syscall_latency_ns: AtomicU64,
        pub syscall_count: AtomicU64,

        /// PTY read/write timing (nanoseconds)
        pub pty_read_latency_ns: AtomicU64,
        pub pty_read_count: AtomicU64,
        pub pty_write_latency_ns: AtomicU64,
        pub pty_write_count: AtomicU64,

        /// Signal delivery timing (nanoseconds)
        pub signal_latency_ns: AtomicU64,
        pub signal_count: AtomicU64,

        /// Context switch timing (nanoseconds)
        pub context_switch_latency_ns: AtomicU64,
        pub context_switch_count: AtomicU64,

        /// Fork timing (nanoseconds)
        pub fork_latency_ns: AtomicU64,
        pub fork_count: AtomicU64,

        /// Execve timing (nanoseconds)
        pub execve_latency_ns: AtomicU64,
        pub execve_count: AtomicU64,
    }

    impl TimingStats {
        pub const fn new() -> Self {
            Self {
                syscall_latency_ns: AtomicU64::new(0),
                syscall_count: AtomicU64::new(0),
                pty_read_latency_ns: AtomicU64::new(0),
                pty_read_count: AtomicU64::new(0),
                pty_write_latency_ns: AtomicU64::new(0),
                pty_write_count: AtomicU64::new(0),
                signal_latency_ns: AtomicU64::new(0),
                signal_count: AtomicU64::new(0),
                context_switch_latency_ns: AtomicU64::new(0),
                context_switch_count: AtomicU64::new(0),
                fork_latency_ns: AtomicU64::new(0),
                fork_count: AtomicU64::new(0),
                execve_latency_ns: AtomicU64::new(0),
                execve_count: AtomicU64::new(0),
            }
        }

        /// Record syscall timing
        #[inline]
        pub fn record_syscall(&self, latency_ns: u64) {
            self.syscall_latency_ns
                .fetch_add(latency_ns, Ordering::Relaxed);
            self.syscall_count.fetch_add(1, Ordering::Relaxed);
        }

        /// Record PTY read timing
        #[inline]
        pub fn record_pty_read(&self, latency_ns: u64) {
            self.pty_read_latency_ns
                .fetch_add(latency_ns, Ordering::Relaxed);
            self.pty_read_count.fetch_add(1, Ordering::Relaxed);
        }

        /// Record PTY write timing
        #[inline]
        pub fn record_pty_write(&self, latency_ns: u64) {
            self.pty_write_latency_ns
                .fetch_add(latency_ns, Ordering::Relaxed);
            self.pty_write_count.fetch_add(1, Ordering::Relaxed);
        }

        /// Record signal delivery timing
        #[inline]
        pub fn record_signal(&self, latency_ns: u64) {
            self.signal_latency_ns
                .fetch_add(latency_ns, Ordering::Relaxed);
            self.signal_count.fetch_add(1, Ordering::Relaxed);
        }

        /// Record context switch timing
        #[inline]
        pub fn record_context_switch(&self, latency_ns: u64) {
            self.context_switch_latency_ns
                .fetch_add(latency_ns, Ordering::Relaxed);
            self.context_switch_count.fetch_add(1, Ordering::Relaxed);
        }

        /// Record fork timing
        #[inline]
        pub fn record_fork(&self, latency_ns: u64) {
            self.fork_latency_ns
                .fetch_add(latency_ns, Ordering::Relaxed);
            self.fork_count.fetch_add(1, Ordering::Relaxed);
        }

        /// Record execve timing
        #[inline]
        pub fn record_execve(&self, latency_ns: u64) {
            self.execve_latency_ns
                .fetch_add(latency_ns, Ordering::Relaxed);
            self.execve_count.fetch_add(1, Ordering::Relaxed);
        }

        /// Get average syscall latency in microseconds
        pub fn avg_syscall_latency_us(&self) -> u64 {
            let total = self.syscall_latency_ns.load(Ordering::Relaxed);
            let count = self.syscall_count.load(Ordering::Relaxed);
            if count > 0 {
                (total / count) / 1000
            } else {
                0
            }
        }

        /// Get average PTY read latency in microseconds
        pub fn avg_pty_read_latency_us(&self) -> u64 {
            let total = self.pty_read_latency_ns.load(Ordering::Relaxed);
            let count = self.pty_read_count.load(Ordering::Relaxed);
            if count > 0 {
                (total / count) / 1000
            } else {
                0
            }
        }

        /// Get average PTY write latency in microseconds
        pub fn avg_pty_write_latency_us(&self) -> u64 {
            let total = self.pty_write_latency_ns.load(Ordering::Relaxed);
            let count = self.pty_write_count.load(Ordering::Relaxed);
            if count > 0 {
                (total / count) / 1000
            } else {
                0
            }
        }

        /// Get average signal delivery latency in microseconds
        pub fn avg_signal_latency_us(&self) -> u64 {
            let total = self.signal_latency_ns.load(Ordering::Relaxed);
            let count = self.signal_count.load(Ordering::Relaxed);
            if count > 0 {
                (total / count) / 1000
            } else {
                0
            }
        }

        /// Get average context switch latency in microseconds
        pub fn avg_context_switch_latency_us(&self) -> u64 {
            let total = self.context_switch_latency_ns.load(Ordering::Relaxed);
            let count = self.context_switch_count.load(Ordering::Relaxed);
            if count > 0 {
                (total / count) / 1000
            } else {
                0
            }
        }

        /// Get average fork latency in milliseconds
        pub fn avg_fork_latency_ms(&self) -> u64 {
            let total = self.fork_latency_ns.load(Ordering::Relaxed);
            let count = self.fork_count.load(Ordering::Relaxed);
            if count > 0 {
                (total / count) / 1_000_000
            } else {
                0
            }
        }

        /// Get average execve latency in milliseconds
        pub fn avg_execve_latency_ms(&self) -> u64 {
            let total = self.execve_latency_ns.load(Ordering::Relaxed);
            let count = self.execve_count.load(Ordering::Relaxed);
            if count > 0 {
                (total / count) / 1_000_000
            } else {
                0
            }
        }
    }

    /// Global timing statistics
    pub static TIMING: TimingStats = TimingStats::new();

    /// Get a reference to the global timing stats
    #[inline]
    pub fn timing() -> &'static TimingStats {
        &TIMING
    }

    /// Simple timer for measuring elapsed time
    /// Uses TSC (Time Stamp Counter) for high-resolution timing
    pub struct Timer {
        start: u64,
    }

    impl Timer {
        /// Start a new timer
        #[inline]
        pub fn start() -> Self {
            Self { start: read_tsc() }
        }

        /// Get elapsed time in nanoseconds (approximate)
        #[inline]
        pub fn elapsed_ns(&self) -> u64 {
            let end = read_tsc();
            let cycles = end.wrapping_sub(self.start);
            // Assume 2.4 GHz CPU for now (cycles / 2.4 = nanoseconds)
            // TODO: Calibrate TSC frequency at boot
            cycles * 10 / 24
        }
    }

    /// Read the Time Stamp Counter
    #[inline]
    fn read_tsc() -> u64 {
        unsafe {
            let low: u32;
            let high: u32;
            core::arch::asm!(
                "rdtsc",
                out("eax") low,
                out("edx") high,
                options(nomem, nostack)
            );
            ((high as u64) << 32) | (low as u64)
        }
    }
}
