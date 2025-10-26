//! MelloFS Checksum Implementation
//!
//! CRC32C (Castagnoli) checksums for data integrity.
//! Provides software implementation with optional hardware acceleration.

/// CRC32C polynomial (reversed)
const CRC32C_POLY: u32 = 0x82F63B78;

/// Compute CRC32C checksum of data
///
/// This is the primary checksum function used throughout MelloFS.
/// Uses software implementation for now; hardware acceleration (SSE4.2)
/// can be added later for x86_64.
///
/// # Arguments
/// * `data` - Byte slice to checksum
///
/// # Returns
/// 32-bit CRC32C checksum
pub fn crc32c(data: &[u8]) -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        // Try hardware-accelerated version if available
        if has_sse42() {
            return crc32c_hw(data);
        }
    }

    // Fall back to software implementation
    crc32c_sw(data)
}

/// Software CRC32C implementation
///
/// Pure software implementation that works on all platforms.
/// Uses bit-by-bit algorithm for simplicity and correctness.
fn crc32c_sw(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;

    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ CRC32C_POLY
            } else {
                crc >> 1
            };
        }
    }

    crc ^ 0xFFFFFFFF
}

/// Hardware-accelerated CRC32C (SSE4.2)
///
/// Uses the CRC32 instruction available on modern x86_64 CPUs.
/// This provides significant performance improvement over software.
#[cfg(target_arch = "x86_64")]
fn crc32c_hw(data: &[u8]) -> u32 {
    use core::arch::x86_64::_mm_crc32_u8;

    let mut crc: u32 = 0xFFFFFFFF;

    // Process byte by byte using hardware instruction
    for &byte in data {
        crc = unsafe { _mm_crc32_u8(crc, byte) };
    }

    crc ^ 0xFFFFFFFF
}

/// Check if SSE4.2 is available (for hardware CRC32C)
#[cfg(target_arch = "x86_64")]
fn has_sse42() -> bool {
    // Check CPUID for SSE4.2 support
    // SSE4.2 is indicated by bit 20 of ECX in CPUID function 1

    use core::sync::atomic::{AtomicBool, Ordering};

    // Cache the result to avoid repeated CPUID calls
    static SSE42_AVAILABLE: AtomicBool = AtomicBool::new(false);
    static SSE42_CHECKED: AtomicBool = AtomicBool::new(false);

    // Check if we've already determined SSE4.2 availability
    if SSE42_CHECKED.load(Ordering::Relaxed) {
        return SSE42_AVAILABLE.load(Ordering::Relaxed);
    }

    // Query CPUID
    let cpuid_result = unsafe { core::arch::x86_64::__cpuid(1) };

    // Check bit 20 of ECX for SSE4.2
    const SSE42_BIT: u32 = 1 << 20;
    let has_sse42 = (cpuid_result.ecx & SSE42_BIT) != 0;

    // Cache the result
    SSE42_AVAILABLE.store(has_sse42, Ordering::Relaxed);
    SSE42_CHECKED.store(true, Ordering::Relaxed);

    has_sse42
}

/// Check if PCLMULQDQ is available (for faster CRC32C)
#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
pub fn has_pclmulqdq() -> bool {
    // Check CPUID for PCLMULQDQ support
    // PCLMULQDQ is indicated by bit 1 of ECX in CPUID function 1

    use core::sync::atomic::{AtomicBool, Ordering};

    // Cache the result
    static PCLMULQDQ_AVAILABLE: AtomicBool = AtomicBool::new(false);
    static PCLMULQDQ_CHECKED: AtomicBool = AtomicBool::new(false);

    if PCLMULQDQ_CHECKED.load(Ordering::Relaxed) {
        return PCLMULQDQ_AVAILABLE.load(Ordering::Relaxed);
    }

    // Query CPUID
    let cpuid_result = unsafe { core::arch::x86_64::__cpuid(1) };

    // Check bit 1 of ECX for PCLMULQDQ
    const PCLMULQDQ_BIT: u32 = 1 << 1;
    let has_pclmul = (cpuid_result.ecx & PCLMULQDQ_BIT) != 0;

    // Cache the result
    PCLMULQDQ_AVAILABLE.store(has_pclmul, Ordering::Relaxed);
    PCLMULQDQ_CHECKED.store(true, Ordering::Relaxed);

    has_pclmul
}

/// Compute CRC32C and return as u64 (for compatibility with existing code)
pub fn crc32c_u64(data: &[u8]) -> u64 {
    crc32c(data) as u64
}

/// Verify checksum matches expected value
///
/// # Arguments
/// * `data` - Data to verify
/// * `expected` - Expected checksum value
///
/// # Returns
/// true if checksum matches, false otherwise
pub fn verify_checksum(data: &[u8], expected: u32) -> bool {
    crc32c(data) == expected
}

/// Verify checksum (u64 version)
pub fn verify_checksum_u64(data: &[u8], expected: u64) -> bool {
    crc32c_u64(data) == expected
}

/// Checksum builder for incremental computation
///
/// Allows computing checksums over multiple chunks of data.
pub struct ChecksumBuilder {
    crc: u32,
}

impl ChecksumBuilder {
    /// Create a new checksum builder
    pub fn new() -> Self {
        Self { crc: 0xFFFFFFFF }
    }

    /// Update checksum with more data
    pub fn update(&mut self, data: &[u8]) {
        for &byte in data {
            self.crc ^= byte as u32;
            for _ in 0..8 {
                self.crc = if self.crc & 1 != 0 {
                    (self.crc >> 1) ^ CRC32C_POLY
                } else {
                    self.crc >> 1
                };
            }
        }
    }

    /// Finalize and return the checksum
    pub fn finalize(self) -> u32 {
        self.crc ^ 0xFFFFFFFF
    }

    /// Finalize and return as u64
    pub fn finalize_u64(self) -> u64 {
        self.finalize() as u64
    }
}

impl Default for ChecksumBuilder {
    fn default() -> Self {
        Self::new()
    }
}
