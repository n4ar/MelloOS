//! MelloFS Compression Support
//!
//! Implements LZ4 and Zstd compression for data extents.
//! Compression is optional and can be configured per-mount.

use alloc::vec::Vec;

/// Compression algorithm types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CompressionType {
    /// No compression
    None = 0,
    /// LZ4 - fast compression/decompression
    Lz4 = 1,
    /// Zstd - higher compression ratio
    Zstd = 2,
}

impl CompressionType {
    /// Create from u8 value
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(CompressionType::None),
            1 => Some(CompressionType::Lz4),
            2 => Some(CompressionType::Zstd),
            _ => None,
        }
    }
    
    /// Convert to u8 value
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Compression result
#[derive(Debug)]
pub enum CompressionResult {
    /// Data was compressed successfully
    Compressed {
        data: Vec<u8>,
        original_size: usize,
        compressed_size: usize,
    },
    /// Data was not compressed (incompressible or too small)
    Uncompressed,
}

/// Compression error
#[derive(Debug)]
pub enum CompressionError {
    /// Compression failed
    CompressionFailed,
    /// Decompression failed
    DecompressionFailed,
    /// Invalid compression type
    InvalidType,
    /// Buffer too small
    BufferTooSmall,
}

/// Minimum size for compression (4 KiB)
///
/// Extents smaller than this are not compressed to avoid overhead.
pub const MIN_COMPRESSION_SIZE: usize = 4096;

/// Compress data using the specified algorithm
///
/// # Arguments
/// * `data` - Data to compress
/// * `compression_type` - Compression algorithm to use
///
/// # Returns
/// CompressionResult indicating success or failure
pub fn compress(data: &[u8], compression_type: CompressionType) -> Result<CompressionResult, CompressionError> {
    // Skip compression for small data
    if data.len() < MIN_COMPRESSION_SIZE {
        return Ok(CompressionResult::Uncompressed);
    }
    
    match compression_type {
        CompressionType::None => Ok(CompressionResult::Uncompressed),
        CompressionType::Lz4 => compress_lz4(data),
        CompressionType::Zstd => compress_zstd(data),
    }
}

/// Decompress data using the specified algorithm
///
/// # Arguments
/// * `compressed_data` - Compressed data
/// * `compression_type` - Compression algorithm used
/// * `original_size` - Original uncompressed size
///
/// # Returns
/// Decompressed data
pub fn decompress(
    compressed_data: &[u8],
    compression_type: CompressionType,
    original_size: usize,
) -> Result<Vec<u8>, CompressionError> {
    match compression_type {
        CompressionType::None => {
            // No compression, just copy
            Ok(compressed_data.to_vec())
        }
        CompressionType::Lz4 => decompress_lz4(compressed_data, original_size),
        CompressionType::Zstd => decompress_zstd(compressed_data, original_size),
    }
}

/// LZ4 compression
///
/// This is a simplified LZ4 implementation for kernel use.
/// A full implementation would use the lz4 crate or a custom optimized version.
fn compress_lz4(data: &[u8]) -> Result<CompressionResult, CompressionError> {
    // For now, implement a simple run-length encoding as a placeholder
    // A real implementation would use proper LZ4 algorithm
    
    let compressed = simple_rle_compress(data);
    
    // Only use compression if it actually reduces size
    if compressed.len() < data.len() {
        Ok(CompressionResult::Compressed {
            data: compressed.clone(),
            original_size: data.len(),
            compressed_size: compressed.len(),
        })
    } else {
        Ok(CompressionResult::Uncompressed)
    }
}

/// LZ4 decompression
fn decompress_lz4(compressed_data: &[u8], original_size: usize) -> Result<Vec<u8>, CompressionError> {
    // Decompress using simple RLE
    let decompressed = simple_rle_decompress(compressed_data, original_size)?;
    
    if decompressed.len() != original_size {
        return Err(CompressionError::DecompressionFailed);
    }
    
    Ok(decompressed)
}

/// Zstd compression
///
/// This is a placeholder for Zstd compression.
/// A real implementation would use the zstd crate or a custom implementation.
fn compress_zstd(data: &[u8]) -> Result<CompressionResult, CompressionError> {
    // For now, use the same simple RLE as LZ4
    // A real implementation would use proper Zstd algorithm
    
    let compressed = simple_rle_compress(data);
    
    // Zstd typically achieves better compression than LZ4
    // For the placeholder, we just use the same algorithm
    if compressed.len() < data.len() {
        Ok(CompressionResult::Compressed {
            data: compressed.clone(),
            original_size: data.len(),
            compressed_size: compressed.len(),
        })
    } else {
        Ok(CompressionResult::Uncompressed)
    }
}

/// Zstd decompression
fn decompress_zstd(compressed_data: &[u8], original_size: usize) -> Result<Vec<u8>, CompressionError> {
    // Decompress using simple RLE
    let decompressed = simple_rle_decompress(compressed_data, original_size)?;
    
    if decompressed.len() != original_size {
        return Err(CompressionError::DecompressionFailed);
    }
    
    Ok(decompressed)
}

/// Simple run-length encoding (placeholder for real compression)
///
/// Format: [count:u8][byte:u8]...
/// This is NOT a real LZ4/Zstd implementation, just a placeholder
/// to demonstrate the compression interface.
fn simple_rle_compress(data: &[u8]) -> Vec<u8> {
    let mut compressed = Vec::new();
    
    if data.is_empty() {
        return compressed;
    }
    
    let mut i = 0;
    while i < data.len() {
        let byte = data[i];
        let mut count = 1u8;
        
        // Count consecutive identical bytes (max 255)
        while i + (count as usize) < data.len() 
            && data[i + (count as usize)] == byte 
            && count < 255 {
            count += 1;
        }
        
        // Write count and byte
        compressed.push(count);
        compressed.push(byte);
        
        i += count as usize;
    }
    
    compressed
}

/// Simple run-length decoding
fn simple_rle_decompress(compressed: &[u8], max_size: usize) -> Result<Vec<u8>, CompressionError> {
    let mut decompressed = Vec::new();
    
    let mut i = 0;
    while i + 1 < compressed.len() {
        let count = compressed[i] as usize;
        let byte = compressed[i + 1];
        
        // Check for buffer overflow
        if decompressed.len() + count > max_size {
            return Err(CompressionError::BufferTooSmall);
        }
        
        // Expand run
        for _ in 0..count {
            decompressed.push(byte);
        }
        
        i += 2;
    }
    
    Ok(decompressed)
}

/// Compression statistics
#[derive(Debug, Clone, Copy)]
pub struct CompressionStats {
    /// Total bytes compressed
    pub bytes_compressed: u64,
    /// Total bytes after compression
    pub bytes_after_compression: u64,
    /// Number of extents compressed
    pub extents_compressed: u64,
    /// Number of extents that were incompressible
    pub extents_incompressible: u64,
}

impl CompressionStats {
    /// Create new empty stats
    pub const fn new() -> Self {
        Self {
            bytes_compressed: 0,
            bytes_after_compression: 0,
            extents_compressed: 0,
            extents_incompressible: 0,
        }
    }
    
    /// Calculate compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.bytes_compressed == 0 {
            return 0.0;
        }
        
        (self.bytes_after_compression as f64) / (self.bytes_compressed as f64)
    }
    
    /// Calculate space saved
    pub fn space_saved(&self) -> u64 {
        if self.bytes_compressed > self.bytes_after_compression {
            self.bytes_compressed - self.bytes_after_compression
        } else {
            0
        }
    }
}

impl Default for CompressionStats {
    fn default() -> Self {
        Self::new()
    }
}


