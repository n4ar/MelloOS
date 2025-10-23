//! MelloFS RAM Superblock Implementation

use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::fs::vfs::superblock::{FsType, SuperBlock, MountOpts, StatFs, FsFeatures, FsError};
use crate::fs::vfs::inode::{Inode, FileMode};
use crate::fs::mfs::ram::inode::RamInode;

/// MelloFS RAM filesystem type
pub struct MfsRamType;

impl FsType for MfsRamType {
    fn name(&self) -> &'static str {
        "mfs_ram"
    }
    
    fn mount(&self, opts: MountOpts) -> Result<Arc<dyn SuperBlock>, FsError> {
        // Create a new RAM filesystem instance
        let sb = MfsRamSuperBlock::new(opts)?;
        Ok(Arc::new(sb))
    }
}

/// MelloFS RAM superblock
pub struct MfsRamSuperBlock {
    /// Mount options
    opts: MountOpts,
    /// Root inode
    root: Arc<RamInode>,
    /// Next inode number
    next_ino: AtomicU64,
    /// Total memory allocated (in bytes)
    total_bytes: AtomicU64,
    /// Maximum memory limit (0 = unlimited)
    max_bytes: u64,
}

impl MfsRamSuperBlock {
    /// Magic number for mfs_ram
    const MAGIC: u64 = 0x4D46535F52414D00; // "MFS_RAM\0"
    
    /// Default maximum size (1 GiB)
    const DEFAULT_MAX_SIZE: u64 = 1024 * 1024 * 1024;
    
    /// Create a new RAM filesystem
    pub fn new(opts: MountOpts) -> Result<Self, FsError> {
        // Parse mount options for size limit
        let max_bytes = Self::DEFAULT_MAX_SIZE;
        
        // Create root directory inode (ino=1, mode=0755)
        let root_mode = FileMode::new(
            FileMode::S_IFDIR | 
            FileMode::S_IRUSR | FileMode::S_IWUSR | FileMode::S_IXUSR |
            FileMode::S_IRGRP | FileMode::S_IXGRP |
            FileMode::S_IROTH | FileMode::S_IXOTH
        );
        
        let root = RamInode::new_dir(1, root_mode, 0, 0)?;
        
        Ok(Self {
            opts,
            root,
            next_ino: AtomicU64::new(2), // Start from 2 (1 is root)
            total_bytes: AtomicU64::new(0),
            max_bytes,
        })
    }
    
    /// Allocate a new inode number
    pub fn alloc_ino(&self) -> u64 {
        self.next_ino.fetch_add(1, Ordering::SeqCst)
    }
    
    /// Track memory allocation
    pub fn alloc_bytes(&self, bytes: u64) -> Result<(), FsError> {
        let current = self.total_bytes.load(Ordering::Relaxed);
        if self.max_bytes > 0 && current + bytes > self.max_bytes {
            return Err(FsError::NoSpace);
        }
        self.total_bytes.fetch_add(bytes, Ordering::Relaxed);
        Ok(())
    }
    
    /// Track memory deallocation
    pub fn free_bytes(&self, bytes: u64) {
        self.total_bytes.fetch_sub(bytes, Ordering::Relaxed);
    }
}

impl SuperBlock for MfsRamSuperBlock {
    fn root(&self) -> Arc<dyn Inode> {
        self.root.clone()
    }
    
    fn statfs(&self) -> StatFs {
        let total_bytes = if self.max_bytes > 0 {
            self.max_bytes
        } else {
            // If unlimited, report a reasonable value (e.g., 1 GiB)
            Self::DEFAULT_MAX_SIZE
        };
        
        let used_bytes = self.total_bytes.load(Ordering::Relaxed);
        let free_bytes = total_bytes.saturating_sub(used_bytes);
        
        // Use 4 KiB block size for reporting
        const BLOCK_SIZE: u64 = 4096;
        let total_blocks = total_bytes / BLOCK_SIZE;
        let free_blocks = free_bytes / BLOCK_SIZE;
        
        StatFs {
            f_type: Self::MAGIC,
            f_bsize: BLOCK_SIZE,
            f_blocks: total_blocks,
            f_bfree: free_blocks,
            f_bavail: free_blocks,
            f_files: self.next_ino.load(Ordering::Relaxed),
            f_ffree: u64::MAX, // Unlimited inodes
            f_namelen: 255,
        }
    }
    
    fn sync(&self) -> Result<(), FsError> {
        // No-op for RAM filesystem - everything is already in memory
        Ok(())
    }
    
    fn feature_flags(&self) -> FsFeatures {
        FsFeatures::XATTR | FsFeatures::INLINE_SMALL
    }
}
