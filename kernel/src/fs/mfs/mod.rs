//! MelloFS - MelloOS Native Filesystem
//!
//! This module contains the MelloFS implementation, including:
//! - mfs_ram: Fast in-memory filesystem for boot and temporary storage
//! - mfs_disk: Persistent CoW filesystem (future)

pub mod ram;
