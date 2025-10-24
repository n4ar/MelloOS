//! MelloFS Disk Checksum Tests
//!
//! Tests for checksum verification and corruption detection.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

// Mock BlockDevice for testing
struct MockBlockDevice {
    blocks: Vec<Vec<u8>>,
    block_size: usize,
}

impl MockBlockDevice {
    fn new(num_blocks: usize, block_size: usize) -> Self {
        let mut blocks = Vec::new();
        for _ in 0..num_blocks {
            blocks.push(vec![0u8; block_size]);
        }
        Self { blocks, block_size }
    }
    
    fn read_block(&self, lba: u64, buf: &mut [u8]) -> Result<(), &'static str> {
        let index = lba as usize;
        if index >= self.blocks.len() {
            return Err("LBA out of range");
        }
        
        let len = core::cmp::min(buf.len(), self.block_size);
        buf[..len].copy_from_slice(&self.blocks[index][..len]);
        Ok(())
    }
    
    fn write_block(&mut self, lba: u64, buf: &[u8]) -> Result<(), &'static str> {
        let index = lba as usize;
        if index >= self.blocks.len() {
            return Err("LBA out of range");
        }
        
        let len = core::cmp::min(buf.len(), self.block_size);
        self.blocks[index][..len].copy_from_slice(&buf[..len]);
        Ok(())
    }
    
    fn corrupt_block(&mut self, lba: u64, offset: usize) {
        let index = lba as usize;
        if index < self.blocks.len() && offset < self.block_size {
            self.blocks[index][offset] ^= 0xFF; // Flip all bits
        }
    }
}

/// Test: CRC32C basic functionality
#[test_case]
fn test_crc32c_basic() {
    // Test empty data
    let empty = b"";
    let checksum = kernel::fs::mfs::disk::checksum::crc32c(empty);
    assert_eq!(checksum, 0, "CRC32C of empty data should be 0");
    
    // Test known value
    let data = b"hello world";
    let checksum = kernel::fs::mfs::disk::checksum::crc32c(data);
    assert_eq!(checksum, 0xc99465aa, "CRC32C of 'hello world' should match known value");
    
    serial_println!("✓ CRC32C basic functionality test passed");
}

/// Test: Checksum verification
#[test_case]
fn test_checksum_verification() {
    let data = b"test data for checksum verification";
    let checksum = kernel::fs::mfs::disk::checksum::crc32c(data);
    
    // Verify correct checksum
    assert!(
        kernel::fs::mfs::disk::checksum::verify_checksum(data, checksum),
        "Checksum verification should succeed for correct checksum"
    );
    
    // Verify incorrect checksum
    assert!(
        !kernel::fs::mfs::disk::checksum::verify_checksum(data, checksum + 1),
        "Checksum verification should fail for incorrect checksum"
    );
    
    serial_println!("✓ Checksum verification test passed");
}

/// Test: Incremental checksum computation
#[test_case]
fn test_incremental_checksum() {
    let data = b"hello world";
    
    // Compute in one pass
    let checksum_single = kernel::fs::mfs::disk::checksum::crc32c(data);
    
    // Compute incrementally
    let mut builder = kernel::fs::mfs::disk::checksum::ChecksumBuilder::new();
    builder.update(b"hello ");
    builder.update(b"world");
    let checksum_incremental = builder.finalize();
    
    assert_eq!(
        checksum_single, checksum_incremental,
        "Incremental checksum should match single-pass checksum"
    );
    
    serial_println!("✓ Incremental checksum test passed");
}

/// Test: Superblock checksum detection
#[test_case]
fn test_superblock_checksum_mismatch() {
    use kernel::fs::mfs::disk::super_::{MfsSuperblock, BLOCK_SIZE_4K};
    
    // Create a valid superblock
    let mut sb = MfsSuperblock::new(BLOCK_SIZE_4K, 1000).expect("Failed to create superblock");
    
    // Verify it's valid
    assert!(sb.verify_checksum(), "New superblock should have valid checksum");
    
    // Corrupt the checksum
    sb.checksum ^= 0xFFFFFFFF;
    
    // Verify detection
    assert!(!sb.verify_checksum(), "Corrupted checksum should be detected");
    
    serial_println!("✓ Superblock checksum mismatch detection test passed");
}

/// Test: B-tree node checksum detection
#[test_case]
fn test_btree_node_checksum_mismatch() {
    use kernel::fs::mfs::disk::btree::BtreeNode;
    
    // Create a B-tree node
    let mut node = BtreeNode::new(0, 4096, 1, 1);
    node.insert_at(0, vec![1, 2, 3], vec![4, 5, 6]);
    
    // Serialize
    let serialized = node.serialize().expect("Failed to serialize node");
    
    // Deserialize (should succeed)
    let deserialized = BtreeNode::deserialize(&serialized, 4096);
    assert!(deserialized.is_ok(), "Deserialization should succeed for valid node");
    
    // Corrupt the data
    let mut corrupted = serialized.clone();
    corrupted[100] ^= 0xFF;
    
    // Try to deserialize corrupted data
    let result = BtreeNode::deserialize(&corrupted, 4096);
    assert!(result.is_err(), "Deserialization should fail for corrupted node");
    
    serial_println!("✓ B-tree node checksum mismatch detection test passed");
}

/// Test: Secondary superblock recovery
#[test_case]
fn test_secondary_superblock_recovery() {
    // This test would require a full BlockDevice implementation
    // For now, we just verify the secondary superblock LBA calculation
    
    use kernel::fs::mfs::disk::super_::{MfsSuperblock, PRIMARY_SUPERBLOCK_BLOCKS};
    
    let total_blocks = 10000u64;
    let secondary_lba = MfsSuperblock::secondary_superblock_lba(total_blocks);
    
    assert_eq!(
        secondary_lba,
        total_blocks - PRIMARY_SUPERBLOCK_BLOCKS,
        "Secondary superblock should be at end of device"
    );
    
    serial_println!("✓ Secondary superblock LBA calculation test passed");
}

/// Test: Corruption detection returns EIO
#[test_case]
fn test_corruption_returns_eio() {
    use kernel::fs::mfs::disk::btree::BtreeNode;
    
    // Create and serialize a node
    let mut node = BtreeNode::new(0, 4096, 1, 1);
    node.insert_at(0, vec![1, 2, 3], vec![4, 5, 6]);
    let mut serialized = node.serialize().expect("Failed to serialize");
    
    // Corrupt the checksum field
    serialized[24] ^= 0xFF;
    
    // Try to deserialize - should fail with checksum error
    let result = BtreeNode::deserialize(&serialized, 4096);
    assert!(result.is_err(), "Should detect checksum mismatch");
    assert_eq!(result.unwrap_err(), "Checksum mismatch", "Should return checksum error");
    
    serial_println!("✓ Corruption detection EIO test passed");
}

// Test runner
#[cfg(test)]
fn run_tests() {
    serial_println!("\n=== MelloFS Disk Checksum Tests ===\n");
    
    test_crc32c_basic();
    test_checksum_verification();
    test_incremental_checksum();
    test_superblock_checksum_mismatch();
    test_btree_node_checksum_mismatch();
    test_secondary_superblock_recovery();
    test_corruption_returns_eio();
    
    serial_println!("\n=== All Checksum Tests Passed ===\n");
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
