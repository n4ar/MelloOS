//! Memory-Mapped I/O (MMIO) Operations
//!
//! Provides safe wrappers for memory-mapped I/O operations with proper volatile semantics.
//! MMIO is used by many modern devices where device registers are mapped into the physical
//! address space rather than using separate I/O port space.
//!
//! # Safety
//!
//! All MMIO operations are inherently unsafe as they directly interact with hardware.
//! Callers must ensure:
//! - The address is valid and mapped to a device register
//! - The address is properly aligned for the operation size
//! - The operation is appropriate for the device state
//! - Proper synchronization in SMP environments

use core::ptr;

/// Read from a memory-mapped register
///
/// # Safety
///
/// The caller must ensure:
/// - `addr` points to a valid, mapped MMIO region
/// - `addr` is properly aligned for type `T`
/// - Reading from this address is safe for the current device state
#[inline]
pub unsafe fn mmio_read<T>(addr: usize) -> T {
    ptr::read_volatile(addr as *const T)
}

/// Write to a memory-mapped register
///
/// # Safety
///
/// The caller must ensure:
/// - `addr` points to a valid, mapped MMIO region
/// - `addr` is properly aligned for type `T`
/// - Writing to this address is safe for the current device state
#[inline]
pub unsafe fn mmio_write<T>(addr: usize, value: T) {
    ptr::write_volatile(addr as *mut T, value);
}

/// Read 8-bit value from MMIO
///
/// # Safety
///
/// See [`mmio_read`] for safety requirements.
#[inline]
pub unsafe fn mmio_read8(addr: usize) -> u8 {
    mmio_read::<u8>(addr)
}

/// Write 8-bit value to MMIO
///
/// # Safety
///
/// See [`mmio_write`] for safety requirements.
#[inline]
pub unsafe fn mmio_write8(addr: usize, value: u8) {
    mmio_write::<u8>(addr, value);
}

/// Read 16-bit value from MMIO
///
/// # Safety
///
/// See [`mmio_read`] for safety requirements.
#[inline]
pub unsafe fn mmio_read16(addr: usize) -> u16 {
    mmio_read::<u16>(addr)
}

/// Write 16-bit value to MMIO
///
/// # Safety
///
/// See [`mmio_write`] for safety requirements.
#[inline]
pub unsafe fn mmio_write16(addr: usize, value: u16) {
    mmio_write::<u16>(addr, value);
}

/// Read 32-bit value from MMIO
///
/// # Safety
///
/// See [`mmio_read`] for safety requirements.
#[inline]
pub unsafe fn mmio_read32(addr: usize) -> u32 {
    mmio_read::<u32>(addr)
}

/// Write 32-bit value to MMIO
///
/// # Safety
///
/// See [`mmio_write`] for safety requirements.
#[inline]
pub unsafe fn mmio_write32(addr: usize, value: u32) {
    mmio_write::<u32>(addr, value);
}

/// Read 64-bit value from MMIO
///
/// # Safety
///
/// See [`mmio_read`] for safety requirements.
#[inline]
pub unsafe fn mmio_read64(addr: usize) -> u64 {
    mmio_read::<u64>(addr)
}

/// Write 64-bit value to MMIO
///
/// # Safety
///
/// See [`mmio_write`] for safety requirements.
#[inline]
pub unsafe fn mmio_write64(addr: usize, value: u64) {
    mmio_write::<u64>(addr, value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn test_mmio_read_write_u8() {
        // Allocate a buffer to simulate MMIO region
        let mut buffer = vec![0u8; 16];
        let addr = buffer.as_mut_ptr() as usize;

        unsafe {
            // Write and read back
            mmio_write8(addr, 0x42);
            assert_eq!(mmio_read8(addr), 0x42);

            // Write different value
            mmio_write8(addr, 0xAA);
            assert_eq!(mmio_read8(addr), 0xAA);
        }
    }

    #[test]
    fn test_mmio_read_write_u16() {
        // Allocate aligned buffer
        let mut buffer = vec![0u16; 8];
        let addr = buffer.as_mut_ptr() as usize;

        unsafe {
            // Write and read back
            mmio_write16(addr, 0x1234);
            assert_eq!(mmio_read16(addr), 0x1234);

            // Write different value
            mmio_write16(addr, 0xABCD);
            assert_eq!(mmio_read16(addr), 0xABCD);
        }
    }

    #[test]
    fn test_mmio_read_write_u32() {
        // Allocate aligned buffer
        let mut buffer = vec![0u32; 4];
        let addr = buffer.as_mut_ptr() as usize;

        unsafe {
            // Write and read back
            mmio_write32(addr, 0x12345678);
            assert_eq!(mmio_read32(addr), 0x12345678);

            // Write different value
            mmio_write32(addr, 0xDEADBEEF);
            assert_eq!(mmio_read32(addr), 0xDEADBEEF);
        }
    }

    #[test]
    fn test_mmio_read_write_u64() {
        // Allocate aligned buffer
        let mut buffer = vec![0u64; 2];
        let addr = buffer.as_mut_ptr() as usize;

        unsafe {
            // Write and read back
            mmio_write64(addr, 0x123456789ABCDEF0);
            assert_eq!(mmio_read64(addr), 0x123456789ABCDEF0);

            // Write different value
            mmio_write64(addr, 0xFEDCBA9876543210);
            assert_eq!(mmio_read64(addr), 0xFEDCBA9876543210);
        }
    }

    #[test]
    fn test_mmio_multiple_locations() {
        // Allocate buffer with multiple locations
        let mut buffer = vec![0u32; 4];
        let base_addr = buffer.as_mut_ptr() as usize;

        unsafe {
            // Write to different offsets
            mmio_write32(base_addr, 0x11111111);
            mmio_write32(base_addr + 4, 0x22222222);
            mmio_write32(base_addr + 8, 0x33333333);
            mmio_write32(base_addr + 12, 0x44444444);

            // Read back and verify
            assert_eq!(mmio_read32(base_addr), 0x11111111);
            assert_eq!(mmio_read32(base_addr + 4), 0x22222222);
            assert_eq!(mmio_read32(base_addr + 8), 0x33333333);
            assert_eq!(mmio_read32(base_addr + 12), 0x44444444);
        }
    }

    #[test]
    fn test_mmio_generic_read_write() {
        // Test generic mmio_read/mmio_write functions
        let mut buffer = vec![0u32; 4];
        let addr = buffer.as_mut_ptr() as usize;

        unsafe {
            // Test with u32
            mmio_write::<u32>(addr, 0xCAFEBABE);
            assert_eq!(mmio_read::<u32>(addr), 0xCAFEBABE);
        }

        // Test with u16
        let mut buffer16 = vec![0u16; 8];
        let addr16 = buffer16.as_mut_ptr() as usize;

        unsafe {
            mmio_write::<u16>(addr16, 0xBEEF);
            assert_eq!(mmio_read::<u16>(addr16), 0xBEEF);
        }
    }
}
