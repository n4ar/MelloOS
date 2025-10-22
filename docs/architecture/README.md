# Architecture Documentation

System architecture and design documentation for MelloOS.

## Documents

- **[architecture.md](architecture.md)**: Complete system architecture overview with diagrams
- **[smp.md](smp.md)**: SMP (Symmetric Multi-Processing) implementation details
- **[task-scheduler.md](task-scheduler.md)**: Task scheduler design and algorithms
- **[memory-management-logging.md](memory-management-logging.md)**: Memory management subsystem
- **[pty-subsystem.md](pty-subsystem.md)**: Pseudo-terminal (PTY) subsystem architecture
- **[signals-job-control.md](signals-job-control.md)**: Signal handling and job control implementation
- **[proc-filesystem.md](proc-filesystem.md)**: /proc virtual filesystem structure and implementation
- **[performance-optimizations.md](performance-optimizations.md)**: Performance optimization strategies
- **[IO Infrastructure.md](IO%20Infrastructure.md)**: I/O port, MMIO, and IRQ infrastructure
- **[device-syscalls.md](device-syscalls.md)**: Device driver syscalls for userland access

## Reading Order

1. **architecture.md** - Start here for overall system understanding
2. **task-scheduler.md** - Core scheduling concepts
3. **memory-management-logging.md** - Memory subsystem details
4. **smp.md** - Multi-core implementation (advanced topic)
5. **pty-subsystem.md** - Terminal emulation infrastructure
6. **signals-job-control.md** - Process management and signals
7. **proc-filesystem.md** - System information interface
8. **performance-optimizations.md** - Performance tuning
9. **IO Infrastructure.md** - Low-level I/O infrastructure
10. **device-syscalls.md** - Device driver syscalls