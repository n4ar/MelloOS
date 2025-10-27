//! MelloFS Disk SuperBlock Implementation
//!
//! Implements the SuperBlock trait for persistent MelloFS.

use super::super_::MfsSuperblock;
use crate::fs::block_dev::BlockDevice;
use crate::fs::vfs::inode::Inode;
use crate::fs::vfs::superblock::{FsError, FsFeatures, StatFs, SuperBlock};
use crate::sync::SpinLock;
use alloc::sync::Arc;
use alloc::vec::Vec;

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
        device
            .read_sectors(0, 8, &mut buffer)
            .map_err(|_| FsError::IoError)?;

        // Parse superblock
        let sb = MfsSuperblock::from_bytes(&buffer).map_err(|_| FsError::InvalidArgument)?;

        // Validate
        sb.validate().map_err(|_| FsError::InvalidArgument)?;

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

    /// Load root inode from disk
    fn load_root_inode_from_disk(&self) -> Result<Arc<dyn Inode>, FsError> {
        let sb = self.superblock.lock();
        
        // Check if root B-tree pointer is valid
        if sb.root_btree.lba == 0 {
            // No root B-tree exists, create a new root directory
            drop(sb);
            return self.create_root_directory();
        }

        // Load root B-tree from disk
        let root_btree_lba = sb.root_btree.lba;
        let root_btree_length = sb.root_btree.length;
        drop(sb);

        // Read root B-tree blocks
        let buffer_size = (root_btree_length as usize) * 4096;
        let mut buffer = alloc::vec![0u8; buffer_size];
        
        self.device
            .read_block(root_btree_lba, &mut buffer)
            .map_err(|_| FsError::IoError)?;

        // Parse root B-tree to find root directory inode
        let root_inode_id = self.find_root_directory_inode(&buffer)?;

        // Load the root directory inode
        self.load_inode_by_id(root_inode_id)
    }

    /// Create a new root directory (for empty filesystem)
    fn create_root_directory(&self) -> Result<Arc<dyn Inode>, FsError> {
        use crate::fs::mfs::disk::inode::MfsDiskInode;
        use crate::fs::vfs::inode::FileMode;

        crate::serial_println!("[MFS_DISK] Creating new root directory");

        // Create root directory inode
        let root_mode = FileMode::new(FileMode::S_IFDIR | 0o755);
        let root_inode = MfsDiskInode::new_directory(
            1, // Root inode ID is always 1
            root_mode,
            0, // uid
            0, // gid
            self.device.clone(),
        )?;

        // Initialize root directory with "." and ".." entries
        root_inode.init_root_directory()?;

        // Update superblock to point to root B-tree
        self.update_root_btree_pointer(1)?;

        crate::serial_println!("[MFS_DISK] Root directory created successfully");

        Ok(Arc::new(root_inode) as Arc<dyn Inode>)
    }

    /// Find root directory inode ID from B-tree data
    fn find_root_directory_inode(&self, btree_data: &[u8]) -> Result<u64, FsError> {
        // Parse B-tree structure to find root directory
        // For simplicity, assume root directory inode ID is always 1
        // In a complete implementation, this would parse the actual B-tree structure
        
        if btree_data.len() < 8 {
            return Err(FsError::InvalidArgument);
        }

        // Read the first 8 bytes as root inode ID
        let root_inode_id = u64::from_le_bytes([
            btree_data[0], btree_data[1], btree_data[2], btree_data[3],
            btree_data[4], btree_data[5], btree_data[6], btree_data[7],
        ]);

        if root_inode_id == 0 {
            // Default to inode ID 1 for root directory
            Ok(1)
        } else {
            Ok(root_inode_id)
        }
    }

    /// Load an inode by its ID
    fn load_inode_by_id(&self, inode_id: u64) -> Result<Arc<dyn Inode>, FsError> {
        use crate::fs::mfs::disk::inode::MfsDiskInode;

        // Calculate inode location on disk
        // Inodes are stored after the superblock area
        let inode_table_start = 32; // LBA 32 (after superblock at LBA 16-31)
        let inode_size = 256; // Each inode is 256 bytes
        let inodes_per_block = 4096 / inode_size; // 16 inodes per 4KB block
        
        let inode_block = inode_table_start + ((inode_id - 1) / inodes_per_block as u64);
        let inode_offset = ((inode_id - 1) % inodes_per_block as u64) * inode_size as u64;

        // Read the block containing the inode
        let mut buffer = alloc::vec![0u8; 4096];
        self.device
            .read_block(inode_block, &mut buffer)
            .map_err(|_| FsError::IoError)?;

        // Extract inode data
        let inode_data = &buffer[inode_offset as usize..(inode_offset as usize + inode_size)];

        // Parse and create inode
        let inode = MfsDiskInode::from_disk_data(inode_id, inode_data, self.device.clone())?;

        crate::serial_println!("[MFS_DISK] Loaded inode {} from disk", inode_id);

        Ok(Arc::new(inode) as Arc<dyn Inode>)
    }

    /// Update superblock root B-tree pointer
    fn update_root_btree_pointer(&self, root_inode_id: u64) -> Result<(), FsError> {
        let mut sb = self.superblock.lock();
        
        // Allocate a block for the root B-tree
        let btree_lba = self.allocate_block()?;
        
        // Create simple B-tree with just the root inode ID
        let mut btree_data = alloc::vec![0u8; 4096];
        btree_data[0..8].copy_from_slice(&root_inode_id.to_le_bytes());
        
        // Write B-tree to disk
        self.device
            .write_block(btree_lba, &btree_data)
            .map_err(|_| FsError::IoError)?;

        // Update superblock
        sb.root_btree.lba = btree_lba;
        sb.root_btree.length = 1;
        sb.root_btree.checksum = self.calculate_checksum(&btree_data);
        sb.root_btree.level = 0;

        // Write updated superblock to disk
        let sb_data = self.superblock_to_bytes(&*sb)?;
        self.device
            .write_block(16, &sb_data) // Primary superblock at LBA 16
            .map_err(|_| FsError::IoError)?;

        crate::serial_println!("[MFS_DISK] Updated root B-tree pointer to LBA {}", btree_lba);

        Ok(())
    }

    /// Allocate a new block on disk
    fn allocate_block(&self) -> Result<u64, FsError> {
        let mut sb = self.superblock.lock();
        
        if sb.free_blocks == 0 {
            return Err(FsError::NoSpace);
        }

        // Simple allocation: use the next available block after inode table
        // In a complete implementation, this would use the allocator B-tree
        let allocated_block = sb.total_blocks - sb.free_blocks;
        sb.free_blocks -= 1;

        crate::serial_println!("[MFS_DISK] Allocated block {}", allocated_block);

        Ok(allocated_block)
    }

    /// Calculate checksum for data
    fn calculate_checksum(&self, data: &[u8]) -> u64 {
        use super::checksum::crc32c_u64;
        crc32c_u64(data)
    }

    /// Convert superblock to bytes for writing to disk
    fn superblock_to_bytes(&self, sb: &MfsSuperblock) -> Result<Vec<u8>, FsError> {
        // Convert superblock structure to bytes
        let bytes = unsafe {
            core::slice::from_raw_parts(
                sb as *const MfsSuperblock as *const u8,
                core::mem::size_of::<MfsSuperblock>(),
            )
        };
        
        let mut result = Vec::with_capacity(4096);
        result.extend_from_slice(bytes);
        
        // Pad to block size
        result.resize(4096, 0);
        
        Ok(result)
    }

    /// Count used inodes in the filesystem
    fn count_used_inodes(&self) -> u64 {
        // Scan inode table to count used inodes
        let inode_table_start = 32; // LBA 32
        let inode_table_blocks = 64; // 64 blocks for inode table
        let inodes_per_block = 4096 / 256; // 16 inodes per block
        let mut used_count = 0;

        for block_idx in 0..inode_table_blocks {
            let block_lba = inode_table_start + block_idx;
            
            // Read inode block
            let mut buffer = alloc::vec![0u8; 4096];
            if self.device.read_block(block_lba, &mut buffer).is_ok() {
                // Check each inode in the block
                for inode_idx in 0..inodes_per_block {
                    let offset = inode_idx * 256;
                    if offset + 256 <= buffer.len() {
                        let inode_data = &buffer[offset..offset + 256];
                        
                        // Check if inode is used (mode != 0)
                        let mode = u16::from_le_bytes([inode_data[0], inode_data[1]]);
                        if mode != 0 {
                            used_count += 1;
                        }
                    }
                }
            }
        }

        used_count
    }
}

impl SuperBlock for MfsDiskSuperBlock {
    fn root(&self) -> Arc<dyn Inode> {
        let mut root = self.root_inode.lock();

        if let Some(ref inode) = *root {
            return inode.clone();
        }

        // Load root inode from disk
        let root_inode = self.load_root_inode_from_disk()
            .expect("Failed to load root inode from disk");

        // Cache the root inode
        *root = Some(root_inode.clone());

        crate::serial_println!("[MFS_DISK] Root inode loaded and cached");
        root_inode
    }

    fn statfs(&self) -> StatFs {
        let sb = self.superblock.lock();

        // Calculate inode statistics
        let inodes_per_block = 4096 / 256; // 16 inodes per 4KB block
        let inode_table_blocks = 64; // Reserve 64 blocks for inode table (1024 inodes)
        let total_inodes = inode_table_blocks * inodes_per_block;
        let used_inodes = self.count_used_inodes();
        let free_inodes = total_inodes.saturating_sub(used_inodes);

        StatFs {
            f_type: 0x4D465344, // "MFSD" magic
            f_bsize: sb.block_size as u64,
            f_blocks: sb.total_blocks,
            f_bfree: sb.free_blocks,
            f_bavail: sb.free_blocks,
            f_files: total_inodes,
            f_ffree: free_inodes,
            f_namelen: 255,
        }
    }

    fn sync(&self) -> Result<(), FsError> {
        // Flush device
        self.device.flush().map_err(|_| FsError::IoError)?;

        crate::serial_println!(
            "[MFS_DISK] Filesystem synced to device '{}'",
            self.device.name()
        );
        Ok(())
    }

    fn feature_flags(&self) -> FsFeatures {
        FsFeatures::empty()
    }
}
