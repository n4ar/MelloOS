//! Superblock Trait and Types
//!
//! This module defines the SuperBlock and FsType traits that filesystem implementations
//! must provide, along with associated types for filesystem features and mount options.

extern crate alloc;

use alloc::sync::Arc;
use alloc::string::String;
use super::inode::Inode;
use super::inode::FsResult;

/// Filesystem type registration and mounting
///
/// Each filesystem implementation registers an FsType that can create
/// SuperBlock instances when the filesystem is mounted.
pub trait FsType: Send + Sync {
    /// Get the filesystem type name (e.g., "mfs_ram", "mfs_disk")
    fn name(&self) -> &'static str;
    
    /// Mount a filesystem
    /// 
    /// # Arguments
    /// * `dev` - Optional block device for persistent filesystems
    /// * `opts` - Mount options
    fn mount(
        &self,
        dev: Option<Arc<dyn crate::drivers::block::BlockDevice>>,
        opts: MountOpts,
    ) -> FsResult<Arc<dyn SuperBlock>>;
}

/// Per-filesystem instance
///
/// Each mounted filesystem has a SuperBlock that provides access to the root inode
/// and filesystem-wide operations.
pub trait SuperBlock: Send + Sync {
    /// Get the root inode of the filesystem
    fn root(&self) -> Arc<dyn Inode>;
    
    /// Get filesystem statistics
    fn statfs(&self) -> FsResult<StatFs>;
    
    /// Sync the filesystem to storage
    fn sync(&self) -> FsResult<()>;
    
    /// Get filesystem feature flags
    fn feature_flags(&self) -> FsFeatures;
    
    /// Get the filesystem ID
    fn fs_id(&self) -> u64;
}

/// Filesystem statistics (for statfs syscall)
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

/// Filesystem feature flags
#[derive(Debug, Clone, Copy, Default)]
pub struct FsFeatures {
    /// Copy-on-Write support
    pub cow: bool,
    /// Checksum support
    pub checksums: bool,
    /// Compression support
    pub compression: bool,
    /// Extended attributes support
    pub xattr: bool,
    /// Inline small files
    pub inline_data: bool,
}

/// Mount options
#[derive(Debug, Clone)]
pub struct MountOpts {
    /// Mount flags
    pub flags: MountFlags,
    /// Block size (for disk filesystems)
    pub block_size: Option<u32>,
    /// Compression type
    pub compress: CompressionType,
    /// Disable access time updates
    pub noatime: bool,
    /// Relative access time updates
    pub relatime: bool,
    /// Disable directory access time updates
    pub nodiratime: bool,
    /// Enable checksums
    pub checksums: bool,
    /// Enable copy-on-write
    pub cow: bool,
    /// TRIM mode for SSDs
    pub trim: TrimMode,
}

impl Default for MountOpts {
    fn default() -> Self {
        Self {
            flags: MountFlags::empty(),
            block_size: None,
            compress: CompressionType::None,
            noatime: false,
            relatime: true,
            nodiratime: false,
            checksums: true,
            cow: true,
            trim: TrimMode::Auto,
        }
    }
}

/// Mount flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountFlags(u64);

impl MountFlags {
    pub const MS_RDONLY: u64 = 1 << 0;       // Read-only mount
    pub const MS_NOSUID: u64 = 1 << 1;       // Ignore suid and sgid bits
    pub const MS_NODEV: u64 = 1 << 2;        // Disallow access to device special files
    pub const MS_NOEXEC: u64 = 1 << 3;       // Disallow program execution
    pub const MS_SYNCHRONOUS: u64 = 1 << 4;  // Writes are synced at once
    pub const MS_REMOUNT: u64 = 1 << 5;      // Alter flags of a mounted FS
    
    pub const fn empty() -> Self {
        Self(0)
    }
    
    pub const fn new(flags: u64) -> Self {
        Self(flags)
    }
    
    pub const fn contains(&self, flag: u64) -> bool {
        (self.0 & flag) != 0
    }
    
    pub fn insert(&mut self, flag: u64) {
        self.0 |= flag;
    }
    
    pub fn remove(&mut self, flag: u64) {
        self.0 &= !flag;
    }
    
    pub const fn is_readonly(&self) -> bool {
        self.contains(Self::MS_RDONLY)
    }
}

/// Compression type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    /// No compression
    None,
    /// LZ4 compression (fast)
    Lz4,
    /// Zstd compression (high ratio)
    Zstd,
}

/// TRIM mode for SSDs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimMode {
    /// TRIM disabled
    Off,
    /// Automatic TRIM
    Auto,
}
