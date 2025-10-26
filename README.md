# MelloOS

A modern x86_64 operating system kernel written in Rust from scratch, featuring true multi-core support (SMP), preemptive multitasking, priority-based scheduling, comprehensive system calls, inter-process communication, user-mode process execution, device drivers, and a complete userland environment with an interactive shell, terminal emulator, and POSIX-like utilities.

## ✨ Highlights

- 🚀 **Multi-Core**: Up to 16 CPU cores with automatic load balancing
- 🐚 **Interactive Shell**: Full-featured POSIX-like shell with job control
- 📺 **Terminal Emulator**: VT/ANSI-compatible with UTF-8 support
- 🛠️ **14 Utilities**: BusyBox-style multi-call binary (ls, cat, grep, ps, etc.)
- 🔌 **Device Drivers**: Keyboard, serial, and virtio-blk block device support
- 🔒 **Memory Protection**: User/kernel isolation with NX bit support
- ⚡ **Fast Syscalls**: Modern syscall/sysret mechanism (20+ syscalls)
- 📡 **Signals**: 31 POSIX signals with job control
- 🖥️ **PTY Subsystem**: Complete pseudo-terminal implementation
- 📊 **/proc Filesystem**: Virtual filesystem for system information
- 🌍 **UTF-8 Support**: International text throughout userland
- 🧪 **Comprehensive Testing**: 15+ test scripts with performance benchmarks

## 🌟 Features

### Phase 7: Device Drivers & I/O ✅ COMPLETE

**Complete device driver infrastructure with keyboard, serial, and block device support:**

- **Driver Framework**: Generic driver model with registration and probing
  - Driver Manager for centralized driver management
  - Device discovery and enumeration (Platform, PS/2, PCI, virtio buses)
  - Device tree for hardware tracking
  - Hot-plug support (future)

- **Input Drivers**:
  - PS/2 keyboard driver with scancode translation
  - Full keyboard layout support (US QWERTY)
  - Special key handling (Ctrl, Alt, Shift, Caps Lock)
  - Interrupt-driven input processing

- **Serial Drivers**:
  - UART16550 serial port driver (COM1)
  - Configurable baud rate and line settings
  - Interrupt-driven I/O
  - Kernel logging and debugging support

- **Block Drivers**:
  - virtio-blk driver for QEMU/virtualization
  - Block device abstraction layer (BlockDevice trait)
  - Sector-based I/O operations
  - Device information queries (capacity, block size)

- **I/O Infrastructure**:
  - Port I/O (inb/outb) for legacy devices
  - MMIO (Memory-Mapped I/O) support
  - IRQ management with IOAPIC routing
  - CPU affinity for interrupt handling
  - SMP-safe interrupt distribution

- **System Calls for Devices**:
  - `SYS_READ_STDIN` (25) - Read from keyboard
  - `SYS_SERIAL_WRITE` (26) - Write to serial port
  - `SYS_SERIAL_READ` (27) - Read from serial port
  - `SYS_BLOCK_READ` (28) - Read disk blocks
  - `SYS_BLOCK_WRITE` (29) - Write disk blocks
  - `SYS_GET_DEVICE_LIST` (30) - Enumerate devices
  - `SYS_GET_BLOCK_DEVICE_INFO` (31) - Query block device info

- **Userland Testing Tools**:
  - `kbd_test` - Keyboard input testing
  - `serial_test` - Serial port communication
  - `disk_bench` - Disk performance benchmarking
  - `dmesg` - Kernel log display
  - `lsdev` - Device enumeration
  - `diskinfo` - Block device information
  - `irq_test` - Interrupt distribution testing

**Complete userland environment with interactive shell, terminal emulator, and utilities:**

- **mello-sh**: Full-featured POSIX-like shell with:
  - Job control (background jobs with `&`, fg/bg commands)
  - Pipeline support (`cmd1 | cmd2 | cmd3`)
  - I/O redirection (`<`, `>`, `>>`)
  - Built-in commands (cd, exit, jobs, fg, bg, export, unset)
  - Command history and line editing
  - Environment variables
  - UTF-8 support for international text

- **mello-term**: VT/ANSI-compatible terminal emulator with:
  - PTY (pseudo-terminal) integration
  - ANSI escape sequence parsing
  - Screen buffer management with scrollback
  - UTF-8 text rendering
  - Clipboard support (copy/paste)
  - Window resize handling (SIGWINCH)

- **mellobox**: Multi-call binary (BusyBox-style) with 14 utilities:
  - File operations: ls, cat, cp, mv, rm, mkdir, touch
  - Text processing: grep, echo
  - Process management: ps, kill
  - System utilities: pwd, true, false

- **PTY Subsystem**: Complete pseudo-terminal implementation:
  - Master/slave PTY pairs with ring buffers
  - Termios support (canonical/raw mode, echo, signals)
  - Job control integration (SIGTTIN, SIGTTOU)
  - Window size management (TIOCGWINSZ, TIOCSWINSZ)
  - Signal generation (Ctrl-C → SIGINT, Ctrl-Z → SIGTSTP)

- **Signal Infrastructure**: POSIX-like signal handling:
  - 31 standard signals (SIGINT, SIGTERM, SIGKILL, SIGCHLD, etc.)
  - Signal handlers (default, ignore, custom)
  - Signal masks and blocking
  - Job control signals (SIGTSTP, SIGCONT, SIGTTIN, SIGTTOU)
  - Security checks for signal delivery

- **/proc Filesystem**: Virtual filesystem for system information:
  - Per-process info: /proc/[pid]/stat, /proc/[pid]/status, /proc/[pid]/cmdline
  - System info: /proc/meminfo, /proc/cpuinfo, /proc/uptime, /proc/stat
  - Debug info: /proc/debug/pty, /proc/debug/sessions, /proc/debug/locks

- **Process Groups & Sessions**: Complete job control support:
  - Process groups for pipeline management
  - Sessions with controlling terminals
  - Foreground/background process group management
  - Orphaned process group handling

### Phase 6: User-Mode Support ✅ COMPLETE

- **Ring 3 Execution**: User processes run in ring 3 with privilege level isolation
- **GDT/TSS Configuration**: Per-CPU Global Descriptor Tables and Task State Segments
- **Fast Syscalls**: syscall/sysret mechanism for efficient kernel transitions (< 100ns)
- **Process Management**: Process Control Blocks (PCB) with fine-grained locking
- **Memory Protection**: User/kernel address space separation (< 512GB user space)
- **ELF Loader**: Load and execute ELF binaries in user space
- **Process Lifecycle**: Fork, exec, exit, wait syscalls
- **User Stack**: 8KB user stacks with guard pages
- **Security**: User pointer validation, capability checks, memory isolation

### Phase 5: SMP Multi-Core Support ✅ COMPLETE

- **Symmetric Multi-Processing**: Support for up to 16 CPU cores with automatic detection
- **ACPI MADT Integration**: CPU discovery via ACPI Multiple APIC Description Table
- **AP Bootstrap**: INIT/SIPI sequence to bring Application Processors online (~500ms per AP)
- **Per-Core Scheduling**: Independent runqueues with automatic load balancing
- **Inter-Processor Interrupts**: Cross-core communication and coordination (RESCHEDULE_IPI)
- **SMP-Safe Synchronization**: SpinLocks with proper lock ordering to prevent deadlocks
- **Per-CPU Data Structures**: GS.BASE-based per-core data to minimize contention
- **Cross-Core IPC**: Message passing between tasks on different CPU cores
- **APIC Timer**: Per-core Local APIC timers for preemptive multitasking at 20 Hz
- **Load Balancing**: Periodic rebalancing every 100ms (2 ticks at 20Hz)

### Phase 4: Advanced Scheduling, System Calls, and IPC ✅ COMPLETE

- **Priority-Based Scheduler**: Three-level priority system (High, Normal, Low) with O(1) task selection
- **System Call Interface**: Legacy `int 0x80` and modern syscall/sysret mechanisms
- **Extended Syscalls**: 20+ syscalls including fork, exec, wait, getpid, yield, pipe, dup2, ioctl
- **Inter-Process Communication**: Port-based message passing with 256 ports and 16-message queues
- **Sleep/Wake Mechanism**: Timer-based task suspension with automatic wake-up
- **Userland Init Process**: First userland process demonstrating syscall and IPC usage
- **Kernel Metrics**: Atomic counters tracking context switches, syscalls, and IPC operations
- **Preemption Control**: Critical section support with preempt_disable/enable

### Phase 3: Task Scheduler ✅ COMPLETE

- **Preemptive Multitasking**: Multiple tasks run concurrently with automatic time-sharing
- **Round-Robin Scheduling**: Fair CPU time distribution within same priority level
- **Context Switching**: Assembly-optimized register save/restore (< 1μs per switch)
- **Timer Interrupts**: APIC-based periodic interrupts at 20 Hz (50ms time slices)
- **Task Management**: Task Control Blocks (TCB) with unique IDs, states, and priorities
- **Per-Task Stacks**: Isolated 8KB stacks for each task

### Phase 2: Memory Management ✅ COMPLETE

- **Physical Memory Manager (PMM)**: Bitmap-based frame allocator for 4KB pages
- **Paging System**: 4-level page tables with per-section permissions (RX, R, RW+NX)
- **Kernel Heap Allocator**: Buddy System algorithm (64B to 1MB blocks)
- **Security Features**: NX bit support, write protection, memory zeroing, guard pages
- **Memory Statistics**: Total/free memory tracking in MB
- **TLB Management**: Efficient TLB invalidation for page table updates

### Phase 1: Basic Kernel ✅ COMPLETE

- **UEFI Boot**: Limine bootloader integration (BIOS and UEFI support)
- **Framebuffer Driver**: Pixel-level graphics with 8x8 bitmap font
- **Serial Console**: COM1 output for debugging and logging
- **Panic Handler**: Comprehensive error handling with stack traces

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                          MelloOS Kernel                             │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              SMP Multi-Core Support (arch/x86_64/smp/)      │   │
│  │  - ACPI MADT parser for CPU discovery                       │   │
│  │  - AP bootstrap (16-bit → 32-bit → 64-bit)                  │   │
│  │  - Per-CPU data structures (GS.BASE)                        │   │
│  │  - Local APIC driver and timer                              │   │
│  │  - Inter-Processor Interrupts (IPI)                         │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │         User-Mode Support (arch/x86_64/gdt, user/)          │   │
│  │  - GDT/TSS per-CPU configuration                            │   │
│  │  - Ring 0 ↔ Ring 3 transitions (IRET, syscall/sysret)      │   │
│  │  - Process Control Blocks (PCB)                             │   │
│  │  - ELF binary loader                                        │   │
│  │  - User/kernel memory separation                            │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │           System Call Interface (sys/syscall.rs)            │   │
│  │  - Fast syscall/sysret mechanism (MSR configuration)        │   │
│  │  - 20+ syscalls: read, write, open, close, fork, exec,     │   │
│  │    wait, kill, pipe, dup2, ioctl, getcwd, chdir, etc.      │   │
│  │  - User pointer validation                                  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              Signal Infrastructure (signal/)                │   │
│  │  - 31 POSIX signals (SIGINT, SIGTERM, SIGKILL, etc.)       │   │
│  │  - Signal handlers (default, ignore, custom)               │   │
│  │  - Signal masks and blocking                                │   │
│  │  - Job control signals (SIGTSTP, SIGCONT, SIGTTIN, SIGTTOU)│   │
│  │  - Security checks for signal delivery                      │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              PTY Subsystem (dev/pty/)                       │   │
│  │  - Master/slave PTY pairs (up to 256)                       │   │
│  │  - Ring buffers for efficient I/O (4KB per direction)       │   │
│  │  - Termios support (canonical/raw, echo, signals)           │   │
│  │  - Window size management (TIOCGWINSZ, TIOCSWINSZ)          │   │
│  │  - Job control integration (foreground/background)          │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              /proc Filesystem (fs/proc/)                    │   │
│  │  - Per-process: /proc/[pid]/stat, status, cmdline          │   │
│  │  - System-wide: /proc/meminfo, cpuinfo, uptime, stat       │   │
│  │  - Debug info: /proc/debug/pty, sessions, locks            │   │
│  │  - Lock-free reads with atomic operations                   │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │         Process Groups & Sessions (sched/process_group.rs)  │   │
│  │  - Process groups for pipeline management                   │   │
│  │  - Sessions with controlling terminals                      │   │
│  │  - Foreground/background process groups                     │   │
│  │  - Orphaned process group handling                          │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              Task Scheduler (sched/)                        │   │
│  │  - Per-CPU runqueues with load balancing                    │   │
│  │  - Priority-based scheduling (High/Normal/Low)              │   │
│  │  - Context switching (< 1μs)                                │   │
│  │  - APIC timer interrupts (20 Hz per core)                   │   │
│  │  - Sleep/wake mechanism                                     │   │
│  │  - Process-Task integration                                 │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │           Device Drivers (drivers/)                         │   │
│  │  - Driver Manager with registration and probing             │   │
│  │  - PS/2 keyboard driver (scancode translation)              │   │
│  │  - UART16550 serial driver (COM1)                           │   │
│  │  - virtio-blk block device driver                           │   │
│  │  - Device tree for hardware tracking                        │   │
│  │  - IRQ management with IOAPIC routing                       │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │           I/O Infrastructure (io/)                          │   │
│  │  - Port I/O (inb/outb) for legacy devices                   │   │
│  │  - MMIO (Memory-Mapped I/O) support                         │   │
│  │  - IRQ handling with CPU affinity                           │   │
│  │  - Device tree management                                   │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │           Memory Management (mm/)                           │   │
│  │  ┌──────────┐ ┌──────────┐ ┌────────────────────────────┐  │   │
│  │  │   PMM    │ │  Paging  │ │    Heap Allocator          │  │   │
│  │  │ (Bitmap) │ │(4-level) │ │   (Buddy System)           │  │   │
│  │  └──────────┘ └──────────┘ └────────────────────────────┘  │   │
│  │  - NX bit support, write protection                         │   │
│  │  - Per-section permissions (RX, R, RW+NX)                   │   │
│  │  - Guard pages for stack/heap overflow protection           │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │           Synchronization (sync/)                           │   │
│  │  - SpinLocks with proper lock ordering                      │   │
│  │  - SeqLocks for lock-free reads                             │   │
│  │  - IRQ-safe variants                                        │   │
│  │  - Lock ordering documentation and enforcement              │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       Userland Environment (Ring 3)                 │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  mello-sh (Shell)                                           │   │
│  │  - Job control (fg/bg, Ctrl-Z)                              │   │
│  │  - Pipelines (cmd1 | cmd2 | cmd3)                           │   │
│  │  - I/O redirection (<, >, >>)                               │   │
│  │  - Built-ins (cd, jobs, export, etc.)                       │   │
│  │  - Command history                                          │   │
│  │  - UTF-8 support                                            │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  mello-term (Terminal Emulator)                             │   │
│  │  - VT/ANSI escape sequences                                 │   │
│  │  - PTY integration                                          │   │
│  │  - Screen buffer with scrollback                            │   │
│  │  - UTF-8 rendering                                          │   │
│  │  - Clipboard support                                        │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  mellobox (Coreutils)                                       │   │
│  │  - File: ls, cat, cp, mv, rm, mkdir, touch                  │   │
│  │  - Text: grep, echo                                         │   │
│  │  - Process: ps, kill                                        │   │
│  │  - System: pwd, true, false                                 │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  Device Testing Tools                                       │   │
│  │  - kbd_test (keyboard input testing)                        │   │
│  │  - serial_test (serial port communication)                  │   │
│  │  - disk_bench (disk performance benchmarking)               │   │
│  │  - dmesg (kernel log display)                               │   │
│  │  - lsdev (device enumeration)                               │   │
│  │  - diskinfo (block device information)                      │   │
│  │  - irq_test (interrupt distribution testing)                │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  init (PID 1)                                               │   │
│  │  - System initialization                                    │   │
│  │  - Process reaping                                          │   │
│  │  - Environment setup                                        │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

## 🚀 Quick Start

### Prerequisites

- Rust toolchain (nightly)
- QEMU (for testing)
- xorriso (for ISO creation)
- make

### Installation

```bash
# Install Rust (nightly required)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default nightly

# Add x86_64 bare-metal target
rustup target add x86_64-unknown-none

# Install build tools (Ubuntu/Debian)
sudo apt install qemu-system-x86 xorriso ovmf build-essential

# Install build tools (macOS)
brew install qemu xorriso llvm

# Verify installation
qemu-system-x86_64 --version
xorriso --version
rustc --version
```

### Dependencies

**Rust Crates:**
- `limine` - Bootloader protocol
- `spin` - Spinlock implementation
- `x86_64` - x86-64 architecture support
- `bitflags` - Bit flag manipulation
- `volatile` - Volatile memory access

**Build Tools:**
- GNU Assembler (as) or Clang - For assembly files
- objcopy - For binary extraction
- xorriso - For ISO creation
- QEMU - For testing and development
- make - Build automation

### Building and Running

```bash
# Build everything (kernel + all userspace programs)
make build

# Build userspace programs separately
make userspace

# Create symlinks for mellobox utilities
make symlinks

# Create bootable ISO with all binaries
make iso

# Run in QEMU (default: 4 CPUs with KVM)
make run

# Run with specific CPU count
./tools/qemu/qemu.sh -smp 2 -enable-kvm

# Quick SMP tests
./tools/qemu/qemu-test-smp2.sh    # 2 CPUs
./tools/qemu/qemu-test-smp4.sh    # 4 CPUs

# Automated boot test with SMP
./tools/testing/test_boot.sh -smp 4

# Clean build artifacts
make clean
```

### Expected Output

**On Screen:**
```
Hello from MelloOS ✨
```

**On Serial Console (SMP Boot with 4 CPUs):**
```
[KERNEL] MelloOS starting...
[MM] Initializing memory management...
[MM] ✓ PMM tests passed (allocated/freed 100 frames)
[MM] ✓ Paging tests passed (mapped/unmapped 10 pages)
[MM] ✓ Allocator tests passed (buddy system working)
[ACPI] RSDP found at 0xE0000
[ACPI] MADT found at 0x3FEE2000
[ACPI] Found 4 CPUs: [0, 1, 2, 3]
[SMP] Initializing SMP...
[APIC] BSP LAPIC initialized at 0xFEE00000
[SMP] BSP online (apic_id=0)
[PERCPU] BSP per-CPU data initialized (cpu_id=0, apic_id=0)
[APIC] core0 timer @20Hz
[SMP] Bringing up 3 Application Processors...
[SMP] AP#1 online (apic_id=1)
[SMP] AP#2 online (apic_id=2)
[SMP] AP#3 online (apic_id=3)
[SMP] SMP initialization complete: 4 CPUs online
[PTY] Initialized PTY subsystem with 256 pairs
[PROC] Virtual filesystem initialized
[PROC] Available at /proc
[SCHED] Initializing scheduler...
[KERNEL] ========================================
[KERNEL] MelloOS Ready
[KERNEL] ========================================
[KERNEL] CPUs: 4 cores online
[KERNEL] Memory: 2048 MB total
[KERNEL] Userland: mello-sh, mello-term, mellobox
[KERNEL] Features: SMP, PTY, Signals, /proc, UTF-8
[KERNEL] ========================================

# Interactive shell prompt (if running mello-sh)
mello-sh$ ls /proc
cpuinfo  meminfo  stat  uptime  1/  2/  3/

mello-sh$ ps
PID   PPID  PGID  SID   STATE  CMD
1     0     1     1     R      init
2     1     2     2     R      mello-sh

mello-sh$ cat /proc/cpuinfo
processor       : 0
vendor_id       : GenuineIntel
cpu family      : 6
model           : 15
model name      : Intel Core Processor
cpu MHz         : 2400

processor       : 1
vendor_id       : GenuineIntel
...

mello-sh$ echo "Hello, MelloOS!" | grep Mello
Hello, MelloOS!

mello-sh$ long_command &
[1] 42
mello-sh$ jobs
[1]+ Running    long_command &
```

## 📁 Project Structure

```
mellos/
├── kernel/                 # Kernel source code
│   ├── src/
│   │   ├── main.rs        # Kernel entry point
│   │   ├── config.rs      # Kernel configuration (SCHED_HZ, MAX_CPUS)
│   │   ├── framebuffer.rs # Graphics driver
│   │   ├── serial.rs      # Serial port driver
│   │   ├── panic.rs       # Panic handler
│   │   ├── metrics.rs     # Kernel metrics and statistics
│   │   ├── arch/          # Architecture-specific code
│   │   │   └── x86_64/    # x86-64 implementation
│   │   │       ├── acpi/  # ACPI/MADT parser
│   │   │       ├── apic/  # Local APIC driver and IPI
│   │   │       ├── fault.rs # Page fault handler
│   │   │       ├── gdt.rs # GDT/TSS for user-mode
│   │   │       ├── smp/   # SMP multi-core support
│   │   │       │   ├── mod.rs # AP bootstrap
│   │   │       │   ├── percpu.rs # Per-CPU data
│   │   │       │   └── boot_ap.S # AP trampoline
│   │   │       ├── syscall/ # Fast syscall support
│   │   │       │   ├── mod.rs # MSR configuration
│   │   │       │   └── entry.S # Syscall entry point
│   │   │       └── user_entry.S # User-mode transition
│   │   ├── drivers/       # Device drivers
│   │   │   ├── mod.rs     # Driver manager
│   │   │   ├── input/     # Input device drivers
│   │   │   │   └── keyboard.rs # PS/2 keyboard
│   │   │   ├── serial/    # Serial port drivers
│   │   │   │   └── uart16550.rs # UART16550
│   │   │   └── block/     # Block device drivers
│   │   │       └── virtio_blk.rs # virtio-blk
│   │   ├── io/            # I/O infrastructure
│   │   │   ├── mod.rs     # I/O module
│   │   │   ├── port.rs    # Port I/O (inb/outb)
│   │   │   ├── mmio.rs    # Memory-mapped I/O
│   │   │   ├── irq.rs     # IRQ management
│   │   │   └── devtree.rs # Device tree
│   │   ├── mm/            # Memory management
│   │   │   ├── pmm.rs     # Physical memory manager
│   │   │   ├── paging.rs  # Virtual memory (4-level)
│   │   │   ├── allocator.rs # Heap allocator (buddy)
│   │   │   ├── security.rs # Memory security features
│   │   │   └── tlb.rs     # TLB management
│   │   ├── sched/         # Task scheduler
│   │   │   ├── mod.rs     # Scheduler core (per-CPU)
│   │   │   ├── task.rs    # Task Control Blocks
│   │   │   ├── context.rs # Context switching
│   │   │   ├── priority.rs # Priority levels
│   │   │   ├── timer.rs   # APIC timer interrupts
│   │   │   └── process_group.rs # Process groups & sessions
│   │   ├── sync/          # Synchronization primitives
│   │   │   ├── spin.rs    # SpinLock implementation
│   │   │   ├── seqlock.rs # SeqLock for lock-free reads
│   │   │   └── lock_ordering.rs # Lock hierarchy
│   │   ├── signal/        # Signal infrastructure
│   │   │   ├── mod.rs     # Signal handling
│   │   │   └── security.rs # Signal security checks
│   │   ├── dev/           # Device drivers
│   │   │   └── pty/       # PTY subsystem
│   │   │       └── mod.rs # PTY implementation
│   │   ├── fs/            # Filesystems
│   │   │   ├── mod.rs     # Filesystem module
│   │   │   └── proc/      # /proc virtual filesystem
│   │   │       └── mod.rs # /proc implementation
│   │   ├── sys/           # System calls
│   │   │   ├── syscall.rs # Syscall dispatcher
│   │   │   ├── ioctl.rs   # ioctl implementation
│   │   │   ├── ipc.rs     # IPC (legacy)
│   │   │   └── port.rs    # Port management (legacy)
│   │   └── user/          # User-mode support
│   │       ├── process.rs # Process Control Blocks
│   │       ├── elf.rs     # ELF binary loader
│   │       └── launch.rs  # Process launch
│   ├── userspace/         # Userland programs
│   │   ├── init/          # Init process (PID 1)
│   │   │   ├── src/main.rs
│   │   │   └── Cargo.toml
│   │   ├── mello-sh/      # Shell
│   │   │   ├── src/
│   │   │   │   ├── main.rs
│   │   │   │   ├── parser.rs    # Command parser
│   │   │   │   ├── executor.rs  # Command executor
│   │   │   │   ├── builtins.rs  # Built-in commands
│   │   │   │   ├── jobs.rs      # Job control
│   │   │   │   ├── history.rs   # Command history
│   │   │   │   └── syscalls.rs  # Syscall wrappers
│   │   │   └── Cargo.toml
│   │   ├── mello-term/    # Terminal emulator
│   │   │   ├── src/
│   │   │   │   ├── main.rs
│   │   │   │   ├── pty.rs       # PTY integration
│   │   │   │   ├── screen.rs    # Screen buffer
│   │   │   │   ├── ansi.rs      # ANSI parser
│   │   │   │   ├── input.rs     # Input handling
│   │   │   │   ├── utf8.rs      # UTF-8 support
│   │   │   │   ├── scrollback.rs # Scrollback buffer
│   │   │   │   └── clipboard.rs # Clipboard support
│   │   │   └── Cargo.toml
│   │   ├── mellobox/      # Coreutils (multi-call binary)
│   │   │   ├── src/
│   │   │   │   ├── main.rs
│   │   │   │   ├── args.rs      # Argument parser
│   │   │   │   ├── error.rs     # Error handling
│   │   │   │   ├── syscalls.rs  # Syscall wrappers
│   │   │   │   └── commands/    # Utility implementations
│   │   │   │       ├── ls.rs, cat.rs, cp.rs, mv.rs, rm.rs
│   │   │   │       ├── grep.rs, echo.rs, ps.rs, kill.rs
│   │   │   │       ├── mkdir.rs, touch.rs, pwd.rs
│   │   │   │       └── true_cmd.rs, false_cmd.rs
│   │   │   └── Cargo.toml
│   │   ├── kbd_test/     # Keyboard testing tool
│   │   ├── serial_test/  # Serial port testing tool
│   │   ├── disk_bench/   # Disk benchmarking tool
│   │   ├── dmesg/        # Kernel log display
│   │   ├── lsdev/        # Device enumeration
│   │   ├── diskinfo/     # Block device info
│   │   └── irq_test/     # Interrupt testing
│   ├── Cargo.toml         # Kernel dependencies
│   ├── build.rs           # Build script (compiles assembly)
│   └── linker.ld          # Kernel linker script
├── boot/
│   └── limine.conf        # Bootloader configuration
├── iso_root/              # ISO filesystem structure
│   ├── bin/               # Userspace binaries
│   │   ├── init, mello-sh, mello-term, mellobox
│   │   └── ls, cat, cp, ... (symlinks to mellobox)
│   ├── boot/              # Kernel and bootloader
│   ├── dev/               # Device files (runtime)
│   └── proc/              # /proc filesystem (runtime)
├── tools/                 # Development tools
│   ├── qemu/              # QEMU virtualization scripts
│   ├── debug/             # Debugging tools
│   └── testing/           # Testing and verification
│       ├── test_boot.sh
│       ├── test_utf8_handling.sh
│       ├── test_job_control.sh
│       ├── test_pipeline.sh
│       ├── test_pty_integration.sh
│       └── benchmark_mellos.sh
├── docs/                  # Documentation
│   ├── architecture/      # System architecture docs
│   │   ├── architecture.md
│   │   ├── smp.md
│   │   ├── pty-subsystem.md
│   │   ├── signals-job-control.md
│   │   ├── proc-filesystem.md
│   │   └── performance-optimizations.md
│   ├── development/       # Development guides
│   │   ├── api-guide.md
│   │   └── testing.md
│   ├── troubleshooting/   # Debugging and issues
│   ├── USER_GUIDE.md      # User guide for shell and utilities
│   ├── DEVELOPER_GUIDE.md # Developer guide
│   ├── TROUBLESHOOTING_GUIDE.md # Comprehensive troubleshooting
│   ├── UTF8_SUPPORT.md    # UTF-8 implementation details
│   └── BUILD_SYSTEM_INTEGRATION.md # Build system docs
├── Makefile               # Build system
├── CHANGELOG.md           # Version history
└── README.md              # This file
```

## 💻 System Calls

MelloOS provides 20+ system calls accessible via the modern `syscall` instruction:

### Core System Calls

| ID | Name | Arguments | Description |
|----|------|-----------|-------------|
| 0 | SYS_READ | (fd, buf, len) | Read from file descriptor |
| 1 | SYS_WRITE | (fd, buf, len) | Write to file descriptor |
| 2 | SYS_OPEN | (path, flags, mode) | Open file |
| 3 | SYS_CLOSE | (fd) | Close file descriptor |
| 60 | SYS_EXIT | (code) | Terminate current process |
| 57 | SYS_FORK | () | Create child process |
| 59 | SYS_EXECVE | (path, argv, envp) | Execute new program |
| 61 | SYS_WAIT4 | (pid, status, options) | Wait for child process |
| 39 | SYS_GETPID | () | Get current process ID |

### Device System Calls

| ID | Name | Arguments | Description |
|----|------|-----------|-------------|
| 25 | SYS_READ_STDIN | (buf, len) | Read from keyboard |
| 26 | SYS_SERIAL_WRITE | (buf, len) | Write to serial port |
| 27 | SYS_SERIAL_READ | (buf, len) | Read from serial port |
| 28 | SYS_BLOCK_READ | (lba, buf, count) | Read disk blocks |
| 29 | SYS_BLOCK_WRITE | (lba, buf, count) | Write disk blocks |
| 30 | SYS_GET_DEVICE_LIST | (devices, max) | Enumerate devices |
| 31 | SYS_GET_BLOCK_DEVICE_INFO | (info) | Query block device |

### I/O and File Operations

| ID | Name | Arguments | Description |
|----|------|-----------|-------------|
| 22 | SYS_PIPE | (fds) | Create pipe |
| 33 | SYS_DUP2 | (oldfd, newfd) | Duplicate file descriptor |
| 79 | SYS_GETCWD | (buf, size) | Get current working directory |
| 80 | SYS_CHDIR | (path) | Change directory |

### Process Control

| ID | Name | Arguments | Description |
|----|------|-----------|-------------|
| 109 | SYS_SETPGID | (pid, pgid) | Set process group ID |
| 111 | SYS_GETPGRP | () | Get process group ID |
| 136 | SYS_TCSETPGRP | (fd, pgid) | Set foreground process group |
| 137 | SYS_TCGETPGRP | (fd) | Get foreground process group |
| 62 | SYS_KILL | (pid, sig) | Send signal to process |
| 13 | SYS_SIGACTION | (sig, act, oldact) | Set signal handler |

### Terminal Control (ioctl)

| Command | Description |
|---------|-------------|
| TCGETS | Get termios settings |
| TCSETS | Set termios settings |
| TIOCGWINSZ | Get window size |
| TIOCSWINSZ | Set window size |
| TIOCGPTN | Get PTY slave number |

### Example: Using System Calls

```rust
// Userland code using fast syscall instruction
use core::arch::asm;

fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            inout("rax") id => ret,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            out("rcx") _,  // Clobbered by syscall
            out("r11") _,  // Clobbered by syscall
            options(nostack)
        );
    }
    ret
}

// Write to stdout
let msg = "Hello from userland!\n";
syscall(1, 1, msg.as_ptr() as usize, msg.len());

// Get process ID
let pid = syscall(39, 0, 0, 0);

// Fork (create child process)
let child_pid = syscall(57, 0, 0, 0);
if child_pid == 0 {
    // Child process
    syscall(1, 1, "I'm the child!\n".as_ptr() as usize, 15);
    syscall(60, 0, 0, 0); // Exit
} else {
    // Parent process
    let mut status = 0;
    syscall(61, child_pid as usize, &mut status as *mut i32 as usize, 0); // Wait
}

// Create a pipe
let mut fds = [0i32; 2];
syscall(22, &mut fds as *mut [i32; 2] as usize, 0, 0);

// Open a file
let path = "/proc/cpuinfo\0";
let fd = syscall(2, path.as_ptr() as usize, 0, 0);

// Read from file
let mut buf = [0u8; 1024];
let bytes_read = syscall(0, fd as usize, buf.as_mut_ptr() as usize, buf.len());

// Close file
syscall(3, fd as usize, 0, 0);
```

## 🐚 Shell Features (mello-sh)

MelloOS includes a full-featured POSIX-like shell with:

### Job Control
```bash
# Run command in background
$ long_running_command &
[1] 42

# List jobs
$ jobs
[1]+ Running    long_running_command &

# Bring job to foreground
$ fg %1

# Suspend current job (Ctrl-Z)
^Z
[1]+ Stopped    long_running_command

# Resume in background
$ bg %1
[1]+ Running    long_running_command &
```

### Pipelines
```bash
# Chain commands with pipes
$ cat /proc/cpuinfo | grep "processor" | wc -l

# Complex pipelines
$ ps | grep mello | cat
```

### I/O Redirection
```bash
# Redirect output
$ echo "Hello" > file.txt

# Append to file
$ echo "World" >> file.txt

# Redirect input
$ cat < file.txt

# Combine redirections
$ grep "pattern" < input.txt > output.txt
```

### Built-in Commands
```bash
$ cd /proc              # Change directory
$ export PATH=/bin      # Set environment variable
$ unset OLDVAR          # Remove environment variable
$ jobs                  # List background jobs
$ fg %1                 # Foreground job
$ bg %1                 # Background job
$ exit                  # Exit shell
```

### Environment Variables
```bash
$ export LANG=C.UTF-8   # UTF-8 support
$ export PATH=/bin      # Search path
$ echo $HOME            # Display variable
```

## 🧰 Userland Utilities

### Core Utilities (mellobox)

MelloOS includes a BusyBox-style multi-call binary with 14 utilities:

### File Operations
- **ls** - List directory contents with color support
- **cat** - Concatenate and display files
- **cp** - Copy files and directories
- **mv** - Move/rename files
- **rm** - Remove files and directories
- **mkdir** - Create directories
- **touch** - Create empty files or update timestamps

### Text Processing
- **grep** - Search for patterns in files (supports -i, -r, -n)
- **echo** - Display text

### Process Management
- **ps** - Display process information
- **kill** - Send signals to processes

### System Utilities
- **pwd** - Print working directory
- **true** - Return success (exit code 0)
- **false** - Return failure (exit code 1)

### Device Testing Tools

MelloOS includes specialized tools for testing device drivers:

- **kbd_test** - Keyboard input testing and scancode display
- **serial_test** - Serial port communication testing
- **disk_bench** - Disk performance benchmarking (read/write throughput)
- **dmesg** - Display kernel log messages
- **lsdev** - Enumerate all detected devices
- **diskinfo** - Display block device information (capacity, block size)
- **irq_test** - Test interrupt distribution across CPUs

### Usage Examples

```bash
# File operations
$ ls -la /proc
$ cat /proc/cpuinfo
$ cp file1.txt file2.txt
$ mkdir /tmp/test
$ touch newfile.txt

# Text processing
$ grep -i "processor" /proc/cpuinfo
$ echo "Hello, World!"

# Process management
$ ps
$ kill -9 42

# Device testing
$ lsdev                    # List all devices
$ diskinfo                 # Show disk information
$ kbd_test                 # Test keyboard input
$ disk_bench               # Benchmark disk performance
$ dmesg                    # View kernel logs
$ irq_test                 # Test interrupt distribution

# Pipelines
$ cat /proc/stat | grep cpu
$ ps | grep mello
$ dmesg | grep "SMP"
```

## 🛠️ Development

### Adding a New Task

```rust
use crate::sched::{spawn_task, priority::TaskPriority};

fn my_task() -> ! {
    loop {
        serial_println!("Task running!");
        
        // Use syscalls
        unsafe {
            let msg = "Hello!\n";
            syscall(0, 0, msg.as_ptr() as usize, msg.len());
        }
        
        // Sleep
        for _ in 0..1_000_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

// Spawn with priority
spawn_task("my_task", my_task, TaskPriority::Normal)
    .expect("Failed to spawn task");
```

### Memory Allocation

```rust
use crate::mm::allocator::{kmalloc, kfree};

// Allocate 1KB
let ptr = kmalloc(1024);
if !ptr.is_null() {
    // Use memory (automatically zeroed)
    unsafe { *ptr = 0x42; }
    
    // Free when done
    kfree(ptr, 1024);
}
```

### Logging

```rust
// Serial output
serial_println!("Debug message: {}", value);

// Memory management logs
mm_log!("Allocated frame at 0x{:x}", addr);
mm_info!("Total memory: {} MB", total_mb);

// Scheduler logs
sched_log!("Context switch to task {}", task_id);
sched_info!("Spawned task: {}", name);

// Syscall logs (automatic)
// [SYSCALL] Task 1 invoked SYS_WRITE (id=0)
```

## 🧪 Testing

### Automated Tests

```bash
# Run build verification
./tools/testing/verify_build.sh

# Test boot in QEMU (single CPU)
./tools/testing/test_boot.sh

# Test SMP boot with multiple CPUs
./tools/testing/test_boot.sh -smp 2
./tools/testing/test_boot.sh -smp 4 -timeout 10
```

### QEMU Testing Commands

```bash
# Basic QEMU launch (4 CPUs, KVM enabled)
./tools/qemu/qemu.sh

# Specific CPU configurations
./tools/qemu/qemu.sh -smp 1           # Single CPU (disable SMP)
./tools/qemu/qemu.sh -smp 2 -enable-kvm  # 2 CPUs with KVM
./tools/qemu/qemu.sh -smp 8           # Maximum 8 CPUs

# Quick test presets
./tools/qemu/qemu.sh -preset smp2     # 2 CPUs + KVM
./tools/qemu/qemu.sh -preset smp4     # 4 CPUs + KVM  
./tools/qemu/qemu.sh -preset debug    # 2 CPUs for debugging
./tools/qemu/qemu.sh -preset single   # Single CPU mode

# Dedicated SMP test scripts
./tools/qemu/qemu-test-smp2.sh        # Optimized 2-CPU test
./tools/qemu/qemu-test-smp4.sh        # Optimized 4-CPU test

# Debug mode with extensive logging
./tools/qemu/qemu-debug-smp.sh        # 2 CPUs with debug output

# Help and options
./tools/qemu/qemu.sh --help           # Show all available options
```

### Integration Tests

The kernel includes comprehensive integration tests across multiple phases:

**Phase 6: User-Mode Integration Tests** (In Progress)
- **Privilege Level Validation**: Kernel at ring 0, user processes at ring 3
- **Basic Syscall Functionality**: sys_getpid, sys_write, sys_yield
- **Fork Chain Stress Test**: Create chain of 10 processes
- **SMP Safety Tests**: User-mode processes on multiple CPU cores
- **Performance Monitoring**: Syscall latency measurement
- **Memory Protection**: User pointer validation and kernel memory protection

**Phase 5: SMP Multi-Core Tests**
- **CPU Detection**: ACPI MADT parsing and CPU enumeration
- **AP Bootstrap**: Application Processor bringup via INIT/SIPI
- **Multi-Core Scheduling**: Task distribution across CPU cores
- **Load Balancing**: Automatic task migration between cores
- **Cross-Core IPC**: Message passing between tasks on different CPUs
- **Synchronization**: SpinLock correctness under concurrent access

**Phase 4: Advanced Scheduling Tests**
- **Test 7.1**: Priority scheduling (High/Normal/Low tasks)
- **Test 7.2**: Sleep/wake mechanism
- **Test 7.3**: Syscall integration (write, sleep)
- **Test 7.4**: IPC integration (sender/receiver)
- **Test 7.5**: IPC stress test (100 ping-pong messages)
- **Test 7.6**: Init process (end-to-end system test)

**Test Results:**
- SMP tests: All CPUs come online, tasks execute on multiple cores
- User-mode tests: Partial (infrastructure complete, full implementation in progress)
- System stability: Stable under multi-core load

### CI/CD

GitHub Actions automatically:
- Builds the kernel on every push to `develop`
- Runs verification tests
- Creates release artifacts for tagged versions
- Generates bootable ISO images

## ⚡ Performance

### Core Performance Metrics
- **Context Switch**: < 1 microsecond (assembly-optimized)
- **Scheduler Overhead**: ~1% CPU at 20 Hz per core
- **Task Selection**: O(1) with per-CPU runqueues
- **Memory Allocation**: O(log n) for buddy system
- **Syscall Latency**: ~100 nanoseconds (syscall/sysret)
- **IPI Latency**: Sub-microsecond for cross-CPU communication
- **AP Boot Time**: ~500ms per Application Processor
- **Load Balancing**: Periodic rebalancing every 100ms (2 ticks at 20Hz)

### Userland Performance
- **PTY Throughput**: 4KB ring buffers with optimized read/write paths
- **Shell Command Parsing**: < 1ms for typical commands
- **Pipeline Creation**: < 5ms for 3-stage pipelines
- **Signal Delivery**: < 10μs from generation to handler
- **UTF-8 Decoding**: Inline optimized for ASCII fast path
- **/proc Read**: Lock-free with atomic operations

### Performance Targets (All Met ✅)
- ✅ Boot time: < 2 seconds (4 CPUs)
- ✅ Shell responsiveness: < 100ms command latency
- ✅ UTF-8 rendering: 60 FPS capable
- ✅ Job control: < 50ms signal delivery
- ✅ Pipeline throughput: > 1 MB/s
- ✅ Memory efficiency: < 16MB kernel heap usage

See [tools/testing/PERFORMANCE_VERIFICATION_REPORT.md](tools/testing/PERFORMANCE_VERIFICATION_REPORT.md) for detailed benchmarks.

## 📊 Kernel Metrics

The kernel tracks comprehensive statistics with atomic counters:

```rust
pub struct KernelMetrics {
    // Scheduling metrics
    pub ctx_switches: AtomicUsize,       // Total context switches
    pub preemptions: AtomicUsize,        // Preemptive switches
    pub timer_ticks: AtomicUsize,        // Timer interrupts (all cores)
    
    // System call metrics
    pub syscall_count: [AtomicUsize; 256], // Per-syscall counts
    pub total_syscalls: AtomicUsize,     // Total syscalls
    
    // IPC metrics (legacy)
    pub ipc_sends: AtomicUsize,          // IPC send operations
    pub ipc_recvs: AtomicUsize,          // IPC receive operations
    
    // Signal metrics
    pub signals_delivered: AtomicUsize,  // Signals delivered
    
    // PTY metrics
    pub pty_bytes_in: AtomicUsize,       // Bytes written to PTY
    pub pty_bytes_out: AtomicUsize,      // Bytes read from PTY
    
    // Memory metrics
    pub page_faults: AtomicUsize,        // Page fault count
    
    // Interrupt metrics
    pub interrupts: AtomicUsize,         // Total interrupts
}
```

All metrics are thread-safe and can be accessed from any CPU core without locks. Metrics are exposed via `/proc/stat` for monitoring.

## 🗺️ Roadmap

### Phase 7: Device Drivers & I/O ✅ COMPLETE
- [x] Generic driver model and framework
- [x] Driver Manager with registration and probing
- [x] PS/2 keyboard driver with scancode translation
- [x] UART16550 serial port driver (COM1)
- [x] virtio-blk block device driver
- [x] Block device abstraction layer (BlockDevice trait)
- [x] Device discovery and enumeration (Platform, PS/2, PCI, virtio)
- [x] IRQ management with IOAPIC routing and CPU affinity
- [x] I/O infrastructure (port I/O, MMIO, IRQ handling)
- [x] Device tree for hardware tracking
- [x] System calls for device access (7 new syscalls)
- [x] Userland testing tools (7 utilities)
- [x] Integration test suite
- [x] SMP-safe interrupt handling
- [x] Documentation and developer guidelines

**Note:** AHCI and NVMe drivers deferred to future optimization phase. virtio-blk provides sufficient functionality for Phase 8 filesystem support.

### Phase 6.6: Advanced Userland & Shell Environment ✅ COMPLETE
- [x] mello-sh shell with job control, pipelines, I/O redirection
- [x] mello-term terminal emulator with PTY integration
- [x] mellobox coreutils (14 utilities)
- [x] PTY subsystem with termios support
- [x] Signal infrastructure (31 POSIX signals)
- [x] /proc virtual filesystem
- [x] Process groups and sessions
- [x] UTF-8 support throughout userland
- [x] Performance optimizations
- [x] Comprehensive testing suite
- [x] Build system integration

### Phase 8: Filesystem & Storage (Next) 🎯
- [ ] VFS (Virtual File System) layer
- [ ] tmpfs (temporary filesystem in RAM)
- [ ] ext2 filesystem support (read-only → read-write)
- [ ] FAT32 filesystem support (read-only → read-write)
- [ ] Mount/umount syscalls
- [ ] File descriptor management
- [ ] Path resolution
- [ ] Directory operations

**Prerequisites:** All previous phases complete ✅, Block device driver operational ✅

### Phase 9: Networking Stack
- [ ] virtio-net driver (for QEMU/virtualization)
- [ ] Network stack architecture
- [ ] IPv4 protocol implementation
- [ ] ICMP (ping) support
- [ ] UDP protocol
- [ ] TCP-lite (simplified TCP)
- [ ] Socket API and syscalls
- [ ] Network buffer management
- [ ] ARP protocol

### Phase 10: GUI / Desktop Base
- [ ] Framebuffer driver enhancements
- [ ] Compositor for window management
- [ ] Input server (mouse and keyboard)
- [ ] Graphical terminal emulator
- [ ] Basic window system
- [ ] Font rendering
- [ ] Graphics primitives
- [ ] Event handling system

## 📚 Documentation

Comprehensive documentation is available in the `docs/` directory:

### User Documentation
- **[USER_GUIDE.md](docs/USER_GUIDE.md)**: Complete user guide for shell, terminal, and utilities
- **[DEVELOPER_GUIDE.md](docs/DEVELOPER_GUIDE.md)**: Guide for adding syscalls, /proc files, and utilities
- **[TROUBLESHOOTING_GUIDE.md](docs/TROUBLESHOOTING_GUIDE.md)**: Comprehensive troubleshooting for PTY, signals, and job control
- **[UTF8_QUICK_START.md](docs/UTF8_QUICK_START.md)**: Quick start guide for UTF-8 support
- **[UTF8_SUPPORT.md](docs/UTF8_SUPPORT.md)**: Complete UTF-8 implementation details

### Architecture Documentation
- **[Documentation Index](docs/README.md)**: Complete documentation overview
- **[System Architecture](docs/architecture/architecture.md)**: Detailed system architecture with diagrams
- **[SMP Implementation](docs/architecture/smp.md)**: Multi-core support implementation details
- **[Task Scheduler](docs/architecture/task-scheduler.md)**: Scheduler design and algorithms
- **[Memory Management](docs/architecture/memory-management-logging.md)**: Memory subsystem details
- **[Device Drivers](docs/architecture/device-drivers.md)**: Driver framework and implementation
- **[Device Syscalls](docs/architecture/device-syscalls.md)**: Device system call interface
- **[I/O Infrastructure](docs/architecture/IO%20Infrastructure.md)**: Port I/O, MMIO, and IRQ management
- **[PTY Subsystem](docs/architecture/pty-subsystem.md)**: Pseudo-terminal architecture
- **[Signals & Job Control](docs/architecture/signals-job-control.md)**: Signal handling and job control
- **[/proc Filesystem](docs/architecture/proc-filesystem.md)**: Virtual filesystem structure
- **[Performance Optimizations](docs/architecture/performance-optimizations.md)**: Performance strategies

### Development Guides
- **[API Guide](docs/development/api-guide.md)**: API usage examples and best practices
- **[Testing Guide](docs/development/testing.md)**: Testing procedures and verification
- **[Build System Integration](docs/BUILD_SYSTEM_INTEGRATION.md)**: Build system documentation
- **[Debugging Guide](docs/development/DEBUGGING_GUIDE.md)**: ⭐ Complete guide to debugging with GDB and VS Code

### Troubleshooting & Debugging
- **[Troubleshooting Guide](docs/troubleshooting/troubleshooting.md)**: Common issues and solutions
- **[SMP AP Boot Issues](docs/troubleshooting/smp-ap-boot-issues.md)**: ⭐ Complete guide to multi-core boot problems
- **[SMP Safety Guidelines](docs/troubleshooting/smp-safety.md)**: Synchronization best practices
- **[SMP Boot Debug](docs/troubleshooting/smp-boot-debug.md)**: Boot process debugging
- **[Triple Fault Debugging](docs/troubleshooting/DEBUG-SMP-TRIPLE-FAULT.md)**: SMP triple fault analysis

### Tools & Utilities
- **[Tools Documentation](tools/README.md)**: Development tools reference
- **[Debug Tools](tools/debug/)**: GDB scripts and debugging utilities
- **[QEMU Scripts](tools/qemu/)**: Virtualization and testing scripts
- **[Testing Scripts](tools/testing/)**: Automated test suites

### Project History & Reports
- **[CHANGELOG](CHANGELOG.md)**: Version history and release notes
- **[SMP Achievement](docs/SMP-ACHIEVEMENT.md)**: Multi-core support milestone
- **[UTF-8 Implementation Summary](tools/testing/UTF8_IMPLEMENTATION_SUMMARY.md)**: UTF-8 feature summary
- **[Performance Verification Report](tools/testing/PERFORMANCE_VERIFICATION_REPORT.md)**: Performance benchmarks
- **[Test Suite Summary](tools/testing/TEST_SUITE_SUMMARY.md)**: Complete test results

## 🔧 Technical Specifications

### Memory Layout

```
Virtual Address Space (x86-64 Canonical Addresses):
0x0000_0000_0000_0000 - 0x0000_7FFF_FFFF_FFFF : User space (512GB)
0x0000_0000_0040_0000 - 0x0000_0000_004F_FFFF : Init process code/data
0x0000_7FFF_FFFF_0000 - 0x0000_7FFF_FFFF_FFFF : User stack (8KB)
0x0000_8000_0000_0000 - 0xFFFF_7FFF_FFFF_FFFF : Non-canonical (invalid)
0xFFFF_8000_0000_0000 - 0xFFFF_9FFF_FFFF_FFFF : HHDM (direct physical mapping)
0xFFFF_A000_0000_0000 - 0xFFFF_A000_00FF_FFFF : Kernel heap (16MB, buddy allocator)
0xFFFF_FFFF_8000_0000 - 0xFFFF_FFFF_FFFF_FFFF : Kernel code/data (higher half)
```

### CPU Configuration

```
Maximum CPUs: 16 (configurable via MAX_CPUS)
Scheduler Frequency: 20 Hz per core (50ms time slices)
APIC Timer: Per-core Local APIC in one-shot mode
IPI Vectors:
  - RESCHEDULE_IPI: 0x30 (48) - Cross-CPU scheduling
  - TLB_SHOOTDOWN: Reserved for future use

Supported Features:
  - SMP (Symmetric Multi-Processing)
  - APIC (Advanced Programmable Interrupt Controller)
  - Fast syscalls (syscall/sysret)
  - NX bit (No-Execute)
  - PAT (Page Attribute Table)
  - TSC (Time Stamp Counter)
```

### Interrupt Vector Mapping

```
CPU Exceptions:      0-31   (Reserved by CPU)
  - Page Fault:      14     (0x0E) - Memory protection violations
APIC Timer:          32     (0x20) - Per-core preemptive scheduling
Keyboard (IRQ1):     33     (0x21) - Not yet implemented
Other IRQs:          34-47  (0x22-0x2F) - Available for devices
RESCHEDULE_IPI:      48     (0x30) - Cross-CPU scheduling signal
Syscall (legacy):    128    (0x80) - System call interface (int 0x80)
Fast Syscall:        N/A    - syscall/sysret via MSR (LSTAR)
```

### Task/Process States

```
Ready → Running → Ready (preempted or yielded)
  ↓       ↓
  ↓       ↓→ Sleeping → Ready (timer wakeup)
  ↓       ↓→ Blocked → Ready (IPC/wait/signal)
  ↓       ↓→ Stopped → Ready (SIGCONT received)
  ↓       ↓→ Zombie → Terminated (parent collected exit code)
  ↓
  └→ Terminated (cleaned up)
```

**Process States** (user-mode):
- Ready: Waiting in runqueue
- Running: Currently executing on a CPU
- Sleeping: Waiting for timer
- Blocked: Waiting for I/O, IPC, or child process
- Stopped: Suspended by signal (SIGTSTP, SIGTTIN, SIGTTOU)
- Zombie: Terminated but exit code not collected
- Terminated: Fully cleaned up and slot reusable

**Signal States**:
- Pending: Signal queued but not yet delivered
- Blocked: Signal masked by process
- Delivered: Signal handler invoked or default action taken

## 🚧 Current Development Status

### What's Working ✅
- **Multi-Core Boot**: Successfully boots and initializes up to 16 CPU cores
- **SMP Scheduling**: Tasks distributed across all available cores with load balancing
- **Cross-CPU Communication**: IPIs and cross-core IPC working correctly
- **User-Mode Execution**: Ring 3 transitions, syscalls, process management
- **Memory Protection**: User/kernel address space separation enforced
- **Complete Userland**: Shell, terminal emulator, and 14 utilities
- **Device Drivers**: PS/2 keyboard, UART16550 serial, virtio-blk block device
- **I/O Infrastructure**: Port I/O, MMIO, IRQ management with CPU affinity
- **PTY Subsystem**: Full pseudo-terminal support with termios
- **Signal Infrastructure**: 31 POSIX signals with handlers
- **Job Control**: Background jobs, fg/bg, process groups, sessions
- **/proc Filesystem**: Virtual filesystem for system information
- **UTF-8 Support**: International text throughout userland
- **Build System**: Automated build with symlinks and ISO creation
- **Testing Tools**: 7 device testing utilities (kbd_test, disk_bench, lsdev, etc.)

### Next Phase 🎯
- **Phase 8: Filesystem & Storage**: Ready to begin
  - VFS (Virtual File System) layer
  - tmpfs and ext2/FAT32 support
  - Mount/umount syscalls
  - File operations

### Recent Achievements 🎉
- **Phase 7 Complete**: Full device driver infrastructure with keyboard, serial, and disk support
- **Driver Framework**: Generic driver model with registration, probing, and device tree
- **Block Device Support**: virtio-blk driver with BlockDevice trait abstraction
- **IRQ Management**: IOAPIC routing with CPU affinity and SMP-safe handling
- **Device Syscalls**: 7 new syscalls for device access (read_stdin, block_read, get_device_list, etc.)
- **Testing Tools**: 7 specialized utilities for device testing and benchmarking
- **Integration Tests**: Comprehensive test suite for driver functionality
- **Documentation**: Complete architecture docs for device drivers and I/O infrastructure

### Test Results 📊
- **Boot Tests**: ✅ All CPUs come online successfully
- **SMP Tests**: ✅ Multi-core scheduling and load balancing working
- **Device Tests**: ✅ Keyboard, serial, and disk drivers operational
- **Driver Tests**: ✅ Device discovery, IRQ handling, and I/O working
- **UTF-8 Tests**: ✅ Thai, emoji, and mixed scripts render correctly
- **Job Control Tests**: ✅ Background jobs, fg/bg, signals working
- **Pipeline Tests**: ✅ Multi-stage pipelines with I/O redirection
- **PTY Tests**: ✅ Terminal emulation and signal generation
- **Performance Tests**: ✅ All targets met or exceeded

See [docs/SMP-ACHIEVEMENT.md](docs/SMP-ACHIEVEMENT.md) for SMP implementation details, [docs/architecture/device-drivers.md](docs/architecture/device-drivers.md) for driver architecture, [tools/testing/TEST_SUITE_SUMMARY.md](tools/testing/TEST_SUITE_SUMMARY.md) for complete test results, and [tools/testing/PERFORMANCE_VERIFICATION_REPORT.md](tools/testing/PERFORMANCE_VERIFICATION_REPORT.md) for performance benchmarks.

## 🤝 Contributing

This is an educational project demonstrating OS development in Rust. Contributions are welcome:

- Report bugs and issues
- Suggest improvements and features
- Submit pull requests
- Improve documentation
- Add test cases

Please follow the existing code style and include tests for new features. See the documentation in `docs/` for architecture details and development guidelines.

## 📄 License

This project is open source and available under the MIT License.

## 🙏 Acknowledgments

- [Limine Bootloader](https://github.com/limine-bootloader/limine) - Modern UEFI bootloader
- [OSDev Wiki](https://wiki.osdev.org/) - Comprehensive OS development resources
- [Writing an OS in Rust](https://os.phil-opp.com/) - Excellent tutorial series
- [xv6](https://github.com/mit-pdos/xv6-public) - Educational Unix-like OS
- Rust embedded and OS development community

## 📖 References

- [Intel 64 and IA-32 Architectures Software Developer's Manual](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)
- [System V AMD64 ABI](https://refspecs.linuxbase.org/elf/x86_64-abi-0.99.pdf)
- [OSDev Wiki: Interrupts](https://wiki.osdev.org/Interrupts)
- [OSDev Wiki: System Calls](https://wiki.osdev.org/System_Calls)

## 📞 Contact

For questions or discussions, please open an issue on GitHub.

---

**MelloOS** - A modern operating system built from scratch in Rust 🦀✨
