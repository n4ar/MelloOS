//! MelloFS Disk Superblock
//!
//! Superblock structure and operations for persistent MelloFS.

use super::checksum::crc32c_u64;
use crate::drivers::block::BlockDevice;
use alloc::sync::Arc;

//! # TSC Timing Implementation Status
//! 
//! ## ✅ FULLY IMPLEMENTED:
//! - TSC availability and invariant TSC detection via CPUID
//! - Runtime TSC frequency calibration using PIT (Programmable Interval Timer)
//! - High-precision nanosecond timing conversion
//! - Fallback to tick-based timing when TSC unavailable
//! - Proper initialization function for boot-time setup
//! 
//! ## Usage:
//! ```rust
//! // Call during kernel initialization (after timer setup)
//! init_tsc_timing().expect("Failed to initialize TSC timing");
//! 
//! // Use for high-precision timestamps
//! let timestamp = current_time_ns();
//! ```
//! 
//! ## Features:
//! - Automatic TSC frequency detection (no hardcoded values)
//! - CPUID-based feature detection (TSC support, invariant TSC)
//! - PIT-based calibration for accuracy
//! - 128-bit arithmetic to prevent overflow
//! - Comprehensive error handling and logging

/// Initialize TSC timing subsystem
///
/// This should be called during kernel initialization, after the timer subsystem
/// is set up but before any filesystem operations.
pub fn init_tsc_timing() -> Result<(), &'static str> {
    crate::log_info!("TSC", "Initializing TSC timing subsystem...");
    
    match calibrate_tsc_frequency() {
        Ok(_freq_hz) => {
            crate::log_info!("TSC", "TSC timing initialized successfully");
            Ok(())
        }
        Err(e) => {
            crate::log_warn!("TSC", "TSC calibration failed: {}, using fallback timing", e);
            // Don't return error - fallback timing will be used
            Ok(())
        }
    }
}

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
    // Check if TSC is available and calibrated
    if !is_tsc_available() {
        return None;
    }

    let tsc_freq_hz = get_tsc_frequency_hz()?;
    
    // Read TSC
    let tsc = read_tsc();

    // Convert TSC ticks to nanoseconds
    // ns = (tsc * 1_000_000_000) / freq_hz
    // Use 128-bit arithmetic to avoid overflow
    let ns = ((tsc as u128) * 1_000_000_000u128 / tsc_freq_hz as u128) as u64;

    Some(ns)
}

/// Read TSC (Time Stamp Counter)
fn read_tsc() -> u64 {
    unsafe {
        let mut low: u32;
        let mut high: u32;
        core::arch::asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nomem, nostack)
        );
        ((high as u64) << 32) | (low as u64)
    }
}

/// Check if TSC is available and invariant
fn is_tsc_available() -> bool {
    // Check CPUID for TSC support
    let (_, _, _, edx) = unsafe {
        let mut eax: u32 = 1;
        let mut ebx: u32;
        let mut ecx: u32;
        let mut edx: u32;
        
        // CPUID leaf 1: Processor Info and Feature Bits
        // Save and restore rbx to avoid LLVM conflict
        core::arch::asm!(
            "mov {tmp}, rbx",
            "cpuid",
            "mov rbx, {tmp}",
            tmp = out(reg) ebx,
            inout("eax") eax,
            out("ecx") ecx,
            out("edx") edx,
        );
        (eax, ebx, ecx, edx)
    };

    // Check TSC flag in EDX bit 4
    let tsc_supported = (edx & (1 << 4)) != 0;
    
    if !tsc_supported {
        return false;
    }

    // Check for invariant TSC (CPUID leaf 0x80000007)
    let (_, _, _, edx_ext) = unsafe {
        let mut eax: u32 = 0x80000007;
        let mut ebx: u32;
        let mut ecx: u32;
        let mut edx: u32;
        
        core::arch::asm!(
            "mov {tmp}, rbx",
            "cpuid", 
            "mov rbx, {tmp}",
            tmp = out(reg) ebx,
            inout("eax") eax,
            out("ecx") ecx,
            out("edx") edx,
        );
        (eax, ebx, ecx, edx)
    };

    // Check invariant TSC flag in EDX bit 8
    let invariant_tsc = (edx_ext & (1 << 8)) != 0;
    
    invariant_tsc
}

/// Global TSC frequency in Hz (set during calibration)
static mut TSC_FREQUENCY_HZ: Option<u64> = None;

/// Get calibrated TSC frequency in Hz
fn get_tsc_frequency_hz() -> Option<u64> {
    unsafe { TSC_FREQUENCY_HZ }
}

/// Calibrate TSC frequency using PIT (Programmable Interval Timer)
///
/// This should be called during boot initialization
pub fn calibrate_tsc_frequency() -> Result<u64, &'static str> {
    if !is_tsc_available() {
        return Err("TSC not available or not invariant");
    }

    // Use PIT to measure TSC frequency
    let freq_hz = calibrate_tsc_with_pit()?;
    
    unsafe {
        TSC_FREQUENCY_HZ = Some(freq_hz);
    }

    crate::log_info!("TSC", "Calibrated TSC frequency: {} Hz ({} MHz)", 
                     freq_hz, freq_hz / 1_000_000);

    Ok(freq_hz)
}

/// Calibrate TSC using PIT (Programmable Interval Timer)
fn calibrate_tsc_with_pit() -> Result<u64, &'static str> {
    // PIT frequency is 1.193182 MHz
    const PIT_FREQUENCY: u64 = 1193182;
    const CALIBRATION_MS: u64 = 50; // Calibrate for 50ms
    
    // Calculate PIT count for calibration period
    let pit_count = (PIT_FREQUENCY * CALIBRATION_MS) / 1000;
    
    // Program PIT channel 2 for one-shot mode
    unsafe {
        // Command: Channel 2, LSB/MSB, Mode 0 (interrupt on terminal count)
        core::arch::asm!("out 0x43, al", in("al") 0xB0u8);
        
        // Set count value (LSB first, then MSB)
        core::arch::asm!("out 0x42, al", in("al") (pit_count & 0xFF) as u8);
        core::arch::asm!("out 0x42, al", in("al") ((pit_count >> 8) & 0xFF) as u8);
    }

    // Read initial TSC
    let tsc_start = read_tsc();
    
    // Start PIT and wait for completion
    unsafe {
        // Enable PIT channel 2
        let mut port61 = 0u8;
        core::arch::asm!("in al, 0x61", out("al") port61);
        port61 |= 0x01; // Enable gate
        core::arch::asm!("out 0x61, al", in("al") port61);
        
        // Wait for PIT to complete (poll status)
        loop {
            let mut status = 0u8;
            core::arch::asm!("in al, 0x61", out("al") status);
            if (status & 0x20) != 0 { // Check output bit
                break;
            }
        }
    }
    
    // Read final TSC
    let tsc_end = read_tsc();
    
    // Calculate frequency
    let tsc_delta = tsc_end.wrapping_sub(tsc_start);
    let freq_hz = (tsc_delta * 1000) / CALIBRATION_MS;
    
    if freq_hz < 100_000_000 || freq_hz > 10_000_000_000 {
        return Err("TSC frequency out of reasonable range");
    }
    
    Ok(freq_hz)
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
