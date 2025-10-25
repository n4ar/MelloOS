//! Virtual File System (VFS) Layer
//!
//! This module provides a unified interface for all filesystem operations in MelloOS.
//! It implements trait-based abstractions that allow multiple filesystem types to coexist
//! and be accessed through a common interface.
//!
//! NOTE: Currently stubbed - requires implementation without alloc crate

pub mod dentry;
pub mod file;
pub mod inode;
pub mod mount;
pub mod path;
pub mod registry;
pub mod superblock;

// Re-export commonly used items
pub use registry::{register_filesystem, lookup_filesystem};
