# Design Document - Device Drivers & I/O Subsystem

## Overview

The Device Drivers & I/O Subsystem transforms MelloOS from a memory-only operating system into one capable of real hardware interaction. This design implements a modular driver architecture with clear separation between device abstraction, driver implementation, and hardware access layers.

The system follows a layered approach:
- **Hardware Layer**: Raw I/O port and MMIO access
- **Driver Layer**: Device-specific implementations (keyboard, serial, disk)
- **Abstraction Layer**: Unified device and driver interfaces
- **Management Layer**: Driver registration, probing, and lifecycle management

This design prioritizes SMP safety, error resilience, and extensibility for future device types.

## Architecture

### High-Level Component Diagram

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
│       ├── virtio_blk.rs   # virtio block device
│       └── ahci.rs         # (future) AHCI SATA driver
├── io/
│   ├── mod.rs
│   ├── port.rs             # inb/outb/inw/outw/inl/outl
│   ├── mmio.rs             # Memory-mapped I/O utilities
│   ├── irq.rs              # IRQ registration and handling
│   └── devtree.rs          # Device tree and bus scanning
└── main.rs                 # init_drivers() call during boot
```


## Components and Interfaces

### 1. Driver Manager (`drivers/mod.rs`)

The Driver Manager is the central orchestrator for device driver lifecycle.

#### Core Data Structures

```rust
/// Represents a device driver
pub struct Driver {
    pub name: &'static str,
    pub probe: fn(&Device) -> bool,
    pub init: fn(&Device) -> Result<(), DriverError>,
    pub shutdown: fn(&Device) -> Result<(), DriverError>,
}

/// Represents a hardware device
pub struct Device {
    pub name: &'static str,
    pub bus: BusType,
    pub io_base: u64,
    pub irq: Option<u8>,
    pub irq_affinity: Option<u8>, // Target CPU core for IRQ routing
    pub driver: Option<&'static str>,
    pub state: DeviceState,
}

/// Device state tracking
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceState {
    Detected,      // Device found but not initialized
    Initializing,  // Driver init in progress
    Active,        // Device operational
    Failed,        // Initialization failed
    Shutdown,      // Device shut down
}

/// Bus types supported by the system
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BusType {
    Platform,   // Platform devices (built-in)
    PS2,        // PS/2 keyboard/mouse
    PCI,        // PCI/PCIe devices
    Virtio,     // Paravirtualized devices
}

/// Driver-specific errors
#[derive(Debug)]
pub enum DriverError {
    ProbeFailure,
    InitFailure,
    IoError,
    ResourceUnavailable,
    NotSupported,
}
```

#### Driver Registry

```rust
static DRIVER_REGISTRY: Mutex<Vec<Driver>> = Mutex::new(Vec::new());
static DEVICE_REGISTRY: Mutex<Vec<Device>> = Mutex::new(Vec::new());

pub fn driver_register(driver: Driver) {
    let mut registry = DRIVER_REGISTRY.lock();
    log::info!("Registering driver: {}", driver.name);
    registry.push(driver);
}

pub fn device_register(device: Device) {
    let mut registry = DEVICE_REGISTRY.lock();
    log::info!("Registering device: {} on {:?} bus", device.name, device.bus);
    registry.push(device);
}
```

#### Probing and Initialization

```rust
pub fn driver_probe_all() {
    let drivers = DRIVER_REGISTRY.lock();
    let mut devices = DEVICE_REGISTRY.lock();
    
    for device in devices.iter_mut() {
        if device.driver.is_some() {
            continue; // Already has a driver
        }
        
        for driver in drivers.iter() {
            if (driver.probe)(device) {
                log::info!("Driver {} matched device {}", driver.name, device.name);
                match (driver.init)(device) {
                    Ok(()) => {
                        device.driver = Some(driver.name);
                        log::info!("Driver {} initialized successfully", driver.name);
                        break;
                    }
                    Err(e) => {
                        log::error!("Driver {} init failed: {:?}", driver.name, e);
                    }
                }
            }
        }
        
        if device.driver.is_none() {
            log::warn!("No driver found for device {}", device.name);
        }
    }
}
```

#### Boot Sequence

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

/// Register all built-in drivers
fn register_builtin_drivers() {
    log::info!("Registering built-in drivers");
    driver_register(crate::drivers::input::keyboard::KEYBOARD_DRIVER);
    driver_register(crate::drivers::serial::uart16550::SERIAL_DRIVER);
    driver_register(crate::drivers::block::virtio_blk::VIRTIO_BLK_DRIVER);
}
```


### 2. I/O Port Utilities (`io/port.rs`)

Provides safe wrappers for x86_64 I/O port operations with mock support for testing.

```rust
use core::arch::asm;

#[cfg(not(test))]
mod port_impl {
    /// Read a byte from an I/O port
    #[inline]
    pub unsafe fn inb(port: u16) -> u8 {
        let value: u8;
        asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack));
        value
    }

    /// Write a byte to an I/O port
    #[inline]
    pub unsafe fn outb(port: u16, value: u8) {
        asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack));
    }

    /// Read a word (16-bit) from an I/O port
    #[inline]
    pub unsafe fn inw(port: u16) -> u16 {
        let value: u16;
        asm!("in ax, dx", out("ax") value, in("dx") port, options(nomem, nostack));
        value
    }

    /// Write a word (16-bit) to an I/O port
    #[inline]
    pub unsafe fn outw(port: u16, value: u16) {
        asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack));
    }

    /// Read a double word (32-bit) from an I/O port
    #[inline]
    pub unsafe fn inl(port: u16) -> u32 {
        let value: u32;
        asm!("in eax, dx", out("eax") value, in("dx") port, options(nomem, nostack));
        value
    }

    /// Write a double word (32-bit) to an I/O port
    #[inline]
    pub unsafe fn outl(port: u16, value: u32) {
        asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack));
    }
}

#[cfg(test)]
mod port_impl {
    use crate::sync::Mutex;
    use alloc::collections::BTreeMap;
    
    static MOCK_PORTS: Mutex<BTreeMap<u16, u32>> = Mutex::new(BTreeMap::new());
    
    /// Mock implementation for testing
    pub unsafe fn inb(port: u16) -> u8 {
        MOCK_PORTS.lock().get(&port).copied().unwrap_or(0) as u8
    }
    
    pub unsafe fn outb(port: u16, value: u8) {
        MOCK_PORTS.lock().insert(port, value as u32);
    }
    
    pub unsafe fn inw(port: u16) -> u16 {
        MOCK_PORTS.lock().get(&port).copied().unwrap_or(0) as u16
    }
    
    pub unsafe fn outw(port: u16, value: u16) {
        MOCK_PORTS.lock().insert(port, value as u32);
    }
    
    pub unsafe fn inl(port: u16) -> u32 {
        MOCK_PORTS.lock().get(&port).copied().unwrap_or(0)
    }
    
    pub unsafe fn outl(port: u16, value: u32) {
        MOCK_PORTS.lock().insert(port, value);
    }
}

// Re-export implementation
pub use port_impl::*;
```

### 3. MMIO Utilities (`io/mmio.rs`)

Memory-mapped I/O access with proper volatile semantics.

```rust
use core::ptr;

/// Read from a memory-mapped register
#[inline]
pub unsafe fn mmio_read<T>(addr: usize) -> T {
    ptr::read_volatile(addr as *const T)
}

/// Write to a memory-mapped register
#[inline]
pub unsafe fn mmio_write<T>(addr: usize, value: T) {
    ptr::write_volatile(addr as *mut T, value);
}

/// Read 32-bit value from MMIO
#[inline]
pub unsafe fn mmio_read32(addr: usize) -> u32 {
    mmio_read::<u32>(addr)
}

/// Write 32-bit value to MMIO
#[inline]
pub unsafe fn mmio_write32(addr: usize, value: u32) {
    mmio_write::<u32>(addr, value);
}
```


### 4. IRQ Management (`io/irq.rs`)

Handles interrupt registration and routing for SMP systems.

```rust
use crate::sync::Mutex;
use alloc::vec::Vec;

type IrqHandler = fn();

static IRQ_HANDLERS: Mutex<[Option<IrqHandler>; 256]> = Mutex::new([None; 256]);

/// Initialize IOAPIC routing before driver registration
pub fn init_ioapic_routing() {
    log::info!("Initializing IOAPIC routing for device drivers");
    // Ensure IOAPIC is ready to accept IRQ configurations
    crate::arch::x86_64::apic::init_ioapic();
}

/// Register an IRQ handler with optional CPU affinity
pub fn register_irq_handler(irq: u8, handler: IrqHandler) -> Result<(), &'static str> {
    register_irq_handler_affinity(irq, handler, None)
}

/// Register an IRQ handler with CPU affinity
pub fn register_irq_handler_affinity(
    irq: u8, 
    handler: IrqHandler, 
    cpu_affinity: Option<u8>
) -> Result<(), &'static str> {
    let mut handlers = IRQ_HANDLERS.lock();
    
    if handlers[irq as usize].is_some() {
        return Err("IRQ already registered");
    }
    
    handlers[irq as usize] = Some(handler);
    
    let target_cpu = cpu_affinity.unwrap_or(0);
    log::info!("Registered IRQ {} handler (CPU affinity: {})", irq, target_cpu);
    
    // Configure IOAPIC routing
    configure_ioapic_irq(irq, target_cpu);
    
    Ok(())
}

/// Unregister an IRQ handler
pub fn unregister_irq_handler(irq: u8) {
    let mut handlers = IRQ_HANDLERS.lock();
    handlers[irq as usize] = None;
    log::info!("Unregistered IRQ {} handler", irq);
}

/// Called by interrupt dispatcher
pub fn handle_irq(irq: u8) {
    let handlers = IRQ_HANDLERS.lock();
    
    if let Some(handler) = handlers[irq as usize] {
        let cpu_id = crate::arch::cpu_id();
        log::trace!("Handling IRQ {} on CPU {}", irq, cpu_id);
        handler();
    } else {
        log::warn!("Unhandled IRQ {}", irq);
    }
}

/// Configure IOAPIC for IRQ routing with CPU affinity
fn configure_ioapic_irq(irq: u8, target_cpu: u8) {
    // Map IRQ to interrupt vector (IRQ + 32 for x86_64)
    let vector = irq + 32;
    
    // Configure IOAPIC redirection entry
    crate::arch::x86_64::apic::ioapic_set_irq(irq, vector, target_cpu);
}
```

### 5. Device Tree (`io/devtree.rs`)

Manages device discovery and registration.

```rust
use crate::drivers::{Device, BusType};
use crate::sync::Mutex;
use alloc::vec::Vec;

static DEVICE_TREE: Mutex<Vec<Device>> = Mutex::new(Vec::new());

/// Scan platform bus for built-in devices
pub fn scan_platform_bus() {
    log::info!("Scanning platform bus");
    // Platform devices are typically hardcoded
    // (e.g., framebuffer, timer, etc.)
}

/// Scan PS/2 bus for keyboard/mouse
pub fn scan_ps2_bus() {
    log::info!("Scanning PS/2 bus");
    
    // Check if PS/2 controller exists
    if ps2_controller_present() {
        // Register keyboard device
        let kbd_device = Device {
            name: "ps2-keyboard",
            bus: BusType::PS2,
            io_base: 0x60,
            irq: Some(1),
            irq_affinity: None, // Let system decide
            driver: None,
            state: DeviceState::Detected,
        };
        crate::drivers::device_register(kbd_device);
    }
}

/// Scan PCI bus for devices
pub fn scan_pci_bus() {
    log::info!("Scanning PCI bus");
    // PCI enumeration will be implemented in future
    // For now, this is a placeholder
}

/// Scan virtio bus for paravirtualized devices
pub fn scan_virtio_bus() {
    log::info!("Scanning virtio bus");
    
    // Detect virtio-blk devices
    // This will use PCI scanning to find virtio devices
    // For now, assume virtio-blk at standard location
    
    let virtio_blk = Device {
        name: "virtio-blk",
        bus: BusType::Virtio,
        io_base: 0, // Will be determined by PCI BAR
        irq: None,  // Will be determined by PCI
        irq_affinity: None, // Let system decide
        driver: None,
        state: DeviceState::Detected,
    };
    crate::drivers::device_register(virtio_blk);
}

fn ps2_controller_present() -> bool {
    // Simple check: try to read from PS/2 status port
    unsafe {
        let status = crate::io::port::inb(0x64);
        // If we get 0xFF, controller likely doesn't exist
        status != 0xFF
    }
}
```


### 6. Keyboard Driver (`drivers/input/keyboard.rs`)

PS/2 keyboard driver with scancode translation.

```rust
use crate::io::port::{inb, outb};
use crate::io::irq::register_irq_handler;
use crate::drivers::{Driver, Device, DriverError};
use crate::sync::Mutex;

const KBD_DATA_PORT: u16 = 0x60;
const KBD_STATUS_PORT: u16 = 0x64;
const KBD_COMMAND_PORT: u16 = 0x64;

static KEYBOARD_BUFFER: Mutex<[u8; 256]> = Mutex::new([0; 256]);
static BUFFER_HEAD: Mutex<usize> = Mutex::new(0);
static BUFFER_TAIL: Mutex<usize> = Mutex::new(0);

/// Scancode to ASCII translation table (US layout, simplified)
static SCANCODE_TO_ASCII: [u8; 128] = [
    0, 27, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'=', 8, // backspace
    b'\t', b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'[', b']', b'\n',
    0, // Ctrl
    b'a', b's', b'd', b'f', b'g', b'h', b'j', b'k', b'l', b';', b'\'', b'`',
    0, // Left shift
    b'\\', b'z', b'x', b'c', b'v', b'b', b'n', b'm', b',', b'.', b'/', 
    0, // Right shift
    b'*',
    0, // Alt
    b' ', // Space
    // ... rest of table
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub fn keyboard_probe(device: &Device) -> bool {
    device.name == "ps2-keyboard"
}

pub fn keyboard_init(device: &Device) -> Result<(), DriverError> {
    log::info!("Initializing PS/2 keyboard driver");
    
    // Register IRQ handler
    register_irq_handler(1, keyboard_irq_handler)
        .map_err(|_| DriverError::InitFailure)?;
    
    // Enable keyboard
    unsafe {
        outb(KBD_COMMAND_PORT, 0xAE); // Enable first PS/2 port
    }
    
    log::info!("PS/2 keyboard initialized");
    Ok(())
}

pub fn keyboard_shutdown(_device: &Device) -> Result<(), DriverError> {
    log::info!("Shutting down PS/2 keyboard");
    crate::io::irq::unregister_irq_handler(1);
    Ok(())
}

fn keyboard_irq_handler() {
    unsafe {
        let scancode = inb(KBD_DATA_PORT);
        
        // Ignore key release events (high bit set)
        if scancode & 0x80 != 0 {
            return;
        }
        
        // Translate scancode to ASCII
        if let Some(ascii) = SCANCODE_TO_ASCII.get(scancode as usize) {
            if *ascii != 0 {
                // Add to buffer
                let mut head = BUFFER_HEAD.lock();
                let tail = BUFFER_TAIL.lock();
                let mut buffer = KEYBOARD_BUFFER.lock();
                
                let next_head = (*head + 1) % 256;
                if next_head != *tail {
                    buffer[*head] = *ascii;
                    *head = next_head;
                }
            }
        }
    }
}

/// Read a character from keyboard buffer (non-blocking)
pub fn keyboard_read() -> Option<u8> {
    let mut head = BUFFER_HEAD.lock();
    let mut tail = BUFFER_TAIL.lock();
    let buffer = KEYBOARD_BUFFER.lock();
    
    if *head == *tail {
        None
    } else {
        let ch = buffer[*tail];
        *tail = (*tail + 1) % 256;
        Some(ch)
    }
}

pub const KEYBOARD_DRIVER: Driver = Driver {
    name: "ps2-keyboard",
    probe: keyboard_probe,
    init: keyboard_init,
    shutdown: keyboard_shutdown,
};
```


### 7. Serial Driver (`drivers/serial/uart16550.rs`)

UART16550 serial port driver for COM1.

```rust
use crate::io::port::{inb, outb};
use crate::drivers::{Driver, Device, DriverError};
use crate::sync::Mutex;

const COM1_PORT: u16 = 0x3F8;

static SERIAL_PORT: Mutex<Option<SerialPort>> = Mutex::new(None);

struct SerialPort {
    base: u16,
}

impl SerialPort {
    fn new(base: u16) -> Self {
        SerialPort { base }
    }
    
    fn init(&self) {
        unsafe {
            // Disable interrupts
            outb(self.base + 1, 0x00);
            
            // Enable DLAB (set baud rate divisor)
            outb(self.base + 3, 0x80);
            
            // Set divisor to 3 (38400 baud)
            outb(self.base + 0, 0x03);
            outb(self.base + 1, 0x00);
            
            // 8 bits, no parity, one stop bit
            outb(self.base + 3, 0x03);
            
            // Enable FIFO, clear them, with 14-byte threshold
            outb(self.base + 2, 0xC7);
            
            // IRQs enabled, RTS/DSR set
            outb(self.base + 4, 0x0B);
        }
    }
    
    fn write_byte(&self, byte: u8) {
        unsafe {
            // Wait for transmit buffer to be empty
            while (inb(self.base + 5) & 0x20) == 0 {}
            outb(self.base, byte);
        }
    }
    
    fn read_byte(&self) -> Option<u8> {
        unsafe {
            // Check if data is available
            if (inb(self.base + 5) & 0x01) != 0 {
                Some(inb(self.base))
            } else {
                None
            }
        }
    }
}

pub fn serial_probe(device: &Device) -> bool {
    device.name == "serial-com1"
}

pub fn serial_init(device: &Device) -> Result<(), DriverError> {
    log::info!("Initializing UART16550 serial driver");
    
    let port = SerialPort::new(COM1_PORT);
    port.init();
    
    let mut serial = SERIAL_PORT.lock();
    *serial = Some(port);
    
    log::info!("Serial port COM1 initialized");
    Ok(())
}

pub fn serial_shutdown(_device: &Device) -> Result<(), DriverError> {
    log::info!("Shutting down serial port");
    let mut serial = SERIAL_PORT.lock();
    *serial = None;
    Ok(())
}

/// Write a byte to serial port
pub fn serial_write(byte: u8) {
    if let Some(port) = SERIAL_PORT.lock().as_ref() {
        port.write_byte(byte);
    }
}

/// Write a string to serial port
pub fn serial_write_str(s: &str) {
    for byte in s.bytes() {
        serial_write(byte);
    }
}

/// Read a byte from serial port (non-blocking)
pub fn serial_read() -> Option<u8> {
    SERIAL_PORT.lock().as_ref().and_then(|port| port.read_byte())
}

/// Macro for serial println
#[macro_export]
macro_rules! serial_println {
    () => ($crate::drivers::serial::serial_write_str("\n"));
    ($($arg:tt)*) => ({
        $crate::drivers::serial::serial_write_str(&format!($($arg)*));
        $crate::drivers::serial::serial_write_str("\n");
    });
}

pub const SERIAL_DRIVER: Driver = Driver {
    name: "uart16550",
    probe: serial_probe,
    init: serial_init,
    shutdown: serial_shutdown,
};
```


### 8. Block Device Driver (`drivers/block/virtio_blk.rs`)

virtio-blk driver for disk I/O in QEMU environments.

```rust
use crate::drivers::{Driver, Device, DriverError};
use crate::sync::Mutex;
use alloc::vec::Vec;

const VIRTIO_BLK_VENDOR_ID: u16 = 0x1AF4;
const VIRTIO_BLK_DEVICE_ID: u16 = 0x1001;

/// Block device trait for filesystem integration
pub trait BlockDevice: Send + Sync {
    fn read_block(&self, lba: u64, buf: &mut [u8]) -> Result<(), BlockError>;
    fn write_block(&self, lba: u64, buf: &[u8]) -> Result<(), BlockError>;
    fn block_count(&self) -> u64;
    fn block_size(&self) -> usize;
}

#[derive(Debug)]
pub enum BlockError {
    IoError,
    InvalidLba,
    BufferTooSmall,
    DeviceNotReady,
}

struct VirtioBlkDevice {
    base_addr: usize,
    capacity: u64,
    block_size: usize,
}

impl VirtioBlkDevice {
    fn new(base_addr: usize) -> Self {
        // Read capacity from virtio config space
        let capacity = unsafe {
            crate::io::mmio::mmio_read32(base_addr + 0x14) as u64
        };
        
        VirtioBlkDevice {
            base_addr,
            capacity,
            block_size: 512, // Standard sector size
        }
    }
    
    fn init(&mut self) -> Result<(), DriverError> {
        log::info!("Initializing virtio-blk device");
        
        // 1. Reset device
        self.reset();
        
        // 2. Set ACKNOWLEDGE status bit
        self.set_status(1);
        
        // 3. Set DRIVER status bit
        self.set_status(2);
        
        // 4. Read feature bits and negotiate
        self.negotiate_features();
        
        // 5. Set FEATURES_OK status bit
        self.set_status(8);
        
        // 6. Set DRIVER_OK status bit
        self.set_status(4);
        
        log::info!("virtio-blk initialized: {} blocks of {} bytes", 
                   self.capacity, self.block_size);
        
        Ok(())
    }
    
    fn reset(&self) {
        unsafe {
            crate::io::mmio::mmio_write32(self.base_addr + 0x70, 0);
        }
    }
    
    fn set_status(&self, status: u8) {
        unsafe {
            let current = crate::io::mmio::mmio_read32(self.base_addr + 0x70) as u8;
            crate::io::mmio::mmio_write32(self.base_addr + 0x70, (current | status) as u32);
        }
    }
    
    fn negotiate_features(&self) {
        // For now, accept default features
        // In full implementation, would read and write feature bits
    }
}

impl BlockDevice for VirtioBlkDevice {
    fn read_block(&self, lba: u64, buf: &mut [u8]) -> Result<(), BlockError> {
        if lba >= self.capacity {
            return Err(BlockError::InvalidLba);
        }
        
        if buf.len() < self.block_size {
            return Err(BlockError::BufferTooSmall);
        }
        
        // Submit read request to virtqueue
        // This is simplified; full implementation would use virtqueue
        log::trace!("Reading block {} from virtio-blk", lba);
        
        // TODO: Implement actual virtqueue submission
        
        Ok(())
    }
    
    fn write_block(&self, lba: u64, buf: &[u8]) -> Result<(), BlockError> {
        if lba >= self.capacity {
            return Err(BlockError::InvalidLba);
        }
        
        if buf.len() < self.block_size {
            return Err(BlockError::BufferTooSmall);
        }
        
        log::trace!("Writing block {} to virtio-blk", lba);
        
        // TODO: Implement actual virtqueue submission
        
        Ok(())
    }
    
    fn block_count(&self) -> u64 {
        self.capacity
    }
    
    fn block_size(&self) -> usize {
        self.block_size
    }
}

static VIRTIO_BLK: Mutex<Option<VirtioBlkDevice>> = Mutex::new(None);

pub fn virtio_blk_probe(device: &Device) -> bool {
    device.name == "virtio-blk"
}

pub fn virtio_blk_init(device: &Device) -> Result<(), DriverError> {
    log::info!("Initializing virtio-blk driver");
    
    let mut blk_device = VirtioBlkDevice::new(device.io_base as usize);
    blk_device.init()?;
    
    let mut global = VIRTIO_BLK.lock();
    *global = Some(blk_device);
    
    Ok(())
}

pub fn virtio_blk_shutdown(_device: &Device) -> Result<(), DriverError> {
    log::info!("Shutting down virtio-blk");
    let mut global = VIRTIO_BLK.lock();
    *global = None;
    Ok(())
}

/// Public API for block I/O
pub fn block_read(lba: u64, buf: &mut [u8]) -> Result<(), BlockError> {
    let device = VIRTIO_BLK.lock();
    device.as_ref()
        .ok_or(BlockError::DeviceNotReady)?
        .read_block(lba, buf)
}

pub fn block_write(lba: u64, buf: &[u8]) -> Result<(), BlockError> {
    let device = VIRTIO_BLK.lock();
    device.as_ref()
        .ok_or(BlockError::DeviceNotReady)?
        .write_block(lba, buf)
}

pub const VIRTIO_BLK_DRIVER: Driver = Driver {
    name: "virtio-blk",
    probe: virtio_blk_probe,
    init: virtio_blk_init,
    shutdown: virtio_blk_shutdown,
};
```


## Data Models

### Device Registry Model

The device registry maintains a flat list of all detected devices. Each device has:

- **Identity**: Name and bus type
- **Resources**: I/O base address, IRQ number
- **State**: Associated driver (if any)

```rust
struct DeviceRegistry {
    devices: Vec<Device>,
}

impl DeviceRegistry {
    fn add(&mut self, device: Device) {
        self.devices.push(device);
    }
    
    fn find_by_name(&self, name: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.name == name)
    }
    
    fn find_by_bus(&self, bus: BusType) -> Vec<&Device> {
        self.devices.iter().filter(|d| d.bus == bus).collect()
    }
}
```

### Driver Registry Model

The driver registry maintains a list of all registered drivers. Each driver provides:

- **Identity**: Driver name
- **Callbacks**: Probe, init, shutdown functions

```rust
struct DriverRegistry {
    drivers: Vec<Driver>,
}

impl DriverRegistry {
    fn add(&mut self, driver: Driver) {
        self.drivers.push(driver);
    }
    
    fn find_by_name(&self, name: &str) -> Option<&Driver> {
        self.drivers.iter().find(|d| d.name == name)
    }
}
```

### IRQ Handler Model

IRQ handlers are stored in a fixed-size array indexed by IRQ number.

```rust
struct IrqTable {
    handlers: [Option<IrqHandler>; 256],
}

impl IrqTable {
    fn register(&mut self, irq: u8, handler: IrqHandler) -> Result<(), &'static str> {
        if self.handlers[irq as usize].is_some() {
            return Err("IRQ already registered");
        }
        self.handlers[irq as usize] = Some(handler);
        Ok(())
    }
    
    fn dispatch(&self, irq: u8) {
        if let Some(handler) = self.handlers[irq as usize] {
            handler();
        }
    }
}
```

## Error Handling

### Driver Errors

All driver operations return `Result<(), DriverError>` to enable graceful error handling:

```rust
pub enum DriverError {
    ProbeFailure,      // Device not compatible with driver
    InitFailure,       // Initialization failed
    IoError,           // I/O operation failed
    ResourceUnavailable, // Required resource not available
    NotSupported,      // Feature not supported
}
```

### Error Recovery Strategy

1. **Probe Failure**: Continue trying other drivers
2. **Init Failure**: Mark device as unavailable, continue boot
3. **Runtime I/O Error**: Return error to caller, don't panic
4. **Interrupt Storm**: Throttle or disable IRQ, log warning

### Logging Strategy

All driver operations are logged with appropriate levels:

- `info`: Driver registration, initialization, shutdown
- `warn`: Probe failures, missing drivers
- `error`: Initialization failures, I/O errors
- `trace`: Interrupt handling, I/O operations

Example:
```rust
log::info!("Registering driver: {}", driver.name);
log::warn!("No driver found for device {}", device.name);
log::error!("Driver {} init failed: {:?}", driver.name, e);
log::trace!("Handling IRQ {} on CPU {}", irq, cpu_id);
```


## Testing Strategy

### Unit Testing

Each driver component should have unit tests for:

1. **I/O Port Operations**: Test inb/outb with mock ports
2. **Scancode Translation**: Test keyboard scancode to ASCII mapping
3. **Buffer Management**: Test keyboard and serial buffers
4. **Error Handling**: Test error conditions and recovery

Example test structure:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scancode_translation() {
        assert_eq!(SCANCODE_TO_ASCII[0x1E], b'a');
        assert_eq!(SCANCODE_TO_ASCII[0x30], b'b');
    }
    
    #[test]
    fn test_keyboard_buffer() {
        // Test buffer wraparound
        // Test buffer full condition
    }
}
```

### Integration Testing

Integration tests verify driver interaction with hardware:

1. **Keyboard Test** (`kbd_test`):
   - User types characters
   - Program echoes them back
   - Verifies scancode translation and IRQ handling

2. **Serial Test** (`serial_test`):
   - Write data to COM1
   - Read it back (loopback mode)
   - Verify data integrity

3. **Disk Test** (`disk_bench`):
   - Read sector 0 (MBR)
   - Verify signature (0x55AA at offset 510)
   - Measure read performance

### Userland Testing Tools

#### kbd_test
```rust
// kernel/userspace/kbd_test/src/main.rs
fn main() {
    println!("Keyboard test - type characters (Ctrl+C to exit)");
    loop {
        if let Some(ch) = syscall_read_stdin() {
            syscall_write_stdout(ch);
        }
    }
}
```

#### serial_test
```rust
// kernel/userspace/serial_test/src/main.rs
fn main() {
    println!("Serial loopback test");
    let test_data = b"Hello, Serial!";
    
    // Write to serial
    for &byte in test_data {
        syscall_serial_write(byte);
    }
    
    // Read back
    let mut received = [0u8; 32];
    for i in 0..test_data.len() {
        received[i] = syscall_serial_read();
    }
    
    if &received[..test_data.len()] == test_data {
        println!("Serial test PASSED");
    } else {
        println!("Serial test FAILED");
    }
}
```

#### disk_bench
```rust
// kernel/userspace/disk_bench/src/main.rs
fn main() {
    println!("Disk benchmark - reading sector 0");
    
    let mut buf = [0u8; 512];
    let start = syscall_get_time();
    
    syscall_block_read(0, &mut buf);
    
    let end = syscall_get_time();
    let elapsed = end - start;
    
    // Check MBR signature
    if buf[510] == 0x55 && buf[511] == 0xAA {
        println!("MBR signature valid");
    } else {
        println!("MBR signature invalid");
    }
    
    println!("Read time: {} us", elapsed);
}
```

#### dmesg
```rust
// kernel/userspace/dmesg/src/main.rs
fn main() {
    // Read kernel log buffer via syscall
    let log = syscall_read_kernel_log();
    println!("{}", log);
}
```

#### lsdev
```rust
// kernel/userspace/lsdev/src/main.rs
fn main() {
    println!("Device Tree:");
    println!("NAME              BUS       IO_BASE    IRQ    DRIVER");
    println!("--------------------------------------------------------");
    
    let devices = syscall_get_device_list();
    for dev in devices {
        println!("{:<16} {:<9} 0x{:08X} {:>3}    {}", 
                 dev.name, dev.bus, dev.io_base, 
                 dev.irq.map_or("N/A".to_string(), |i| i.to_string()),
                 dev.driver.unwrap_or("none"));
    }
}
```

#### diskinfo
```rust
// kernel/userspace/diskinfo/src/main.rs
fn main() {
    let info = syscall_get_block_device_info();
    
    println!("Block Device Information:");
    println!("  Block count: {}", info.block_count);
    println!("  Block size:  {} bytes", info.block_size);
    println!("  Total size:  {} MB", 
             (info.block_count * info.block_size as u64) / (1024 * 1024));
}
```

### SMP Testing

Test interrupt handling under SMP conditions:

1. **IRQ Distribution**: Verify interrupts are handled on correct CPU
2. **Concurrent Access**: Multiple CPUs accessing drivers simultaneously
3. **Context Switch Safety**: Interrupts during task switches

Test script:
```bash
#!/bin/bash
# tools/testing/test_drivers_smp.sh

echo "Testing driver subsystem with SMP..."

# Boot with 4 CPUs
make run CPUS=4 &
QEMU_PID=$!

sleep 5

# Run tests
expect << EOF
spawn telnet localhost 1234
expect "mello-sh>"
send "lsdev\r"
expect "mello-sh>"
send "kbd_test &\r"
expect "mello-sh>"
send "irq_test\r"
expect "mello-sh>"
send "exit\r"
EOF

kill $QEMU_PID
```

### Performance Testing

Measure driver performance:

1. **Keyboard Latency**: Time from key press to character available
2. **Serial Throughput**: Bytes per second through COM1
3. **Disk Throughput**: Blocks per second read/write

Expected targets:
- Keyboard latency: < 10ms
- Serial throughput: > 38400 baud (4800 bytes/sec)
- Disk throughput: > 10 MB/sec (virtio-blk)


## Integration with Existing Systems

### Integration with SMP (Phase 5)

The driver subsystem must work correctly with the existing SMP infrastructure:

1. **Per-CPU IRQ Handling**: Use existing LAPIC/IOAPIC code for interrupt routing
2. **Lock Ordering**: Follow existing lock ordering rules to prevent deadlocks
3. **CPU Affinity**: Support pinning IRQs to specific CPUs

```rust
// Use existing SMP primitives
use crate::arch::x86_64::apic::{ioapic_set_irq, lapic_eoi};
use crate::arch::x86_64::smp::cpu_id;

fn handle_irq(irq: u8) {
    let cpu = cpu_id();
    log::trace!("IRQ {} on CPU {}", irq, cpu);
    
    // Call registered handler
    dispatch_irq_handler(irq);
    
    // Send EOI to LAPIC
    lapic_eoi();
}
```

### Integration with Scheduler (Phase 3)

Drivers may need to interact with the scheduler:

1. **Blocking I/O**: Tasks waiting for I/O should sleep
2. **Wakeup**: IRQ handlers should wake waiting tasks
3. **Priority**: I/O-bound tasks may get priority boost

```rust
// Block current task waiting for keyboard input
pub fn keyboard_read_blocking() -> u8 {
    loop {
        if let Some(ch) = keyboard_read() {
            return ch;
        }
        // Sleep until keyboard IRQ
        crate::sched::sleep_until_irq(1);
    }
}
```

### Integration with Syscalls (Phase 4)

New syscalls for device access:

```rust
// kernel/src/sys/syscall.rs

pub fn sys_read_stdin() -> Option<u8> {
    crate::drivers::input::keyboard::keyboard_read()
}

pub fn sys_serial_write(byte: u8) {
    crate::drivers::serial::serial_write(byte);
}

pub fn sys_serial_read() -> Option<u8> {
    crate::drivers::serial::serial_read()
}

pub fn sys_block_read(lba: u64, buf: &mut [u8]) -> Result<(), BlockError> {
    crate::drivers::block::block_read(lba, buf)
}

pub fn sys_block_write(lba: u64, buf: &[u8]) -> Result<(), BlockError> {
    crate::drivers::block::block_write(lba, buf)
}

pub fn sys_get_device_list() -> Vec<DeviceInfo> {
    crate::drivers::get_device_list()
}
```

### Integration with Memory Management (Phase 2)

Drivers need memory for:

1. **DMA Buffers**: Physically contiguous memory for device DMA
2. **MMIO Mapping**: Map device registers into kernel address space
3. **Driver State**: Allocate structures for driver state

```rust
// Allocate DMA buffer
let dma_buf = crate::mm::alloc_dma_buffer(4096)?;

// Map MMIO region
let mmio_addr = crate::mm::map_mmio(phys_addr, size)?;
```

### Integration with Userland (Phase 6)

Userland programs access drivers through syscalls:

```rust
// Userland wrapper for keyboard input
pub fn read_stdin() -> Option<u8> {
    unsafe {
        syscall0(SYS_READ_STDIN) as u8
    }
}

// Userland wrapper for block I/O
pub fn read_block(lba: u64, buf: &mut [u8]) -> Result<(), ()> {
    unsafe {
        let ret = syscall3(SYS_BLOCK_READ, lba, buf.as_mut_ptr() as u64, buf.len() as u64);
        if ret == 0 {
            Ok(())
        } else {
            Err(())
        }
    }
}
```

## Boot Sequence Integration

The driver initialization must fit into the existing boot sequence:

```rust
// kernel/src/main.rs

pub fn kernel_main() {
    // Phase 1: Boot & Init
    init_console();
    
    // Phase 2: Memory Management
    init_memory();
    
    // Phase 3: Scheduler
    init_scheduler();
    
    // Phase 4: Syscalls
    init_syscalls();
    
    // Phase 5: SMP
    init_smp();
    
    // Phase 6: Userland
    init_userland();
    
    // Phase 7: Drivers (NEW)
    init_drivers();
    
    // Start init process
    start_init();
}

fn init_drivers() {
    log::info!("=== Phase 7: Initializing Driver Subsystem ===");
    
    // Initialize I/O infrastructure
    crate::io::init();
    
    // Initialize driver manager
    crate::drivers::init_drivers();
    
    log::info!("Driver subsystem initialized");
}
```

## Future Extensibility

This design provides foundation for future phases:

### Phase 8: Filesystem

The BlockDevice trait enables filesystem mounting:

```rust
// Future filesystem code
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

## Design Decisions and Rationales

### 1. Static Driver Registration

**Decision**: Drivers are registered at compile time, not dynamically loaded.

**Rationale**: 
- Simpler implementation for Phase 7
- No need for dynamic linking or module loading
- All drivers are built into kernel
- Future phases can add dynamic loading if needed

### 2. Polling vs. Interrupt-Driven

**Decision**: Use interrupt-driven I/O for keyboard and serial, polling for disk (initially).

**Rationale**:
- Keyboard and serial are low-bandwidth, interrupt-driven is efficient
- Disk I/O will use interrupts in full virtio implementation
- Polling is simpler for initial implementation

### 3. Synchronous Block I/O

**Decision**: Block I/O operations are synchronous (blocking).

**Rationale**:
- Simpler implementation for Phase 7
- Sufficient for Phase 8 filesystem needs
- Async I/O can be added in future if needed

### 4. Single Block Device

**Decision**: Support only one block device initially.

**Rationale**:
- Sufficient for Phase 8 (single root filesystem)
- Multiple device support can be added later
- Simplifies device management

### 5. No DMA Support

**Decision**: No DMA (Direct Memory Access) support in Phase 7.

**Rationale**:
- virtio can work without DMA initially
- DMA adds complexity (IOMMU, physical memory management)
- Can be added in future optimization phase

### 6. Simplified virtio Implementation

**Decision**: Simplified virtio without full virtqueue implementation.

**Rationale**:
- Full virtio is complex (virtqueues, descriptors, etc.)
- Simplified version sufficient for basic disk I/O
- Can be enhanced in future phases

## Security Considerations

### 1. I/O Port Access Control

Only kernel code can access I/O ports. Userland must use syscalls.

### 2. Buffer Validation

All buffers passed from userland must be validated:
- Check buffer is in userland address space
- Check buffer size is reasonable
- Prevent buffer overflows

### 3. IRQ Handler Safety

IRQ handlers must be carefully written:
- No panics in IRQ context
- Minimal work in IRQ handler
- Proper synchronization with SMP

### 4. Device Resource Isolation

Each device has isolated resources:
- Separate I/O port ranges
- Separate IRQ numbers
- Separate memory regions

## Performance Considerations

### 1. Interrupt Overhead

Minimize work in interrupt handlers:
- Copy data to buffer
- Wake waiting tasks
- Defer complex processing

### 2. Lock Contention

Minimize lock contention:
- Use per-device locks where possible
- Keep critical sections short
- Use lock-free structures where appropriate

### 3. Cache Efficiency

Optimize for cache efficiency:
- Keep hot data structures small
- Align structures to cache lines
- Minimize false sharing in SMP

### 4. I/O Batching

Batch I/O operations where possible:
- Read/write multiple blocks at once
- Coalesce small writes
- Use DMA for large transfers (future)

## Conclusion

This design provides a solid foundation for device driver support in MelloOS. The modular architecture allows for easy addition of new drivers, while the clear interfaces enable integration with existing kernel subsystems. The design prioritizes correctness and simplicity for Phase 7, with clear paths for future enhancements in subsequent phases.

