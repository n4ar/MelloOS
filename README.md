# MelloOS

A minimal x86_64 operating system kernel written in Rust, featuring preemptive multitasking, priority-based scheduling, system calls, and inter-process communication.

## 🌟 Features

### Phase 5: SMP Multi-Core Support (Current) ✅

- **Symmetric Multi-Processing**: Support for up to 8 CPU cores with automatic detection
- **ACPI MADT Integration**: CPU discovery via ACPI Multiple APIC Description Table
- **AP Bootstrap**: INIT/SIPI sequence to bring Application Processors online
- **Per-Core Scheduling**: Independent runqueues with automatic load balancing
- **Inter-Processor Interrupts**: Cross-core communication and coordination
- **SMP-Safe Synchronization**: SpinLocks with exponential backoff and IRQ-safe variants
- **Per-CPU Data Structures**: Cache-aligned per-core data to minimize contention
- **Cross-Core IPC**: Message passing between tasks on different CPU cores

### Phase 4: Advanced Scheduling, System Calls, and IPC ✅

- **Priority-Based Scheduler**: Three-level priority system (High, Normal, Low) with O(1) task selection
- **System Call Interface**: x86 `int 0x80` mechanism with 5 syscalls (write, exit, sleep, ipc_send, ipc_recv)
- **Inter-Process Communication**: Port-based message passing with 256 ports and 16-message queues
- **Sleep/Wake Mechanism**: Timer-based task suspension with automatic wake-up
- **Userland Init Process**: First userland process demonstrating syscall and IPC usage
- **Kernel Metrics**: Atomic counters tracking context switches, syscalls, and IPC operations
- **Preemption Control**: Critical section support with preempt_disable/enable

### Phase 3: Task Scheduler ✅

- **Preemptive Multitasking**: Multiple tasks run concurrently with automatic time-sharing
- **Round-Robin Scheduling**: Fair CPU time distribution within same priority level
- **Context Switching**: Assembly-optimized register save/restore (< 1μs per switch)
- **Timer Interrupts**: PIT-based periodic interrupts at 100 Hz (10ms time slices)
- **Task Management**: Task Control Blocks (TCB) with unique IDs, states, and priorities
- **Per-Task Stacks**: Isolated 8KB stacks for each task

### Phase 2: Memory Management ✅

- **Physical Memory Manager (PMM)**: Bitmap-based frame allocator for 4KB pages
- **Paging System**: 4-level page tables with per-section permissions (RX, R, RW+NX)
- **Kernel Heap Allocator**: Buddy System algorithm (64B to 1MB blocks)
- **Security Features**: NX bit support, write protection, memory zeroing, guard pages
- **Memory Statistics**: Total/free memory tracking in MB

### Phase 1: Basic Kernel ✅

- **UEFI Boot**: Limine bootloader integration
- **Framebuffer Driver**: Pixel-level graphics with 8x8 bitmap font
- **Serial Console**: COM1 output for debugging
- **Panic Handler**: Basic error handling

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     MelloOS Kernel                          │
│                                                             │
│  ┌───────────────┐  ┌──────────────┐  ┌─────────────────┐ │
│  │  Framebuffer  │  │    Serial    │  │   Panic Handler │ │
│  │    Driver     │  │     Port     │  │                 │ │
│  └───────────────┘  └──────────────┘  └─────────────────┘ │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │           System Call Interface (sys/)               │  │
│  │  - Syscall dispatcher (int 0x80)                     │  │
│  │  - 5 syscalls: write, exit, sleep, ipc_send/recv    │  │
│  │  - Kernel metrics collection                         │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │           IPC Subsystem (sys/ipc.rs)                 │  │
│  │  - Port-based message passing                        │  │
│  │  - 256 ports with 16-message queues                  │  │
│  │  - Blocking receive with FIFO wake policy            │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │           Task Scheduler (sched/)                    │  │
│  │  - Priority-based scheduling (High/Normal/Low)       │  │
│  │  - Sleep/wake mechanism                              │  │
│  │  - Context switching (< 1μs)                         │  │
│  │  - Timer interrupts (100 Hz)                         │  │
│  │  - Preemption control                                │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │        Memory Management (mm/)                       │  │
│  │  ┌────────────┐ ┌──────────┐ ┌──────────────────┐   │  │
│  │  │    PMM     │ │  Paging  │ │  Heap Allocator  │   │  │
│  │  │  (Bitmap)  │ │ (4-level)│ │ (Buddy System)   │   │  │
│  │  └────────────┘ └──────────┘ └──────────────────┘   │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  Userland Processes                         │
│  - Init process (PID 1)                                    │
│  - Syscall wrappers for kernel services                    │
└─────────────────────────────────────────────────────────────┘
```

## 🚀 Quick Start

### Prerequisites

- Rust toolchain (nightly)
- QEMU (for testing)
- xorriso (for ISO creation)
- make

### Installation

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add x86_64 target
rustup target add x86_64-unknown-none

# Install dependencies (Ubuntu/Debian)
sudo apt install qemu-system-x86 xorriso ovmf

# Install dependencies (macOS)
brew install qemu xorriso
```

### Building and Running

```bash
# Build the kernel
make build

# Build userspace init process
make userspace

# Create bootable ISO
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

**On Serial Console (SMP Boot):**
```
[MM] Initializing memory management...
[MM] ✓ PMM tests passed
[MM] ✓ Paging tests passed
[MM] ✓ Allocator tests passed
[ACPI] RSDP found at 0x...
[ACPI] MADT found at 0x...
[SMP] CPUs detected: 4 (apic_ids=[0,1,2,3])
[APIC] BSP LAPIC initialized at 0xFEE00000
[SMP] BSP online (apic_id=0)
[SMP] Trampoline copied to 0x8000
[SMP] Sending INIT to AP#1 (apic_id=1)
[SMP] Sending SIPI to AP#1 (vector=0x08)
[SMP] AP#1 online
[APIC] core1 timer @1000000Hz
[SMP] Sending INIT to AP#2 (apic_id=2)
[SMP] Sending SIPI to AP#2 (vector=0x08)
[SMP] AP#2 online
[APIC] core2 timer @1000000Hz
[SMP] Sending INIT to AP#3 (apic_id=3)
[SMP] Sending SIPI to AP#3 (vector=0x08)
[SMP] AP#3 online
[APIC] core3 timer @1000000Hz
[IPC] Initializing IPC subsystem...
[IPC] Created 16 system ports (0-15)
[SCHED] INFO: Initializing scheduler...
[KERNEL] ========================================
[KERNEL] Phase 5 SMP Integration Tests
[KERNEL] ========================================
[SCHED] Created task A (priority=10)
[SCHED] Created task B (priority=5)
[SCHED] Created task C (priority=8)
[SCHED] Created task D (priority=3)
[SCHED][core0] run A
[SCHED][core1] run C
[SCHED][core2] run B
[SCHED][core3] run D
[USERLAND] Hello from userland! ✨
[SCHED] send RESCHED IPI → core1
...
```

## 📁 Project Structure

```
mellos/
├── kernel/                 # Kernel source code
│   ├── src/
│   │   ├── main.rs        # Kernel entry point
│   │   ├── framebuffer.rs # Graphics driver
│   │   ├── serial.rs      # Serial port driver
│   │   ├── panic.rs       # Panic handler
│   │   ├── init_loader.rs # Init process loader
│   │   ├── mm/            # Memory management
│   │   │   ├── mod.rs     # MM coordinator
│   │   │   ├── pmm.rs     # Physical memory manager
│   │   │   ├── paging.rs  # Virtual memory
│   │   │   ├── allocator.rs # Heap allocator
│   │   │   └── log.rs     # MM logging utilities
│   │   ├── sched/         # Task scheduler
│   │   │   ├── mod.rs     # Scheduler core
│   │   │   ├── task.rs    # Task management
│   │   │   ├── context.rs # Context switching
│   │   │   ├── priority.rs # Priority scheduler
│   │   │   └── timer.rs   # Timer interrupts
│   │   └── sys/           # System calls and IPC
│   │       ├── mod.rs     # Syscall subsystem
│   │       ├── syscall.rs # Syscall dispatcher
│   │       ├── ipc.rs     # IPC structures
│   │       └── port.rs    # Port management
│   ├── userspace/         # Userland programs
│   │   └── init/          # Init process
│   │       ├── src/main.rs # Init entry point
│   │       └── linker.ld  # Init linker script
│   ├── Cargo.toml         # Kernel dependencies
│   ├── build.rs           # Build script
│   └── linker.ld          # Kernel linker script
├── boot/
│   └── limine.cfg         # Bootloader configuration
├── tools/                  # Development tools
│   ├── qemu/              # QEMU virtualization scripts
│   ├── debug/             # Debugging tools
│   ├── testing/           # Testing and verification
│   └── README.md          # Tools documentation
├── docs/                  # Documentation
│   ├── architecture/      # System architecture docs
│   ├── development/       # Development guides
│   ├── troubleshooting/   # Debugging and issues
│   └── README.md          # Documentation index
├── Makefile               # Build system
├── CHANGELOG.md           # Version history
└── README.md              # This file
```

## 💻 System Calls

MelloOS provides 5 system calls accessible via `int 0x80`:

| ID | Name | Arguments | Description |
|----|------|-----------|-------------|
| 0 | SYS_WRITE | (fd, buf, len) | Write data to serial output |
| 1 | SYS_EXIT | (code) | Terminate current task |
| 2 | SYS_SLEEP | (ticks) | Sleep for specified ticks |
| 3 | SYS_IPC_SEND | (port_id, buf, len) | Send message to port |
| 4 | SYS_IPC_RECV | (port_id, buf, len) | Receive message (blocking) |

### Example: Using System Calls

```rust
// Userland code
use core::arch::asm;

fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "int 0x80",
            inout("rax") id => ret,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            options(nostack)
        );
    }
    ret
}

// Write to serial
let msg = "Hello from userland!\n";
syscall(0, 0, msg.as_ptr() as usize, msg.len());

// Sleep for 100 ticks
syscall(2, 100, 0, 0);

// Send IPC message
let data = b"ping";
syscall(3, 2, data.as_ptr() as usize, data.len());

// Receive IPC message (blocking)
let mut buf = [0u8; 64];
let bytes = syscall(4, 1, buf.as_mut_ptr() as usize, buf.len());
```

## 📬 Inter-Process Communication (IPC)

MelloOS implements port-based message passing:

- **256 ports** (0-255) for communication endpoints
- **16-message queues** per port (max 4096 bytes per message)
- **Non-blocking send** (returns error if queue full)
- **Blocking receive** (task sleeps until message arrives)
- **FIFO wake policy** (first blocked task woken first)

### Example: IPC Communication

```rust
// Sender task
fn sender_task() -> ! {
    loop {
        let msg = b"ping";
        sys_ipc_send(2, msg); // Send to port 2
        sys_sleep(100);
    }
}

// Receiver task
fn receiver_task() -> ! {
    loop {
        let mut buf = [0u8; 64];
        let bytes = sys_ipc_recv(1, &mut buf); // Receive from port 1
        // Process message...
    }
}
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

The kernel includes comprehensive Phase 5 SMP integration tests:

**SMP-Specific Tests:**
- **CPU Detection**: ACPI MADT parsing and CPU enumeration
- **AP Bootstrap**: Application Processor bringup via INIT/SIPI
- **Multi-Core Scheduling**: Task distribution across CPU cores
- **Load Balancing**: Automatic task migration between cores
- **Cross-Core IPC**: Message passing between tasks on different CPUs
- **Synchronization**: SpinLock correctness under concurrent access

**Legacy Phase 4 Tests:**
- **Test 7.1**: Priority scheduling (High/Normal/Low tasks)
- **Test 7.2**: Sleep/wake mechanism
- **Test 7.3**: Syscall integration (write, sleep)
- **Test 7.4**: IPC integration (sender/receiver)
- **Test 7.5**: IPC stress test (100 ping-pong messages)
- **Test 7.6**: Init process (end-to-end system test)

Expected output shows all CPUs coming online, tasks executing on multiple cores, successful cross-core IPC, and proper load balancing behavior.

### CI/CD

GitHub Actions automatically:
- Builds the kernel on every push to `develop`
- Runs verification tests
- Creates release artifacts for tagged versions
- Generates bootable ISO images

## ⚡ Performance

- **Context Switch**: < 1 microsecond
- **Scheduler Overhead**: ~1% CPU at 100 Hz
- **Task Selection**: O(1) with priority bitmap
- **Memory Allocation**: O(log n) for buddy system
- **IPC Send**: O(1) enqueue + O(1) wake
- **IPC Receive**: O(1) dequeue (or block if empty)

## 📊 Kernel Metrics

The kernel tracks various statistics:

```rust
pub struct KernelMetrics {
    pub ctx_switches: AtomicUsize,      // Total context switches
    pub preemptions: AtomicUsize,       // Preemptive switches
    pub syscall_count: [AtomicUsize; 5], // Per-syscall counts
    pub ipc_sends: AtomicUsize,         // IPC send operations
    pub ipc_recvs: AtomicUsize,         // IPC receive operations
    pub ipc_queue_full: AtomicUsize,    // Queue full errors
    pub sleep_count: AtomicUsize,       // Tasks put to sleep
    pub wake_count: AtomicUsize,        // Tasks woken
    pub timer_ticks: AtomicUsize,       // Timer interrupts
}
```

## 🗺️ Roadmap

### Phase 6: User Space (Next)
- [ ] User mode execution (Ring 3)
- [ ] Process isolation with separate page tables
- [ ] ELF binary loading
- [ ] User/kernel memory separation
- [ ] Copy-to/from-user validation
- [ ] Separate user/kernel stacks

### Phase 7: File System
- [ ] VFS (Virtual File System) layer
- [ ] Simple file system implementation (FAT or custom)
- [ ] Device file support (/dev)
- [ ] File descriptors and file operations

### Phase 8: Advanced Features
- [ ] Network stack (TCP/IP)
- [ ] Device drivers (keyboard, disk, network)
- [ ] Advanced scheduling (CFS, real-time)
- [ ] Virtual memory management (demand paging, swap)
- [ ] NUMA awareness and CPU affinity

## 📚 Documentation

Comprehensive documentation is available in the `docs/` directory:

- **[Documentation Index](docs/README.md)**: Complete documentation overview
- **[System Architecture](docs/architecture/architecture.md)**: Detailed system architecture with diagrams
- **[SMP Implementation](docs/architecture/smp.md)**: Multi-core support implementation details
- **[API Guide](docs/development/api-guide.md)**: API usage examples and best practices
- **[Testing Guide](docs/development/testing.md)**: Testing procedures and verification
- **[Troubleshooting](docs/troubleshooting/troubleshooting.md)**: Common issues and solutions
- **[Tools Documentation](tools/README.md)**: Development tools reference
- **[CHANGELOG](CHANGELOG.md)**: Version history and release notes

## 🔧 Technical Specifications

### Memory Layout

```
Virtual Address Space:
0x0000_0000_0000_0000 - 0x0000_7FFF_FFFF_FFFF : User space (reserved)
0x0000_0000_0040_0000 - 0x0000_0000_004F_FFFF : Init process (1MB)
0xFFFF_8000_0000_0000 - 0xFFFF_9FFF_FFFF_FFFF : HHDM (direct physical mapping)
0xFFFF_A000_0000_0000 - 0xFFFF_A000_00FF_FFFF : Kernel heap (16MB)
0xFFFF_FFFF_8000_0000 - 0xFFFF_FFFF_FFFF_FFFF : Kernel code/data
```

### Interrupt Vector Mapping

```
CPU Exceptions:      0-31   (Reserved by CPU)
Timer (IRQ0):        32     (0x20) - PIT interrupt
Keyboard (IRQ1):     33     (0x21) - Not yet implemented
Other IRQs:          34-47  (0x22-0x2F) - Available
Syscall:             128    (0x80) - System call interface
```

### Task States

```
Ready → Running → Ready (preempted)
  ↓       ↓
  ↓       ↓→ Sleeping → Ready (woken)
  ↓       ↓→ Blocked → Ready (message arrived)
  ↓
  └→ Dead (future)
```

## 🤝 Contributing

This is an educational project demonstrating OS development in Rust. Contributions are welcome:

- Report bugs and issues
- Suggest improvements and features
- Submit pull requests
- Improve documentation

Please follow the existing code style and include tests for new features.

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
