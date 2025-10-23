/// SpinLock implementation for multi-core synchronization
///
/// This module provides a spinlock primitive that uses atomic operations
/// and exponential backoff to efficiently synchronize access to shared data
/// across multiple CPU cores.
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// A mutual exclusion primitive useful for protecting shared data
///
/// This spinlock will block threads waiting for the lock to become available.
/// The lock is automatically released when the guard goes out of scope.
///
/// # Examples
///
/// ```
/// let lock = SpinLock::new(5);
/// {
///     let mut data = lock.lock();
///     *data += 1;
/// } // lock is released here
/// ```
pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

/// A guard that provides mutable access to the data protected by a SpinLock
///
/// When the guard is dropped, the lock is automatically released.
pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// Creates a new spinlock wrapping the supplied data
    pub const fn new(data: T) -> Self {
        SpinLock {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquires the lock, blocking the current thread until it is available
    ///
    /// This function will block until the lock is acquired. It uses exponential
    /// backoff to reduce bus contention when multiple cores are competing for
    /// the same lock.
    ///
    /// Returns a guard that will automatically release the lock when dropped.
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        let mut backoff = 1;
        const MAX_BACKOFF: usize = 256;

        loop {
            // Try to acquire the lock using compare_exchange
            // Use Acquire ordering to ensure all subsequent reads see the latest data
            if self
                .locked
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return SpinLockGuard { lock: self };
            }

            // Lock is held by another core, spin with exponential backoff
            for _ in 0..backoff {
                core::hint::spin_loop();
            }

            // Exponential backoff: double the wait time up to MAX_BACKOFF
            if backoff < MAX_BACKOFF {
                backoff *= 2;
            }
        }
    }

    /// Attempts to acquire the lock without blocking
    ///
    /// Returns `Some(SpinLockGuard)` if the lock was successfully acquired,
    /// or `None` if the lock is currently held by another thread.
    ///
    /// This function does not block and will return immediately.
    pub fn try_lock(&self) -> Option<SpinLockGuard<'_, T>> {
        // Try to acquire the lock once
        // Use Acquire ordering to ensure all subsequent reads see the latest data
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SpinLockGuard { lock: self })
        } else {
            None
        }
    }

    /// Attempts to acquire the lock with a timeout
    ///
    /// Returns `Some(SpinLockGuard)` if the lock was successfully acquired
    /// within the timeout period, or `None` if the timeout expired.
    ///
    /// # Arguments
    /// * `timeout_ms` - Maximum time to wait in milliseconds
    ///
    /// This function uses exponential backoff and checks the timeout periodically.
    pub fn try_lock_timeout(&self, timeout_ms: u64) -> Option<SpinLockGuard<'_, T>> {
        // Get current timestamp (assuming we have a TSC-based timer)
        let start = unsafe { core::arch::x86_64::_rdtsc() };
        // Approximate TSC frequency (2.4 GHz typical)
        // This is a rough estimate; real implementation should use calibrated value
        const TSC_PER_MS: u64 = 2_400_000;
        let timeout_tsc = timeout_ms * TSC_PER_MS;

        let mut backoff = 1;
        const MAX_BACKOFF: usize = 256;

        loop {
            // Try to acquire the lock
            if self
                .locked
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return Some(SpinLockGuard { lock: self });
            }

            // Check if timeout expired
            let now = unsafe { core::arch::x86_64::_rdtsc() };
            if now - start >= timeout_tsc {
                return None;
            }

            // Spin with exponential backoff
            for _ in 0..backoff {
                core::hint::spin_loop();
            }

            // Exponential backoff: double the wait time up to MAX_BACKOFF
            if backoff < MAX_BACKOFF {
                backoff *= 2;
            }
        }
    }

    /// Consumes the lock and returns the underlying data
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        // Release the lock using Release ordering to ensure all writes
        // are visible to the next thread that acquires the lock
        self.lock.locked.store(false, Ordering::Release);
    }
}

// Debug implementations for better diagnostics
impl<T: core::fmt::Debug> core::fmt::Debug for SpinLock<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "SpinLock {{ data: {:?} }}", &*guard),
            None => write!(f, "SpinLock {{ <locked> }}"),
        }
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for SpinLockGuard<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "SpinLockGuard {{ data: {:?} }}", &**self)
    }
}

/// An IRQ-safe spinlock that disables interrupts while the lock is held
///
/// This variant of SpinLock saves the interrupt flag state (RFLAGS) and
/// disables interrupts when acquiring the lock. When the guard is dropped,
/// the original interrupt state is restored.
///
/// This is necessary when the same lock might be accessed from both normal
/// code and interrupt handlers, preventing deadlocks.
///
/// # Examples
///
/// ```
/// let lock = IrqSpinLock::new(5);
/// {
///     let mut data = lock.lock();
///     *data += 1;
/// } // lock is released and interrupts are restored here
/// ```
pub struct IrqSpinLock<T> {
    inner: SpinLock<T>,
}

/// A guard that provides mutable access to data protected by an IrqSpinLock
///
/// When the guard is dropped, the lock is released and the saved interrupt
/// state is restored.
pub struct IrqSpinLockGuard<'a, T> {
    guard: SpinLockGuard<'a, T>,
    flags: u64,
}

unsafe impl<T: Send> Sync for IrqSpinLock<T> {}
unsafe impl<T: Send> Send for IrqSpinLock<T> {}

impl<T> IrqSpinLock<T> {
    /// Creates a new IRQ-safe spinlock wrapping the supplied data
    pub const fn new(data: T) -> Self {
        IrqSpinLock {
            inner: SpinLock::new(data),
        }
    }

    /// Acquires the lock, disabling interrupts
    ///
    /// This function saves the current RFLAGS register (including the interrupt
    /// flag), disables interrupts, and then acquires the lock. When the returned
    /// guard is dropped, the lock is released and the original interrupt state
    /// is restored.
    ///
    /// Returns a guard that will automatically release the lock and restore
    /// interrupts when dropped.
    pub fn lock(&self) -> IrqSpinLockGuard<'_, T> {
        // Save current RFLAGS register
        let flags = unsafe { save_flags() };

        // Disable interrupts
        unsafe { disable_interrupts() };

        // Acquire the inner spinlock
        let guard = self.inner.lock();

        IrqSpinLockGuard { guard, flags }
    }

    /// Attempts to acquire the lock without blocking, disabling interrupts
    ///
    /// Returns `Some(IrqSpinLockGuard)` if the lock was successfully acquired,
    /// or `None` if the lock is currently held by another thread.
    ///
    /// If the lock cannot be acquired, interrupts are not disabled and the
    /// original interrupt state is preserved.
    pub fn try_lock(&self) -> Option<IrqSpinLockGuard<'_, T>> {
        // Save current RFLAGS register
        let flags = unsafe { save_flags() };

        // Disable interrupts
        unsafe { disable_interrupts() };

        // Try to acquire the inner spinlock
        match self.inner.try_lock() {
            Some(guard) => Some(IrqSpinLockGuard { guard, flags }),
            None => {
                // Failed to acquire lock, restore interrupts
                unsafe { restore_flags(flags) };
                None
            }
        }
    }

    /// Consumes the lock and returns the underlying data
    pub fn into_inner(self) -> T {
        self.inner.into_inner()
    }
}

impl<T> Deref for IrqSpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.guard
    }
}

impl<T> DerefMut for IrqSpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.guard
    }
}

impl<T> Drop for IrqSpinLockGuard<'_, T> {
    fn drop(&mut self) {
        // The inner SpinLockGuard will be dropped automatically, releasing the lock
        // Now restore the saved interrupt state
        unsafe { restore_flags(self.flags) };
    }
}

// Debug implementations for IRQ-safe variants
impl<T: core::fmt::Debug> core::fmt::Debug for IrqSpinLock<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "IrqSpinLock {{ data: {:?} }}", &*guard),
            None => write!(f, "IrqSpinLock {{ <locked> }}"),
        }
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for IrqSpinLockGuard<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "IrqSpinLockGuard {{ data: {:?} }}", &**self)
    }
}

/// Save the current RFLAGS register value
///
/// Returns the RFLAGS value which includes the interrupt enable flag (IF)
#[inline]
unsafe fn save_flags() -> u64 {
    let flags: u64;
    core::arch::asm!(
        "pushfq",
        "pop {flags}",
        flags = out(reg) flags,
        options(nomem, preserves_flags)
    );
    flags
}

/// Disable interrupts by clearing the interrupt flag (IF) in RFLAGS
#[inline]
unsafe fn disable_interrupts() {
    core::arch::asm!("cli", options(nomem, nostack));
}

/// Restore the RFLAGS register to a previously saved value
///
/// This restores the interrupt enable flag (IF) to its previous state
#[inline]
unsafe fn restore_flags(flags: u64) {
    core::arch::asm!(
        "push {flags}",
        "popfq",
        flags = in(reg) flags,
        options(nomem, preserves_flags)
    );
}
