//! Virtual File System (VFS) Layer
//!
//! This module provides a unified interface for all filesystem operations in MelloOS.
//! It implements trait-based abstractions that allow multiple filesystem types to coexist
//! and be accessed through a common interface.

pub mod inode;
pub mod superblock;
pub mod dentry;
pub mod path;
pub mod mount;
pub mod file;

pub use inode::{Inode, FileMode, Stat, DirEnt, DirCookie, SetAttr};
pub use superblock::{FsType, SuperBlock, StatFs, FsFeatures, MountOpts};
