//! System Call and IPC Subsystem
//!
//! This module provides the system call interface and inter-process communication (IPC)
//! mechanisms for MelloOS. It enables userland processes to request kernel services and
//! communicate with each other through message passing.
//!
//! # Components
//!
//! - **syscall**: System call entry point, dispatcher, and handlers
//! - **ipc**: IPC message structures and error types
//! - **port**: Port management and message queuing
//!
//! # System Calls
//!
//! MelloOS provides five system calls in Phase 4:
//!
//! | ID | Name | Description |
//! |----|------|-------------|
//! | 0 | SYS_WRITE | Write data to serial output |
//! | 1 | SYS_EXIT | Terminate current task |
//! | 2 | SYS_SLEEP | Sleep for specified ticks |
//! | 3 | SYS_IPC_SEND | Send message to port |
//! | 4 | SYS_IPC_RECV | Receive message from port (blocking) |
//!
//! # IPC Architecture
//!
//! IPC uses port-based message passing:
//! - 256 ports (0-255) available for communication
//! - Each port has a FIFO message queue (max 16 messages)
//! - Messages are limited to 4096 bytes
//! - Receive operations block if no messages available
//! - FIFO wake policy for blocked tasks
//!
//! # Example Usage
//!
//! ```rust,no_run
//! // Userland code
//! use core::arch::asm;
//!
//! fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
//!     let ret: isize;
//!     unsafe {
//!         asm!(
//!             "int 0x80",
//!             inout("rax") id => ret,
//!             in("rdi") arg1,
//!             in("rsi") arg2,
//!             in("rdx") arg3,
//!             options(nostack)
//!         );
//!     }
//!     ret
//! }
//!
//! // Write to serial
//! let msg = "Hello from userland!\n";
//! syscall(0, 0, msg.as_ptr() as usize, msg.len());
//!
//! // Send IPC message
//! let data = b"ping";
//! syscall(3, 2, data.as_ptr() as usize, data.len());
//!
//! // Receive IPC message (blocking)
//! let mut buf = [0u8; 64];
//! let bytes = syscall(4, 1, buf.as_mut_ptr() as usize, buf.len());
//!
//! // Sleep for 100 ticks
//! syscall(2, 100, 0, 0);
//! ```

pub mod syscall;
pub mod ipc;
pub mod port;

use core::sync::atomic::{AtomicUsize, Ordering};

/// Kernel metrics for observability and debugging
///
/// Tracks various system statistics using atomic counters for thread-safe updates.
/// All counters use relaxed ordering for performance.
///
/// # Metrics Tracked
///
/// - **ctx_switches**: Total context switches (voluntary + preemptive)
/// - **preemptions**: Preemptive context switches only
/// - **syscall_count**: Per-syscall invocation counts (indexed by syscall ID)
/// - **ipc_sends**: Total IPC send operations
/// - **ipc_recvs**: Total IPC receive operations
/// - **ipc_queue_full**: Number of times IPC queue was full
/// - **sleep_count**: Tasks put to sleep
/// - **wake_count**: Tasks woken from sleep
/// - **timer_ticks**: Total timer interrupts
///
/// # Example
///
/// ```rust,no_run
/// use crate::sys::METRICS;
/// use core::sync::atomic::Ordering;
///
/// // Increment context switch counter
/// METRICS.ctx_switches.fetch_add(1, Ordering::Relaxed);
///
/// // Increment syscall counter for SYS_WRITE (ID 0)
/// METRICS.increment_syscall(0);
///
/// // Read current value
/// let switches = METRICS.ctx_switches.load(Ordering::Relaxed);
/// ```
pub struct KernelMetrics {
    pub ctx_switches: AtomicUsize,
    pub preemptions: AtomicUsize,
    pub syscall_count: [AtomicUsize; 5],
    pub ipc_sends: AtomicUsize,
    pub ipc_recvs: AtomicUsize,
    pub ipc_queue_full: AtomicUsize,
    pub sleep_count: AtomicUsize,
    pub wake_count: AtomicUsize,
    pub timer_ticks: AtomicUsize,
}

impl KernelMetrics {
    /// Create a new KernelMetrics instance with all counters initialized to zero
    pub const fn new() -> Self {
        const ATOMIC_ZERO: AtomicUsize = AtomicUsize::new(0);
        Self {
            ctx_switches: ATOMIC_ZERO,
            preemptions: ATOMIC_ZERO,
            syscall_count: [ATOMIC_ZERO; 5],
            ipc_sends: ATOMIC_ZERO,
            ipc_recvs: ATOMIC_ZERO,
            ipc_queue_full: ATOMIC_ZERO,
            sleep_count: ATOMIC_ZERO,
            wake_count: ATOMIC_ZERO,
            timer_ticks: ATOMIC_ZERO,
        }
    }

    /// Increment the syscall counter for a specific syscall ID
    ///
    /// # Arguments
    /// * `syscall_id` - Syscall ID (0-4)
    ///
    /// # Safety
    /// Only increments if syscall_id < 5 to prevent out-of-bounds access
    pub fn increment_syscall(&self, syscall_id: usize) {
        if syscall_id < 5 {
            self.syscall_count[syscall_id].fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Global kernel metrics instance
pub static METRICS: KernelMetrics = KernelMetrics::new();
