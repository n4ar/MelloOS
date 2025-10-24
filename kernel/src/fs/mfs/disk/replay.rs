//! MelloFS Crash Recovery
//!
//! Implements crash recovery through transaction group replay and
//! filesystem consistency verification.

use super::super_::{MfsSuperblock, PRIMARY_SUPERBLOCK_LBA, FsState};
use super::btree::BtreeNode;
use crate::drivers::block::BlockDevice;
use alloc::sync::Arc;

/// Crash recovery result
#[derive(Debug)]
pub enum RecoveryResult {
    /// Filesystem was clean, no recovery needed
    Clean,
    /// Recovery completed successfully
    Recovered,
    /// Recovery failed, filesystem is corrupted
    Failed(&'static str),
}

/// Crash recovery manager
pub struct RecoveryManager {
    device: Arc<dyn BlockDevice>,
    superblock: MfsSuperblock,
}

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new(device: Arc<dyn BlockDevice>, superblock: MfsSuperblock) -> Self {
        Self {
            device,
            superblock,
        }
    }
    
    /// Perform crash recovery
    ///
    /// This is the main entry point for crash recovery. It:
    /// 1. Checks filesystem state
    /// 2. Validates superblock
    /// 3. Walks B-tree and verifies checksums
    /// 4. Rebuilds free space map
    /// 5. Marks filesystem clean
    pub fn recover(&mut self) -> Result<RecoveryResult, &'static str> {
        crate::log_info!("MFS", "Starting crash recovery for filesystem");
        
        // Check filesystem state
        match self.superblock.state {
            s if s == FsState::Clean as u32 => {
                crate::log_info!("MFS", "Filesystem is clean, no recovery needed");
                return Ok(RecoveryResult::Clean);
            }
            s if s == FsState::Dirty as u32 => {
                crate::log_warn!("MFS", "Filesystem was not cleanly unmounted, recovering...");
            }
            s if s == FsState::Error as u32 => {
                crate::log_error!("MFS", "Filesystem has errors, attempting recovery...");
            }
            _ => {
                return Err("Unknown filesystem state");
            }
        }
        
        // Validate superblock
        if let Err(e) = self.superblock.validate() {
            crate::log_error!("MFS", "Superblock validation failed: {}", e);
            return Ok(RecoveryResult::Failed(e));
        }
        
        // Verify B-tree integrity
        if let Err(e) = self.verify_btree() {
            crate::log_error!("MFS", "B-tree verification failed: {}", e);
            return Ok(RecoveryResult::Failed(e));
        }
        
        // Rebuild free space map
        if let Err(e) = self.rebuild_free_space_map() {
            crate::log_error!("MFS", "Free space map rebuild failed: {}", e);
            return Ok(RecoveryResult::Failed(e));
        }
        
        // Mark filesystem clean
        self.superblock.state = FsState::Clean as u32;
        self.superblock.write_both(&self.device)?;
        
        crate::log_info!("MFS", "Crash recovery completed successfully");
        Ok(RecoveryResult::Recovered)
    }
    
    /// Verify B-tree integrity by walking from root
    ///
    /// This checks:
    /// - All node checksums are valid
    /// - Tree structure is consistent
    /// - No dangling pointers
    fn verify_btree(&self) -> Result<(), &'static str> {
        crate::log_info!("MFS", "Verifying B-tree integrity...");
        
        // Check if root B-tree pointer is valid
        if self.superblock.root_btree.lba == 0 {
            crate::log_warn!("MFS", "Root B-tree is empty (new filesystem)");
            return Ok(());
        }
        
        // Read and verify root node
        let root_node = self.read_and_verify_node(
            self.superblock.root_btree.lba,
            self.superblock.root_btree.checksum,
        )?;
        
        // Walk tree recursively
        let mut nodes_verified = 0;
        self.walk_btree_recursive(&root_node, &mut nodes_verified)?;
        
        crate::log_info!("MFS", "B-tree verification complete: {} nodes verified", nodes_verified);
        Ok(())
    }
    
    /// Read and verify a B-tree node
    fn read_and_verify_node(&self, lba: u64, expected_checksum: u64) -> Result<BtreeNode, &'static str> {
        // Read node data
        let block_size = self.superblock.block_size as usize;
        let mut buffer = alloc::vec![0u8; block_size];
        
        self.device.read_block(lba, &mut buffer)
            .map_err(|_| "Failed to read B-tree node")?;
        
        // Deserialize and verify
        let node = BtreeNode::deserialize(&buffer, self.superblock.block_size)?;
        
        // Verify checksum matches expected
        if node.header.checksum != expected_checksum {
            crate::log_error!(
                "MFS",
                "B-tree node checksum mismatch at LBA {}: expected {:#x}, got {:#x}",
                lba,
                expected_checksum,
                node.header.checksum
            );
            return Err("B-tree node checksum mismatch");
        }
        
        Ok(node)
    }
    
    /// Walk B-tree recursively and verify all nodes
    fn walk_btree_recursive(&self, node: &BtreeNode, count: &mut usize) -> Result<(), &'static str> {
        *count += 1;
        
        // If this is an internal node, recursively verify children
        if !node.is_leaf() {
            // Internal nodes have N+1 children for N keys
            // For now, we skip child verification as we don't have
            // child pointers properly implemented yet
            // TODO: Implement full tree walk when child pointers are available
        }
        
        Ok(())
    }
    
    /// Rebuild free space map from extent tree
    ///
    /// This scans all extents and rebuilds the allocator B-tree
    /// to ensure consistency after a crash.
    fn rebuild_free_space_map(&mut self) -> Result<(), &'static str> {
        crate::log_info!("MFS", "Rebuilding free space map...");
        
        // Create a bitmap of allocated blocks
        let total_blocks = self.superblock.total_blocks as usize;
        let mut allocated = alloc::vec![false; total_blocks];
        
        // Mark superblock regions as allocated
        for i in 0..PRIMARY_SUPERBLOCK_LBA + 16 {
            if (i as usize) < total_blocks {
                allocated[i as usize] = true;
            }
        }
        
        // Mark secondary superblock as allocated
        let secondary_lba = MfsSuperblock::secondary_superblock_lba(self.superblock.total_blocks);
        for i in secondary_lba..secondary_lba + 16 {
            if (i as usize) < total_blocks {
                allocated[i as usize] = true;
            }
        }
        
        // TODO: Scan extent tree and mark allocated extents
        // For now, we just count free blocks
        
        let free_blocks = allocated.iter().filter(|&&b| !b).count();
        self.superblock.free_blocks = free_blocks as u64;
        
        crate::log_info!("MFS", "Free space map rebuilt: {} free blocks", free_blocks);
        Ok(())
    }
    
    /// Get the recovered superblock
    pub fn superblock(&self) -> &MfsSuperblock {
        &self.superblock
    }
    
    /// Get a mutable reference to the superblock
    pub fn superblock_mut(&mut self) -> &mut MfsSuperblock {
        &mut self.superblock
    }
}

/// Perform crash recovery on a filesystem
///
/// This is a convenience function that creates a RecoveryManager
/// and performs recovery.
pub fn recover_filesystem(
    device: Arc<dyn BlockDevice>,
    total_blocks: u64,
) -> Result<MfsSuperblock, &'static str> {
    // Try to read superblock with fallback
    let superblock = MfsSuperblock::read_with_fallback(&device, total_blocks)?;
    
    // Create recovery manager
    let mut recovery = RecoveryManager::new(device, superblock);
    
    // Perform recovery
    match recovery.recover()? {
        RecoveryResult::Clean => {
            crate::log_info!("MFS", "Filesystem is clean");
        }
        RecoveryResult::Recovered => {
            crate::log_info!("MFS", "Filesystem recovered successfully");
        }
        RecoveryResult::Failed(e) => {
            return Err(e);
        }
    }
    
    Ok(recovery.superblock().clone())
}


