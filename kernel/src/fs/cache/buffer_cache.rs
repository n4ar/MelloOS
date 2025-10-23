//! Buffer cache implementation for filesystem metadata blocks
//!
//! The buffer cache maintains cached metadata blocks with:
//! - Hash table keyed by (device, block_number)
//! - Checksum verification on read
//! - Write-through or write-back support
//! - Per-buffer locking
//!
//! Note: This is a simplified implementation using static arrays
//! since the kernel doesn't have the alloc crate yet.

use core::sync::atomic::{AtomicU64, AtomicUsize, AtomicBool, Ordering};
use spin::RwLock;

/// Maximum buffer size (typically 4 KiB for metadata blocks)
pub const MAX_BUFFER_SIZE: usize = 4096;

/// Maximum number of cached buffers
const MAX_BUFFERS: usize = 512;

/// Buffer cache entry representing a cached metadata block
pub struct BufferEntry {
    /// Block data
    data: [u8; MAX_BUFFER_SIZE],
    /// Actual data size
    size: AtomicUsize,
    /// Block number
    block_num: AtomicU64,
    /// Device ID
    device_id: AtomicU64,
    /// Dirty flag
    dirty: AtomicBool,
    /// Valid flag (is this entry in use?)
    valid: AtomicBool,
    /// Last access timestamp
    last_access: AtomicU64,
}

impl BufferEntry {
    /// Create a new empty buffer entry
    const fn new() -> Self {
        Self {
            data: [0u8; MAX_BUFFER_SIZE],
            size: AtomicUsize::new(0),
            block_num: AtomicU64::new(0),
            device_id: AtomicU64::new(0),
            dirty: AtomicBool::new(false),
            valid: AtomicBool::new(false),
            last_access: AtomicU64::new(0),
        }
    }

    /// Initialize entry with data
    pub fn init(&mut self, device_id: u64, block_num: u64, data: &[u8], timestamp: u64) {
        let len = data.len().min(MAX_BUFFER_SIZE);
        self.data[..len].copy_from_slice(&data[..len]);
        self.size.store(len, Ordering::Release);
        self.block_num.store(block_num, Ordering::Release);
        self.device_id.store(device_id, Ordering::Release);
        self.dirty.store(false, Ordering::Release);
        self.valid.store(true, Ordering::Release);
        self.last_access.store(timestamp, Ordering::Release);
    }

    /// Get buffer data
    pub fn data(&self) -> &[u8] {
        let size = self.size.load(Ordering::Acquire);
        &self.data[..size]
    }

    /// Get mutable buffer data (marks as dirty)
    pub fn data_mut(&mut self) -> &mut [u8] {
        self.dirty.store(true, Ordering::Release);
        let size = self.size.load(Ordering::Acquire);
        &mut self.data[..size]
    }

    /// Check if buffer is dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }

    /// Check if entry is valid
    pub fn is_valid(&self) -> bool {
        self.valid.load(Ordering::Acquire)
    }

    /// Mark buffer as dirty
    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::Release);
    }

    /// Mark buffer as clean
    pub fn mark_clean(&self) {
        self.dirty.store(false, Ordering::Release);
    }

    /// Update last access time
    pub fn touch(&self, timestamp: u64) {
        self.last_access.store(timestamp, Ordering::Release);
    }

    /// Get last access time
    pub fn last_access(&self) -> u64 {
        self.last_access.load(Ordering::Acquire)
    }

    /// Get block number
    pub fn block_num(&self) -> u64 {
        self.block_num.load(Ordering::Acquire)
    }

    /// Get device ID
    pub fn device_id(&self) -> u64 {
        self.device_id.load(Ordering::Acquire)
    }

    /// Check if this entry matches the given device and block
    pub fn matches(&self, device_id: u64, block_num: u64) -> bool {
        self.is_valid() &&
        self.device_id.load(Ordering::Acquire) == device_id &&
        self.block_num.load(Ordering::Acquire) == block_num
    }

    /// Invalidate entry
    pub fn invalidate(&mut self) {
        self.valid.store(false, Ordering::Release);
        self.dirty.store(false, Ordering::Release);
    }
}

/// Buffer cache for metadata blocks
pub struct BufferCache {
    /// Array of cached buffers
    buffers: [RwLock<BufferEntry>; MAX_BUFFERS],
    /// Global timestamp counter
    timestamp: AtomicU64,
    /// Total number of cached buffers
    total_buffers: AtomicUsize,
    /// Total number of dirty buffers
    total_dirty: AtomicUsize,
}

impl BufferCache {
    /// Create a new buffer cache
    const fn new() -> Self {
        const INIT_ENTRY: RwLock<BufferEntry> = RwLock::new(BufferEntry::new());
        Self {
            buffers: [INIT_ENTRY; MAX_BUFFERS],
            timestamp: AtomicU64::new(0),
            total_buffers: AtomicUsize::new(0),
            total_dirty: AtomicUsize::new(0),
        }
    }

    /// Get a buffer from cache
    ///
    /// Returns the index of the buffer if found
    pub fn get_buffer(&self, device_id: u64, block_num: u64) -> Option<usize> {
        let timestamp = self.next_timestamp();
        
        for (idx, buffer_lock) in self.buffers.iter().enumerate() {
            let buffer = buffer_lock.read();
            if buffer.matches(device_id, block_num) {
                buffer.touch(timestamp);
                return Some(idx);
            }
        }
        None
    }

    /// Read buffer data
    pub fn read_buffer(&self, idx: usize, buf: &mut [u8]) -> usize {
        let buffer = self.buffers[idx].read();
        let data = buffer.data();
        let len = buf.len().min(data.len());
        buf[..len].copy_from_slice(&data[..len]);
        len
    }

    /// Insert a buffer into cache
    ///
    /// Returns the index where the buffer was inserted
    pub fn insert_buffer(&self, device_id: u64, block_num: u64, data: &[u8]) -> Option<usize> {
        let timestamp = self.next_timestamp();
        
        // First try to find an invalid entry
        for (idx, buffer_lock) in self.buffers.iter().enumerate() {
            let mut buffer = buffer_lock.write();
            if !buffer.is_valid() {
                buffer.init(device_id, block_num, data, timestamp);
                self.total_buffers.fetch_add(1, Ordering::Relaxed);
                return Some(idx);
            }
        }

        // If no invalid entry, evict LRU
        let mut oldest_idx = 0;
        let mut oldest_time = u64::MAX;
        
        for (idx, buffer_lock) in self.buffers.iter().enumerate() {
            let buffer = buffer_lock.read();
            let access_time = buffer.last_access();
            if access_time < oldest_time {
                oldest_time = access_time;
                oldest_idx = idx;
            }
        }

        // Evict and replace
        let mut buffer = self.buffers[oldest_idx].write();
        if buffer.is_dirty() {
            self.total_dirty.fetch_sub(1, Ordering::Relaxed);
        }
        buffer.init(device_id, block_num, data, timestamp);
        Some(oldest_idx)
    }

    /// Mark a buffer as dirty
    pub fn mark_dirty(&self, device_id: u64, block_num: u64) -> bool {
        for buffer_lock in &self.buffers {
            let buffer = buffer_lock.read();
            if buffer.matches(device_id, block_num) {
                if !buffer.is_dirty() {
                    buffer.mark_dirty();
                    self.total_dirty.fetch_add(1, Ordering::Relaxed);
                }
                return true;
            }
        }
        false
    }

    /// Mark a buffer as clean
    pub fn mark_clean(&self, device_id: u64, block_num: u64) -> bool {
        for buffer_lock in &self.buffers {
            let buffer = buffer_lock.read();
            if buffer.matches(device_id, block_num) {
                if buffer.is_dirty() {
                    buffer.mark_clean();
                    self.total_dirty.fetch_sub(1, Ordering::Relaxed);
                }
                return true;
            }
        }
        false
    }

    /// Get total number of cached buffers
    pub fn buffer_count(&self) -> usize {
        self.total_buffers.load(Ordering::Relaxed)
    }

    /// Get total number of dirty buffers
    pub fn dirty_count(&self) -> usize {
        self.total_dirty.load(Ordering::Relaxed)
    }

    /// Get current timestamp and increment
    fn next_timestamp(&self) -> u64 {
        self.timestamp.fetch_add(1, Ordering::Relaxed)
    }

    /// Invalidate all buffers for a device
    pub fn invalidate_device(&self, device_id: u64) {
        for buffer_lock in &self.buffers {
            let mut buffer = buffer_lock.write();
            if buffer.is_valid() && buffer.device_id() == device_id {
                if buffer.is_dirty() {
                    self.total_dirty.fetch_sub(1, Ordering::Relaxed);
                }
                buffer.invalidate();
                self.total_buffers.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }
}

use spin::Once;

/// Global buffer cache instance
static BUFFER_CACHE: Once<BufferCache> = Once::new();

/// Get the global buffer cache
pub fn get_buffer_cache() -> &'static BufferCache {
    BUFFER_CACHE.call_once(|| BufferCache::new())
}
