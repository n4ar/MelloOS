//! Sequence Lock (Seqlock) Implementation
//!
//! A seqlock allows multiple readers to access data concurrently without blocking,
//! while writers get exclusive access. Readers detect concurrent writes by checking
//! a sequence number before and after reading.
//!
//! This is ideal for /proc reads where we want to avoid holding locks during
//! file generation, but need to detect if the process state changed during the read.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Sequence lock for lock-free reads
///
/// Writers increment the sequence number (making it odd) before writing,
/// and increment it again (making it even) after writing. Readers check
/// that the sequence number is even and unchanged before and after reading.
///
/// # Example
/// ```rust,ignore
/// let seqlock = SeqLock::new(42);
///
/// // Writer
/// {
///     let mut guard = seqlock.write();
///     *guard = 100;
/// }
///
/// // Reader (lock-free)
/// let value = seqlock.read(|data| *data);
/// ```
pub struct SeqLock<T> {
    /// Sequence number (odd = write in progress, even = stable)
    seq: AtomicUsize,
    /// Protected data
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SeqLock<T> {}
unsafe impl<T: Send> Send for SeqLock<T> {}

impl<T> SeqLock<T> {
    /// Create a new seqlock with the given initial value
    pub const fn new(data: T) -> Self {
        Self {
            seq: AtomicUsize::new(0),
            data: UnsafeCell::new(data),
        }
    }

    /// Read the data without blocking
    ///
    /// This function calls the provided closure with a reference to the data.
    /// If a concurrent write is detected, it retries automatically.
    ///
    /// # Arguments
    /// * `f` - Closure that reads the data and returns a value
    ///
    /// # Returns
    /// The value returned by the closure
    ///
    /// # Note
    /// The closure may be called multiple times if concurrent writes occur.
    /// It should be fast and not have side effects.
    pub fn read<F, R>(&self, f: F) -> R
    where
        F: Fn(&T) -> R,
    {
        loop {
            // Read sequence number (must be even for stable data)
            let seq1 = self.seq.load(Ordering::Acquire);
            
            // If odd, a write is in progress - spin until even
            if seq1 & 1 != 0 {
                core::hint::spin_loop();
                continue;
            }

            // Read the data
            let result = unsafe { f(&*self.data.get()) };

            // Check if sequence number changed
            let seq2 = self.seq.load(Ordering::Acquire);
            
            // If sequence unchanged, read was consistent
            if seq1 == seq2 {
                return result;
            }

            // Sequence changed, retry
            core::hint::spin_loop();
        }
    }

    /// Try to read the data without blocking
    ///
    /// This function attempts to read the data once. If a concurrent write
    /// is detected, it returns None instead of retrying.
    ///
    /// # Arguments
    /// * `f` - Closure that reads the data and returns a value
    ///
    /// # Returns
    /// Some(value) if read was successful, None if concurrent write detected
    pub fn try_read<F, R>(&self, f: F) -> Option<R>
    where
        F: Fn(&T) -> R,
    {
        // Read sequence number (must be even for stable data)
        let seq1 = self.seq.load(Ordering::Acquire);
        
        // If odd, a write is in progress
        if seq1 & 1 != 0 {
            return None;
        }

        // Read the data
        let result = unsafe { f(&*self.data.get()) };

        // Check if sequence number changed
        let seq2 = self.seq.load(Ordering::Acquire);
        
        // If sequence unchanged, read was consistent
        if seq1 == seq2 {
            Some(result)
        } else {
            None
        }
    }

    /// Acquire write access to the data
    ///
    /// This function blocks until no other writer is active, then returns
    /// a guard that provides exclusive access to the data.
    ///
    /// # Returns
    /// A guard that provides mutable access to the data
    pub fn write(&self) -> SeqLockWriteGuard<T> {
        // Spin until we can acquire the lock
        loop {
            let seq = self.seq.load(Ordering::Acquire);
            
            // If odd, another writer is active - spin
            if seq & 1 != 0 {
                core::hint::spin_loop();
                continue;
            }

            // Try to increment sequence number (make it odd)
            if self
                .seq
                .compare_exchange(seq, seq + 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                // Successfully acquired write lock
                return SeqLockWriteGuard {
                    seqlock: self,
                    seq: seq + 1,
                };
            }

            // Failed to acquire, retry
            core::hint::spin_loop();
        }
    }

    /// Try to acquire write access without blocking
    ///
    /// # Returns
    /// Some(guard) if write lock was acquired, None if another writer is active
    pub fn try_write(&self) -> Option<SeqLockWriteGuard<T>> {
        let seq = self.seq.load(Ordering::Acquire);
        
        // If odd, another writer is active
        if seq & 1 != 0 {
            return None;
        }

        // Try to increment sequence number (make it odd)
        if self
            .seq
            .compare_exchange(seq, seq + 1, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            // Successfully acquired write lock
            Some(SeqLockWriteGuard {
                seqlock: self,
                seq: seq + 1,
            })
        } else {
            None
        }
    }
}

/// Write guard for SeqLock
///
/// Provides exclusive mutable access to the data. When dropped, increments
/// the sequence number again to signal that the write is complete.
pub struct SeqLockWriteGuard<'a, T> {
    seqlock: &'a SeqLock<T>,
    seq: usize,
}

impl<'a, T> core::ops::Deref for SeqLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.seqlock.data.get() }
    }
}

impl<'a, T> core::ops::DerefMut for SeqLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.seqlock.data.get() }
    }
}

impl<'a, T> Drop for SeqLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        // Increment sequence number again (make it even)
        // This signals that the write is complete
        self.seqlock.seq.store(self.seq + 1, Ordering::Release);
    }
}

// Debug implementations
impl<T: core::fmt::Debug> core::fmt::Debug for SeqLock<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let seq = self.seq.load(Ordering::Relaxed);
        if seq & 1 != 0 {
            write!(f, "SeqLock {{ <write in progress> }}")
        } else {
            // Try to read without blocking
            // Note: We can't use format! in no_std, so just indicate data is available
            write!(f, "SeqLock {{ <data available> }}")
        }
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for SeqLockWriteGuard<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "SeqLockWriteGuard {{ data: {:?} }}", &**self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seqlock_basic() {
        let seqlock = SeqLock::new(42);

        // Read initial value
        let value = seqlock.read(|data| *data);
        assert_eq!(value, 42);

        // Write new value
        {
            let mut guard = seqlock.write();
            *guard = 100;
        }

        // Read new value
        let value = seqlock.read(|data| *data);
        assert_eq!(value, 100);
    }

    #[test]
    fn test_seqlock_try_read() {
        let seqlock = SeqLock::new(42);

        // Try read should succeed
        let value = seqlock.try_read(|data| *data);
        assert_eq!(value, Some(42));
    }

    #[test]
    fn test_seqlock_try_write() {
        let seqlock = SeqLock::new(42);

        // Try write should succeed
        let guard = seqlock.try_write();
        assert!(guard.is_some());
    }
}
