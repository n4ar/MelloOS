//! MelloFS RAM Filesystem
//!
//! Fast in-memory filesystem for boot and temporary storage.
//! Features:
//! - O(log N) directory lookups using BTreeMap
//! - Chunked file storage for efficient memory use
//! - Hardlink and symlink support
//! - Extended attributes
//! - SMP-safe with fine-grained locking

pub mod dir;
pub mod file;
pub mod inode;
pub mod super_impl;
pub mod xattr;

pub use super_impl::MfsRamType;
