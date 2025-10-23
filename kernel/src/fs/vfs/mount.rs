//! Mount Table
//!
//! This module implements the global mount point table and mount/umount operations.
//! It tracks all mounted filesystems and provides mount point lookup during path resolution.

// TODO: Implement mount table without alloc crate
// The kernel doesn't have alloc, so we need to use static arrays instead of BTreeMap/Vec
