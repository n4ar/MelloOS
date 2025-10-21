pub mod lock_ordering;
/// Synchronization primitives for multi-core support
/// This module provides spinlocks and other synchronization mechanisms
/// required for safe concurrent access to shared data structures.
mod spin;

pub use spin::{IrqSpinLock, IrqSpinLockGuard, SpinLock, SpinLockGuard};
