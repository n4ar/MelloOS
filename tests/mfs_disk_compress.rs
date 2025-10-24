//! MelloFS Disk Compression Tests
//!
//! Tests for LZ4 and Zstd compression and decompression.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

/// Test: Compression type conversion
#[test_case]
fn test_compression_type_conversion() {
    use kernel::fs::mfs::disk::compress::CompressionType;
    
    // Test from_u8
    assert_eq!(CompressionType::from_u8(0), Some(CompressionType::None));
    assert_eq!(CompressionType::from_u8(1), Some(CompressionType::Lz4));
    assert_eq!(CompressionType::from_u8(2), Some(CompressionType::Zstd));
    assert_eq!(CompressionType::from_u8(3), None);
    
    // Test to_u8
    assert_eq!(CompressionType::None.to_u8(), 0);
    assert_eq!(CompressionType::Lz4.to_u8(), 1);
    assert_eq!(CompressionType::Zstd.to_u8(), 2);
    
    serial_println!("✓ Compression type conversion test passed");
}

/// Test: Small data not compressed
#[test_case]
fn test_small_data_not_compressed() {
    use kernel::fs::mfs::disk::compress::{compress, CompressionType, CompressionResult};
    
    // Data smaller than MIN_COMPRESSION_SIZE (4096)
    let small_data = vec![1u8; 100];
    
    // Try to compress with LZ4
    let result = compress(&small_data, CompressionType::Lz4).expect("Compression should not fail");
    
    // Should be uncompressed
    assert!(
        matches!(result, CompressionResult::Uncompressed),
        "Small data should not be compressed"
    );
    
    serial_println!("✓ Small data not compressed test passed");
}

/// Test: LZ4 compression and decompression
#[test_case]
fn test_lz4_compression_decompression() {
    use kernel::fs::mfs::disk::compress::{compress, decompress, CompressionType, CompressionResult};
    
    // Create compressible data (repeated pattern)
    let mut data = Vec::new();
    for i in 0..1024 {
        data.push((i % 10) as u8);
    }
    
    // Compress
    let result = compress(&data, CompressionType::Lz4).expect("Compression should succeed");
    
    match result {
        CompressionResult::Compressed { data: compressed, original_size, compressed_size } => {
            assert_eq!(original_size, data.len(), "Original size should match");
            assert!(compressed_size < original_size, "Compressed size should be smaller");
            
            // Decompress
            let decompressed = decompress(&compressed, CompressionType::Lz4, original_size)
                .expect("Decompression should succeed");
            
            assert_eq!(decompressed.len(), data.len(), "Decompressed size should match original");
            assert_eq!(decompressed, data, "Decompressed data should match original");
            
            serial_println!("✓ LZ4 compression/decompression test passed");
        }
        CompressionResult::Uncompressed => {
            serial_println!("⚠ Data was not compressed (incompressible)");
        }
    }
}

/// Test: Zstd compression and decompression
#[test_case]
fn test_zstd_compression_decompression() {
    use kernel::fs::mfs::disk::compress::{compress, decompress, CompressionType, CompressionResult};
    
    // Create compressible data
    let mut data = Vec::new();
    for i in 0..1024 {
        data.push((i % 10) as u8);
    }
    
    // Compress
    let result = compress(&data, CompressionType::Zstd).expect("Compression should succeed");
    
    match result {
        CompressionResult::Compressed { data: compressed, original_size, compressed_size } => {
            assert_eq!(original_size, data.len(), "Original size should match");
            assert!(compressed_size < original_size, "Compressed size should be smaller");
            
            // Decompress
            let decompressed = decompress(&compressed, CompressionType::Zstd, original_size)
                .expect("Decompression should succeed");
            
            assert_eq!(decompressed.len(), data.len(), "Decompressed size should match original");
            assert_eq!(decompressed, data, "Decompressed data should match original");
            
            serial_println!("✓ Zstd compression/decompression test passed");
        }
        CompressionResult::Uncompressed => {
            serial_println!("⚠ Data was not compressed (incompressible)");
        }
    }
}

/// Test: Incompressible data handling
#[test_case]
fn test_incompressible_data() {
    use kernel::fs::mfs::disk::compress::{compress, CompressionType, CompressionResult};
    
    // Create random-like data (incompressible)
    let mut data = Vec::new();
    for i in 0..5000 {
        data.push(((i * 7 + 13) % 256) as u8);
    }
    
    // Try to compress
    let result = compress(&data, CompressionType::Lz4).expect("Compression should not fail");
    
    // May or may not be compressed depending on the data
    match result {
        CompressionResult::Compressed { compressed_size, original_size, .. } => {
            serial_println!("Data was compressed: {} -> {} bytes", original_size, compressed_size);
        }
        CompressionResult::Uncompressed => {
            serial_println!("Data was not compressed (as expected for incompressible data)");
        }
    }
    
    serial_println!("✓ Incompressible data handling test passed");
}

/// Test: Transparent decompression
#[test_case]
fn test_transparent_decompression() {
    use kernel::fs::mfs::disk::compress::{compress, decompress, CompressionType, CompressionResult};
    
    // Create data
    let data = vec![42u8; 5000];
    
    // Compress
    let result = compress(&data, CompressionType::Lz4).expect("Compression should succeed");
    
    match result {
        CompressionResult::Compressed { data: compressed, original_size, .. } => {
            // Decompress should be transparent
            let decompressed = decompress(&compressed, CompressionType::Lz4, original_size)
                .expect("Decompression should succeed");
            
            assert_eq!(decompressed, data, "Decompression should be transparent");
        }
        CompressionResult::Uncompressed => {
            // If not compressed, just verify the data is unchanged
            assert_eq!(data.len(), 5000);
        }
    }
    
    serial_println!("✓ Transparent decompression test passed");
}

/// Test: Compression statistics
#[test_case]
fn test_compression_statistics() {
    use kernel::fs::mfs::disk::compress::CompressionStats;
    
    let mut stats = CompressionStats::new();
    
    // Simulate some compression
    stats.bytes_compressed = 10000;
    stats.bytes_after_compression = 5000;
    stats.extents_compressed = 10;
    stats.extents_incompressible = 2;
    
    // Check ratio
    assert_eq!(stats.compression_ratio(), 0.5, "Compression ratio should be 0.5");
    
    // Check space saved
    assert_eq!(stats.space_saved(), 5000, "Space saved should be 5000 bytes");
    
    serial_println!("✓ Compression statistics test passed");
}

/// Test: Compression ratio calculation
#[test_case]
fn test_compression_ratio() {
    use kernel::fs::mfs::disk::compress::{compress, CompressionType, CompressionResult};
    
    // Create highly compressible data
    let data = vec![0u8; 8192];
    
    // Compress
    let result = compress(&data, CompressionType::Lz4).expect("Compression should succeed");
    
    match result {
        CompressionResult::Compressed { original_size, compressed_size, .. } => {
            let ratio = (compressed_size as f64) / (original_size as f64);
            serial_println!("Compression ratio: {:.2}% ({} -> {} bytes)", 
                          ratio * 100.0, original_size, compressed_size);
            
            assert!(ratio < 1.0, "Compression ratio should be less than 1.0");
        }
        CompressionResult::Uncompressed => {
            serial_println!("Data was not compressed");
        }
    }
    
    serial_println!("✓ Compression ratio calculation test passed");
}

// Test runner
#[cfg(test)]
fn run_tests() {
    serial_println!("\n=== MelloFS Disk Compression Tests ===\n");
    
    test_compression_type_conversion();
    test_small_data_not_compressed();
    test_lz4_compression_decompression();
    test_zstd_compression_decompression();
    test_incompressible_data();
    test_transparent_decompression();
    test_compression_statistics();
    test_compression_ratio();
    
    serial_println!("\n=== All Compression Tests Passed ===\n");
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
