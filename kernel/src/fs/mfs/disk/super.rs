//! MelloFS Disk Superblock
//!
//! Superblock structure and operations for persistent MelloFS.

use super::checksum::crc32c_u64;
use crate::drivers::block::BlockDevice;
use alloc::sync::Arc;

/// Get current time in nanoseconds since Unix epoch
///
/// Uses TSC (Time Stamp Counter) for high-resolution timing.
/// Falls back to tick count if TSC is not available.
pub fn current_time_ns() -> u64 {
    // Try to use TSC for high-resolution timing
    if let Some(tsc_ns) = get_tsc_time_ns() {
        return tsc_ns;
    }

    // Fallback to tick-based timing (less accurate but always available)
    get_tick_time_ns()
}

/// Get time from TSC (Time Stamp Counter)
///
/// Returns None if TSC is not available or not calibrated.
fn get_tsc_time_ns() -> Option<u64> {
    // Read TSC
    let tsc = unsafe {
        let mut low: u32;
        let mut high: u32;
        core::arch::asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nomem, nostack)
        );
        ((high as u64) << 32) | (low as u64)
    };

    // TODO: Calibrate TSC frequency on boot
    // For now, assume 2.4 GHz (common frequency)
    // This should be calibrated against PIT or HPET
    const TSC_FREQ_GHZ: u64 = 2400; // 2.4 GHz in MHz

    // Convert TSC ticks to nanoseconds
    // ns = tsc / (freq_ghz)
    // To avoid overflow: ns = (tsc * 1000) / freq_mhz
    let ns = tsc.wrapping_mul(1000) / TSC_FREQ_GHZ;

    Some(ns)
}

/// Get time from system tick counter
///
/// Less accurate than TSC but always available.
/// Assumes 100 Hz timer (10ms per tick).
fn get_tick_time_ns() -> u64 {
    use crate::sched::timer;

    // Get current tick count
    let ticks = timer::get_tick_count() as u64;

    // Convert to nanoseconds (100 Hz = 10ms per tick = 10,000,000 ns per tick)
    const NS_PER_TICK: u64 = 10_000_000;
    ticks * NS_PER_TICK
}

/// Magic number for MelloFS disk format: "MFSD"
pub const MFS_MAGIC: u32 = 0x4D465344;

/// Current format version
pub const MFS_VERSION: u32 = 1;

/// Superblock location (LBA)
pub const PRIMARY_SUPERBLOCK_LBA: u64 = 16;
pub const PRIMARY_SUPERBLOCK_BLOCKS: u64 = 16;

/// Secondary superblock is at the end of the device (last 16 blocks)
/// The actual LBA is computed as: total_blocks - SECONDARY_SUPERBLOCK_BLOCKS

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
        let bytes =
            unsafe { core::slice::from_raw_parts(&sb_copy as *const _ as *const u8, Self::SIZE) };

        crc32c_u64(bytes)
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
    pub fn read_from_device(device: &Arc<dyn BlockDevice>, lba: u64) -> Result<Self, &'static str> {
        // Read superblock blocks
        let mut buffer = alloc::vec![0u8; Self::SIZE];
        device
            .read_block(lba, &mut buffer)
            .map_err(|_| "Failed to read superblock")?;

        // Parse superblock
        let sb = unsafe { core::ptr::read(buffer.as_ptr() as *const MfsSuperblock) };

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
        // Update timestamps
        self.modified_time = current_time_ns();

        // Update checksum
        self.checksum = self.compute_checksum();

        // Convert to bytes
        let bytes =
            unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, Self::SIZE) };

        // Write to device
        device
            .write_block(lba, bytes)
            .map_err(|_| "Failed to write superblock")?;

        Ok(())
    }

    /// Get secondary superblock LBA
    ///
    /// Secondary superblock is stored at the end of the device
    pub fn secondary_superblock_lba(total_blocks: u64) -> u64 {
        if total_blocks > PRIMARY_SUPERBLOCK_BLOCKS {
            total_blocks - PRIMARY_SUPERBLOCK_BLOCKS
        } else {
            0 // Invalid
        }
    }

    /// Write both primary and secondary superblocks
    pub fn write_both(&mut self, device: &Arc<dyn BlockDevice>) -> Result<(), &'static str> {
        // Write primary superblock
        self.write_to_device(device, PRIMARY_SUPERBLOCK_LBA)?;

        // Write secondary superblock
        let secondary_lba = Self::secondary_superblock_lba(self.total_blocks);
        if secondary_lba > 0 {
            self.write_to_device(device, secondary_lba)?;
        }

        Ok(())
    }

    /// Try to read superblock, falling back to secondary if primary fails
    pub fn read_with_fallback(
        device: &Arc<dyn BlockDevice>,
        total_blocks: u64,
    ) -> Result<Self, &'static str> {
        // Try primary superblock first
        match Self::read_from_device(device, PRIMARY_SUPERBLOCK_LBA) {
            Ok(sb) => {
                crate::log_info!("MFS", "Loaded primary superblock");
                return Ok(sb);
            }
            Err(e) => {
                crate::log_warn!("MFS", "Primary superblock failed: {}, trying secondary", e);
            }
        }

        // Try secondary superblock
        let secondary_lba = Self::secondary_superblock_lba(total_blocks);
        if secondary_lba > 0 {
            match Self::read_from_device(device, secondary_lba) {
                Ok(sb) => {
                    crate::log_info!("MFS", "Loaded secondary superblock");
                    return Ok(sb);
                }
                Err(e) => {
                    crate::log_error!("MFS", "Secondary superblock also failed: {}", e);
                }
            }
        }

        Err("Both primary and secondary superblocks failed")
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

impl MfsSuperblock {
    /// Parse superblock from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < Self::SIZE {
            return Err("Buffer too small for superblock");
        }

        // Parse superblock structure
        // For now, use unsafe to cast bytes to struct
        // In production, should use proper deserialization
        let sb = unsafe { core::ptr::read(bytes.as_ptr() as *const Self) };

        // Verify checksum
        if !sb.verify_checksum() {
            return Err("Superblock checksum mismatch");
        }

        Ok(sb)
    }
}
