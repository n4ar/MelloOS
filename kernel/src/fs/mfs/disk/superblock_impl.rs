//! MelloFS Disk SuperBlock Implementation
//!
//! Implements the SuperBlock trait for persistent MelloFS.

use super::super_::MfsSuperblock;
use crate::fs::vfs::superblock::{SuperBlock, StatFs, FsFeatures, FsError};
use crate::fs::vfs::inode::Inode;
use crate::fs::block_dev::BlockDevice;
use crate::sync::SpinLock;
use alloc::sync::Arc;

/// MelloFS Disk SuperBlock
pub struct MfsDiskSuperBlock {
    /// Block device for storage
    device: Arc<dyn BlockDevice>,
    /// Cached superblock data
    superblock: SpinLock<MfsSuperblock>,
    /// Cached root inode
    root_inode: SpinLock<Option<Arc<dyn Inode>>>,
}

impl MfsDiskSuperBlock {
    /// Create a new MfsDiskSuperBlock from a block device
    pub fn new(device: Arc<dyn BlockDevice>) -> Result<Self, FsError> {
        // Read superblock from device
        let mut buffer = alloc::vec![0u8; 4096];
        device.read_sectors(0, 8, &mut buffer)
            .map_err(|_| FsError::IoError)?;
        
        // Parse superblock
        let sb = MfsSuperblock::from_bytes(&buffer)
            .map_err(|_| FsError::InvalidArgument)?;
        
        // Validate
        sb.validate()
            .map_err(|_| FsError::InvalidArgument)?;
        
        crate::serial_println!("[MFS_DISK] Loaded superblock:");
        crate::serial_println!("[MFS_DISK]   Block size: {} bytes", sb.block_size);
        crate::serial_println!("[MFS_DISK]   Total blocks: {}", sb.total_blocks);
        crate::serial_println!("[MFS_DISK]   Free blocks: {}", sb.free_blocks);
        
        Ok(Self {
            device,
            superblock: SpinLock::new(sb),
            root_inode: SpinLock::new(None),
        })
    }
    
    /// Get the block device
    pub fn device(&self) -> &Arc<dyn BlockDevice> {
        &self.device
    }
}

impl SuperBlock for MfsDiskSuperBlock {
    fn root(&self) -> Arc<dyn Inode> {
        let root = self.root_inode.lock();
        
        if let Some(ref inode) = *root {
            return inode.clone();
        }
        
        // TODO: Load root inode from disk
        // For now, panic since we haven't implemented disk inode loading
        panic!("[MFS_DISK] Root inode loading not yet implemented");
    }
    
    fn statfs(&self) -> StatFs {
        let sb = self.superblock.lock();
        
        StatFs {
            f_type: 0x4D465344, // "MFSD" magic
            f_bsize: sb.block_size as u64,
            f_blocks: sb.total_blocks,
            f_bfree: sb.free_blocks,
            f_bavail: sb.free_blocks,
            f_files: 1024, // Placeholder
            f_ffree: 1000,  // Placeholder
            f_namelen: 255,
        }
    }
    
    fn sync(&self) -> Result<(), FsError> {
        // Flush device
        self.device.flush()
            .map_err(|_| FsError::IoError)?;
        
        crate::serial_println!("[MFS_DISK] Filesystem synced to device '{}'", self.device.name());
        Ok(())
    }
    
    fn feature_flags(&self) -> FsFeatures {
        FsFeatures::empty()
    }
}
