//! I/O Infrastructure Module
//!
//! This module provides low-level I/O operations for device drivers:
//! - Port I/O (inb/outb/inw/outw/inl/outl)
//! - Memory-mapped I/O (MMIO)
//! - IRQ management
//! - Device tree and bus scanning

pub mod mmio;
pub mod port;
mod test_integration;

// Re-export commonly used functions
pub use mmio::{mmio_read, mmio_read32, mmio_write, mmio_write32};
pub use port::{inb, inl, inw, outb, outl, outw};
