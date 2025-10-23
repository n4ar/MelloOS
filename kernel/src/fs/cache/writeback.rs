//! Write-back coalescing and dirty page flushing
//!
//! This module implements:
//! - Background flusher thread for writeback
//! - Batching of adjacent dirty pages (128-1024 KiB)
//! - Deadline-based scheduling (default: 30 seconds)
//! - Sync-triggered immediate flush

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::RwLock;

/// Minimum batch size for write-back (128 KiB)
pub const MIN_WRITEBACK_BATCH: usize = 128 * 1024;

/// Maximum batch size for write-back (1024 KiB = 1 MiB)
pub const MAX_WRITEBACK_BATCH: usize = 1024 * 1024;

/// Default writeback deadline in milliseconds (30 seconds)
pub const DEFAULT_WRITEBACK_DEADLINE_MS: u64 = 30_000;

/// Writeback configuration
pub struct WritebackConfig {
    /// Minimum batch size in bytes
    pub min_batch_size: usize,
    /// Maximum batch size in bytes
    pub max_batch_size: usize,
    /// Writeback deadline in milliseconds
    pub deadline_ms: u64,
    /// Enable immediate flush on sync
    pub sync_immediate: bool,
}

impl WritebackConfig {
    /// Create default writeback configuration
    pub const fn default() -> Self {
        Self {
            min_batch_size: MIN_WRITEBACK_BATCH,
            max_batch_size: MAX_WRITEBACK_BATCH,
            deadline_ms: DEFAULT_WRITEBACK_DEADLINE_MS,
            sync_immediate: true,
        }
    }
}

/// Dirty page descriptor for writeback
#[derive(Clone, Copy)]
pub struct DirtyPage {
    /// Inode number
    pub inode: u64,
    /// Page number within file
    pub page_num: u64,
    /// Timestamp when page became dirty
    pub dirty_time: u64,
}

/// Writeback batch - a group of adjacent dirty pages
#[derive(Clone, Copy)]
pub struct WritebackBatch {
    /// Inode number
    pub inode: u64,
    /// Starting page number
    pub start_page: u64,
    /// Number of pages in batch
    pub page_count: usize,
    /// Total size in bytes
    pub size: usize,
}

impl WritebackBatch {
    /// Create a new writeback batch
    pub fn new(inode: u64, start_page: u64) -> Self {
        Self {
            inode,
            start_page,
            page_count: 0,
            size: 0,
        }
    }

    /// Try to add a page to the batch
    ///
    /// Returns true if the page was added, false if it doesn't fit
    pub fn try_add_page(&mut self, page_num: u64, page_size: usize, max_batch_size: usize) -> bool {
        // Check if page is adjacent
        if page_num != self.start_page + self.page_count as u64 {
            return false;
        }

        // Check if adding this page would exceed max batch size
        if self.size + page_size > max_batch_size {
            return false;
        }

        self.page_count += 1;
        self.size += page_size;
        true
    }

    /// Check if batch is ready for writeback
    pub fn is_ready(&self, min_batch_size: usize) -> bool {
        self.size >= min_batch_size
    }
}

/// Writeback scheduler state
pub struct WritebackScheduler {
    /// Configuration
    config: RwLock<WritebackConfig>,
    /// Is flusher thread running?
    running: AtomicBool,
    /// Last flush timestamp
    last_flush: AtomicU64,
    /// Force immediate flush flag
    force_flush: AtomicBool,
}

impl WritebackScheduler {
    /// Create a new writeback scheduler
    pub const fn new() -> Self {
        Self {
            config: RwLock::new(WritebackConfig::default()),
            running: AtomicBool::new(false),
            last_flush: AtomicU64::new(0),
            force_flush: AtomicBool::new(false),
        }
    }

    /// Start the writeback scheduler
    pub fn start(&self) {
        self.running.store(true, Ordering::Release);
        // TODO: Spawn background flusher thread when task scheduler supports it
    }

    /// Stop the writeback scheduler
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    /// Check if scheduler is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Trigger immediate flush (for sync syscall)
    pub fn trigger_flush(&self) {
        self.force_flush.store(true, Ordering::Release);
    }

    /// Check if immediate flush is requested
    pub fn should_flush_now(&self) -> bool {
        self.force_flush.load(Ordering::Acquire)
    }

    /// Clear flush flag
    pub fn clear_flush_flag(&self) {
        self.force_flush.store(false, Ordering::Release);
    }

    /// Update last flush timestamp
    pub fn update_last_flush(&self, timestamp: u64) {
        self.last_flush.store(timestamp, Ordering::Release);
    }

    /// Get last flush timestamp
    pub fn last_flush(&self) -> u64 {
        self.last_flush.load(Ordering::Acquire)
    }

    /// Check if deadline has passed
    pub fn is_deadline_passed(&self, current_time: u64) -> bool {
        let last = self.last_flush.load(Ordering::Acquire);
        let config = self.config.read();
        current_time - last >= config.deadline_ms
    }

    /// Get configuration
    pub fn config(&self) -> WritebackConfig {
        let config = self.config.read();
        WritebackConfig {
            min_batch_size: config.min_batch_size,
            max_batch_size: config.max_batch_size,
            deadline_ms: config.deadline_ms,
            sync_immediate: config.sync_immediate,
        }
    }

    /// Update configuration
    pub fn set_config(&self, config: WritebackConfig) {
        *self.config.write() = config;
    }
}

use spin::Once;

/// Global writeback scheduler
static WRITEBACK_SCHEDULER: Once<WritebackScheduler> = Once::new();

/// Get the global writeback scheduler
pub fn get_writeback_scheduler() -> &'static WritebackScheduler {
    WRITEBACK_SCHEDULER.call_once(|| WritebackScheduler::new())
}

/// Coalesce dirty pages into writeback batches
///
/// This function takes a list of dirty pages and groups adjacent pages
/// into batches for efficient I/O.
pub fn coalesce_dirty_pages(
    dirty_pages: &[(u64, u64)], // (inode, page_num)
    page_size: usize,
    config: &WritebackConfig,
) -> [Option<WritebackBatch>; 64] {
    let mut batches: [Option<WritebackBatch>; 64] = [None; 64];
    let mut batch_count = 0;

    if dirty_pages.is_empty() {
        return batches;
    }

    // Sort pages by (inode, page_num) for coalescing
    // Note: In a real implementation, we'd use a proper sort
    // For now, we'll process pages in order

    let mut current_batch: Option<WritebackBatch> = None;

    for &(inode, page_num) in dirty_pages {
        if let Some(ref mut batch) = current_batch {
            // Try to add to current batch
            if batch.inode == inode && batch.try_add_page(page_num, page_size, config.max_batch_size) {
                continue;
            }

            // Current batch is full or page is not adjacent
            // Save current batch if it's ready
            if batch.is_ready(config.min_batch_size) && batch_count < 64 {
                batches[batch_count] = Some(*batch);
                batch_count += 1;
            }

            // Start new batch
            current_batch = Some(WritebackBatch::new(inode, page_num));
            if let Some(ref mut batch) = current_batch {
                batch.try_add_page(page_num, page_size, config.max_batch_size);
            }
        } else {
            // Start first batch
            current_batch = Some(WritebackBatch::new(inode, page_num));
            if let Some(ref mut batch) = current_batch {
                batch.try_add_page(page_num, page_size, config.max_batch_size);
            }
        }
    }

    // Save last batch
    if let Some(batch) = current_batch {
        if batch.is_ready(config.min_batch_size) && batch_count < 64 {
            batches[batch_count] = Some(batch);
        }
    }

    batches
}

/// Flush dirty pages for a specific inode
///
/// This is called by the background flusher or on explicit sync
pub fn flush_inode_pages(_inode: u64) -> Result<(), &'static str> {
    // TODO: Implement actual flushing when filesystem is ready
    // This will:
    // 1. Get dirty pages from page cache
    // 2. Coalesce into batches
    // 3. Write batches to disk
    // 4. Mark pages as clean
    Ok(())
}

/// Flush all dirty pages
///
/// This is called on sync syscall
pub fn flush_all_pages() -> Result<(), &'static str> {
    // TODO: Implement when filesystem is ready
    Ok(())
}
