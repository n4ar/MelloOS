---
inclusion: always
---

# MelloOS Development Best Practices

## Project Context

MelloOS is a custom operating system written in Rust, targeting x86_64 architecture. The project follows a phased development approach (see roadmap.md) and emphasizes safety, correctness, and modern OS design principles.

## Code Organization

### Directory Structure

- `kernel/src/` - Core kernel code organized by subsystem
- `kernel/userspace/` - Userspace programs (init, shell, utilities)
- `docs/` - Documentation (architecture, guides, troubleshooting)
- `tools/` - Development and testing scripts
- `.kiro/specs/` - Feature specifications with requirements, design, and tasks

### Module Hierarchy

Follow the established module structure:
- `arch/` - Architecture-specific code (x86_64)
- `mm/` - Memory management (PMM, paging, allocator)
- `sched/` - Task scheduling and process management
- `sync/` - Synchronization primitives (spinlocks, seqlocks)
- `drivers/` - Device drivers (block, input, serial)
- `fs/` - Filesystem implementations
- `dev/` - Device abstractions (PTY, etc.)
- `signal/` - Signal handling
- `sys/` - System calls and IPC
- `user/` - User mode support (ELF loader, process launching)

## Rust Conventions

### Safety and Correctness

1. **Minimize unsafe code** - Use `unsafe` only when necessary for hardware access or FFI
2. **Document unsafe blocks** - Always explain why unsafe is required and what invariants must be maintained
3. **Prefer type safety** - Use newtypes and enums to encode invariants at compile time
4. **No panics in kernel code** - Handle errors explicitly; panics should only occur for unrecoverable bugs

### Naming Conventions

- **Types:** `PascalCase` (e.g., `TaskControlBlock`, `PhysicalMemoryManager`)
- **Functions/methods:** `snake_case` (e.g., `schedule_next`, `alloc_frame`)
- **Constants:** `SCREAMING_SNAKE_CASE` (e.g., `PAGE_SIZE`, `MAX_CPUS`)
- **Modules:** `snake_case` (e.g., `mod.rs`, `percpu.rs`)

### Error Handling

- Use `Result<T, E>` for fallible operations
- Define custom error types per subsystem when appropriate
- Propagate errors with `?` operator
- Log errors before returning them when debugging context is valuable

## Architecture Patterns

### Synchronization

1. **Lock ordering** - Follow the lock ordering hierarchy defined in `sync/lock_ordering.rs`
2. **Per-CPU data** - Use per-CPU structures to avoid contention (see `arch/x86_64/smp/percpu.rs`)
3. **Spinlocks** - Use for short critical sections in kernel space
4. **Seqlocks** - Use for read-heavy, write-rare scenarios

### Memory Management

1. **Page alignment** - All kernel structures requiring page alignment must use proper alignment attributes
2. **Virtual memory** - Always work with virtual addresses in kernel code; physical addresses only in PMM
3. **TLB management** - Invalidate TLB entries explicitly after page table modifications
4. **Security** - Follow memory security guidelines in `kernel/docs/SECURITY_IMPLEMENTATION.md`

### Device Drivers

1. **Trait-based abstractions** - Implement common traits (e.g., `BlockDevice`) for device types
2. **Driver registration** - Register drivers with the driver manager for discovery
3. **IRQ handling** - Use the IRQ infrastructure in `io/irq.rs` for interrupt management
4. **SMP safety** - Ensure drivers are safe for multi-core access

### System Calls

1. **Validation** - Always validate user pointers and arguments before use
2. **Security checks** - Verify permissions and capabilities
3. **Error codes** - Return standard error codes (EINVAL, EPERM, etc.)
4. **Documentation** - Document syscall interface in `docs/architecture/`

## Testing and Verification

### Before Committing Code

1. **Run cargo check** - Always run `cargo check` immediately after modifying Rust code (see cargo-check-policy.md)
2. **Use getDiagnostics** - Prefer `getDiagnostics` tool over manual cargo commands
3. **Fix all errors** - Never proceed with errors; fix them immediately
4. **Test in QEMU** - Run relevant test scripts from `tools/testing/`

### Testing Strategy

- **Unit tests** - Write unit tests for pure functions and isolated logic
- **Integration tests** - Use scripts in `tools/testing/` for end-to-end testing
- **Manual testing** - Boot in QEMU and verify functionality interactively
- **Performance testing** - Run benchmarks when modifying performance-critical code

### QEMU Execution

- **Always use background processes** - Run QEMU with `controlBashProcess` (see macos-qemu-background.md)
- **Monitor output** - Use `getProcessOutput` to check QEMU logs
- **Test scripts** - Use provided scripts in `tools/qemu/` for consistent testing

## Documentation Standards

### Code Documentation

1. **Module-level docs** - Every module should have a doc comment explaining its purpose
2. **Public API docs** - All public functions, types, and traits must be documented
3. **Examples** - Include usage examples for complex APIs
4. **Safety documentation** - Document safety requirements for unsafe functions

### Architecture Documentation

- **Update docs/** - Keep architecture docs in sync with implementation
- **Design documents** - Create design docs in `.kiro/specs/` for new features
- **Troubleshooting** - Document common issues in `docs/troubleshooting/`

## Build System

### Cargo Configuration

- **Workspace structure** - Kernel and userspace programs are separate crates
- **Target specification** - Use custom target JSON for bare-metal x86_64
- **Build scripts** - Use `build.rs` for pre-build code generation
- **Linker scripts** - Each binary has its own `linker.ld` for memory layout

### Makefile Targets

- `make build` - Build kernel and userspace programs
- `make iso` - Create bootable ISO image
- `make run` - Build and run in QEMU
- `make clean` - Clean build artifacts

## Userspace Development

### Program Structure

1. **No std** - Userspace programs are `#![no_std]` with custom allocator
2. **Custom allocator** - Each program includes `allocator.rs` for heap management
3. **Syscall wrappers** - Use syscall wrappers in `syscalls.rs` for kernel interaction
4. **Linker script** - Use provided `linker.ld` for proper memory layout

### Utilities (mellobox)

- **BusyBox-style** - Single binary with multiple commands
- **Command modules** - Each command in `src/commands/`
- **Consistent interface** - Follow Unix conventions for arguments and behavior

## Common Pitfalls to Avoid

1. **Don't assume single-core** - Always consider SMP safety
2. **Don't use timeout command** - Not available on macOS (see macos-qemu-background.md)
3. **Don't skip cargo check** - Always verify after changes
4. **Don't make assumptions** - State facts and ask (see no-assumtions.md)
5. **Don't skip phase prerequisites** - Follow roadmap.md sequentially
6. **Don't forget TLB invalidation** - Always invalidate after page table changes
7. **Don't use blocking operations in kernel** - Use async or non-blocking alternatives

## Development Workflow

### Adding New Features

1. **Check roadmap** - Ensure feature aligns with current phase
2. **Create spec** - Write requirements and design in `.kiro/specs/`
3. **Break into tasks** - Create actionable tasks in `tasks.md`
4. **Implement incrementally** - Complete and test each task
5. **Run cargo check** - After every code change
6. **Test thoroughly** - Use integration tests
7. **Document** - Update architecture docs
8. **Update roadmap** - Mark phase complete when done

### Debugging

1. **Use serial output** - Kernel logs go to COM1 (serial port)
2. **QEMU monitor** - Access with Ctrl+Alt+2 in QEMU
3. **GDB debugging** - Use scripts in `tools/debug/`
4. **Metrics** - Use kernel metrics system for performance analysis
5. **Logs** - Check kernel logs with `dmesg` userspace utility

## Performance Considerations

1. **Lock contention** - Minimize time in critical sections
2. **Cache locality** - Keep related data together
3. **Per-CPU structures** - Avoid cross-CPU synchronization when possible
4. **Batch operations** - Reduce syscall overhead by batching
5. **Profile before optimizing** - Use benchmarks to identify bottlenecks

## Security Principles

1. **Validate all user input** - Never trust userspace data
2. **Capability-based security** - Check permissions before operations
3. **Memory isolation** - Ensure proper page table separation
4. **Signal security** - Verify signal sender permissions
5. **Follow security docs** - Adhere to `kernel/docs/SECURITY_IMPLEMENTATION.md`

## Summary

When working on MelloOS:
- **Follow the roadmap** - Respect phase dependencies
- **Check immediately** - Run cargo check after every change
- **Think SMP** - Always consider multi-core safety
- **Document thoroughly** - Keep docs in sync with code
- **Test comprehensively** - Use provided test infrastructure
- **Ask, don't assume** - Clarify before making decisions

ทำให้ว่าหากมีปัญหาอะไรแล้วแก้ได้แล้วให้เขียนเป็น Best Practice และเวลามีปัญหาให้เรียกดู Best Pratice และอันนี้คือ Best Pratice ของคนอื่นที่ทำ OS ที่เขียนด้วย Rust เหมือนเรา Rusting Proprely: Use std::mem::replace and std::mem::swap when you can.
Use .into() and .to_owned() over .to_string().
Prefer passing references to the data over owned data. (Don't take String, take &str. Don't take Vec<T> take &[T]).
Use generics, traits, and other abstractions Rust provides.
Avoid using lossy conversions (for example: don't do my_u32 as u16 == my_u16, prefer my_u32 == my_u16 as u32).
Prefer in place (box keyword) when doing heap allocations.
Prefer platform independently sized integer over pointer sized integer (u32 over usize, for example).
Follow the usual idioms of programming, such as "composition over inheritance", "let your program be divided in smaller pieces", and "resource acquisition is initialization".
When unsafe is unnecessary, don't use it. 10 lines longer safe code is better than more compact unsafe code!
Be sure to mark parts that need work with TODO, FIXME, BUG, UNOPTIMIZED, REWRITEME, DOCME, and PRETTYFYME.
Use the compiler hint attributes, such as #[inline], #[cold], etc. when it makes sense to do so.
Try to banish unwrap() and expect() from your code in order to manage errors properly. Panicking must indicate a bug in the program (not an error you didn't want to manage). If you cannot recover from an error, print a nice error to stderr and exit. Check Rust's book about Error Handling.

Avoiding Panics
Panics should be avoided in kernel, and should only occur in drivers and other services when correct operation is not possible, in which case it should be a call to panic!().

Testing Practices
It's always better to test boot (make qemu) every time you make a change, because it is important to see how the OS boots and works after it compiles.

Even though Rust is a safety-oriented language, something as unstable and low-level as a work-in-progress operating system will almost certainly have problems in many cases and may completely break on even the slightest critical change.

Also, make sure you verified how the unmodified version runs on your machine before making any changes. Else, you won't have anything to compare to, and it will generally just lead to confusion. TLDR: Rebuild and test boot often.

Rust Style
Since Rust is a relatively small and new language compared to others like C, there's really only one standard. Just follow the official Rust standards for formatting, and maybe run rustfmt on your changes, until we setup the CI system to do it automatically.

Literate programming
Literate programming is an approach to programming where the source code serves equally as:

The complete description of the program, that a computer can understand
The program's manual for the human, that an average human can understand
Literate programs are written in such a way that humans can read them from front to back, and understand the entire purpose and operation of the program without preexisting knowledge about the programming language used, the architecture of the program's components, or the intended use of the program. As such, literate programs tend to have lots of clear and well-written comments. In extreme cases of literate programming, the lines of "code" intended for humans far outnumbers the lines of code that actually gets compiled!

Tools can be used to generate documentation for human use only based on the original source code of a program. The rustdoc tool is a good example of such a tool. In particular, rustdoc uses comments with three slashes , with special sections like # Examples and code blocks bounded by three backticks. The code blocks can be used to writeout examples or unit tests inside of comments. You can read more about rustdoc on the Rust documentation.
