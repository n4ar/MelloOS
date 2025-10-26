# MelloOS Development Roadmap

## Overview
This roadmap tracks the development phases of MelloOS, a custom operating system built from scratch. Each phase builds upon the previous ones, creating a complete OS with modern features.

## Development Phases

### Phase 1: Boot & Init ✅ เสร็จ
**เป้าหมาย:** บูตผ่าน Limine → "Hello from MelloOS"

**สถานะ:** Complete
- Bootloader integration with Limine
- Initial kernel entry point
- Basic console output
- System initialization

---

### Phase 2: Memory Management ✅ เสร็จ
**เป้าหมาย:** PMM, Paging 4-level, kmalloc/slab

**สถานะ:** Complete
- Physical Memory Manager (PMM)
- 4-level page table implementation
- Virtual memory management
- Kernel heap allocator (kmalloc)
- Slab allocator for efficient object allocation

---

### Phase 3: Task & Scheduler ✅ เสร็จ
**เป้าหมาย:** TCB, context switch, RR scheduler, timer

**สถานะ:** Complete
- Task Control Block (TCB) structure
- Context switching mechanism
- Round-Robin (RR) scheduler
- Timer interrupt handling
- Basic multitasking support

---

### Phase 4: Priority + Syscall & IPC ✅ เสร็จ
**เป้าหมาย:** priority, sleep/wake, syscall table, ports IPC

**สถานะ:** Complete
- Priority-based scheduling
- Sleep/wake mechanisms
- System call table and interface
- Inter-Process Communication (IPC) via ports
- Process synchronization primitives

---

### Phase 5: SMP (Multi-core) ✅ เสร็จ
**เป้าหมาย:** AP bring-up, LAPIC, IPI, per-CPU runqueue

**สถานะ:** Complete
- Application Processor (AP) initialization
- Local APIC (LAPIC) configuration
- Inter-Processor Interrupts (IPI)
- Per-CPU run queues
- Multi-core task distribution

---

### Phase 6: Userland Foundation ✅ เสร็จ
**เป้าหมาย:** ring3, syscall interface, ELF loader, PID 1

**สถานะ:** Complete
- Ring 3 (user mode) support
- User-kernel syscall interface
- ELF binary loader
- Init process (PID 1)
- User space execution environment

---

### Phase 6.1–6.6: Advanced Userland ✅ เสร็จ
**เป้าหมาย:** fork/exec/exit/wait, libc/rt เริ่มต้น, shell+tools (echo/ls/ps/cat), error handling

**สถานะ:** Complete
- Process management: fork, exec, exit, wait
- Basic libc and runtime support
- Shell implementation (mello-sh)
- Core utilities: echo, ls, ps, cat (mellobox)
- Terminal emulator (mello-term)
- Error handling and signal support
- PTY (pseudo-terminal) subsystem
- Job control and process groups
- UTF-8 support
- Performance optimizations

---

### Phase 7: Device Drivers & I/O ✅ เสร็จ
**เป้าหมาย:** driver model, keyboard/serial, disk (virtio-blk)

**สถานะ:** Complete
**ความสำคัญ:** Required for persistent storage and advanced I/O

**Completed Components:**
- Generic driver model and framework
- Driver Manager with registration and probing
- PS/2 keyboard driver with scancode translation
- UART16550 serial port driver (COM1)
- virtio-blk block device driver
- Block device abstraction layer (BlockDevice trait)
- Device discovery and enumeration (Platform, PS/2, PCI, virtio buses)
- IRQ management with IOAPIC routing and CPU affinity
- I/O infrastructure (port I/O, MMIO, IRQ handling)
- Device tree for hardware tracking
- System calls for device access
- Userland testing tools:
  - kbd_test (keyboard testing)
  - serial_test (serial port testing)
  - disk_bench (disk performance)
  - dmesg (kernel log display)
  - lsdev (device enumeration)
  - diskinfo (block device info)
  - irq_test (interrupt distribution)
- Integration test suite
- SMP-safe interrupt handling
- Documentation and developer guidelines

**Note:** AHCI and NVMe drivers deferred to future optimization phase. virtio-blk provides sufficient functionality for Phase 8 filesystem support.

---

### Phase 8: Filesystem & Storage ✅ เสร็จ
**เป้าหมาย:** VFS + MFS (RAM + Disk), mount/umount

**สถานะ:** Complete
**ความสำคัญ:** Required for file operations and persistent data

**Planned Components:**
- Virtual File System (VFS) layer
- tmpfs (temporary filesystem in RAM)
- ext2 filesystem support:
  - Read-only initially
  - Read-write implementation
- FAT32 filesystem support:
  - Read-only initially
  - Read-write implementation
- Mount/umount syscalls
- File descriptor management
- Path resolution
- Directory operations

---

### Phase 9: Networking Stack ⏳ อนาคต
**เป้าหมาย:** virtio-net, IPv4, ICMP/UDP/TCP-lite, socket syscalls

**สถานะ:** Future Phase - Not Started
**ความสำคัญ:** Required for network communication

**Planned Components:**
- virtio-net driver (for QEMU/virtualization)
- Network stack architecture
- IPv4 protocol implementation
- ICMP (ping) support
- UDP protocol
- TCP-lite (simplified TCP)
- Socket API and syscalls
- Network buffer management
- ARP protocol

---

### Phase 10: GUI / Desktop Base ⏳ อนาคต
**เป้าหมาย:** framebuffer/compositor, input server, terminal GUI

**สถานะ:** Future Phase - Not Started
**ความสำคัญ:** Required for graphical user interface

**Planned Components:**
- Framebuffer driver
- Compositor for window management
- Input server (mouse and keyboard)
- Graphical terminal emulator
- Basic window system
- Font rendering
- Graphics primitives
- Event handling system

---

## Current Focus

**ปัจจุบัน:** Phase 7 (Device Drivers & I/O) - Complete ✅

**ถัดไป:** Phase 8 (Filesystem & Storage) - Ready to begin

**Prerequisites for Phase 8:**
- All previous phases completed ✅
- Block device driver operational ✅
- BlockDevice trait implemented ✅
- Device syscalls available ✅
- Testing infrastructure in place ✅

---

## Development Guidelines

### When Working on MelloOS:

1. **Always reference this roadmap** to understand current phase and dependencies
2. **Check phase status** before starting new features
3. **Follow sequential order** - don't skip phases unless explicitly discussed
4. **Document achievements** as phases complete
5. **Update roadmap status** when milestones are reached

### Phase Transition Checklist:

Before moving to next phase:
- [ ] All requirements of current phase met
- [ ] Core functionality tested and stable
- [ ] Documentation updated
- [ ] Integration tests passing
- [ ] User approval received

---

## Notes

- **เสร็จ (Complete):** Phase is fully implemented and tested
- **⏳ ถัดไป (Next):** Phase is ready to begin
- **⏳ อนาคต (Future):** Phase planned for later

This roadmap is a living document and will be updated as development progresses.
