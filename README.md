# MelloOS

MelloOS is a modern operating system built from scratch in Rust, focusing on safety, performance, and extensibility. The project demonstrates advanced OS concepts including memory management, preemptive multitasking, and hardware interrupt handling.

## 🌟 Features

### Core System
- ✨ **Bare-metal kernel** written in Rust (`no_std`)
- 🚀 **UEFI boot** via Limine bootloader (v8.x)
- 🖥️ **Framebuffer driver** with 8x8 bitmap font rendering
- 📝 **Serial port** debugging output (COM1)
- 🔧 **Automated build system** with Makefile

### Memory Management System
- 🧠 **Physical Memory Manager (PMM)** - Bitmap-based frame allocator (4KB frames)
- 📄 **Virtual Memory (Paging)** - 4-level page tables with per-section permissions
- 💾 **Kernel Heap Allocator** - Buddy System algorithm (64B to 1MB blocks)
- 🔒 **Security Features** - NX bit, write protection, memory zeroing, guard pages

### Task Scheduler
- ⚡ **Preemptive Multitasking** - Round-Robin scheduling (up to 64 tasks)
- 🔄 **Context Switching** - Assembly-optimized (< 1 microsecond)
- ⏱️ **Timer Interrupt System** - PIT at 100 Hz with PIC remapping
- 🎯 **Fair Scheduling** - 10ms time slices, O(1) task selection

> 📚 **Detailed Architecture:** See [docs/architecture.md](docs/architecture.md)

## � Prernequisites

### Required Tools

1. **Rust Toolchain** (latest stable) - `rustup target add x86_64-unknown-none`
2. **QEMU** (version 5.0+) - System emulator for x86_64
3. **xorriso** - ISO 9660 filesystem creation tool
4. **OVMF** (optional) - UEFI firmware for QEMU
5. **Git** - Version control

### Installation

<details>
<summary><b>macOS</b></summary>

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup target add x86_64-unknown-none

# Install development tools
brew install qemu xorriso git
brew install --cask edk2-ovmf  # Optional
```
</details>

<details>
<summary><b>Linux (Ubuntu/Debian)</b></summary>

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup target add x86_64-unknown-none

# Install development tools
sudo apt update
sudo apt install -y qemu-system-x86 xorriso ovmf git make
```
</details>

<details>
<summary><b>Linux (Arch)</b></summary>

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup target add x86_64-unknown-none

# Install development tools
sudo pacman -S qemu-full xorriso edk2-ovmf git make
```
</details>

## 🚀 Quick Start

```bash
# Clone the repository
git clone <repository-url>
cd mellos

# Build and run in one command
make run
```

### Build Commands

| Command | Description |
|---------|-------------|
| `make build` | Compile kernel binary |
| `make iso` | Create bootable ISO image |
| `make run` | Build and run in QEMU |
| `make clean` | Remove build artifacts |

### Expected Output

**On Screen:**
```
Hello from MelloOS ✨
```

**On Serial Console:**
```
[MM] Initializing memory management...
[MM] ✓ PMM tests passed
[MM] ✓ Paging tests passed
[MM] ✓ Allocator tests passed
[SCHED] INFO: Initializing scheduler...
[SCHED] INFO: Spawned task 1: Task A
[SCHED] INFO: Spawned task 2: Task B
[TIMER] Timer initialized at 100 Hz
[KERNEL] Boot complete! Entering idle loop...
A
[SCHED] Switch #1 → Task 2 (Task B)
B
[SCHED] Switch #2 → Task 1 (Task A)
A
...
```

## 📁 Project Structure

```
mellos/
├── kernel/src/
│   ├── main.rs              # Kernel entry point
│   ├── mm/                  # Memory management
│   │   ├── pmm.rs           # Physical memory
│   │   ├── paging.rs        # Virtual memory
│   │   └── allocator.rs     # Heap allocator
│   └── sched/               # Task scheduler
│       ├── mod.rs           # Scheduler core
│       ├── task.rs          # Task management
│       ├── context.rs       # Context switching
│       └── timer.rs         # Timer interrupts
├── docs/                    # Documentation
│   ├── architecture.md      # System architecture
│   ├── api-guide.md         # API usage guide
│   ├── testing.md           # Testing procedures
│   └── troubleshooting.md   # Common issues
├── .kiro/specs/             # Design specifications
├── tools/                   # Development scripts
└── Makefile                 # Build automation
```

## 💻 Development

### API Usage

```rust
// Memory allocation
let ptr = kmalloc(1024);
if !ptr.is_null() {
    // Use memory
    kfree(ptr, 1024);
}

// Task spawning
fn my_task() -> ! {
    loop {
        serial_println!("Task running!");
    }
}
spawn_task("my_task", my_task).expect("Failed to spawn");
```

> 📚 **Complete API Guide:** See [docs/api-guide.md](docs/api-guide.md)

### Testing

```bash
# Run automated tests
./tools/verify_build.sh

# Manual testing in QEMU
make run
```

> 📚 **Testing Guide:** See [docs/testing.md](docs/testing.md)

### Troubleshooting

Common issues and solutions:

- **Build errors:** Check Rust toolchain and targets
- **QEMU errors:** Verify QEMU installation and OVMF path
- **Runtime errors:** Enable serial debugging and check logs

> 📚 **Troubleshooting Guide:** See [docs/troubleshooting.md](docs/troubleshooting.md)

## ✅ Current Status

### Completed Features

- ✅ **Phase 1:** Boot and Display
- ✅ **Phase 2:** Memory Management (PMM, Paging, Heap)
- ✅ **Phase 3:** Task Scheduler (Round-Robin, Context Switching, Timer)

### Current Capabilities

**What MelloOS Can Do:**
- Boot via UEFI with Limine bootloader
- Manage physical and virtual memory
- Allocate/free dynamic memory (kmalloc/kfree)
- Run multiple tasks with preemptive multitasking
- Context switch in < 1 microsecond
- Handle timer interrupts at 100 Hz

**Current Limitations:**
- No keyboard/disk/network drivers
- No user space (all code runs in kernel mode)
- No file system or system calls
- Single-core only (no SMP support)
- No priority-based scheduling

## 🗺️ Roadmap

| Phase | Status | Target | Description |
|-------|--------|--------|-------------|
| Phase 1 | ✅ Complete | - | Boot and Display |
| Phase 2 | ✅ Complete | - | Memory Management |
| Phase 3 | ✅ Complete | - | Task Scheduler |
| Phase 4 | 🚧 Planned | Q2 2025 | Advanced Scheduling (priorities, sleep/wake) |
| Phase 5 | 📋 Planned | Q3 2025 | SMP Support (multi-core) |
| Phase 6 | 📋 Planned | Q4 2025 | User Space (ring 0/3, system calls) |
| Phase 7 | 📋 Planned | 2026 | Device Drivers (keyboard, disk, network) |
| Phase 8 | 📋 Planned | 2026 | File System (VFS, FAT32, ext2) |

<details>
<summary><b>View Detailed Roadmap</b></summary>

### Phase 4: Advanced Scheduling
- Priority-based scheduling
- Sleep/wake mechanisms
- Wait queues
- Scheduler statistics

### Phase 5: SMP Support
- Multi-core detection (ACPI)
- Per-CPU data structures
- Spinlocks and synchronization
- Load balancing

### Phase 6: User Space
- Ring 0/3 separation
- System call interface
- Process management (fork, exec)
- Address space isolation

### Phase 7: Device Drivers
- Driver framework
- PS/2 keyboard/mouse
- ATA/SATA/NVMe storage
- E1000/Virtio-net network

### Phase 8: File System
- VFS abstraction
- FAT32 (read/write)
- ext2 (read-only)
- Mounting and path resolution

</details>

## 📚 Documentation

- **[Architecture Guide](docs/architecture.md)** - System design and components
- **[API Guide](docs/api-guide.md)** - How to use kernel APIs
- **[Testing Guide](docs/testing.md)** - Testing procedures
- **[Troubleshooting](docs/troubleshooting.md)** - Common issues and solutions

## 🧪 CI/CD

### Automated Testing

GitHub Actions runs on every push to `develop`:
- ✅ Build kernel
- ✅ Create ISO
- ✅ Run verification tests
- ✅ Test bootability in QEMU

### Branch Protection

Recommended settings for `develop` and `main` branches:
- Require Pull Request reviews
- Require status checks to pass
- No direct pushes

See [.github/BRANCH_PROTECTION.md](.github/BRANCH_PROTECTION.md) for setup guide.

### Automated Releases

Creating a version tag (e.g., `v1.0.0`) triggers:
- Kernel build
- ISO creation
- GitHub Release with downloadable ISO

## 🙏 Acknowledgments

### Projects and Communities
- **[Limine Bootloader](https://github.com/limine-bootloader/limine)** - Modern UEFI bootloader
- **[Rust Embedded](https://github.com/rust-embedded)** - Embedded Rust tools
- **[OSDev Wiki](https://wiki.osdev.org/)** - OS development resources
- **[Phil Opp's Blog](https://os.phil-opp.com/)** - "Writing an OS in Rust"

### Technical References
- Intel 64 and IA-32 Architectures Software Developer's Manual
- AMD64 Architecture Programmer's Manual
- System V AMD64 ABI
- xv6 (MIT) - Educational Unix-like OS
- Linux Kernel source code

## 📄 License

This project is open source and available for educational purposes.

## 🤝 Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`make run` and verify output)
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

### Code Style
- Follow Rust standard formatting (`cargo fmt`)
- Run Clippy for lints (`cargo clippy`)
- Add comments for complex logic
- Update documentation for API changes

## 📞 Contact

For questions or discussions:
- Open an issue on GitHub
- Check the [documentation](docs/)
- Review the [specifications](.kiro/specs/)

---

**MelloOS** - A modern operating system built from scratch in Rust 🦀✨
