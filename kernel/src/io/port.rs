//! I/O Port Operations
//!
//! Provides safe wrappers for x86_64 I/O port operations with mock support for testing.
//!
//! # Safety
//!
//! All port I/O operations are inherently unsafe as they directly interact with hardware.
//! Callers must ensure:
//! - The port address is valid for the intended device
//! - The operation is appropriate for the device state
//! - Proper synchronization in SMP environments

use core::arch::asm;

#[cfg(not(test))]
mod port_impl {
    use super::*;

    /// Read a byte (8-bit) from an I/O port
    ///
    /// # Safety
    ///
    /// The caller must ensure the port address is valid and the operation is safe.
    #[inline]
    pub unsafe fn inb(port: u16) -> u8 {
        let value: u8;
        asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack));
        value
    }

    /// Write a byte (8-bit) to an I/O port
    ///
    /// # Safety
    ///
    /// The caller must ensure the port address is valid and the operation is safe.
    #[inline]
    pub unsafe fn outb(port: u16, value: u8) {
        asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack));
    }

    /// Read a word (16-bit) from an I/O port
    ///
    /// # Safety
    ///
    /// The caller must ensure the port address is valid and the operation is safe.
    #[inline]
    pub unsafe fn inw(port: u16) -> u16 {
        let value: u16;
        asm!("in ax, dx", out("ax") value, in("dx") port, options(nomem, nostack));
        value
    }

    /// Write a word (16-bit) to an I/O port
    ///
    /// # Safety
    ///
    /// The caller must ensure the port address is valid and the operation is safe.
    #[inline]
    pub unsafe fn outw(port: u16, value: u16) {
        asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack));
    }

    /// Read a double word (32-bit) from an I/O port
    ///
    /// # Safety
    ///
    /// The caller must ensure the port address is valid and the operation is safe.
    #[inline]
    pub unsafe fn inl(port: u16) -> u32 {
        let value: u32;
        asm!("in eax, dx", out("eax") value, in("dx") port, options(nomem, nostack));
        value
    }

    /// Write a double word (32-bit) to an I/O port
    ///
    /// # Safety
    ///
    /// The caller must ensure the port address is valid and the operation is safe.
    #[inline]
    pub unsafe fn outl(port: u16, value: u32) {
        asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack));
    }
}

#[cfg(test)]
mod port_impl {
    use crate::sync::spin::SpinLock;
    use alloc::collections::BTreeMap;

    /// Mock port storage for testing
    static MOCK_PORTS: SpinLock<BTreeMap<u16, u32>> = SpinLock::new(BTreeMap::new());

    /// Mock implementation for testing - read byte
    pub unsafe fn inb(port: u16) -> u8 {
        MOCK_PORTS.lock().get(&port).copied().unwrap_or(0) as u8
    }

    /// Mock implementation for testing - write byte
    pub unsafe fn outb(port: u16, value: u8) {
        MOCK_PORTS.lock().insert(port, value as u32);
    }

    /// Mock implementation for testing - read word
    pub unsafe fn inw(port: u16) -> u16 {
        MOCK_PORTS.lock().get(&port).copied().unwrap_or(0) as u16
    }

    /// Mock implementation for testing - write word
    pub unsafe fn outw(port: u16, value: u16) {
        MOCK_PORTS.lock().insert(port, value as u32);
    }

    /// Mock implementation for testing - read double word
    pub unsafe fn inl(port: u16) -> u32 {
        MOCK_PORTS.lock().get(&port).copied().unwrap_or(0)
    }

    /// Mock implementation for testing - write double word
    pub unsafe fn outl(port: u16, value: u32) {
        MOCK_PORTS.lock().insert(port, value);
    }

    /// Test helper: Clear all mock ports
    #[allow(dead_code)]
    pub fn clear_mock_ports() {
        MOCK_PORTS.lock().clear();
    }

    /// Test helper: Get value from mock port
    #[allow(dead_code)]
    pub fn get_mock_port(port: u16) -> Option<u32> {
        MOCK_PORTS.lock().get(&port).copied()
    }
}

// Re-export implementation
pub use port_impl::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_byte_operations() {
        clear_mock_ports();

        // Test outb/inb
        unsafe {
            outb(0x3F8, 0x42);
            assert_eq!(inb(0x3F8), 0x42);
        }

        // Test different port
        unsafe {
            outb(0x60, 0xAA);
            assert_eq!(inb(0x60), 0xAA);
        }

        // Test that ports are independent
        unsafe {
            assert_eq!(inb(0x3F8), 0x42);
        }
    }

    #[test]
    fn test_port_word_operations() {
        clear_mock_ports();

        // Test outw/inw
        unsafe {
            outw(0x3F8, 0x1234);
            assert_eq!(inw(0x3F8), 0x1234);
        }

        // Test different values
        unsafe {
            outw(0x60, 0xABCD);
            assert_eq!(inw(0x60), 0xABCD);
        }
    }

    #[test]
    fn test_port_dword_operations() {
        clear_mock_ports();

        // Test outl/inl
        unsafe {
            outl(0x3F8, 0x12345678);
            assert_eq!(inl(0x3F8), 0x12345678);
        }

        // Test different values
        unsafe {
            outl(0x60, 0xDEADBEEF);
            assert_eq!(inl(0x60), 0xDEADBEEF);
        }
    }

    #[test]
    fn test_port_default_value() {
        clear_mock_ports();

        // Reading from uninitialized port should return 0
        unsafe {
            assert_eq!(inb(0x999), 0);
            assert_eq!(inw(0x999), 0);
            assert_eq!(inl(0x999), 0);
        }
    }

    #[test]
    fn test_port_overwrite() {
        clear_mock_ports();

        // Write and overwrite
        unsafe {
            outb(0x3F8, 0x11);
            assert_eq!(inb(0x3F8), 0x11);

            outb(0x3F8, 0x22);
            assert_eq!(inb(0x3F8), 0x22);
        }
    }

    #[test]
    fn test_mixed_size_operations() {
        clear_mock_ports();

        // Write with outl, read with inb (should get lower byte)
        unsafe {
            outl(0x3F8, 0x12345678);
            assert_eq!(inb(0x3F8), 0x78);
        }

        // Write with outw, read with inl (should get full value as u32)
        unsafe {
            outw(0x60, 0xABCD);
            assert_eq!(inl(0x60), 0xABCD);
        }
    }
}
