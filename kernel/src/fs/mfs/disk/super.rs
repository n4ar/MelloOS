//! MelloFS Disk Superblock
//!
//! Superblock structure and operations for persistent MelloFS.

use crate::drivers::block::BlockDevice;
use alloc::sync::Arc;
use core::mem;

/// Magic number for MelloFS disk format: "MFSD"
pub const MFS_MAGIC: u32 = 0x4D465344;

/// Current format version
pub const MFS_VERSION: u32 = 1;

/// Superblock location (LBA)
pub const PRIMARY_SUPERBLOCK_LBA: u64 = 16;
pub const PRIMARY_SUPERBLOCK_BLOCKS: u64 = 16;

/// Supported block sizes
pub const BLOCK_SIZE_4K: u32 = 4096;
pub const BLOCK_SIZE_8K: u32 = 8192;
pub const BLOCK_SIZE_16K: u32 = 16384;

/// Filesystem states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FsState {
    Clean = 0x00,
    Dirty = 0x01,
    Error = 0x02,
}

/// B-tree pointer structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BtreePtr {
    /// Physical block address
    pub lba: u64,
    /// Length in blocks
    pub length: u32,
    /// Expected CRC32C checksum
    pub checksum: u64,
    /// Tree level (0 = leaf, >0 = internal)
    pub level: u8,
    /// Reserved padding
    _reserved: [u8; 3],
}

impl BtreePtr {
    pub const fn new() -> Self {
        Self {
            lba: 0,
            length: 0,
            checksum: 0,
            level: 0,
            _reserved: [0; 3],
        }
    }
}

/// MelloFS Superblock
///
/// Size: 256 bytes (fits in single sector)
/// Location: LBA 16-31 (primary), last 16 blocks (secondary)
#[derive(Debug, Clone)]
#[repr(C)]
pub struct MfsSuperblock {
    /// Magic number: 0x4D465344 ("MFSD")
    pub magic: u32,
    /// Format version (1)
    pub version: u32,
    /// Filesystem UUID
    pub uuid: [u8; 16],
    /// Last committed transaction group ID
    pub txg_id: u64,
    
    /// Root B-tree pointer
    pub root_btree: BtreePtr,
    
    /// Allocator B-tree pointer
    pub alloc_btree: BtreePtr,
    
    /// Feature flags (bitfield)
    pub features: u64,
    /// Block size (4096, 8192, or 16384)
    pub block_size: u32,
    /// Reserved padding
    _reserved3: u32,
    
    /// Total filesystem blocks
    pub total_blocks: u64,
    /// Free blocks count
    pub free_blocks: u64,
    
    /// Creation timestamp (Unix epoch ns)
    pub created_time: u64,
    /// Last modification timestamp
    pub modified_time: u64,
    /// Last mount timestamp
    pub mounted_time: u64,
    
    /// Number of times mounted
    pub mount_count: u32,
    /// Filesystem state
    pub state: u32,
    
    /// Filesystem label (UTF-8, null-terminated)
    pub label: [u8; 64],
    
    /// Reserved for future use
    _reserved4: [u8; 48],
    
    /// CRC32C checksum of bytes 0x0000-0x00FF
    pub checksum: u64,
}

impl MfsSuperblock {
    /// Size of superblock structure
    pub const SIZE: usize = 256;
    
    /// Create a new superblock with default values
    pub fn new(block_size: u32, total_blocks: u64) -> Result<Self, &'static str> {
        // Validate block size
        if !Self::is_valid_block_size(block_size) {
            return Err("Invalid block size");
        }
        
        let mut sb = Self {
            magic: MFS_MAGIC,
            version: MFS_VERSION,
            uuid: [0; 16], // Will be set by caller
            txg_id: 0,
            root_btree: BtreePtr::new(),
            alloc_btree: BtreePtr::new(),
            features: 0,
            block_size,
            _reserved3: 0,
            total_blocks,
            free_blocks: total_blocks,
            created_time: 0, // Will be set by caller
            modified_time: 0,
            mounted_time: 0,
            mount_count: 0,
            state: FsState::Clean as u32,
            label: [0; 64],
            _reserved4: [0; 48],
            checksum: 0,
        };
        
        // Compute checksum
        sb.checksum = sb.compute_checksum();
        
        Ok(sb)
    }
    
    /// Check if block size is valid
    pub fn is_valid_block_size(size: u32) -> bool {
        matches!(size, BLOCK_SIZE_4K | BLOCK_SIZE_8K | BLOCK_SIZE_16K)
    }
    
    /// Compute CRC32C checksum of superblock
    pub fn compute_checksum(&self) -> u64 {
        // Create a copy with checksum field zeroed
        let mut sb_copy = self.clone();
        sb_copy.checksum = 0;
        
        // Compute CRC32C of first 256 bytes
        let bytes = unsafe {
            core::slice::from_raw_parts(
                &sb_copy as *const _ as *const u8,
                Self::SIZE,
            )
        };
        
        crc32c(bytes) as u64
    }
    
    /// Verify superblock checksum
    pub fn verify_checksum(&self) -> bool {
        let expected = self.checksum;
        let actual = self.compute_checksum();
        expected == actual
    }
    
    /// Validate superblock structure
    pub fn validate(&self) -> Result<(), &'static str> {
        // Check magic number
        if self.magic != MFS_MAGIC {
            return Err("Invalid magic number");
        }
        
        // Check version
        if self.version != MFS_VERSION {
            return Err("Unsupported version");
        }
        
        // Check block size
        if !Self::is_valid_block_size(self.block_size) {
            return Err("Invalid block size");
        }
        
        // Verify checksum
        if !self.verify_checksum() {
            return Err("Checksum mismatch");
        }
        
        // Check total blocks
        if self.total_blocks == 0 {
            return Err("Invalid total blocks");
        }
        
        // Check free blocks
        if self.free_blocks > self.total_blocks {
            return Err("Invalid free blocks count");
        }
        
        Ok(())
    }
    
    /// Read superblock from block device
    pub fn read_from_device(
        device: &Arc<dyn BlockDevice>,
        lba: u64,
    ) -> Result<Self, &'static str> {
        // Read superblock blocks
        let mut buffer = alloc::vec![0u8; Self::SIZE];
        device.read(lba, &mut buffer)?;
        
        // Parse superblock
        let sb = unsafe {
            core::ptr::read(buffer.as_ptr() as *const MfsSuperblock)
        };
        
        // Validate
        sb.validate()?;
        
        Ok(sb)
    }
    
    /// Write superblock to block device
    pub fn write_to_device(
        &mut self,
        device: &Arc<dyn BlockDevice>,
        lba: u64,
    ) -> Result<(), &'static str> {
        // Update checksum
        self.checksum = self.compute_checksum();
        
        // Convert to bytes
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                Self::SIZE,
            )
        };
        
        // Write to device
        device.write(lba, bytes)?;
        
        Ok(())
    }
    
    /// Set filesystem label
    pub fn set_label(&mut self, label: &str) {
        let bytes = label.as_bytes();
        let len = core::cmp::min(bytes.len(), 63); // Leave room for null terminator
        self.label[..len].copy_from_slice(&bytes[..len]);
        self.label[len] = 0; // Null terminator
    }
    
    /// Get filesystem label
    pub fn get_label(&self) -> &str {
        // Find null terminator
        let len = self.label.iter().position(|&b| b == 0).unwrap_or(64);
        core::str::from_utf8(&self.label[..len]).unwrap_or("")
    }
}

/// CRC32C (Castagnoli) implementation
///
/// Uses software implementation. Hardware acceleration (SSE4.2) can be added later.
fn crc32c(data: &[u8]) -> u32 {
    const POLY: u32 = 0x82F63B78; // Reversed polynomial
    let mut crc: u32 = 0xFFFFFFFF;
    
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ POLY
            } else {
                crc >> 1
            };
        }
    }
    
    crc ^ 0xFFFFFFFF
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_superblock_size() {
        assert_eq!(mem::size_of::<MfsSuperblock>(), MfsSuperblock::SIZE);
    }
    
    #[test]
    fn test_superblock_creation() {
        let sb = MfsSuperblock::new(BLOCK_SIZE_4K, 1000).unwrap();
        assert_eq!(sb.magic, MFS_MAGIC);
        assert_eq!(sb.version, MFS_VERSION);
        assert_eq!(sb.block_size, BLOCK_SIZE_4K);
        assert_eq!(sb.total_blocks, 1000);
        assert_eq!(sb.free_blocks, 1000);
    }
    
    #[test]
    fn test_superblock_checksum() {
        let sb = MfsSuperblock::new(BLOCK_SIZE_4K, 1000).unwrap();
        assert!(sb.verify_checksum());
    }
    
    #[test]
    fn test_invalid_block_size() {
        assert!(MfsSuperblock::new(2048, 1000).is_err());
        assert!(MfsSuperblock::new(8192, 1000).is_ok());
    }
    
    #[test]
    fn test_label() {
        let mut sb = MfsSuperblock::new(BLOCK_SIZE_4K, 1000).unwrap();
        sb.set_label("test_fs");
        assert_eq!(sb.get_label(), "test_fs");
    }
}
