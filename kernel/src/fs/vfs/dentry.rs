//! Dentry Cache
//!
//! This module implements the directory entry cache for fast path resolution.
//! The dentry cache stores mappings from (parent_ino, name) to child inodes,
//! with LRU eviction and support for negative entries.

// TODO: Implement dentry cache without alloc crate
// The kernel doesn't have alloc, so we need to use static arrays instead of Vec/BTreeMap
