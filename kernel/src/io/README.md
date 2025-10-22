# I/O Infrastructure Module

This module provides the foundation for device driver development in MelloOS.

## Components

### Port I/O (`port.rs`)

Provides safe wrappers for x86_64 I/O port operations:

- `inb(port)` / `outb(port, value)` - 8-bit operations
- `inw(port)` / `outw(port, value)` - 16-bit operations  
- `inl(port)` / `outl(port, value)` - 32-bit operations

**Features:**
- Hardware implementation for production use
- Mock implementation for testing (enabled with `#[cfg(test)]`)
- Comprehensive unit tests for all operations

**Example Usage:**
```rust
use crate::io::{inb, outb};

unsafe {
    // Read from COM1 data port
    let data = inb(0x3F8);
    
    // Write to keyboard data port
    outb(0x60, 0xED);
}
```

### Memory-Mapped I/O (`mmio.rs`)

Provides volatile read/write operations for memory-mapped device registers:

- `mmio_read<T>(addr)` / `mmio_write<T>(addr, value)` - Generic operations
- `mmio_read8/16/32/64(addr)` - Sized read operations
- `mmio_write8/16/32/64(addr, value)` - Sized write operations

**Features:**
- Proper volatile semantics to prevent compiler optimizations
- Type-safe operations with alignment requirements
- Comprehensive unit tests

**Example Usage:**
```rust
use crate::io::{mmio_read32, mmio_write32};

unsafe {
    // Read from virtio device config space
    let capacity = mmio_read32(device_base + 0x14);
    
    // Write to device status register
    mmio_write32(device_base + 0x70, 0x04);
}
```

## Testing

The module includes comprehensive unit tests:

### Port I/O Tests
- Byte, word, and double-word operations
- Port independence verification
- Default value handling
- Overwrite behavior
- Mixed-size operations

### MMIO Tests
- All size variants (8/16/32/64-bit)
- Multiple location handling
- Generic read/write operations
- Alignment verification

**Note:** Tests use mock implementations and cannot be run in the kernel's no_std environment. They are provided for documentation and can be verified in a test harness.

## Integration

The I/O infrastructure is integrated into the kernel and ready for use by device drivers:

```rust
// In kernel code
use crate::io::{inb, outb, mmio_read32, mmio_write32};

// Port I/O example
unsafe {
    let status = inb(0x64);  // Read PS/2 status
    outb(0x60, 0xAE);        // Enable keyboard
}

// MMIO example  
unsafe {
    let value = mmio_read32(base_addr + offset);
    mmio_write32(base_addr + offset, new_value);
}
```

## Safety

All I/O operations are marked `unsafe` because they:
- Directly interact with hardware
- Can cause undefined behavior if used incorrectly
- Require proper synchronization in SMP environments

Callers must ensure:
- Port/address is valid for the intended device
- Operation is appropriate for device state
- Proper locking in multi-core scenarios

## Future Extensions

This module will be extended with:
- IRQ management (`irq.rs`)
- Device tree and bus scanning (`devtree.rs`)
- DMA support
- Additional bus-specific utilities
