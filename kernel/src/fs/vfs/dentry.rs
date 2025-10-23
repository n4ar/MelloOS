//! Dentry Cache
//!
//! This module implements the directory entry cache for fast path resolution.
//! The dentry cache stores mappings from (parent_ino, name) to child inodes,
//! with LRU eviction and support for negative entries.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use core::hash::{Hash, Hasher};
use spin::RwLock;
use super::inode::Inode;

/// Maximum number of entries in the dentry cache
const DENTRY_CACHE_SIZE: usize = 4096;

/// Number of hash buckets for fine-grained locking
const HASH_BUCKETS: usize = 256;

/// Dentry cache key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DentryKey {
    parent_ino: u64,
    name_hash: u64,
}

impl DentryKey {
    fn new(parent_ino: u64, name: &str) -> Self {
        Self {
            parent_ino,
            name_hash: Self::hash_name(name),
        }
    }
    
    fn hash_name(name: &str) -> u64 {
        // Simple FNV-1a hash
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in name.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
    
    fn bucket_index(&self) -> usize {
        (self.name_hash as usize) % HASH_BUCKETS
    }
}

/// Dentry cache entry
#[derive(Debug, Clone)]
enum DentryEntry {
    /// Positive entry - name exists and points to this inode
    Positive {
        name: String,
        inode: Arc<dyn Inode>,
        lru_index: usize,
    },
    /// Negative entry - name does not exist (failed lookup)
    Negative {
        name: String,
        lru_index: usize,
    },
}

impl DentryEntry {
    fn name(&self) -> &str {
        match self {
            Self::Positive { name, .. } => name,
            Self::Negative { name, .. } => name,
        }
    }
    
    fn lru_index(&self) -> usize {
        match self {
            Self::Positive { lru_index, .. } => *lru_index,
            Self::Negative { lru_index, .. } => *lru_index,
        }
    }
    
    fn set_lru_index(&mut self, index: usize) {
        match self {
            Self::Positive { lru_index, .. } => *lru_index = index,
            Self::Negative { lru_index, .. } => *lru_index = index,
        }
    }
}

/// Hash bucket for dentry cache
struct DentryBucket {
    entries: BTreeMap<DentryKey, DentryEntry>,
}

impl DentryBucket {
    fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
}

/// LRU list entry
struct LruEntry {
    key: DentryKey,
    bucket_index: usize,
}

/// Global dentry cache
pub struct DentryCache {
    buckets: Vec<RwLock<DentryBucket>>,
    lru: RwLock<Vec<LruEntry>>,
    size: RwLock<usize>,
}

impl DentryCache {
    /// Create a new dentry cache
    pub fn new() -> Self {
        let mut buckets = Vec::with_capacity(HASH_BUCKETS);
        for _ in 0..HASH_BUCKETS {
            buckets.push(RwLock::new(DentryBucket::new()));
        }
        
        Self {
            buckets,
            lru: RwLock::new(Vec::new()),
            size: RwLock::new(0),
        }
    }
    
    /// Look up a dentry in the cache
    ///
    /// Returns:
    /// - Some(Some(inode)) for positive entries
    /// - Some(None) for negative entries
    /// - None for cache miss
    pub fn lookup(&self, parent_ino: u64, name: &str) -> Option<Option<Arc<dyn Inode>>> {
        let key = DentryKey::new(parent_ino, name);
        let bucket_index = key.bucket_index();
        let bucket = self.buckets[bucket_index].read();
        
        if let Some(entry) = bucket.entries.get(&key) {
            // Verify name matches (hash collision check)
            if entry.name() != name {
                return None;
            }
            
            // Update LRU (move to end)
            self.touch_lru(entry.lru_index());
            
            match entry {
                DentryEntry::Positive { inode, .. } => Some(Some(Arc::clone(inode))),
                DentryEntry::Negative { .. } => Some(None),
            }
        } else {
            None
        }
    }
    
    /// Insert a positive dentry (name exists)
    pub fn insert_positive(&self, parent_ino: u64, name: String, inode: Arc<dyn Inode>) {
        let key = DentryKey::new(parent_ino, &name);
        let bucket_index = key.bucket_index();
        
        // Check if we need to evict
        self.maybe_evict();
        
        // Add to LRU
        let lru_index = {
            let mut lru = self.lru.write();
            lru.push(LruEntry {
                key: key.clone(),
                bucket_index,
            });
            lru.len() - 1
        };
        
        // Insert into bucket
        let mut bucket = self.buckets[bucket_index].write();
        bucket.entries.insert(
            key,
            DentryEntry::Positive {
                name,
                inode,
                lru_index,
            },
        );
        
        // Update size
        *self.size.write() += 1;
    }
    
    /// Insert a negative dentry (name does not exist)
    pub fn insert_negative(&self, parent_ino: u64, name: String) {
        let key = DentryKey::new(parent_ino, &name);
        let bucket_index = key.bucket_index();
        
        // Check if we need to evict
        self.maybe_evict();
        
        // Add to LRU
        let lru_index = {
            let mut lru = self.lru.write();
            lru.push(LruEntry {
                key: key.clone(),
                bucket_index,
            });
            lru.len() - 1
        };
        
        // Insert into bucket
        let mut bucket = self.buckets[bucket_index].write();
        bucket.entries.insert(
            key,
            DentryEntry::Negative {
                name,
                lru_index,
            },
        );
        
        // Update size
        *self.size.write() += 1;
    }
    
    /// Invalidate a specific dentry
    pub fn invalidate(&self, parent_ino: u64, name: &str) {
        let key = DentryKey::new(parent_ino, name);
        let bucket_index = key.bucket_index();
        let mut bucket = self.buckets[bucket_index].write();
        
        if let Some(entry) = bucket.entries.remove(&key) {
            // Remove from LRU
            let lru_index = entry.lru_index();
            self.remove_from_lru(lru_index);
            
            // Update size
            *self.size.write() -= 1;
        }
    }
    
    /// Invalidate all dentries for a parent directory
    pub fn invalidate_dir(&self, parent_ino: u64) {
        // We need to scan all buckets since we don't know which names are cached
        for bucket_lock in &self.buckets {
            let mut bucket = bucket_lock.write();
            let keys_to_remove: Vec<_> = bucket
                .entries
                .iter()
                .filter(|(k, _)| k.parent_ino == parent_ino)
                .map(|(k, _)| k.clone())
                .collect();
            
            for key in keys_to_remove {
                if let Some(entry) = bucket.entries.remove(&key) {
                    let lru_index = entry.lru_index();
                    self.remove_from_lru(lru_index);
                    *self.size.write() -= 1;
                }
            }
        }
    }
    
    /// Clear the entire cache
    pub fn clear(&self) {
        for bucket_lock in &self.buckets {
            let mut bucket = bucket_lock.write();
            bucket.entries.clear();
        }
        self.lru.write().clear();
        *self.size.write() = 0;
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> DentryCacheStats {
        let size = *self.size.read();
        DentryCacheStats {
            size,
            capacity: DENTRY_CACHE_SIZE,
        }
    }
    
    // Private helper methods
    
    fn maybe_evict(&self) {
        let size = *self.size.read();
        if size >= DENTRY_CACHE_SIZE {
            self.evict_lru();
        }
    }
    
    fn evict_lru(&self) {
        // Remove the oldest entry (front of LRU list)
        let entry_to_remove = {
            let mut lru = self.lru.write();
            if lru.is_empty() {
                return;
            }
            lru.remove(0)
        };
        
        // Remove from bucket
        let mut bucket = self.buckets[entry_to_remove.bucket_index].write();
        bucket.entries.remove(&entry_to_remove.key);
        
        // Update size
        *self.size.write() -= 1;
        
        // Update LRU indices for remaining entries
        self.reindex_lru();
    }
    
    fn touch_lru(&self, index: usize) {
        let mut lru = self.lru.write();
        if index >= lru.len() {
            return;
        }
        
        // Move entry to end (most recently used)
        let entry = lru.remove(index);
        lru.push(entry);
        
        // Update indices in affected entries
        drop(lru);
        self.reindex_lru();
    }
    
    fn remove_from_lru(&self, index: usize) {
        let mut lru = self.lru.write();
        if index < lru.len() {
            lru.remove(index);
        }
        drop(lru);
        self.reindex_lru();
    }
    
    fn reindex_lru(&self) {
        let lru = self.lru.read();
        for (new_index, lru_entry) in lru.iter().enumerate() {
            let mut bucket = self.buckets[lru_entry.bucket_index].write();
            if let Some(entry) = bucket.entries.get_mut(&lru_entry.key) {
                entry.set_lru_index(new_index);
            }
        }
    }
}

/// Dentry cache statistics
#[derive(Debug, Clone, Copy)]
pub struct DentryCacheStats {
    pub size: usize,
    pub capacity: usize,
}

use spin::Once;

/// Global dentry cache instance
static DENTRY_CACHE: Once<DentryCache> = Once::new();

/// Get the global dentry cache
pub fn dentry_cache() -> &'static DentryCache {
    DENTRY_CACHE.call_once(|| DentryCache::new())
}
