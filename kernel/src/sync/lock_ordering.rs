//! Lock Ordering Documentation and Assertions
//!
//! This module documents the lock ordering rules for the MelloOS kernel
//! to prevent deadlocks in SMP environments.
//!
//! # Lock Hierarchy
//!
//! Locks must be acquired in the following order (from outermost to innermost):
//!
//! 1. **PORT_MANAGER.table_lock** - Port creation/deletion
//! 2. **TASK_TABLE** - Task table access
//! 3. **SCHED** - Scheduler state
//! 4. **Per-CPU runqueue locks** - Must be acquired in CPU ID order (lower ID first)
//! 5. **Per-port locks** - Individual port operations
//! 6. **Per-task locks** - Individual task state (implicit in get_task_mut)
//!
//! # Lock Ordering Rules
//!
//! ## Rule 1: Global before Per-Object
//! Always acquire global locks (PORT_MANAGER, TASK_TABLE, SCHED) before
//! per-object locks (port locks, task locks).
//!
//! ## Rule 2: CPU ID Ordering
//! When acquiring multiple per-CPU runqueue locks (e.g., during task migration),
//! always lock in ascending CPU ID order to prevent deadlocks.
//!
//! Example:
//! ```rust,ignore
//! let (first_cpu, second_cpu) = if from_cpu < to_cpu {
//!     (from_cpu, to_cpu)
//! } else {
//!     (to_cpu, from_cpu)
//! };
//! let lock1 = percpu_for(first_cpu).runqueue.lock();
//! let lock2 = percpu_for(second_cpu).runqueue.lock();
//! ```
//!
//! ## Rule 3: Port before Task
//! When both port and task locks are needed, acquire port lock first.
//!
//! ## Rule 4: Preemption Disable
//! Disable preemption (preempt_disable) before acquiring any spinlock
//! that might be accessed from interrupt context. Re-enable after release.
//!
//! ## Rule 5: No Nested Port Locks
//! Never hold more than one port lock at a time. If multiple ports need
//! to be accessed, release the first lock before acquiring the second.
//!
//! # Common Lock Patterns
//!
//! ## Pattern 1: Task Creation
//! ```rust,ignore
//! let mut sched = SCHED.lock();
//! let mut task_table = TASK_TABLE.lock();
//! // ... create task ...
//! drop(task_table);
//! drop(sched);
//! enqueue_task(task_id, None); // Acquires per-CPU runqueue lock
//! ```
//!
//! ## Pattern 2: IPC Send
//! ```rust,ignore
//! let mut port_mgr = PORT_MANAGER.lock();
//! let port = &mut port_mgr.ports[port_id];
//! preempt_disable();
//! let _lock = port.lock.lock();
//! // ... send message ...
//! drop(_lock);
//! preempt_enable();
//! ```
//!
//! ## Pattern 3: Task Migration
//! ```rust,ignore
//! let (first_cpu, second_cpu) = if from_cpu < to_cpu {
//!     (from_cpu, to_cpu)
//! } else {
//!     (to_cpu, from_cpu)
//! };
//! let mut lock1 = percpu_for(first_cpu).runqueue.lock();
//! let mut lock2 = percpu_for(second_cpu).runqueue.lock();
//! // ... migrate task ...
//! drop(lock2);
//! drop(lock1);
//! ```
//!
//! # Deadlock Prevention
//!
//! To prevent deadlocks:
//! 1. Always follow the lock hierarchy
//! 2. Never acquire locks in reverse order
//! 3. Hold locks for the minimum time necessary
//! 4. Never call functions that might acquire locks while holding locks
//!    (unless the lock ordering is documented and safe)
//! 5. Use try_lock() when appropriate to avoid blocking
//!
//! # Debug Assertions
//!
//! In debug builds, the kernel includes assertions to verify lock ordering:
//! - CPU ID ordering in migrate_task()
//! - No nested port locks
//! - Preemption disabled when required

use core::sync::atomic::{AtomicBool, Ordering};

/// Global flag to track if PORT_MANAGER.table_lock is held (debug only)
#[cfg(debug_assertions)]
static PORT_TABLE_LOCK_HELD: AtomicBool = AtomicBool::new(false);

/// Global flag to track if TASK_TABLE is held (debug only)
#[cfg(debug_assertions)]
static TASK_TABLE_LOCK_HELD: AtomicBool = AtomicBool::new(false);

/// Global flag to track if SCHED is held (debug only)
#[cfg(debug_assertions)]
static SCHED_LOCK_HELD: AtomicBool = AtomicBool::new(false);

/// Assert that no global locks are held
///
/// This should be called before acquiring per-object locks to verify
/// the lock hierarchy is being followed.
#[cfg(debug_assertions)]
pub fn assert_no_global_locks_held() {
    debug_assert!(
        !PORT_TABLE_LOCK_HELD.load(Ordering::Relaxed),
        "PORT_MANAGER.table_lock is held - violates lock ordering"
    );
    debug_assert!(
        !TASK_TABLE_LOCK_HELD.load(Ordering::Relaxed),
        "TASK_TABLE is held - violates lock ordering"
    );
    debug_assert!(
        !SCHED_LOCK_HELD.load(Ordering::Relaxed),
        "SCHED is held - violates lock ordering"
    );
}

/// Assert that CPU IDs are in ascending order
///
/// This should be called when acquiring multiple per-CPU locks to verify
/// they are acquired in the correct order.
#[inline]
pub fn assert_cpu_id_order(first_cpu: usize, second_cpu: usize) {
    debug_assert!(
        first_cpu < second_cpu,
        "CPU IDs must be in ascending order: {} >= {}",
        first_cpu,
        second_cpu
    );
}

/// Mark PORT_MANAGER.table_lock as acquired (debug only)
#[cfg(debug_assertions)]
pub fn mark_port_table_lock_acquired() {
    PORT_TABLE_LOCK_HELD.store(true, Ordering::Relaxed);
}

/// Mark PORT_MANAGER.table_lock as released (debug only)
#[cfg(debug_assertions)]
pub fn mark_port_table_lock_released() {
    PORT_TABLE_LOCK_HELD.store(false, Ordering::Relaxed);
}

/// Mark TASK_TABLE as acquired (debug only)
#[cfg(debug_assertions)]
pub fn mark_task_table_lock_acquired() {
    TASK_TABLE_LOCK_HELD.store(true, Ordering::Relaxed);
}

/// Mark TASK_TABLE as released (debug only)
#[cfg(debug_assertions)]
pub fn mark_task_table_lock_released() {
    TASK_TABLE_LOCK_HELD.store(false, Ordering::Relaxed);
}

/// Mark SCHED as acquired (debug only)
#[cfg(debug_assertions)]
pub fn mark_sched_lock_acquired() {
    SCHED_LOCK_HELD.store(true, Ordering::Relaxed);
}

/// Mark SCHED as released (debug only)
#[cfg(debug_assertions)]
pub fn mark_sched_lock_released() {
    SCHED_LOCK_HELD.store(false, Ordering::Relaxed);
}

// No-op versions for release builds
#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn assert_no_global_locks_held() {}

#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn mark_port_table_lock_acquired() {}

#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn mark_port_table_lock_released() {}

#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn mark_task_table_lock_acquired() {}

#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn mark_task_table_lock_released() {}

#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn mark_sched_lock_acquired() {}

#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn mark_sched_lock_released() {}
