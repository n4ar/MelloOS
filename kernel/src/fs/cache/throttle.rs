//! Dirty page throttling
//!
//! This module implements throttling mechanisms to prevent excessive dirty pages:
//! - Per-filesystem dirty page limits (e.g., 10% of cache)
//! - Global dirty page limit (e.g., 20% of RAM)
//! - Writer slowdown when limits exceeded

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use spin::RwLock;

/// Default per-filesystem dirty page limit (10% of cache)
pub const DEFAULT_PER_FS_DIRTY_PERCENT: usize = 10;

/// Default global dirty page limit (20% of RAM)
pub const DEFAULT_GLOBAL_DIRTY_PERCENT: usize = 20;

/// Throttle configuration
#[derive(Clone, Copy)]
pub struct ThrottleConfig {
    /// Per-filesystem dirty page limit as percentage of cache
    pub per_fs_dirty_percent: usize,
    /// Global dirty page limit as percentage of total memory
    pub global_dirty_percent: usize,
    /// Maximum number of pages per filesystem
    pub max_pages_per_fs: usize,
    /// Total system memory in pages
    pub total_memory_pages: usize,
}

impl ThrottleConfig {
    /// Create default throttle configuration
    pub const fn default() -> Self {
        Self {
            per_fs_dirty_percent: DEFAULT_PER_FS_DIRTY_PERCENT,
            global_dirty_percent: DEFAULT_GLOBAL_DIRTY_PERCENT,
            max_pages_per_fs: 1024,     // Default: 4 MiB per filesystem
            total_memory_pages: 262144, // Default: 1 GiB
        }
    }

    /// Calculate per-filesystem dirty page limit
    pub fn per_fs_limit(&self) -> usize {
        (self.max_pages_per_fs * self.per_fs_dirty_percent) / 100
    }

    /// Calculate global dirty page limit
    pub fn global_limit(&self) -> usize {
        (self.total_memory_pages * self.global_dirty_percent) / 100
    }
}

/// Per-filesystem throttle state
pub struct FilesystemThrottle {
    /// Filesystem ID
    fs_id: AtomicU64,
    /// Is this entry in use?
    in_use: AtomicBool,
    /// Number of dirty pages for this filesystem
    dirty_pages: AtomicUsize,
    /// Number of times throttled
    throttle_count: AtomicUsize,
}

impl FilesystemThrottle {
    /// Create a new filesystem throttle
    const fn new() -> Self {
        Self {
            fs_id: AtomicU64::new(0),
            in_use: AtomicBool::new(false),
            dirty_pages: AtomicUsize::new(0),
            throttle_count: AtomicUsize::new(0),
        }
    }

    /// Initialize for a filesystem
    pub fn init(&self, fs_id: u64) {
        self.fs_id.store(fs_id, Ordering::Release);
        self.in_use.store(true, Ordering::Release);
        self.dirty_pages.store(0, Ordering::Release);
        self.throttle_count.store(0, Ordering::Release);
    }

    /// Check if this is for the given filesystem
    pub fn is_for_fs(&self, fs_id: u64) -> bool {
        self.in_use.load(Ordering::Acquire) && self.fs_id.load(Ordering::Acquire) == fs_id
    }

    /// Check if in use
    pub fn is_in_use(&self) -> bool {
        self.in_use.load(Ordering::Acquire)
    }

    /// Increment dirty page count
    pub fn inc_dirty(&self) -> usize {
        self.dirty_pages.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Decrement dirty page count
    pub fn dec_dirty(&self) -> usize {
        self.dirty_pages
            .fetch_sub(1, Ordering::Relaxed)
            .saturating_sub(1)
    }

    /// Get dirty page count
    pub fn dirty_count(&self) -> usize {
        self.dirty_pages.load(Ordering::Relaxed)
    }

    /// Increment throttle count
    pub fn inc_throttle(&self) {
        self.throttle_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get throttle count
    pub fn throttle_count(&self) -> usize {
        self.throttle_count.load(Ordering::Relaxed)
    }

    /// Reset
    pub fn reset(&self) {
        self.in_use.store(false, Ordering::Release);
        self.dirty_pages.store(0, Ordering::Release);
        self.throttle_count.store(0, Ordering::Release);
    }
}

/// Maximum number of filesystems to track
const MAX_FILESYSTEMS: usize = 16;

/// Global throttle manager
pub struct ThrottleManager {
    /// Configuration
    config: RwLock<ThrottleConfig>,
    /// Per-filesystem throttle state
    fs_throttles: [FilesystemThrottle; MAX_FILESYSTEMS],
    /// Global dirty page count
    global_dirty: AtomicUsize,
    /// Global throttle count
    global_throttle_count: AtomicUsize,
}

impl ThrottleManager {
    /// Create a new throttle manager
    const fn new() -> Self {
        const INIT_THROTTLE: FilesystemThrottle = FilesystemThrottle::new();
        Self {
            config: RwLock::new(ThrottleConfig::default()),
            fs_throttles: [INIT_THROTTLE; MAX_FILESYSTEMS],
            global_dirty: AtomicUsize::new(0),
            global_throttle_count: AtomicUsize::new(0),
        }
    }

    /// Register a filesystem
    pub fn register_filesystem(&self, fs_id: u64) -> Option<usize> {
        // Check if already registered
        for (idx, throttle) in self.fs_throttles.iter().enumerate() {
            if throttle.is_for_fs(fs_id) {
                return Some(idx);
            }
        }

        // Find free slot
        for (idx, throttle) in self.fs_throttles.iter().enumerate() {
            if !throttle.is_in_use() {
                throttle.init(fs_id);
                return Some(idx);
            }
        }

        None
    }

    /// Unregister a filesystem
    pub fn unregister_filesystem(&self, fs_id: u64) {
        for throttle in &self.fs_throttles {
            if throttle.is_for_fs(fs_id) {
                let dirty = throttle.dirty_count();
                self.global_dirty.fetch_sub(dirty, Ordering::Relaxed);
                throttle.reset();
                return;
            }
        }
    }

    /// Mark a page as dirty
    ///
    /// Returns true if the write should be throttled
    pub fn mark_dirty(&self, fs_id: u64) -> bool {
        // Update per-filesystem count
        let mut fs_dirty = 0;
        for throttle in &self.fs_throttles {
            if throttle.is_for_fs(fs_id) {
                fs_dirty = throttle.inc_dirty();
                break;
            }
        }

        // Update global count
        let global_dirty = self.global_dirty.fetch_add(1, Ordering::Relaxed) + 1;

        // Check limits
        let config = self.config.read();
        let per_fs_limit = config.per_fs_limit();
        let global_limit = config.global_limit();

        let should_throttle = fs_dirty > per_fs_limit || global_dirty > global_limit;

        if should_throttle {
            // Update throttle counters
            for throttle in &self.fs_throttles {
                if throttle.is_for_fs(fs_id) {
                    throttle.inc_throttle();
                    break;
                }
            }
            self.global_throttle_count.fetch_add(1, Ordering::Relaxed);
        }

        should_throttle
    }

    /// Mark a page as clean
    pub fn mark_clean(&self, fs_id: u64) {
        // Update per-filesystem count
        for throttle in &self.fs_throttles {
            if throttle.is_for_fs(fs_id) {
                throttle.dec_dirty();
                break;
            }
        }

        // Update global count
        self.global_dirty.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get global dirty page count
    pub fn global_dirty_count(&self) -> usize {
        self.global_dirty.load(Ordering::Relaxed)
    }

    /// Get per-filesystem dirty page count
    pub fn fs_dirty_count(&self, fs_id: u64) -> usize {
        for throttle in &self.fs_throttles {
            if throttle.is_for_fs(fs_id) {
                return throttle.dirty_count();
            }
        }
        0
    }

    /// Get global throttle count
    pub fn global_throttle_count(&self) -> usize {
        self.global_throttle_count.load(Ordering::Relaxed)
    }

    /// Get per-filesystem throttle count
    pub fn fs_throttle_count(&self, fs_id: u64) -> usize {
        for throttle in &self.fs_throttles {
            if throttle.is_for_fs(fs_id) {
                return throttle.throttle_count();
            }
        }
        0
    }

    /// Check if global limit is exceeded
    pub fn is_global_limit_exceeded(&self) -> bool {
        let config = self.config.read();
        let global_dirty = self.global_dirty.load(Ordering::Relaxed);
        global_dirty > config.global_limit()
    }

    /// Check if per-filesystem limit is exceeded
    pub fn is_fs_limit_exceeded(&self, fs_id: u64) -> bool {
        let config = self.config.read();
        let per_fs_limit = config.per_fs_limit();

        for throttle in &self.fs_throttles {
            if throttle.is_for_fs(fs_id) {
                return throttle.dirty_count() > per_fs_limit;
            }
        }
        false
    }

    /// Get configuration
    pub fn config(&self) -> ThrottleConfig {
        *self.config.read()
    }

    /// Update configuration
    pub fn set_config(&self, config: ThrottleConfig) {
        *self.config.write() = config;
    }
}

use core::sync::atomic::AtomicBool;
use spin::Once;

/// Global throttle manager
static THROTTLE_MANAGER: Once<ThrottleManager> = Once::new();

/// Get the global throttle manager
pub fn get_throttle_manager() -> &'static ThrottleManager {
    THROTTLE_MANAGER.call_once(|| ThrottleManager::new())
}

/// Throttle a writer by yielding CPU
///
/// This is called when dirty page limits are exceeded
pub fn throttle_writer() {
    // TODO: Implement proper yielding when scheduler supports it
    // For now, just spin a bit to slow down the writer
    for _ in 0..1000 {
        core::hint::spin_loop();
    }
}
