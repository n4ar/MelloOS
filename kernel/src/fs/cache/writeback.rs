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
#[allow(dead_code)]
const MIN_WRITEBACK_BATCH: usize = 128 * 1024;

/// Maximum batch size for write-back (1024 KiB = 1 MiB)
#[allow(dead_code)]
const MAX_WRITEBACK_BATCH: usize = 1024 * 1024;

/// Default writeback deadline in milliseconds (30 seconds)
#[allow(dead_code)]
const DEFAULT_WRITEBACK_DEADLINE_MS: u64 = 30_000;

/// Writeback configuration
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
    #[allow(dead_code)]
    pub const fn new() -> Self {
        Self {
            config: RwLock::new(WritebackConfig::default()),
            running: AtomicBool::new(false),
            last_flush: AtomicU64::new(0),
            force_flush: AtomicBool::new(false),
        }
    }

    /// Start the writeback scheduler
    #[allow(dead_code)]
    pub fn start(&self) {
        self.running.store(true, Ordering::Release);
        // TODO: Spawn background flusher thread when task scheduler supports it
    }

    /// Stop the writeback scheduler
    #[allow(dead_code)]
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    /// Check if scheduler is running
    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Trigger immediate flush (for sync syscall)
    #[allow(dead_code)]
    pub fn trigger_flush(&self) {
        self.force_flush.store(true, Ordering::Release);
    }

    /// Check if immediate flush is requested
    #[allow(dead_code)]
    pub fn should_flush_now(&self) -> bool {
        self.force_flush.load(Ordering::Acquire)
    }

    /// Clear flush flag
    #[allow(dead_code)]
    pub fn clear_flush_flag(&self) {
        self.force_flush.store(false, Ordering::Release);
    }

    /// Update last flush timestamp
    #[allow(dead_code)]
    pub fn update_last_flush(&self, timestamp: u64) {
        self.last_flush.store(timestamp, Ordering::Release);
    }

    /// Get last flush timestamp
    #[allow(dead_code)]
    pub fn last_flush(&self) -> u64 {
        self.last_flush.load(Ordering::Acquire)
    }

    /// Check if deadline has passed
    #[allow(dead_code)]
    pub fn is_deadline_passed(&self, current_time: u64) -> bool {
        let last = self.last_flush.load(Ordering::Acquire);
        let config = self.config.read();
        current_time - last >= config.deadline_ms
    }

    /// Get configuration
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn set_config(&self, config: WritebackConfig) {
        *self.config.write() = config;
    }
}

use spin::Once;

/// Global writeback scheduler
#[allow(dead_code)]
static WRITEBACK_SCHEDULER: Once<WritebackScheduler> = Once::new();

/// Get the global writeback scheduler
#[allow(dead_code)]
pub fn get_writeback_scheduler() -> &'static WritebackScheduler {
    WRITEBACK_SCHEDULER.call_once(|| WritebackScheduler::new())
}

/// Coalesce dirty pages into writeback batches
///
/// This function takes a list of dirty pages and groups adjacent pages
/// into batches for efficient I/O.
#[allow(dead_code)]
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
            if batch.inode == inode
                && batch.try_add_page(page_num, page_size, config.max_batch_size)
            {
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

/// Flush dirty pages to block device
///
/// Writes a batch of dirty pages to the underlying block device.
///
/// # Arguments
/// * `device` - Block device to write to
/// * `inode` - Inode number (for logging)
/// * `start_page` - Starting page number
/// * `pages` - Slice of (page_num, data) tuples to write
///
/// # Returns
/// Number of pages successfully written, or error
fn flush_to_device(
    device: &dyn crate::fs::block_dev::BlockDevice,
    inode: u64,
    pages: &[(u64, [u8; crate::fs::cache::page_cache::CACHE_PAGE_SIZE])],
) -> Result<usize, &'static str> {
    use crate::fs::cache::page_cache::CACHE_PAGE_SIZE;

    if pages.is_empty() {
        return Ok(0);
    }

    let sector_size = device.sector_size() as usize;
    let sectors_per_page = CACHE_PAGE_SIZE / sector_size;

    let mut flushed_count = 0;

    for (page_num, data) in pages {
        // Calculate sector offset for this page
        let sector = page_num * sectors_per_page as u64;

        // Write page to device
        match device.write_sectors(sector, sectors_per_page as u32, data) {
            Ok(()) => {
                flushed_count += 1;
            }
            Err(e) => {
                crate::serial_println!(
                    "[WRITEBACK] Error flushing page {} of inode {}: {:?}",
                    page_num,
                    inode,
                    e
                );
                // Continue trying to flush other pages
            }
        }
    }

    // Flush device write cache
    if let Err(e) = device.flush() {
        crate::serial_println!(
            "[WRITEBACK] Warning: device flush failed for inode {}: {:?}",
            inode,
            e
        );
        // Don't fail the entire operation if device flush fails
    }

    if flushed_count > 0 {
        crate::serial_println!(
            "[WRITEBACK] Flushed {} pages for inode {}",
            flushed_count,
            inode
        );
    }

    Ok(flushed_count)
}

/// Flush dirty pages for a specific inode
///
/// This is called by the background flusher or on explicit sync.
/// It retrieves dirty pages from the page cache, writes them to the
/// block device, and marks them as clean.
///
/// # Arguments
/// * `inode` - Inode number to flush
///
/// # Returns
/// Ok(()) on success, Err with description on failure
#[allow(dead_code)]
pub fn flush_inode_pages(inode: u64) -> Result<(), &'static str> {
    use crate::fs::block_dev::block_device_manager;
    use crate::fs::cache::page_cache::get_page_cache;

    // Get the page cache for this inode
    let page_cache = get_page_cache();
    let cache_idx = match page_cache.get_file_cache(inode) {
        Some(idx) => idx,
        None => {
            // No cache for this inode, nothing to flush
            return Ok(());
        }
    };

    let file_cache = match page_cache.get_cache(cache_idx) {
        Some(cache) => cache,
        None => return Err("Invalid cache index"),
    };

    // Check if there are any dirty pages
    let dirty_count = file_cache.dirty_count();
    if dirty_count == 0 {
        return Ok(());
    }

    // Get all dirty pages (we'll flush all of them)
    let dirty_pages = file_cache.get_dirty_pages(0, u64::MAX);
    if dirty_pages.is_empty() {
        return Ok(());
    }

    // Get the block device (assume first device for now)
    let device = match block_device_manager().get_device(0) {
        Some(dev) => dev,
        None => {
            crate::serial_println!("[WRITEBACK] No block device available for inode {}", inode);
            return Err("No block device available");
        }
    };

    // Flush pages to device
    let flushed = flush_to_device(device.as_ref(), inode, &dirty_pages)?;

    // Mark flushed pages as clean in the cache
    for (page_num, _) in &dirty_pages[..flushed] {
        file_cache.mark_clean(*page_num);
    }

    Ok(())
}

/// Flush all dirty pages
///
/// This is called on sync syscall. It iterates through all file caches
/// and flushes dirty pages for each inode.
///
/// # Returns
/// Ok(()) on success, Err with description on failure
#[allow(dead_code)]
pub fn flush_all_pages() -> Result<(), &'static str> {
    use crate::fs::cache::page_cache::get_page_cache;

    let page_cache = get_page_cache();
    let mut total_flushed = 0;
    let mut errors = 0;

    // Iterate through all file caches
    for idx in 0..64 {
        // MAX_CACHED_FILES = 64
        if let Some(file_cache) = page_cache.get_cache(idx) {
            if !file_cache.is_in_use() {
                continue;
            }

            let inode = file_cache.inode();
            let dirty_count = file_cache.dirty_count();

            if dirty_count > 0 {
                match flush_inode_pages(inode) {
                    Ok(()) => {
                        total_flushed += dirty_count;
                    }
                    Err(e) => {
                        crate::serial_println!("[WRITEBACK] Error flushing inode {}: {}", inode, e);
                        errors += 1;
                    }
                }
            }
        }
    }

    if total_flushed > 0 {
        crate::serial_println!(
            "[WRITEBACK] Flushed {} total dirty pages ({} errors)",
            total_flushed,
            errors
        );
    }

    if errors > 0 {
        Err("Some pages failed to flush")
    } else {
        Ok(())
    }
}
