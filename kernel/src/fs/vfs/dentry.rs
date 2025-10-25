//! Dentry Cache
//!
//! This module implements the directory entry cache for fast path resolution.
//! The dentry cache stores mappings from (parent_ino, name) to child inodes,
//! with LRU eviction and support for negative entries.

use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex as SpinLock;

/// Number of hash buckets in the dentry cache
const DENTRY_BUCKETS: usize = 256;

/// Maximum entries per bucket (for LRU eviction)
const MAX_ENTRIES_PER_BUCKET: usize = 16;

/// A single dentry cache entry
#[derive(Clone, Debug)]
struct DentryEntry {
    /// Parent inode number
    parent_ino: u64,
    /// Component name
    name: String,
    /// Child inode number (0 for negative entry)
    child_ino: u64,
    /// Is this a negative entry (file not found)?
    negative: bool,
    /// Access counter for LRU (higher = more recently used)
    access_count: u64,
}

/// A single bucket in the dentry cache
struct DentryBucket {
    /// Entries in this bucket
    entries: Vec<DentryEntry>,
    /// Global access counter for LRU
    access_counter: u64,
}

impl DentryBucket {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            access_counter: 0,
        }
    }
    
    /// Lookup an entry in this bucket
    fn lookup(&mut self, parent_ino: u64, name: &str) -> Option<u64> {
        for entry in &mut self.entries {
            if entry.parent_ino == parent_ino && entry.name == name {
                // Update access count for LRU
                self.access_counter += 1;
                entry.access_count = self.access_counter;
                
                // Return child ino (0 for negative entry)
                return if entry.negative {
                    Some(0)
                } else {
                    Some(entry.child_ino)
                };
            }
        }
        None
    }
    
    /// Insert an entry into this bucket
    fn insert(&mut self, parent_ino: u64, name: String, child_ino: u64, negative: bool) {
        // Check if entry already exists
        for entry in &mut self.entries {
            if entry.parent_ino == parent_ino && entry.name == name {
                // Update existing entry
                entry.child_ino = child_ino;
                entry.negative = negative;
                self.access_counter += 1;
                entry.access_count = self.access_counter;
                return;
            }
        }
        
        // Add new entry
        self.access_counter += 1;
        let new_entry = DentryEntry {
            parent_ino,
            name,
            child_ino,
            negative,
            access_count: self.access_counter,
        };
        
        self.entries.push(new_entry);
        
        // Evict LRU entry if bucket is full
        if self.entries.len() > MAX_ENTRIES_PER_BUCKET {
            self.evict_lru();
        }
    }
    
    /// Evict the least recently used entry
    fn evict_lru(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        
        // Find entry with lowest access count
        let mut lru_idx = 0;
        let mut lru_count = self.entries[0].access_count;
        
        for (idx, entry) in self.entries.iter().enumerate().skip(1) {
            if entry.access_count < lru_count {
                lru_idx = idx;
                lru_count = entry.access_count;
            }
        }
        
        self.entries.remove(lru_idx);
    }
    
    /// Invalidate all entries for a given parent inode
    fn invalidate(&mut self, parent_ino: u64) {
        self.entries.retain(|e| e.parent_ino != parent_ino);
    }
}

/// Global dentry cache
struct DentryCache {
    /// Hash buckets
    buckets: [SpinLock<DentryBucket>; DENTRY_BUCKETS],
}

impl DentryCache {
    /// Create a new dentry cache
    const fn new() -> Self {
        const INIT: SpinLock<DentryBucket> = SpinLock::new(DentryBucket {
            entries: Vec::new(),
            access_counter: 0,
        });
        
        Self {
            buckets: [INIT; DENTRY_BUCKETS],
        }
    }
    
    /// Compute hash for (parent_ino, name) pair
    /// Simple FNV-1a hash function
    fn hash(parent_ino: u64, name: &str) -> usize {
        const FNV_OFFSET: u64 = 14695981039346656037;
        const FNV_PRIME: u64 = 1099511628211;
        
        let mut hash = FNV_OFFSET;
        
        // Hash parent_ino
        for byte in parent_ino.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        
        // Hash name
        for byte in name.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        
        (hash as usize) % DENTRY_BUCKETS
    }
    
    /// Lookup an entry in the cache
    ///
    /// Returns:
    /// - Some(child_ino) if found (child_ino > 0)
    /// - Some(0) if negative entry (file not found)
    /// - None if not in cache
    pub fn lookup(&self, parent_ino: u64, name: &str) -> Option<u64> {
        let bucket_idx = Self::hash(parent_ino, name);
        let mut bucket = self.buckets[bucket_idx].lock();
        bucket.lookup(parent_ino, name)
    }
    
    /// Insert an entry into the cache
    ///
    /// # Arguments
    /// * `parent_ino` - Parent inode number
    /// * `name` - Component name
    /// * `child_ino` - Child inode number (must be > 0)
    pub fn insert(&self, parent_ino: u64, name: &str, child_ino: u64) {
        if child_ino == 0 {
            // Don't cache invalid inode numbers
            return;
        }
        
        let bucket_idx = Self::hash(parent_ino, name);
        let mut bucket = self.buckets[bucket_idx].lock();
        bucket.insert(parent_ino, name.into(), child_ino, false);
    }
    
    /// Insert a negative entry (file not found)
    ///
    /// # Arguments
    /// * `parent_ino` - Parent inode number
    /// * `name` - Component name that was not found
    pub fn insert_negative(&self, parent_ino: u64, name: &str) {
        let bucket_idx = Self::hash(parent_ino, name);
        let mut bucket = self.buckets[bucket_idx].lock();
        bucket.insert(parent_ino, name.into(), 0, true);
    }
    
    /// Invalidate all entries for a given parent inode
    ///
    /// This should be called when a directory is modified (create, unlink, rename)
    ///
    /// # Arguments
    /// * `parent_ino` - Parent inode number to invalidate
    pub fn invalidate(&self, parent_ino: u64) {
        // Invalidate all buckets (parent entries could be in any bucket)
        for bucket in &self.buckets {
            let mut b = bucket.lock();
            b.invalidate(parent_ino);
        }
    }
    
    /// Clear the entire cache
    pub fn clear(&self) {
        for bucket in &self.buckets {
            let mut b = bucket.lock();
            b.entries.clear();
            b.access_counter = 0;
        }
    }
}

/// Global dentry cache instance
static DENTRY_CACHE: DentryCache = DentryCache::new();

/// Lookup an entry in the dentry cache (public API)
///
/// Returns:
/// - Some(child_ino) if found (child_ino > 0)
/// - Some(0) if negative entry (file not found)
/// - None if not in cache
pub fn lookup(parent_ino: u64, name: &str) -> Option<u64> {
    DENTRY_CACHE.lookup(parent_ino, name)
}

/// Insert an entry into the dentry cache (public API)
pub fn insert(parent_ino: u64, name: &str, child_ino: u64) {
    DENTRY_CACHE.insert(parent_ino, name, child_ino)
}

/// Insert a negative entry (file not found) (public API)
pub fn insert_negative(parent_ino: u64, name: &str) {
    DENTRY_CACHE.insert_negative(parent_ino, name)
}

/// Invalidate all entries for a given parent inode (public API)
pub fn invalidate(parent_ino: u64) {
    DENTRY_CACHE.invalidate(parent_ino)
}

/// Clear the entire dentry cache (public API)
pub fn clear() {
    DENTRY_CACHE.clear()
}

#[cfg(test)]
mod tests {
    // Tests will be added once filesystem is operational
    // Test cases:
    // - Insert and lookup
    // - Negative entries
    // - LRU eviction
    // - Invalidation
    // - Hash collision handling
}
