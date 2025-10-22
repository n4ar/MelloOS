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
