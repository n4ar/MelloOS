//! Superblock Trait and Types
//!
//! This module defines the SuperBlock and FsType traits that filesystem implementations
//! must provide, along with associated types for filesystem features and mount options.

use crate::fs::vfs::inode::Inode;
use alloc::string::String;
use alloc::sync::Arc;

/// Filesystem type trait for registration and mounting
pub trait FsType: Send + Sync {
    /// Returns the filesystem type name (e.g., "mfs_ram", "mfs_disk")
    fn name(&self) -> &'static str;

    /// Mount a filesystem instance
    ///
    /// # Arguments
    /// * `opts` - Mount options
    ///
    /// # Returns
    /// A new SuperBlock instance on success
    fn mount(&self, opts: MountOpts) -> Result<Arc<dyn SuperBlock>, FsError>;
}

/// Superblock trait representing a mounted filesystem instance
pub trait SuperBlock: Send + Sync {
    /// Get the root inode of this filesystem
    fn root(&self) -> Arc<dyn Inode>;

    /// Get filesystem statistics
    fn statfs(&self) -> StatFs;

    /// Sync all dirty data and metadata to storage
    fn sync(&self) -> Result<(), FsError>;

    /// Get filesystem feature flags
    fn feature_flags(&self) -> FsFeatures;
}

/// Mount options
#[derive(Debug, Clone)]
pub struct MountOpts {
    pub flags: MountFlags,
    pub data: Option<String>,
}

impl Default for MountOpts {
    fn default() -> Self {
        Self {
            flags: MountFlags::empty(),
            data: None,
        }
    }
}

bitflags::bitflags! {
    /// Mount flags
    #[derive(Debug, Clone, Copy)]
    pub struct MountFlags: u64 {
        const MS_RDONLY = 1 << 0;
        const MS_NOSUID = 1 << 1;
        const MS_NODEV = 1 << 2;
        const MS_NOEXEC = 1 << 3;
        const MS_SYNCHRONOUS = 1 << 4;
        const MS_REMOUNT = 1 << 5;
    }
}

/// Filesystem statistics
#[derive(Debug, Clone, Copy)]
pub struct StatFs {
    /// Filesystem type
    pub f_type: u64,
    /// Optimal transfer block size
    pub f_bsize: u64,
    /// Total data blocks in filesystem
    pub f_blocks: u64,
    /// Free blocks in filesystem
    pub f_bfree: u64,
    /// Free blocks available to unprivileged user
    pub f_bavail: u64,
    /// Total file nodes in filesystem
    pub f_files: u64,
    /// Free file nodes in filesystem
    pub f_ffree: u64,
    /// Maximum length of filenames
    pub f_namelen: u64,
}

bitflags::bitflags! {
    /// Filesystem feature flags
    #[derive(Debug, Clone, Copy)]
    pub struct FsFeatures: u64 {
        const COW = 1 << 0;
        const CHECKSUM = 1 << 1;
        const COMPRESSION = 1 << 2;
        const XATTR = 1 << 3;
        const INLINE_SMALL = 1 << 4;
    }
}

/// Filesystem error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    /// Invalid argument
    InvalidArgument,
    /// Not found
    NotFound,
    /// Already exists
    AlreadyExists,
    /// Permission denied
    PermissionDenied,
    /// Not a directory
    NotADirectory,
    /// Is a directory
    IsADirectory,
    /// No space left on device
    NoSpace,
    /// I/O error
    IoError,
    /// Out of memory
    OutOfMemory,
    /// Bad address
    BadAddress,
    /// Too many symbolic links
    TooManySymlinks,
    /// Name too long
    NameTooLong,
    /// Read-only filesystem
    ReadOnlyFilesystem,
    /// Too many open files
    TooManyOpenFiles,
    /// Not supported
    NotSupported,
}
