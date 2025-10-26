//! Page cache implementation for file data caching
//!
//! The page cache maintains per-file radix trees of cached pages with:
//! - Adaptive read-ahead (2-32 pages based on access patterns)
//! - Sequential access detection
//! - Dirty page tracking for write-back
//! - LRU eviction policy
//!
//! Note: This is a simplified implementation using static arrays
//! since the kernel doesn't have the alloc crate yet.

use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use spin::RwLock;

/// Size of a page in bytes (4 KiB)
pub const CACHE_PAGE_SIZE: usize = 4096;

/// Maximum number of cached pages per file
const MAX_PAGES_PER_FILE: usize = 256;

/// Maximum number of files with cached pages
const MAX_CACHED_FILES: usize = 64;

/// Minimum read-ahead window size (pages)
const MIN_READAHEAD: usize = 2;

/// Maximum read-ahead window size (pages)
const MAX_READAHEAD: usize = 32;

/// Number of sequential accesses before growing read-ahead window
const SEQUENTIAL_THRESHOLD: usize = 2;

/// Page cache entry representing a cached page
pub struct PageCacheEntry {
    /// Page data
    data: [u8; CACHE_PAGE_SIZE],
    /// Page number
    page_num: u64,
    /// Dirty flag
    dirty: AtomicBool,
    /// Valid flag (is this entry in use?)
    valid: AtomicBool,
    /// Last access timestamp (for LRU)
    last_access: AtomicU64,
}

impl PageCacheEntry {
    /// Create a new empty page cache entry
    const fn new() -> Self {
        Self {
            data: [0u8; CACHE_PAGE_SIZE],
            page_num: 0,
            dirty: AtomicBool::new(false),
            valid: AtomicBool::new(false),
            last_access: AtomicU64::new(0),
        }
    }

    /// Initialize entry with data
    pub fn init(&mut self, page_num: u64, data: &[u8], timestamp: u64) {
        self.page_num = page_num;
        let len = data.len().min(CACHE_PAGE_SIZE);
        self.data[..len].copy_from_slice(&data[..len]);
        self.dirty.store(false, Ordering::Release);
        self.valid.store(true, Ordering::Release);
        self.last_access.store(timestamp, Ordering::Release);
    }

    /// Get page data
    pub fn data(&self) -> &[u8; CACHE_PAGE_SIZE] {
        &self.data
    }

    /// Get mutable page data (marks as dirty)
    pub fn data_mut(&mut self) -> &mut [u8; CACHE_PAGE_SIZE] {
        self.dirty.store(true, Ordering::Release);
        &mut self.data
    }

    /// Check if page is dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }

    /// Check if entry is valid
    pub fn is_valid(&self) -> bool {
        self.valid.load(Ordering::Acquire)
    }

    /// Mark page as clean
    pub fn mark_clean(&self) {
        self.dirty.store(false, Ordering::Release);
    }

    /// Mark page as dirty
    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::Release);
    }

    /// Update last access time
    pub fn touch(&self, timestamp: u64) {
        self.last_access.store(timestamp, Ordering::Release);
    }

    /// Get last access time
    pub fn last_access(&self) -> u64 {
        self.last_access.load(Ordering::Acquire)
    }

    /// Get page number
    pub fn page_num(&self) -> u64 {
        self.page_num
    }

    /// Invalidate entry
    pub fn invalidate(&mut self) {
        self.valid.store(false, Ordering::Release);
        self.dirty.store(false, Ordering::Release);
    }
}

/// Read-ahead window tracking for adaptive read-ahead
pub struct ReadAheadWindow {
    /// Current window size (in pages)
    size: AtomicUsize,
    /// Last accessed page number
    last_page: AtomicU64,
    /// Has last_page been set?
    has_last_page: AtomicBool,
    /// Number of sequential accesses detected
    sequential_count: AtomicUsize,
}

impl ReadAheadWindow {
    /// Create a new read-ahead window
    const fn new() -> Self {
        Self {
            size: AtomicUsize::new(MIN_READAHEAD),
            last_page: AtomicU64::new(0),
            has_last_page: AtomicBool::new(false),
            sequential_count: AtomicUsize::new(0),
        }
    }

    /// Update window based on access pattern
    ///
    /// Returns the new window size if read-ahead should be triggered
    pub fn update(&self, page_num: u64) -> Option<usize> {
        let has_last = self.has_last_page.load(Ordering::Acquire);
        let last = self.last_page.load(Ordering::Acquire);

        let is_sequential = has_last && page_num == last + 1;

        if is_sequential {
            let count = self.sequential_count.fetch_add(1, Ordering::Relaxed) + 1;

            // Grow window if we've seen enough sequential accesses
            if count >= SEQUENTIAL_THRESHOLD {
                let current_size = self.size.load(Ordering::Relaxed);
                if current_size < MAX_READAHEAD {
                    let new_size = (current_size * 2).min(MAX_READAHEAD);
                    self.size.store(new_size, Ordering::Relaxed);
                }
            }
        } else {
            // Random access detected - reset window
            self.sequential_count.store(0, Ordering::Relaxed);
            self.size.store(MIN_READAHEAD, Ordering::Relaxed);
        }

        self.last_page.store(page_num, Ordering::Release);
        self.has_last_page.store(true, Ordering::Release);

        // Trigger read-ahead if sequential
        if is_sequential {
            Some(self.size.load(Ordering::Relaxed))
        } else {
            None
        }
    }

    /// Get current window size
    pub fn size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    /// Reset window (e.g., after seek)
    pub fn reset(&self) {
        self.size.store(MIN_READAHEAD, Ordering::Relaxed);
        self.has_last_page.store(false, Ordering::Relaxed);
        self.sequential_count.store(0, Ordering::Relaxed);
    }
}

/// Per-file page cache
pub struct FilePageCache {
    /// Inode number for this file
    inode: AtomicU64,
    /// Is this cache entry in use?
    in_use: AtomicBool,
    /// Array of cached pages
    pages: [RwLock<PageCacheEntry>; MAX_PAGES_PER_FILE],
    /// Read-ahead window for this file
    readahead: ReadAheadWindow,
    /// Number of dirty pages
    dirty_count: AtomicUsize,
    /// File size in bytes
    file_size: AtomicU64,
}

impl FilePageCache {
    /// Create a new file page cache
    const fn new() -> Self {
        const INIT_ENTRY: RwLock<PageCacheEntry> = RwLock::new(PageCacheEntry::new());
        Self {
            inode: AtomicU64::new(0),
            in_use: AtomicBool::new(false),
            pages: [INIT_ENTRY; MAX_PAGES_PER_FILE],
            readahead: ReadAheadWindow::new(),
            dirty_count: AtomicUsize::new(0),
            file_size: AtomicU64::new(0),
        }
    }

    /// Initialize cache for an inode
    pub fn init(&self, inode: u64) {
        self.inode.store(inode, Ordering::Release);
        self.in_use.store(true, Ordering::Release);
        self.dirty_count.store(0, Ordering::Release);
        self.file_size.store(0, Ordering::Release);
        self.readahead.reset();
    }

    /// Check if this cache is for the given inode
    pub fn is_for_inode(&self, inode: u64) -> bool {
        self.in_use.load(Ordering::Acquire) && self.inode.load(Ordering::Acquire) == inode
    }

    /// Check if cache is in use
    pub fn is_in_use(&self) -> bool {
        self.in_use.load(Ordering::Acquire)
    }

    /// Get a page from cache
    ///
    /// Returns the index of the page if found
    pub fn get_page(&self, page_num: u64, timestamp: u64) -> Option<usize> {
        for (idx, page_lock) in self.pages.iter().enumerate() {
            let page = page_lock.read();
            if page.is_valid() && page.page_num() == page_num {
                page.touch(timestamp);
                return Some(idx);
            }
        }
        None
    }

    /// Read page data
    pub fn read_page(&self, idx: usize, buf: &mut [u8]) -> usize {
        let page = self.pages[idx].read();
        let len = buf.len().min(CACHE_PAGE_SIZE);
        buf[..len].copy_from_slice(&page.data()[..len]);
        len
    }

    /// Insert a page into cache
    ///
    /// Returns the index where the page was inserted, or None if cache is full
    pub fn insert_page(&self, page_num: u64, data: &[u8], timestamp: u64) -> Option<usize> {
        // First try to find an invalid entry
        for (idx, page_lock) in self.pages.iter().enumerate() {
            let mut page = page_lock.write();
            if !page.is_valid() {
                page.init(page_num, data, timestamp);
                return Some(idx);
            }
        }

        // If no invalid entry, evict LRU
        let mut oldest_idx = 0;
        let mut oldest_time = u64::MAX;

        for (idx, page_lock) in self.pages.iter().enumerate() {
            let page = page_lock.read();
            let access_time = page.last_access();
            if access_time < oldest_time {
                oldest_time = access_time;
                oldest_idx = idx;
            }
        }

        // Evict and replace
        let mut page = self.pages[oldest_idx].write();
        if page.is_dirty() {
            self.dirty_count.fetch_sub(1, Ordering::Relaxed);
        }
        page.init(page_num, data, timestamp);
        Some(oldest_idx)
    }

    /// Mark a page as dirty
    pub fn mark_dirty(&self, page_num: u64) -> bool {
        for page_lock in &self.pages {
            let page = page_lock.read();
            if page.is_valid() && page.page_num() == page_num {
                if !page.is_dirty() {
                    page.mark_dirty();
                    self.dirty_count.fetch_add(1, Ordering::Relaxed);
                }
                return true;
            }
        }
        false
    }

    /// Mark a page as clean
    ///
    /// Marks the specified page as clean (not dirty).
    /// This should be called after successfully writing a page to disk.
    ///
    /// # Arguments
    /// * `page_num` - Page number to mark as clean
    ///
    /// # Returns
    /// true if the page was found and marked clean, false otherwise
    pub fn mark_clean(&self, page_num: u64) -> bool {
        for page_lock in &self.pages {
            let page = page_lock.read();
            if page.is_valid() && page.page_num() == page_num {
                if page.is_dirty() {
                    page.mark_clean();
                    self.dirty_count.fetch_sub(1, Ordering::Relaxed);
                }
                return true;
            }
        }
        false
    }

    /// Get number of dirty pages
    pub fn dirty_count(&self) -> usize {
        self.dirty_count.load(Ordering::Relaxed)
    }

    /// Get dirty pages in a range
    ///
    /// Returns a vector of (page_num, data) tuples for all dirty pages
    /// in the specified range.
    ///
    /// # Arguments
    /// * `start_page` - Starting page number (inclusive)
    /// * `end_page` - Ending page number (exclusive)
    ///
    /// # Returns
    /// Vector of (page_num, page_data) tuples for dirty pages in range
    pub fn get_dirty_pages(
        &self,
        start_page: u64,
        end_page: u64,
    ) -> alloc::vec::Vec<(u64, [u8; CACHE_PAGE_SIZE])> {
        let mut dirty_pages = alloc::vec::Vec::new();

        for page_lock in &self.pages {
            let page = page_lock.read();
            if page.is_valid() && page.is_dirty() {
                let page_num = page.page_num();
                if page_num >= start_page && page_num < end_page {
                    let mut data = [0u8; CACHE_PAGE_SIZE];
                    data.copy_from_slice(page.data());
                    dirty_pages.push((page_num, data));
                }
            }
        }

        dirty_pages
    }

    /// Update read-ahead window and get read-ahead size
    pub fn update_readahead(&self, page_num: u64) -> Option<usize> {
        self.readahead.update(page_num)
    }

    /// Reset read-ahead window
    pub fn reset_readahead(&self) {
        self.readahead.reset()
    }

    /// Get current read-ahead window size
    pub fn readahead_size(&self) -> usize {
        self.readahead.size()
    }

    /// Set file size
    pub fn set_file_size(&self, size: u64) {
        self.file_size.store(size, Ordering::Relaxed);
    }

    /// Get file size
    pub fn file_size(&self) -> u64 {
        self.file_size.load(Ordering::Relaxed)
    }

    /// Invalidate all pages
    pub fn invalidate_all(&self) {
        for page_lock in &self.pages {
            let mut page = page_lock.write();
            if page.is_valid() {
                page.invalidate();
            }
        }
        self.dirty_count.store(0, Ordering::Relaxed);
        self.in_use.store(false, Ordering::Release);
    }
}

/// Global page cache managing all file caches
pub struct PageCache {
    /// Array of file page caches
    file_caches: [FilePageCache; MAX_CACHED_FILES],
    /// Global timestamp counter for LRU
    timestamp: AtomicU64,
}

impl PageCache {
    /// Create a new global page cache
    const fn new() -> Self {
        const INIT_CACHE: FilePageCache = FilePageCache::new();
        Self {
            file_caches: [INIT_CACHE; MAX_CACHED_FILES],
            timestamp: AtomicU64::new(0),
        }
    }

    /// Get or create a file page cache
    ///
    /// Returns the index of the cache
    pub fn get_file_cache(&self, inode: u64) -> Option<usize> {
        // First try to find existing cache
        for (idx, cache) in self.file_caches.iter().enumerate() {
            if cache.is_for_inode(inode) {
                return Some(idx);
            }
        }

        // Try to find an unused cache
        for (idx, cache) in self.file_caches.iter().enumerate() {
            if !cache.is_in_use() {
                cache.init(inode);
                return Some(idx);
            }
        }

        // No free cache available
        None
    }

    /// Get cache by index
    pub fn get_cache(&self, idx: usize) -> Option<&FilePageCache> {
        if idx < MAX_CACHED_FILES {
            Some(&self.file_caches[idx])
        } else {
            None
        }
    }

    /// Remove a file cache
    pub fn remove_file_cache(&self, inode: u64) {
        for cache in &self.file_caches {
            if cache.is_for_inode(inode) {
                cache.invalidate_all();
                return;
            }
        }
    }

    /// Get current timestamp and increment
    pub fn next_timestamp(&self) -> u64 {
        self.timestamp.fetch_add(1, Ordering::Relaxed)
    }
}

use spin::Once;

/// Global page cache instance
static PAGE_CACHE: Once<PageCache> = Once::new();

/// Get the global page cache
pub fn get_page_cache() -> &'static PageCache {
    PAGE_CACHE.call_once(|| PageCache::new())
}
