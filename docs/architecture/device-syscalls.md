# Device Driver Syscalls

This document describes the syscalls added in Phase 7 for device driver access.

## Overview

Phase 7 introduces seven new syscalls that provide userland access to device drivers:
- Keyboard input (stdin)
- Serial port I/O
- Block device I/O
- Device enumeration
- Block device information

## Syscall Numbers

| Number | Name | Description |
|--------|------|-------------|
| 25 | SYS_READ_STDIN | Read keyboard input |
| 26 | SYS_SERIAL_WRITE | Write to serial port |
| 27 | SYS_SERIAL_READ | Read from serial port |
| 28 | SYS_BLOCK_READ | Read blocks from disk |
| 29 | SYS_BLOCK_WRITE | Write blocks to disk |
| 30 | SYS_GET_DEVICE_LIST | Query device tree |
| 31 | SYS_GET_BLOCK_DEVICE_INFO | Get disk information |

## Syscall Descriptions

### SYS_READ_STDIN (25)

Read keyboard input from the PS/2 keyboard driver buffer.

**Arguments:**
- `arg1`: Pointer to buffer
- `arg2`: Maximum bytes to read

**Returns:**
- Number of bytes read on success
- -1 on error

**Behavior:**
- Non-blocking read
- Returns immediately if no data is available
- Reads from PS/2 keyboard driver circular buffer

**Example:**
```rust
let mut buf = [0u8; 64];
let bytes_read = read_stdin(&mut buf);
if bytes_read > 0 {
    // Process keyboard input
}
```

### SYS_SERIAL_WRITE (26)

Write data to the serial port (COM1).

**Arguments:**
- `arg1`: Pointer to buffer
- `arg2`: Number of bytes to write

**Returns:**
- Number of bytes written on success
- -1 on error

**Behavior:**
- Writes to UART16550 serial port
- Blocking write (waits for transmit buffer)
- Useful for debugging and logging

**Example:**
```rust
let msg = b"Debug message\n";
serial_write(msg);
```

### SYS_SERIAL_READ (27)

Read data from the serial port (COM1).

**Arguments:**
- `arg1`: Pointer to buffer
- `arg2`: Maximum bytes to read

**Returns:**
- Number of bytes read on success
- -1 on error

**Behavior:**
- Non-blocking read
- Returns immediately if no data is available
- Reads from UART16550 receive buffer

**Example:**
```rust
let mut buf = [0u8; 64];
let bytes_read = serial_read(&mut buf);
```

### SYS_BLOCK_READ (28)

Read blocks from disk storage.

**Arguments:**
- `arg1`: LBA (Logical Block Address) - starting sector number
- `arg2`: Pointer to buffer (must be at least 512 * count bytes)
- `arg3`: Number of blocks to read

**Returns:**
- Number of blocks read on success
- -1 on error

**Behavior:**
- Reads 512-byte sectors from virtio-blk device
- Buffer must be large enough for all blocks
- Partial reads possible if error occurs mid-operation

**Example:**
```rust
// Read MBR (sector 0)
let mut buf = [0u8; 512];
let blocks_read = block_read(0, &mut buf, 1);
if blocks_read == 1 {
    // Check MBR signature at offset 510
    if buf[510] == 0x55 && buf[511] == 0xAA {
        println!("Valid MBR found");
    }
}
```

### SYS_BLOCK_WRITE (29)

Write blocks to disk storage.

**Arguments:**
- `arg1`: LBA (Logical Block Address) - starting sector number
- `arg2`: Pointer to buffer (must be at least 512 * count bytes)
- `arg3`: Number of blocks to write

**Returns:**
- Number of blocks written on success
- -1 on error

**Behavior:**
- Writes 512-byte sectors to virtio-blk device
- Buffer must contain all data to write
- Partial writes possible if error occurs mid-operation

**Example:**
```rust
// Write to sector 1
let buf = [0u8; 512];
let blocks_written = block_write(1, &buf, 1);
```

### SYS_GET_DEVICE_LIST (30)

Query the device tree to get a list of all detected devices.

**Arguments:**
- `arg1`: Pointer to array of DeviceInfo structures
- `arg2`: Maximum number of devices to return

**Returns:**
- Number of devices returned on success
- -1 on error

**DeviceInfo Structure:**
```rust
#[repr(C)]
pub struct DeviceInfo {
    pub name: [u8; 32],      // Device name (null-terminated)
    pub bus_type: u32,       // Bus type (0=Platform, 1=PS2, 2=PCI, 3=Virtio)
    pub io_base: u64,        // I/O base address
    pub irq: u32,            // IRQ number (0xFFFFFFFF if none)
    pub state: u32,          // Device state (0=Detected, 1=Initializing, 2=Active, 3=Failed, 4=Shutdown)
    pub has_driver: u32,     // 1 if driver is loaded, 0 otherwise
}
```

**Example:**
```rust
let mut devices = [DeviceInfo::default(); 16];
let count = get_device_list(&mut devices);
for i in 0..count {
    let name = core::str::from_utf8(&devices[i].name).unwrap_or("???");
    println!("Device: {}", name);
}
```

### SYS_GET_BLOCK_DEVICE_INFO (31)

Get information about the block device (disk).

**Arguments:**
- `arg1`: Pointer to BlockDeviceInfo structure

**Returns:**
- 0 on success
- -1 on error

**BlockDeviceInfo Structure:**
```rust
#[repr(C)]
pub struct BlockDeviceInfo {
    pub block_count: u64,    // Total number of blocks
    pub block_size: u32,     // Size of each block in bytes
    pub capacity_mb: u32,    // Total capacity in megabytes
}
```

**Example:**
```rust
let mut info = BlockDeviceInfo::default();
if get_block_device_info(&mut info) == 0 {
    println!("Disk capacity: {} MB", info.capacity_mb);
    println!("Block size: {} bytes", info.block_size);
    println!("Total blocks: {}", info.block_count);
}
```

## Error Handling

All syscalls return -1 on error. Common error conditions:

- **Invalid buffer pointer**: Buffer not in user space or NULL
- **Invalid buffer size**: Buffer too small for operation
- **Device not ready**: Driver not initialized or device unavailable
- **I/O error**: Hardware error during operation
- **Invalid LBA**: Block address out of range

## Usage in Userland

The syscalls are available through wrapper functions in:
- `kernel/userspace/mellobox/src/syscalls.rs`
- `kernel/userspace/mello-sh/src/syscalls.rs`

These wrappers provide a safe Rust interface to the syscalls.

## Implementation Notes

### Keyboard Input
- Uses PS/2 keyboard driver with 256-byte circular buffer
- Scancode to ASCII translation for US keyboard layout
- IRQ 1 handler fills buffer on key press

### Serial Port
- UART16550 driver on COM1 (0x3F8)
- 38400 baud, 8N1 configuration
- Non-blocking reads, blocking writes

### Block Device
- virtio-blk driver for QEMU/KVM environments
- 512-byte sector size (standard)
- Simplified virtqueue implementation (stub for now)

### Device Tree
- Scans Platform, PS/2, PCI, and virtio buses
- Maintains registry of detected devices
- Tracks driver association and device state

## Future Enhancements

- USB keyboard support
- Multiple block devices
- Network device syscalls
- Graphics device syscalls
- DMA support for block I/O
- Asynchronous I/O operations
