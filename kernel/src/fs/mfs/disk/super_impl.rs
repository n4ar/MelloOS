//! MelloFS Disk Filesystem Implementation
//!
//! FsType implementation for persistent MelloFS.

use super::super_::MfsSuperblock;
use super::btree::BtreeOps;
use super::keys::*;
use super::extent::ExtentManager;
use super::allocator::{SpaceAllocator, AllocStrategy};
use super::txg::{TxgManager, TxgConfig};

use crate::sync::SpinLock;
use crate::fs::block_dev::{BlockDevice, BlockError};
use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;

/// MelloFS Disk filesystem type
pub struct MfsDiskType;

impl MfsDiskType {
    pub const NAME: &'static str = "mfs_disk";
    
    /// Mount a MelloFS disk filesystem
    ///
    /// This is a simplified implementation that creates the filesystem instance
    /// without full VFS integration (which will be added later).
    pub fn mount_simple(
        device_size: u64,
        block_size: u32,
    ) -> Result<Arc<MfsDiskFs>, &'static str> {
        // Create a dummy superblock for testing
        let sb = MfsSuperblock::new(block_size, device_size / block_size as u64)?;
        
        // Validate
        sb.validate()?;
        
        // Create filesystem instance
        let fs = Arc::new(MfsDiskFs {
            superblock: SpinLock::new(sb.clone()),
            btree_ops: BtreeOps::new(sb.block_size),
            extent_mgr: SpinLock::new(ExtentManager::new(sb.block_size)),
            allocator: SpinLock::new({
                let mut alloc = SpaceAllocator::new(AllocStrategy::BestFit);
                alloc.init(32, sb.total_blocks); // Start after metadata area
                alloc
            }),
            txg_mgr: TxgManager::new(TxgConfig::default()),
            block_device: None, // No block device in simple mount
        });
        
        Ok(fs)
    }
}

/// MelloFS Disk filesystem instance
pub struct MfsDiskFs {
    /// Superblock
    superblock: SpinLock<MfsSuperblock>,
    /// B-tree operations
    btree_ops: BtreeOps,
    /// Extent manager
    extent_mgr: SpinLock<ExtentManager>,
    /// Space allocator
    allocator: SpinLock<SpaceAllocator>,
    /// Transaction group manager
    txg_mgr: TxgManager,
    /// Block device (optional for now)
    block_device: Option<Arc<dyn BlockDevice>>,
}

impl MfsDiskFs {
    /// Lookup inode by number
    pub fn lookup_inode(&self, _ino: u64) -> Result<InodeVal, &'static str> {
        // This is a placeholder implementation
        // Real implementation would:
        // 1. Load root B-tree node from disk
        // 2. Search for inode key
        // 3. Parse and return inode value
        
        Err("Not implemented")
    }
    
    /// Create a new inode
    pub fn create_inode(
        &self,
        mode: u16,
        uid: u32,
        gid: u32,
    ) -> Result<u64, &'static str> {
        // Allocate new inode number
        // (This is simplified; real implementation would track next inode number)
        let ino = 2; // Start from 2 (1 is root)
        
        // Create inode value
        let inode_val = InodeVal::new(mode, uid, gid);
        
        // Insert into B-tree
        // (This is simplified; real implementation would handle CoW and TxG)
        
        Ok(ino)
    }
    
    /// Read directory entries
    pub fn read_dir(&self, _parent_ino: u64) -> Result<Vec<(String, u64, u8)>, &'static str> {
        // This is a placeholder implementation
        // Real implementation would:
        // 1. Search B-tree for all DIR_KEY entries with matching parent_ino
        // 2. Parse directory values
        // 3. Return list of entries
        
        Err("Not implemented")
    }
    
    /// Sync filesystem (commit current transaction group)
    pub fn sync(&self) -> Result<(), &'static str> {
        // Get current time (simplified)
        let current_time = 0u64;
        
        // Begin commit
        if let Some(mut txg) = self.txg_mgr.begin_commit() {
            // Execute commit procedure
            super::txg::TxgCommitProcedure::commit(&mut txg)?;
            
            // Complete commit
            self.txg_mgr.complete_commit(txg);
            
            // Free old blocks
            let old_blocks = self.txg_mgr.collect_old_blocks();
            let mut allocator = self.allocator.lock();
            for extent in old_blocks {
                allocator.free(extent);
            }
        }
        
        Ok(())
    }
}

// Tests would go here but are omitted for kernel code


impl MfsDiskType {
    /// Mount a MelloFS disk filesystem from a block device
    pub fn mount_from_device(
        device: Arc<dyn BlockDevice>,
    ) -> Result<Arc<MfsDiskFs>, &'static str> {
        // Read superblock from device
        let mut sb_buffer = [0u8; 4096]; // Assume 4K block size
        device.read_sectors(0, 8, &mut sb_buffer)
            .map_err(|_| "Failed to read superblock")?;
        
        // Parse superblock
        let sb = MfsSuperblock::from_bytes(&sb_buffer)?;
        
        // Validate
        sb.validate()?;
        
        crate::serial_println!("[MFS_DISK] Mounted filesystem from device '{}'", device.name());
        crate::serial_println!("[MFS_DISK]   Block size: {} bytes", sb.block_size);
        crate::serial_println!("[MFS_DISK]   Total blocks: {}", sb.total_blocks);
        crate::serial_println!("[MFS_DISK]   Free blocks: {}", sb.free_blocks);
        
        // Create filesystem instance
        let fs = Arc::new(MfsDiskFs {
            superblock: SpinLock::new(sb.clone()),
            btree_ops: BtreeOps::new(sb.block_size),
            extent_mgr: SpinLock::new(ExtentManager::new(sb.block_size)),
            allocator: SpinLock::new({
                let mut alloc = SpaceAllocator::new(AllocStrategy::BestFit);
                alloc.init(32, sb.total_blocks);
                alloc
            }),
            txg_mgr: TxgManager::new(TxgConfig::default()),
            block_device: Some(device),
        });
        
        Ok(fs)
    }
}

impl MfsDiskFs {
    /// Get the block device
    pub fn block_device(&self) -> Option<Arc<dyn BlockDevice>> {
        self.block_device.clone()
    }
    
    /// Read block from device
    pub fn read_block(&self, block_num: u64, buffer: &mut [u8]) -> Result<(), BlockError> {
        if let Some(ref device) = self.block_device {
            let sb = self.superblock.lock();
            let sectors_per_block = sb.block_size / device.sector_size();
            let start_sector = block_num * sectors_per_block as u64;
            
            device.read_sectors(start_sector, sectors_per_block, buffer)
        } else {
            Err(BlockError::DeviceNotReady)
        }
    }
    
    /// Write block to device
    pub fn write_block(&self, block_num: u64, buffer: &[u8]) -> Result<(), BlockError> {
        if let Some(ref device) = self.block_device {
            let sb = self.superblock.lock();
            let sectors_per_block = sb.block_size / device.sector_size();
            let start_sector = block_num * sectors_per_block as u64;
            
            device.write_sectors(start_sector, sectors_per_block, buffer)
        } else {
            Err(BlockError::DeviceNotReady)
        }
    }
    
    /// Sync filesystem to device (flush)
    pub fn flush_device(&self) -> Result<(), BlockError> {
        if let Some(ref device) = self.block_device {
            device.flush()
        } else {
            Ok(())
        }
    }
}
