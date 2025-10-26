# Implementation Plan - Device Drivers & I/O Subsystem

This implementation plan breaks down Phase 7 into incremental, actionable tasks. Each task builds upon previous ones, ensuring the driver subsystem is developed systematically from foundation to full functionality.

## Task List

- [x] 1. Create I/O infrastructure foundation
  - Create `kernel/src/io/mod.rs` module with submodule declarations
  - Implement `kernel/src/io/port.rs` with inb/outb/inw/outw/inl/outl functions and mock support for testing
  - Implement `kernel/src/io/mmio.rs` with mmio_read/mmio_write functions for memory-mapped I/O
  - Add unit tests for I/O port operations using mock interface
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [x] 2. Implement IRQ management system
  - Create `kernel/src/io/irq.rs` with IRQ handler registration and dispatch
  - Implement `init_ioapic_routing()` function to initialize IOAPIC before driver registration
  - Implement `register_irq_handler()` and `register_irq_handler_affinity()` for IRQ registration with CPU affinity support
  - Implement `handle_irq()` function to dispatch interrupts to registered handlers
  - Add IRQ logging with CPU core ID tracking
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 10.1, 10.2, 10.3, 10.4, 10.5_

- [x] 3. Create device tree and bus scanning infrastructure
  - Create `kernel/src/io/devtree.rs` for device discovery and registration
  - Implement `scan_platform_bus()` for built-in devices
  - Implement `scan_ps2_bus()` to detect PS/2 keyboard controller
  - Implement `scan_pci_bus()` placeholder for future PCI enumeration
  - Implement `scan_virtio_bus()` to detect virtio devices
  - Add `ps2_controller_present()` helper function
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 4. Implement driver manager core
  - Create `kernel/src/drivers/mod.rs` with Driver and Device structures
  - Define `Driver` struct with name, probe, init, and shutdown callbacks
  - Define `Device` struct with name, bus, io_base, irq, irq_affinity, driver, and state fields
  - Define `BusType` enum (Platform, PS2, PCI, Virtio)
  - Define `DeviceState` enum (Detected, Initializing, Active, Failed, Shutdown)
  - Define `DriverError` enum for error handling
  - Implement driver and device registries using Mutex-protected vectors
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 6.1, 6.2_

- [x] 5. Implement driver probing and initialization
  - Implement `driver_register()` function to add drivers to registry
  - Implement `device_register()` function to add devices to device tree
  - Implement `driver_probe_all()` to match drivers with devices
  - Implement `register_builtin_drivers()` to register keyboard, serial, and virtio-blk drivers
  - Implement `init_drivers()` boot sequence function
  - Add driver lifecycle logging (register, probe, init, fail, unload)
  - _Requirements: 1.1, 1.2, 1.3, 1.6, 9.1, 9.2, 9.6_

- [x] 6. Implement PS/2 keyboard driver
  - Create `kernel/src/drivers/input/mod.rs` and `keyboard.rs`
  - Define scancode to ASCII translation table for US keyboard layout
  - Implement keyboard buffer using circular buffer (256 bytes)
  - Implement `keyboard_probe()` to match ps2-keyboard devices
  - Implement `keyboard_init()` to configure PS/2 controller and register IRQ handler
  - Implement `keyboard_irq_handler()` to read scancodes and translate to ASCII
  - Implement `keyboard_read()` non-blocking function to read from buffer
  - Implement `keyboard_shutdown()` to unregister IRQ handler
  - Define `KEYBOARD_DRIVER` constant for registration
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 7. Implement UART16550 serial driver
  - Create `kernel/src/drivers/serial/mod.rs` and `uart16550.rs`
  - Define `SerialPort` struct with base port address
  - Implement `SerialPort::init()` to configure UART (38400 baud, 8N1)
  - Implement `SerialPort::write_byte()` to transmit bytes
  - Implement `SerialPort::read_byte()` to receive bytes (non-blocking)
  - Implement `serial_probe()`, `serial_init()`, and `serial_shutdown()` functions
  - Implement `serial_write()`, `serial_write_str()`, and `serial_read()` public API
  - Implement `serial_println!` macro for kernel debugging
  - Define `SERIAL_DRIVER` constant for registration
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [x] 8. Implement virtio-blk block device driver
  - Create `kernel/src/drivers/block/mod.rs` and `virtio_blk.rs`
  - Define `BlockDevice` trait with read_block, write_block, block_count, and block_size methods
  - Define `BlockError` enum for block I/O errors
  - Define `VirtioBlkDevice` struct with base_addr, capacity, and block_size fields
  - Implement `VirtioBlkDevice::new()` to read capacity from virtio config space
  - Implement `VirtioBlkDevice::init()` to initialize virtio device (reset, status bits, feature negotiation)
  - Implement `BlockDevice` trait for `VirtioBlkDevice` with read_block and write_block operations
  - Implement `virtio_blk_probe()`, `virtio_blk_init()`, and `virtio_blk_shutdown()` functions
  - Implement `block_read()` and `block_write()` public API functions
  - Define `VIRTIO_BLK_DRIVER` constant for registration
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 9. Add syscalls for device access
  - Add `sys_read_stdin()` syscall to read keyboard input
  - Add `sys_serial_write()` and `sys_serial_read()` syscalls for serial I/O
  - Add `sys_block_read()` and `sys_block_write()` syscalls for disk I/O
  - Add `sys_get_device_list()` syscall to query device tree
  - Add `sys_get_block_device_info()` syscall for disk information
  - Update syscall table in `kernel/src/sys/syscall.rs`
  - Add userland wrappers in shared syscall library
  - _Requirements: 3.5, 4.2, 4.3, 5.2, 5.3, 6.5, 8.4, 8.5, 8.6_

- [x] 10. Integrate driver subsystem into kernel boot sequence
  - Add `init_drivers()` call in `kernel/src/main.rs` after SMP initialization
  - Ensure IOAPIC is initialized before driver registration
  - Call `register_builtin_drivers()` to register all drivers
  - Call bus scanning functions in deterministic order (Platform, PS/2, PCI, virtio)
  - Call `driver_probe_all()` to match and initialize drivers
  - Add driver subsystem logging to kernel boot messages
  - _Requirements: 1.1, 1.2, 1.3, 6.3, 9.2_

- [x] 11. Create userland testing tool: kbd_test
  - Create `kernel/userspace/kbd_test/` directory with Cargo.toml and linker.ld
  - Implement main.rs that reads from stdin and echoes characters back
  - Use `sys_read_stdin()` syscall to get keyboard input
  - Use `sys_write()` syscall to output characters
  - Add Ctrl+C detection to exit program
  - Build and install to iso_root/bin/
  - _Requirements: 8.1_

- [x] 12. Create userland testing tool: serial_test
  - Create `kernel/userspace/serial_test/` directory with Cargo.toml and linker.ld
  - Implement main.rs that performs serial loopback test
  - Use `sys_serial_write()` to send test data
  - Use `sys_serial_read()` to receive data back
  - Verify data integrity and report results
  - Build and install to iso_root/bin/
  - _Requirements: 8.2_

- [x] 13. Create userland testing tool: disk_bench
  - Create `kernel/userspace/disk_bench/` directory with Cargo.toml and linker.ld
  - Implement main.rs that reads sector 0 (MBR) from disk
  - Use `sys_block_read()` syscall to read 512-byte sector
  - Verify MBR signature (0x55AA at offset 510)
  - Measure and report read time
  - Build and install to iso_root/bin/
  - _Requirements: 8.3_

- [x] 14. Create userland testing tool: dmesg
  - Create `kernel/userspace/dmesg/` directory with Cargo.toml and linker.ld
  - Implement main.rs that displays kernel log buffer
  - Use `sys_read_kernel_log()` syscall to retrieve log entries
  - Format and display driver lifecycle events
  - Build and install to iso_root/bin/
  - _Requirements: 8.4_

- [x] 15. Create userland testing tool: lsdev
  - Create `kernel/userspace/lsdev/` directory with Cargo.toml and linker.ld
  - Implement main.rs that lists all detected devices
  - Use `sys_get_device_list()` syscall to query device tree
  - Display device name, bus type, I/O base, IRQ, and driver in table format
  - Build and install to iso_root/bin/
  - _Requirements: 8.5_

- [x] 16. Create userland testing tool: diskinfo
  - Create `kernel/userspace/diskinfo/` directory with Cargo.toml and linker.ld
  - Implement main.rs that displays block device information
  - Use `sys_get_block_device_info()` syscall to get disk details
  - Display block count, block size, and total capacity in MB
  - Build and install to iso_root/bin/
  - _Requirements: 8.6_

- [x] 17. Create userland testing tool: irq_test
  - Create `kernel/userspace/irq_test/` directory with Cargo.toml and linker.ld
  - Implement main.rs that triggers and monitors interrupt events
  - Use syscalls to query IRQ statistics per CPU
  - Display interrupt distribution across cores
  - Verify IOAPIC routing is working correctly
  - Build and install to iso_root/bin/
  - _Requirements: 8.7_

- [x] 18. Create integration test script for driver subsystem
  - Create `tools/testing/test_drivers.sh` script
  - Test keyboard driver by running kbd_test and sending input
  - Test serial driver by running serial_test
  - Test disk driver by running disk_bench and verifying MBR
  - Test device enumeration by running lsdev
  - Test IRQ distribution by running irq_test on SMP system
  - Verify all tests pass and log results
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 10.1, 10.2, 10.3, 10.4, 10.5_

- [x] 19. Update build system and documentation
  - Update Makefile to build all userland testing tools
  - Update kernel Cargo.toml with new driver modules
  - Create `docs/architecture/device-drivers.md` documenting driver architecture
  - Update `docs/DEVELOPER_GUIDE.md` with driver development guidelines
  - Update `docs/USER_GUIDE.md` with new testing tools usage
  - Update `.kiro/steering/roadmap.md` to mark Phase 7 as complete
  - _Requirements: All_

