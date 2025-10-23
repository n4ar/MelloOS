# Device Drivers & I/O Subsystem Architecture

## Overview

The Device Drivers & I/O Subsystem (Phase 7) provides MelloOS with the ability to interact with hardware devices. This document describes the architecture, design decisions, and implementation details of the driver framework and built-in drivers.

## Architecture

### Layered Design

The driver subsystem follows a layered architecture:

```
┌─────────────────────────────────────────────────────────────┐
│                    Userland Applications                     │
│  (kbd_test, serial_test, disk_bench, dmesg, lsdev, etc.)   │
└────────────────────────┬────────────────────────────────────┘
                         │ syscalls (read/write/ioctl)
┌────────────────────────┴────────────────────────────────────┐
│                    Kernel Subsystems                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   VFS/stdin  │  │  Block I/O   │  │  TTY/Console │     │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘     │
└─────────┼──────────────────┼──────────────────┼─────────────┘
          │                  │                  │
┌─────────┴──────────────────┴──────────────────┴─────────────┐
│              Driver Manager & Device Tree                    │
│  ┌────────────────────────────────────────────────────┐     │
│  │  Driver Registry  │  Device Registry  │  Probing   │     │
│  └────────────────────────────────────────────────────┘     │
└────────────────────────┬────────────────────────────────────┘
                         │
┌────────────────────────┴────────────────────────────────────┐
│                    Device Drivers                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Keyboard   │  │    Serial    │  │  virtio-blk  │     │
│  │   (PS/2)     │  │  (UART16550) │  │   (Block)    │     │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘     │
└─────────┼──────────────────┼──────────────────┼─────────────┘
          │                  │                  │
┌─────────┴──────────────────┴──────────────────┴─────────────┐
│              Interrupt & I/O Infrastructure                  │
│  ┌────────────────────────────────────────────────────┐     │
│  │  IRQ Handler  │  IOAPIC Routing  │  I/O Ports     │     │
│  └────────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────────┘
```

### Directory Structure

```
kernel/src/
├── drivers/
│   ├── mod.rs              # Driver manager, registry, probing
│   ├── input/
│   │   ├── mod.rs
│   │   └── keyboard.rs     # PS/2 keyboard driver
│   ├── serial/
│   │   ├── mod.rs
│   │   └── uart16550.rs    # COM port driver
│   └── block/
│       ├── mod.rs
│       └── virtio_blk.rs   # virtio block device
├── io/
│   ├── mod.rs
│   ├── port.rs             # inb/outb/inw/outw/inl/outl
│   ├── mmio.rs             # Memory-mapped I/O utilities
│   ├── irq.rs              # IRQ registration and handling
│   └── devtree.rs          # Device tree and bus scanning
└── main.rs                 # init_drivers() call during boot
```

## Core Components

### 1. Driver Manager

The Driver Manager (`drivers/mod.rs`) orchestrates the driver lifecycle:

- **Driver Registry**: Maintains list of available drivers
- **Device Registry**: Tracks all detected hardware devices
- **Probing**: Matches drivers with compatible devices
- **Initialization**: Calls driver init functions
- **Lifecycle Management**: Handles driver startup and shutdown

**Key Data Structures:**

```rust
pub struct Driver {
    pub name: &'static str,
    pub probe: fn(&Device) -> bool,
    pub init: fn(&Device) -> Result<(), DriverError>,
    pub shutdown: fn(&Device) -> Result<(), DriverError>,
}

pub struct Device {
    pub name: &'static str,
    pub bus: BusType,
    pub io_base: u64,
    pub irq: Option<u8>,
    pub irq_affinity: Option<u8>,
    pub driver: Option<&'static str>,
    pub state: DeviceState,
}
```

### 2. I/O Infrastructure

#### Port I/O (`io/port.rs`)

Provides safe wrappers for x86_64 I/O port operations:

- `inb/outb`: 8-bit port I/O
- `inw/outw`: 16-bit port I/O
- `inl/outl`: 32-bit port I/O

Includes mock implementation for testing.

#### MMIO (`io/mmio.rs`)

Memory-mapped I/O utilities with volatile semantics:

- `mmio_read/mmio_write`: Generic MMIO access
- `mmio_read32/mmio_write32`: 32-bit MMIO operations

#### IRQ Management (`io/irq.rs`)

Interrupt handling infrastructure:

- **IRQ Registration**: `register_irq_handler()` and `register_irq_handler_affinity()`
- **IRQ Dispatch**: `handle_irq()` routes interrupts to registered handlers
- **IOAPIC Integration**: Configures interrupt routing for SMP systems
- **CPU Affinity**: Supports pinning IRQs to specific CPU cores

#### Device Tree (`io/devtree.rs`)

Device discovery and enumeration:

- **Bus Scanning**: Scans Platform, PS/2, PCI, and virtio buses
- **Device Registration**: Adds discovered devices to device tree
- **Bus Types**: Platform, PS/2, PCI, Virtio

### 3. Device Drivers

#### PS/2 Keyboard Driver (`drivers/input/keyboard.rs`)

**Features:**
- Scancode to ASCII translation (US layout)
- Circular buffer (256 bytes)
- IRQ 1 interrupt handling
- Non-blocking read interface

**Key Functions:**
- `keyboard_init()`: Configure PS/2 controller
- `keyboard_irq_handler()`: Handle keyboard interrupts
- `keyboard_read()`: Read character from buffer

#### UART16550 Serial Driver (`drivers/serial/uart16550.rs`)

**Features:**
- COM1 port (0x3F8) support
- 38400 baud, 8N1 configuration
- Transmit and receive functions
- `serial_println!` macro for kernel debugging

**Key Functions:**
- `serial_init()`: Configure UART
- `serial_write()`: Transmit byte
- `serial_read()`: Receive byte (non-blocking)
- `serial_write_str()`: Transmit string

#### virtio-blk Block Driver (`drivers/block/virtio_blk.rs`)

**Features:**
- Paravirtualized block device for QEMU/KVM
- BlockDevice trait for filesystem integration
- Read/write operations with LBA addressing
- Capacity and block size queries

**Key Functions:**
- `virtio_blk_init()`: Initialize virtio device
- `block_read()`: Read block from disk
- `block_write()`: Write block to disk

**BlockDevice Trait:**
```rust
pub trait BlockDevice: Send + Sync {
    fn read_block(&self, lba: u64, buf: &mut [u8]) -> Result<(), BlockError>;
    fn write_block(&self, lba: u64, buf: &[u8]) -> Result<(), BlockError>;
    fn block_count(&self) -> u64;
    fn block_size(&self) -> usize;
}
```

## Boot Sequence

The driver subsystem initializes during kernel boot:

```rust
pub fn init_drivers() {
    log::info!("Initializing driver subsystem");
    
    // 1. Initialize IOAPIC routing
    crate::io::irq::init_ioapic_routing();
    
    // 2. Register all drivers
    register_builtin_drivers();
    
    // 3. Scan buses in deterministic order
    scan_platform_bus();
    scan_ps2_bus();
    scan_pci_bus();
    scan_virtio_bus();
    
    // 4. Probe and initialize
    driver_probe_all();
    
    log::info!("Driver initialization complete");
}
```

**Boot Order:**
1. IOAPIC initialization (before driver registration)
2. Driver registration (keyboard, serial, virtio-blk)
3. Bus scanning (Platform → PS/2 → PCI → virtio)
4. Driver probing and initialization
5. Device activation

## System Call Interface

Userland programs access drivers through syscalls:

### Device Access Syscalls

- `sys_read_stdin()`: Read keyboard input
- `sys_serial_write()`: Write to serial port
- `sys_serial_read()`: Read from serial port
- `sys_block_read()`: Read disk sector
- `sys_block_write()`: Write disk sector

### Device Information Syscalls

- `sys_get_device_list()`: Query device tree
- `sys_get_block_device_info()`: Get disk information

## Userland Testing Tools

### kbd_test

Tests keyboard driver by echoing typed characters.

```bash
kbd_test
# Type characters, they will be echoed back
# Press Ctrl+C to exit
```

### serial_test

Tests serial port with loopback verification.

```bash
serial_test
# Performs serial loopback test
# Reports success or failure
```

### disk_bench

Reads sector 0 (MBR) and verifies disk signature.

```bash
disk_bench
# Reads MBR from disk
# Verifies 0x55AA signature
# Reports read time
```

### dmesg

Displays kernel log buffer including driver messages.

```bash
dmesg
# Shows kernel log with driver lifecycle events
```

### lsdev

Lists all detected devices in the device tree.

```bash
lsdev
# Output:
# NAME              BUS       IO_BASE    IRQ    DRIVER
# --------------------------------------------------------
# ps2-keyboard      PS2       0x00000060   1    ps2-keyboard
# serial-com1       Platform  0x000003F8   4    uart16550
# virtio-blk        Virtio    0x00000000  N/A   virtio-blk
```

### diskinfo

Displays block device information.

```bash
diskinfo
# Output:
# Block Device Information:
#   Block count: 204800
#   Block size:  512 bytes
#   Total size:  100 MB
```

### irq_test

Tests interrupt distribution across CPU cores.

```bash
irq_test
# Triggers interrupts and reports distribution
# Verifies IOAPIC routing is working correctly
```

## SMP Integration

The driver subsystem is fully SMP-safe:

### IRQ Affinity

IRQs can be pinned to specific CPU cores:

```rust
register_irq_handler_affinity(irq, handler, Some(cpu_id));
```

### Interrupt Distribution

- IOAPIC routes interrupts to target CPUs
- IRQ handlers log which CPU handled the interrupt
- Lock-free or properly synchronized data structures

### Concurrent Access

- Per-device locks prevent race conditions
- Atomic operations for shared counters
- Seqlock for consistent reads

## Error Handling

### Driver Errors

```rust
pub enum DriverError {
    ProbeFailure,           // Device not compatible
    InitFailure,            // Initialization failed
    IoError,                // I/O operation failed
    ResourceUnavailable,    // Required resource not available
    NotSupported,           // Feature not supported
}
```

### Error Recovery Strategy

1. **Probe Failure**: Continue trying other drivers
2. **Init Failure**: Mark device as unavailable, continue boot
3. **Runtime I/O Error**: Return error to caller, don't panic
4. **Interrupt Storm**: Throttle or disable IRQ, log warning

### Logging

All driver operations are logged:

- `info`: Driver registration, initialization, shutdown
- `warn`: Probe failures, missing drivers
- `error`: Initialization failures, I/O errors
- `trace`: Interrupt handling, I/O operations

## Integration with Other Subsystems

### SMP (Phase 5)

- Uses existing LAPIC/IOAPIC infrastructure
- Follows lock ordering rules
- Supports per-CPU interrupt handling

### Scheduler (Phase 3)

- Blocking I/O can sleep tasks
- IRQ handlers wake waiting tasks
- I/O-bound tasks may get priority boost

### Syscalls (Phase 4)

- New syscalls for device access
- Proper error code propagation
- Buffer validation for userland pointers

### Memory Management (Phase 2)

- DMA buffer allocation (future)
- MMIO region mapping
- Driver state allocation

### Userland (Phase 6)

- Userland wrappers for syscalls
- Testing tools for verification
- Error handling in userspace

## Future Extensibility

### Phase 8: Filesystem

The BlockDevice trait enables filesystem mounting:

```rust
let block_dev = get_block_device("virtio-blk")?;
let fs = Ext2Filesystem::mount(block_dev)?;
```

### Phase 9: Networking

Similar driver model for network devices:

```rust
pub trait NetworkDevice {
    fn send_packet(&self, packet: &[u8]) -> Result<(), NetError>;
    fn recv_packet(&self) -> Option<Vec<u8>>;
}
```

### Phase 10: Graphics

Framebuffer driver follows same pattern:

```rust
pub trait FramebufferDevice {
    fn get_info(&self) -> FbInfo;
    fn write_pixel(&self, x: u32, y: u32, color: u32);
}
```

## Design Decisions

### Static Driver Registration

**Decision**: Drivers are registered at compile time.

**Rationale**: Simpler implementation, no dynamic linking needed, sufficient for Phase 7.

### Interrupt-Driven I/O

**Decision**: Use interrupts for keyboard and serial, polling for disk (initially).

**Rationale**: Efficient for low-bandwidth devices, simpler initial implementation.

### Synchronous Block I/O

**Decision**: Block I/O operations are synchronous (blocking).

**Rationale**: Simpler implementation, sufficient for Phase 8 filesystem needs.

### Single Block Device

**Decision**: Support only one block device initially.

**Rationale**: Sufficient for single root filesystem, can be extended later.

### Simplified virtio

**Decision**: Simplified virtio without full virtqueue implementation.

**Rationale**: Full virtio is complex, simplified version sufficient for basic disk I/O.

## Performance Considerations

### Interrupt Overhead

- Minimal work in interrupt handlers
- Defer complex processing to task context
- Use lock-free structures where possible

### Lock Contention

- Per-device locks minimize contention
- Short critical sections
- Avoid holding locks across I/O operations

### Cache Efficiency

- Small, aligned data structures
- Minimize false sharing in SMP
- Keep hot data together

### I/O Batching

- Batch operations where possible (future)
- Coalesce small writes
- Use DMA for large transfers (future)

## Security Considerations

### I/O Port Access Control

Only kernel code can access I/O ports. Userland must use syscalls.

### Buffer Validation

All buffers from userland are validated:
- Check buffer is in userland address space
- Check buffer size is reasonable
- Prevent buffer overflows

### IRQ Handler Safety

IRQ handlers are carefully written:
- No panics in IRQ context
- Proper synchronization with SMP
- Minimal work in handler

### Device Resource Isolation

Each device has isolated resources:
- Separate I/O port ranges
- Separate IRQ numbers
- Separate memory regions

## Testing

### Unit Tests

- I/O port operations with mock interface
- Scancode translation tables
- Buffer management
- Error handling

### Integration Tests

- `test_drivers.sh`: Comprehensive driver testing
- Keyboard input/output verification
- Serial loopback testing
- Disk read/write verification
- Device enumeration testing
- IRQ distribution testing (SMP)

### Performance Tests

- Keyboard latency: < 10ms
- Serial throughput: > 38400 baud
- Disk throughput: > 10 MB/sec (virtio-blk)

## Troubleshooting

### Common Issues

**Issue**: Device not detected
- Check bus scanning order
- Verify device is present in QEMU
- Check device tree with `lsdev`

**Issue**: IRQ not firing
- Verify IOAPIC initialization
- Check IRQ registration
- Verify device IRQ configuration

**Issue**: I/O operation fails
- Check device state
- Verify I/O port/MMIO addresses
- Check for hardware errors

### Debug Tools

- `dmesg`: View driver log messages
- `lsdev`: List detected devices
- `irq_test`: Verify interrupt routing
- Serial output: Kernel debug messages

## References

- **Requirements**: `.kiro/specs/device-drivers-io/requirements.md`
- **Design**: `.kiro/specs/device-drivers-io/design.md`
- **Tasks**: `.kiro/specs/device-drivers-io/tasks.md`
- **Device Syscalls**: `docs/architecture/device-syscalls.md`
- **I/O Infrastructure**: `docs/architecture/IO Infrastructure.md`

## Conclusion

The Device Drivers & I/O Subsystem provides MelloOS with essential hardware interaction capabilities. The modular architecture allows for easy addition of new drivers, while clear interfaces enable integration with existing kernel subsystems. The design prioritizes correctness and simplicity for Phase 7, with clear paths for future enhancements in subsequent phases.
