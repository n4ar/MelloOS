# Requirements Document - Device Drivers & I/O Subsystem

## Introduction

This document specifies the requirements for Phase 7 of MelloOS development: Device Drivers & I/O Subsystem. This phase transforms MelloOS from a memory-only operating system into one capable of real hardware interaction. The system will implement a driver model, interrupt routing, device abstraction layer, and fundamental drivers for keyboard, serial port, and disk devices. This foundation enables future filesystem support in Phase 8.

## Glossary

- **Driver Manager**: The kernel subsystem responsible for registering, probing, and initializing device drivers
- **Device**: A hardware component that can be controlled through a driver (keyboard, disk, serial port, etc.)
- **Driver**: Software component that provides an interface between the kernel and a specific hardware device
- **IRQ (Interrupt Request)**: Hardware signal that interrupts the CPU to handle time-sensitive events
- **IOAPIC**: I/O Advanced Programmable Interrupt Controller used for routing interrupts in multicore systems
- **Block Device**: Storage device that reads/writes data in fixed-size blocks (sectors)
- **virtio-blk**: Paravirtualized block device driver for QEMU/KVM environments
- **AHCI**: Advanced Host Controller Interface for SATA disk controllers
- **UART16550**: Universal Asynchronous Receiver/Transmitter chip for serial communication
- **PS/2**: Legacy interface standard for keyboards and mice
- **Device Tree**: Registry of all detected hardware devices in the system
- **Bus**: Communication pathway connecting devices (PCI, Platform, virtio, PS/2)
- **LBA (Logical Block Addressing)**: Method of specifying disk sectors by linear address
- **Sector**: Fixed-size unit of storage on a disk (typically 512 or 4096 bytes)
- **MBR (Master Boot Record)**: First sector of a disk containing boot code and partition table
- **COM Port**: Serial communication port (COM1 typically at I/O address 0x3F8)
- **Scancode**: Byte value representing a keyboard key press or release
- **MMIO (Memory-Mapped I/O)**: Technique where device registers are accessed through memory addresses

## Requirements

### Requirement 1: Driver Manager Infrastructure

**User Story:** As a kernel developer, I want a driver manager system so that device drivers can be registered, probed, and initialized in a consistent manner.

#### Acceptance Criteria

1. WHEN the kernel initializes, THE Driver Manager SHALL register all available drivers into a driver registry
2. WHEN a device is detected, THE Driver Manager SHALL probe all registered drivers to find a compatible driver
3. WHEN a compatible driver is found, THE Driver Manager SHALL invoke the driver initialization function with the device information
4. THE Driver Manager SHALL maintain a device registry containing all detected hardware devices
5. THE Driver Manager SHALL provide an API for drivers to register themselves with name, probe function, and initialization function
6. WHEN a driver is unregistered, THE Driver Manager SHALL properly release all allocated resources and unregister associated devices

### Requirement 2: Interrupt Handling System

**User Story:** As a kernel developer, I want a robust interrupt handling system so that device interrupts can be routed correctly across multiple CPU cores without race conditions.

#### Acceptance Criteria

1. WHEN an IRQ is registered, THE Interrupt System SHALL associate the IRQ number with a handler function
2. WHEN a hardware interrupt occurs, THE Interrupt System SHALL invoke the registered handler on the appropriate CPU core
3. WHILE running on SMP systems, THE Interrupt System SHALL synchronize interrupt handling to prevent race conditions
4. THE Interrupt System SHALL support IRQ remapping through IOAPIC for multicore routing
5. WHEN an interrupt is handled, THE Interrupt System SHALL log the IRQ source and handling CPU core ID

### Requirement 3: Keyboard Driver

**User Story:** As a user, I want keyboard input to work so that I can type commands in the shell and interact with the system.

#### Acceptance Criteria

1. WHEN the keyboard driver initializes, THE Keyboard Driver SHALL configure the PS/2 controller at I/O ports 0x60 and 0x64
2. WHEN a key is pressed, THE Keyboard Driver SHALL read the scancode from port 0x60
3. WHEN a scancode is received, THE Keyboard Driver SHALL translate it to ASCII using a scancode lookup table
4. THE Keyboard Driver SHALL register an interrupt handler for IRQ 1 (keyboard interrupt)
5. WHEN keyboard input is available, THE Keyboard Driver SHALL make it accessible to userland through stdin read operations

### Requirement 4: Serial Port Driver

**User Story:** As a kernel developer, I want a serial port driver so that kernel logs and console output can be transmitted for debugging and monitoring.

#### Acceptance Criteria

1. WHEN the serial driver initializes, THE Serial Driver SHALL configure UART16550 at COM1 port 0x3F8
2. THE Serial Driver SHALL provide a write function that transmits bytes through the serial port
3. THE Serial Driver SHALL provide a read function that receives bytes from the serial port
4. THE Serial Driver SHALL support string output through a serial_write_str function
5. THE Serial Driver SHALL provide a serial_println macro for kernel debugging across all subsystems

### Requirement 5: Block Device Driver (virtio-blk)

**User Story:** As a kernel developer, I want a block device driver so that the system can read from and write to disk storage.

#### Acceptance Criteria

1. WHEN the virtio-blk driver initializes, THE Block Driver SHALL detect and configure virtio block devices
2. THE Block Driver SHALL provide a read function that reads sectors from disk using LBA addressing
3. THE Block Driver SHALL provide a write function that writes sectors to disk using LBA addressing
4. THE Block Driver SHALL use virtqueue mechanism to communicate with the virtio device
5. THE Block Driver SHALL implement a BlockDevice trait with read_block, write_block, and block_count methods for Phase 8 filesystem integration

### Requirement 6: Device Abstraction Layer

**User Story:** As a kernel developer, I want a device abstraction layer so that different types of devices can be managed uniformly regardless of their bus type.

#### Acceptance Criteria

1. THE Device Abstraction Layer SHALL define a Device structure containing name, bus type, I/O base address, and IRQ number
2. THE Device Abstraction Layer SHALL support multiple bus types including PS/2, PCI, virtio, and Platform
3. WHEN the system boots, THE Device Abstraction Layer SHALL scan all buses in deterministic order: Platform, PS/2, PCI, virtio
4. THE Device Abstraction Layer SHALL provide a device_register function for adding devices to the device tree
5. THE Device Abstraction Layer SHALL provide a device tree query interface for listing all registered devices

### Requirement 7: I/O Port and MMIO Utilities

**User Story:** As a driver developer, I want I/O port and MMIO utility functions so that I can safely access hardware registers.

#### Acceptance Criteria

1. THE I/O Utilities SHALL provide inb, outb, inw, outw, inl, outl functions for port I/O operations
2. THE I/O Utilities SHALL provide read and write functions for memory-mapped I/O (MMIO) operations
3. THE I/O Utilities SHALL ensure all I/O operations are properly synchronized in SMP environments
4. THE I/O Utilities SHALL validate I/O addresses before access to prevent invalid operations
5. THE I/O Utilities SHALL be accessible to all driver modules through a common API

### Requirement 8: Userland Testing Tools

**User Story:** As a system tester, I want userland testing programs so that I can verify driver functionality without kernel modifications.

#### Acceptance Criteria

1. THE System SHALL provide a kbd_test program that echoes keyboard input back to the console
2. THE System SHALL provide a serial_test program that performs loopback testing on serial ports
3. THE System SHALL provide a disk_bench program that reads sector 0 and displays the disk signature
4. THE System SHALL provide a dmesg command that displays kernel driver logs
5. THE System SHALL provide an lsdev command that lists all detected devices with their properties
6. THE System SHALL provide a diskinfo command that displays block device information including size and sector count
7. THE System SHALL provide an irq_test program that triggers and reports interrupt events per CPU for validation of IOAPIC routing

### Requirement 9: Driver Stability and Error Handling

**User Story:** As a system administrator, I want drivers to handle errors gracefully so that a single device failure does not crash the entire system.

#### Acceptance Criteria

1. WHEN a driver probe fails, THE Driver Manager SHALL log the failure and continue probing other drivers
2. WHEN a driver initialization fails, THE Driver Manager SHALL mark the device as unavailable and continue system boot
3. WHEN an I/O operation fails, THE Driver SHALL return an error code without causing a kernel panic
4. WHEN an interrupt storm occurs, THE Interrupt System SHALL detect and throttle excessive interrupts
5. THE Driver System SHALL support driver reload and reinitialization without system reboot
6. THE Driver System SHALL log driver lifecycle events including register, probe, init, fail, and unload via kernel log subsystem

### Requirement 10: SMP Interrupt Safety

**User Story:** As a kernel developer, I want interrupt handling to be SMP-safe so that interrupts work correctly on multicore systems without race conditions.

#### Acceptance Criteria

1. WHEN an interrupt occurs during a context switch, THE Interrupt System SHALL handle it without corrupting task state
2. WHILE multiple cores handle interrupts simultaneously, THE Interrupt System SHALL prevent data races through proper locking
3. THE Interrupt System SHALL support interrupt affinity to route specific IRQs to specific CPU cores
4. WHEN an interrupt handler accesses shared data, THE Interrupt System SHALL use appropriate synchronization primitives
5. THE Interrupt System SHALL log interrupt distribution across cores for debugging and performance analysis

## Success Criteria

The Device Drivers & I/O Subsystem phase is complete when:

1. Kernel boots with driver model initialized
2. Keyboard input reaches the shell successfully (via stdin interface ready for Phase 8 /dev/kbd integration)
3. Serial port operates correctly for stdout and logging
4. Disk driver can read sectors from storage
5. IRQ system remains stable during SMP operation
6. Device tree displays all detected devices via lsdev command
7. All userland testing tools execute successfully
8. System can handle device errors without crashing
9. Block device interface is ready for Phase 8 filesystem integration
10. BlockDevice trait exposes read_block, write_block, and block_count methods for filesystem mount tests in Phase 8

## Dependencies

- Phase 1-6 must be complete (Boot, Memory, Scheduler, Syscalls, SMP, Userland)
- IOAPIC and LAPIC must be functional for interrupt routing
- SMP infrastructure must be stable for multicore interrupt handling
- Userland execution environment must support test programs

## Out of Scope

- USB device support (future phase)
- Network device drivers (Phase 9)
- Graphics drivers beyond basic framebuffer (Phase 10)
- Hot-plug device support
- Power management for devices
- DMA (Direct Memory Access) support
- Advanced AHCI features (NCQ, hot-swap)
