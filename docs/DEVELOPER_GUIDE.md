# MelloOS Developer Guide

## Introduction

This guide provides information for developers who want to extend or modify MelloOS. It covers adding new system calls, /proc files, mellobox utilities, and debugging techniques.

## Development Environment

### Prerequisites

- Rust nightly toolchain
- QEMU for testing
- GDB for debugging
- Basic understanding of OS concepts

### Building the System

```bash
# Build kernel and userspace
make

# Build kernel only
cd kernel && cargo build --release

# Build specific userspace program
cd kernel/userspace/mello-sh && cargo build --release
```

### Running and Testing

```bash
# Run in QEMU
make run

# Run with debugging
make debug

# Run tests
cd kernel && cargo test
```

## Adding New System Calls

### Step 1: Define System Call Number

Edit `kernel/src/sys/syscall.rs`:

```rust
// System call numbers
pub const SYS_READ: usize = 0;
pub const SYS_WRITE: usize = 1;
// ... existing syscalls ...
pub const SYS_MY_NEW_CALL: usize = 100;  // Add your syscall
```

### Step 2: Implement System Call Handler

Add the handler function in `kernel/src/sys/syscall.rs` or appropriate module:

```rust
/// My new system call
///
/// # Arguments
/// * `arg1` - First argument description
/// * `arg2` - Second argument description
///
/// # Returns
/// * `Ok(result)` - Success with result value
/// * `Err(errno)` - Error code
pub fn sys_my_new_call(arg1: usize, arg2: usize) -> Result<usize, Errno> {
    // Validate arguments
    if arg1 == 0 {
        return Err(EINVAL);
    }
    
    // Get current task
    let task = current_task();
    
    // Perform operation
    let result = do_something(task, arg1, arg2)?;
    
    // Log for debugging
    log::debug!("[syscall] my_new_call({}, {}) = {}", arg1, arg2, result);
    
    Ok(result)
}
```

### Step 3: Add to System Call Dispatcher

In `kernel/src/sys/syscall.rs`, add to the `syscall_handler` function:

```rust
pub fn syscall_handler(
    syscall_num: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> Result<usize, Errno> {
    match syscall_num {
        SYS_READ => sys_read(arg1, arg2, arg3),
        SYS_WRITE => sys_write(arg1, arg2, arg3),
        // ... existing syscalls ...
        SYS_MY_NEW_CALL => sys_my_new_call(arg1, arg2),
        _ => Err(ENOSYS),
    }
}
```

### Step 4: Add Userspace Wrapper

For shell, add to `kernel/userspace/mello-sh/src/syscalls.rs`:

```rust
pub fn my_new_call(arg1: usize, arg2: usize) -> Result<usize, i32> {
    unsafe {
        let result: isize;
        asm!(
            "syscall",
            in("rax") SYS_MY_NEW_CALL,
            in("rdi") arg1,
            in("rsi") arg2,
            lateout("rax") result,
            options(nostack)
        );
        
        if result < 0 {
            Err((-result) as i32)
        } else {
            Ok(result as usize)
        }
    }
}
```

For mellobox, add to `kernel/userspace/mellobox/src/syscalls.rs` similarly.

### Step 5: Test the System Call

Create a test program:

```rust
// In userspace program
fn test_my_syscall() {
    match my_new_call(42, 100) {
        Ok(result) => println!("Success: {}", result),
        Err(errno) => println!("Error: {}", errno),
    }
}
```

### Best Practices

1. **Validate all user pointers**: Use `copy_from_user` and `copy_to_user`
2. **Check permissions**: Verify user has rights to perform operation
3. **Handle errors gracefully**: Return appropriate errno codes
4. **Log important operations**: Use `log::debug!` or `log::info!`
5. **Document thoroughly**: Add doc comments explaining behavior
6. **Test edge cases**: Test with invalid arguments, null pointers, etc.

### Common Error Codes

| Code   | Value | Description |
|--------|-------|-------------|
| EINVAL | 22    | Invalid argument |
| EPERM  | 1     | Operation not permitted |
| EACCES | 13    | Permission denied |
| ENOENT | 2     | No such file or directory |
| ESRCH  | 3     | No such process |
| EIO    | 5     | I/O error |
| ENOMEM | 12    | Out of memory |
| ENOSYS | 38    | Function not implemented |

## Adding New /proc Files

### Step 1: Define File Structure

In `kernel/src/fs/proc/mod.rs`, add your file to the appropriate location:

```rust
// For per-process files
fn init_proc_pid_files() -> Vec<ProcFile> {
    vec![
        ProcFile::new("stat", proc_pid_stat),
        ProcFile::new("cmdline", proc_pid_cmdline),
        ProcFile::new("status", proc_pid_status),
        ProcFile::new("myfile", proc_pid_myfile),  // Add here
    ]
}

// For system-wide files
fn init_proc_system_files() -> Vec<ProcFile> {
    vec![
        ProcFile::new("meminfo", proc_meminfo),
        ProcFile::new("cpuinfo", proc_cpuinfo),
        ProcFile::new("myinfo", proc_myinfo),  // Add here
    ]
}
```

### Step 2: Implement File Generator

```rust
/// Generate content for /proc/<pid>/myfile
fn proc_pid_myfile(pid: Pid, buf: &mut [u8]) -> Result<usize, Errno> {
    // Find the task
    let task = find_task(pid).ok_or(ESRCH)?;
    
    // Use seqlock for consistent read
    let seq = task.seqlock.read_begin();
    
    // Read data
    let data = MyData {
        field1: task.field1,
        field2: task.field2,
    };
    
    // Check for concurrent modification
    if !task.seqlock.read_retry(seq) {
        return Err(EAGAIN);  // Caller should retry
    }
    
    // Format output
    let output = format!(
        "Field1: {}\nField2: {}\n",
        data.field1,
        data.field2
    );
    
    // Copy to buffer
    let len = output.len().min(buf.len());
    buf[..len].copy_from_slice(&output.as_bytes()[..len]);
    
    Ok(len)
}
```

### Step 3: Handle Concurrent Access

For system-wide files that don't need seqlock:

```rust
fn proc_myinfo(buf: &mut [u8]) -> Result<usize, Errno> {
    // Get data from kernel subsystems
    let stats = get_my_statistics();
    
    // Format output
    let output = format!(
        "Counter1: {}\nCounter2: {}\n",
        stats.counter1,
        stats.counter2
    );
    
    let len = output.len().min(buf.len());
    buf[..len].copy_from_slice(&output.as_bytes()[..len]);
    
    Ok(len)
}
```

### Step 4: Test the File

```bash
# Boot system
make run

# In shell, read your file
cat /proc/self/myfile
cat /proc/myinfo
```

### Best Practices

1. **Use seqlock for process data**: Ensures consistent snapshots
2. **Handle ESRCH**: Process may exit during read
3. **Limit output size**: Don't generate huge files
4. **Format consistently**: Follow existing /proc conventions
5. **Document format**: Add comments explaining output format
6. **Handle errors**: Return appropriate errno codes

### File Format Guidelines

**Key-value format:**
```
Key1: value1
Key2: value2
```

**Space-separated format (Linux-compatible):**
```
field1 field2 field3 field4
```

**Multi-line format:**
```
Section1:
  Item1: value1
  Item2: value2
Section2:
  Item3: value3
```

## Adding New Mellobox Utilities

### Step 1: Create Utility Module

Create `kernel/userspace/mellobox/src/commands/myutil.rs`:

```rust
use crate::args::Args;
use crate::error::{Error, Result};
use crate::syscalls::*;

/// My utility - does something useful
pub fn main(args: &[&str]) -> Result<i32> {
    // Parse arguments
    let mut opts = Args::new(args);
    let mut verbose = false;
    let mut output_file = None;
    
    while let Some(opt) = opts.next_opt()? {
        match opt {
            "-v" | "--verbose" => verbose = true,
            "-o" | "--output" => {
                output_file = Some(opts.next_arg()
                    .ok_or(Error::Usage("Missing output file"))?);
            }
            "-h" | "--help" => {
                print_usage();
                return Ok(0);
            }
            _ => return Err(Error::Usage("Unknown option")),
        }
    }
    
    // Get positional arguments
    let input_files = opts.remaining();
    if input_files.is_empty() {
        return Err(Error::Usage("No input files specified"));
    }
    
    // Perform operation
    for file in input_files {
        if verbose {
            println!("Processing: {}", file);
        }
        process_file(file)?;
    }
    
    Ok(0)
}

fn print_usage() {
    println!("Usage: myutil [OPTIONS] <files...>");
    println!();
    println!("Options:");
    println!("  -v, --verbose    Verbose output");
    println!("  -o, --output     Output file");
    println!("  -h, --help       Show this help");
}

fn process_file(path: &str) -> Result<()> {
    // Open file
    let fd = open(path, O_RDONLY, 0)?;
    
    // Read and process
    let mut buf = [0u8; 4096];
    loop {
        let n = read(fd, &mut buf)?;
        if n == 0 {
            break;
        }
        
        // Process data
        process_data(&buf[..n])?;
    }
    
    close(fd)?;
    Ok(())
}

fn process_data(data: &[u8]) -> Result<()> {
    // Your processing logic here
    Ok(())
}
```

### Step 2: Register Utility

In `kernel/userspace/mellobox/src/commands/mod.rs`:

```rust
pub mod cat;
pub mod cp;
// ... existing modules ...
pub mod myutil;  // Add your module

use crate::error::Result;

pub fn dispatch(name: &str, args: &[&str]) -> Result<i32> {
    match name {
        "cat" => cat::main(args),
        "cp" => cp::main(args),
        // ... existing utilities ...
        "myutil" => myutil::main(args),
        _ => Err(Error::NotFound),
    }
}
```

### Step 3: Create Symlink

In the build system, create a symlink:

```bash
# In Makefile or build script
ln -sf mellobox iso_root/bin/myutil
```

### Step 4: Test the Utility

```bash
# Build
make

# Run
make run

# In shell
myutil --help
myutil -v file1.txt file2.txt
```

### Best Practices

1. **Follow UNIX conventions**: Use standard option formats
2. **Provide help**: Implement `-h` or `--help`
3. **Handle errors gracefully**: Print clear error messages
4. **Return correct exit codes**: 0 for success, 1 for errors, 2 for usage
5. **Support stdin**: Read from stdin if no files specified
6. **Be efficient**: Use buffered I/O, avoid unnecessary allocations
7. **Handle UTF-8**: Use Rust's native UTF-8 string handling

### Argument Parsing Patterns

**Simple flags:**
```rust
let mut verbose = false;
while let Some(opt) = opts.next_opt()? {
    match opt {
        "-v" => verbose = true,
        _ => return Err(Error::Usage("Unknown option")),
    }
}
```

**Options with arguments:**
```rust
let mut output = None;
while let Some(opt) = opts.next_opt()? {
    match opt {
        "-o" => output = Some(opts.next_arg().ok_or(...)?),
        _ => return Err(Error::Usage("Unknown option")),
    }
}
```

**Combined short options:**
```rust
// Handles -abc as -a -b -c
while let Some(opt) = opts.next_opt()? {
    match opt {
        "-a" => flag_a = true,
        "-b" => flag_b = true,
        "-c" => flag_c = true,
        _ => return Err(Error::Usage("Unknown option")),
    }
}
```

## Debugging Techniques

### Kernel Debugging

#### Using Serial Output

```rust
// In kernel code
use crate::serial;

serial_println!("Debug: value = {}", value);
serial_println!("Task: pid={}, state={:?}", task.pid, task.state);
```

#### Using Log Macros

```rust
use log::{debug, info, warn, error};

debug!("Detailed debug info: {}", value);
info!("Important event occurred");
warn!("Potential issue: {}", issue);
error!("Critical error: {}", error);
```

#### GDB Debugging

```bash
# Terminal 1: Start QEMU with GDB server
make debug

# Terminal 2: Connect GDB
gdb kernel/target/x86_64-unknown-none/release/kernel
(gdb) target remote :1234
(gdb) break sys_my_new_call
(gdb) continue
```

**Useful GDB commands:**
```
break function_name    # Set breakpoint
continue               # Continue execution
step                   # Step into
next                   # Step over
print variable         # Print variable
backtrace              # Show call stack
info registers         # Show registers
x/10x address          # Examine memory
```

#### Panic Dumps

When kernel panics, it dumps:
- CPU number
- Current task (PID, PGID, SID, TTY)
- Last system call
- Register state (RIP, RSP, CR2)
- Stack trace

Example:
```
KERNEL PANIC on CPU 0
Message: assertion failed: ptr != null
Task: PID=123 PGID=123 SID=123
TTY: Some(pts/0)
Last syscall: SYS_READ
RIP: 0xffffffff80001234
RSP: 0xffffffff80100000
CR2: 0x0000000000000000
Stack trace:
  0xffffffff80001234
  0xffffffff80002345
  0xffffffff80003456
```

### Userspace Debugging

#### Printf Debugging

```rust
// In userspace code
println!("Debug: entering function");
println!("Value: {}", value);
println!("Error: {:?}", error);
```

#### Exit Codes

```rust
// Return different codes for different errors
match operation() {
    Ok(_) => return 0,
    Err(Error::NotFound) => return 1,
    Err(Error::Permission) => return 2,
    Err(_) => return 3,
}
```

#### Tracing System Calls

Add logging to syscall wrappers:

```rust
pub fn read(fd: i32, buf: &mut [u8]) -> Result<usize, i32> {
    println!("[syscall] read(fd={}, len={})", fd, buf.len());
    let result = unsafe { /* syscall */ };
    println!("[syscall] read returned: {:?}", result);
    result
}
```

### Performance Debugging

#### Timing Operations

```rust
use crate::syscalls::clock_gettime;

let start = clock_gettime(CLOCK_MONOTONIC)?;
perform_operation()?;
let end = clock_gettime(CLOCK_MONOTONIC)?;

let duration_ns = (end.tv_sec - start.tv_sec) * 1_000_000_000
                + (end.tv_nsec - start.tv_nsec);
println!("Operation took {} ns", duration_ns);
```

#### Profiling

```rust
// Add counters
static OPERATION_COUNT: AtomicUsize = AtomicUsize::new(0);
static OPERATION_TIME: AtomicU64 = AtomicU64::new(0);

fn my_operation() {
    let start = rdtsc();
    // ... operation ...
    let end = rdtsc();
    
    OPERATION_COUNT.fetch_add(1, Ordering::Relaxed);
    OPERATION_TIME.fetch_add(end - start, Ordering::Relaxed);
}

// Print statistics
fn print_stats() {
    let count = OPERATION_COUNT.load(Ordering::Relaxed);
    let time = OPERATION_TIME.load(Ordering::Relaxed);
    println!("Operations: {}, Avg time: {} cycles", count, time / count);
}
```

### Debug /proc Files

Use debug /proc files for runtime inspection:

```bash
# PTY state
cat /proc/debug/pty

# Session tree
cat /proc/debug/sessions

# Lock statistics
cat /proc/debug/locks
```

### Common Issues and Solutions

#### Issue: System Call Returns EFAULT

**Cause:** Invalid user pointer

**Solution:**
```rust
// Validate pointer before use
let user_ptr = arg1 as *const MyStruct;
if !is_user_address(user_ptr as usize, size_of::<MyStruct>()) {
    return Err(EFAULT);
}

// Use copy_from_user
let data = copy_from_user(user_ptr)?;
```

#### Issue: Deadlock

**Cause:** Lock ordering violation

**Solution:**
```rust
// Always acquire locks in same order
// Correct:
let _lock1 = global_lock.lock();
let _lock2 = local_lock.lock();

// Wrong:
let _lock2 = local_lock.lock();
let _lock1 = global_lock.lock();  // Deadlock!
```

#### Issue: Race Condition

**Cause:** Concurrent access without synchronization

**Solution:**
```rust
// Use atomic operations
task.pending_signals.fetch_or(1 << sig, Ordering::SeqCst);

// Or use locks
let _lock = task.lock.lock();
task.state = TaskState::Running;
```

#### Issue: Memory Leak

**Cause:** Forgetting to free resources

**Solution:**
```rust
// Use RAII patterns
struct FileGuard(i32);

impl Drop for FileGuard {
    fn drop(&mut self) {
        close(self.0).ok();
    }
}

let _guard = FileGuard(fd);
// File automatically closed when guard drops
```

## Code Style Guidelines

### Rust Style

Follow standard Rust conventions:

```rust
// Use snake_case for functions and variables
fn my_function(my_variable: usize) -> Result<()> {
    // ...
}

// Use CamelCase for types
struct MyStruct {
    field: usize,
}

// Use SCREAMING_SNAKE_CASE for constants
const MAX_SIZE: usize = 4096;

// Document public APIs
/// Does something useful
///
/// # Arguments
/// * `arg` - Description of argument
///
/// # Returns
/// * `Ok(result)` - Success
/// * `Err(errno)` - Error
pub fn my_api(arg: usize) -> Result<usize> {
    // ...
}
```

### Error Handling

```rust
// Use Result for fallible operations
fn operation() -> Result<T, Error> {
    // ...
}

// Use ? operator for propagation
let result = operation()?;

// Handle errors explicitly
match operation() {
    Ok(value) => process(value),
    Err(e) => handle_error(e),
}
```

### Safety

```rust
// Document unsafe code
unsafe {
    // SAFETY: Pointer is valid because...
    *ptr = value;
}

// Minimize unsafe blocks
fn safe_wrapper(ptr: *const T) -> Result<T> {
    if ptr.is_null() {
        return Err(EINVAL);
    }
    
    unsafe {
        // SAFETY: Checked for null above
        Ok(*ptr)
    }
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_my_function() {
        let result = my_function(42);
        assert_eq!(result, expected_value);
    }
    
    #[test]
    fn test_error_case() {
        let result = my_function(0);
        assert!(result.is_err());
    }
}
```

### Integration Tests

Create test scripts in `tools/testing/`:

```bash
#!/bin/bash
# test_my_feature.sh

set -e

echo "Testing my feature..."

# Start QEMU
make run &
QEMU_PID=$!

# Wait for boot
sleep 5

# Run tests
# ...

# Cleanup
kill $QEMU_PID
```

## Contributing

### Workflow

1. Create feature branch
2. Implement changes
3. Test thoroughly
4. Document changes
5. Submit for review

### Commit Messages

```
component: Brief description

Detailed explanation of what changed and why.

Fixes: #123
```

### Code Review Checklist

- [ ] Code compiles without warnings
- [ ] Tests pass
- [ ] Documentation updated
- [ ] Error handling correct
- [ ] No memory leaks
- [ ] Follows style guidelines
- [ ] Security considerations addressed

## Resources

### Documentation

- **Architecture docs**: `docs/architecture/`
- **User guide**: `docs/USER_GUIDE.md`
- **Troubleshooting**: `docs/TROUBLESHOOTING_GUIDE.md`

### External Resources

- [OSDev Wiki](https://wiki.osdev.org/)
- [Rust OS Development](https://os.phil-opp.com/)
- [Linux System Call Reference](https://man7.org/linux/man-pages/)
- [POSIX Specification](https://pubs.opengroup.org/onlinepubs/9699919799/)

## Adding New Device Drivers

### Step 1: Define Driver Structure

Create a new driver module in the appropriate category:

```rust
// kernel/src/drivers/mydevice/mod.rs
use crate::drivers::{Driver, Device, DriverError};
use crate::io::port::{inb, outb};
use crate::io::irq::register_irq_handler;
use crate::sync::Mutex;

// Device-specific constants
const MY_DEVICE_PORT: u16 = 0x1234;
const MY_DEVICE_IRQ: u8 = 5;

// Device state
static DEVICE_STATE: Mutex<Option<MyDeviceState>> = Mutex::new(None);

struct MyDeviceState {
    base_port: u16,
    // ... other state fields
}
```

### Step 2: Implement Driver Functions

```rust
/// Probe function - check if device is compatible
pub fn my_device_probe(device: &Device) -> bool {
    // Check device name, bus type, or other properties
    device.name == "my-device" && device.bus == BusType::Platform
}

/// Initialize function - set up device
pub fn my_device_init(device: &Device) -> Result<(), DriverError> {
    log::info!("Initializing my device driver");
    
    // 1. Validate device resources
    if device.io_base == 0 {
        return Err(DriverError::ResourceUnavailable);
    }
    
    // 2. Initialize hardware
    unsafe {
        outb(MY_DEVICE_PORT, 0x01);  // Enable device
    }
    
    // 3. Register IRQ handler if needed
    if let Some(irq) = device.irq {
        register_irq_handler(irq, my_device_irq_handler)
            .map_err(|_| DriverError::InitFailure)?;
    }
    
    // 4. Initialize driver state
    let state = MyDeviceState {
        base_port: device.io_base as u16,
    };
    
    let mut global_state = DEVICE_STATE.lock();
    *global_state = Some(state);
    
    log::info!("My device initialized successfully");
    Ok(())
}

/// Shutdown function - clean up device
pub fn my_device_shutdown(device: &Device) -> Result<(), DriverError> {
    log::info!("Shutting down my device");
    
    // 1. Unregister IRQ handler
    if let Some(irq) = device.irq {
        crate::io::irq::unregister_irq_handler(irq);
    }
    
    // 2. Disable hardware
    unsafe {
        outb(MY_DEVICE_PORT, 0x00);  // Disable device
    }
    
    // 3. Clear driver state
    let mut state = DEVICE_STATE.lock();
    *state = None;
    
    Ok(())
}

/// IRQ handler - handle device interrupts
fn my_device_irq_handler() {
    // Read device status
    let status = unsafe { inb(MY_DEVICE_PORT) };
    
    // Process interrupt
    if status & 0x01 != 0 {
        // Handle specific condition
        process_device_event();
    }
    
    // Clear interrupt
    unsafe {
        outb(MY_DEVICE_PORT + 1, 0xFF);
    }
}
```

### Step 3: Define Driver Constant

```rust
/// Driver registration constant
pub const MY_DEVICE_DRIVER: Driver = Driver {
    name: "my-device",
    probe: my_device_probe,
    init: my_device_init,
    shutdown: my_device_shutdown,
};
```

### Step 4: Register Driver

In `kernel/src/drivers/mod.rs`, add your driver to the registration function:

```rust
fn register_builtin_drivers() {
    log::info!("Registering built-in drivers");
    driver_register(crate::drivers::input::keyboard::KEYBOARD_DRIVER);
    driver_register(crate::drivers::serial::uart16550::SERIAL_DRIVER);
    driver_register(crate::drivers::block::virtio_blk::VIRTIO_BLK_DRIVER);
    driver_register(crate::drivers::mydevice::MY_DEVICE_DRIVER);  // Add here
}
```

### Step 5: Add Device to Bus Scan

In `kernel/src/io/devtree.rs`, add device detection:

```rust
pub fn scan_platform_bus() {
    log::info!("Scanning platform bus");
    
    // Detect my device
    if my_device_present() {
        let device = Device {
            name: "my-device",
            bus: BusType::Platform,
            io_base: 0x1234,
            irq: Some(5),
            irq_affinity: None,
            driver: None,
            state: DeviceState::Detected,
        };
        crate::drivers::device_register(device);
    }
}

fn my_device_present() -> bool {
    // Check if device exists
    unsafe {
        let id = inb(MY_DEVICE_PORT);
        id == EXPECTED_DEVICE_ID
    }
}
```

### Step 6: Add Public API

Provide functions for other kernel subsystems to use:

```rust
/// Public API for device operations
pub fn my_device_read() -> Option<u8> {
    let state = DEVICE_STATE.lock();
    if let Some(dev) = state.as_ref() {
        unsafe {
            Some(inb(dev.base_port))
        }
    } else {
        None
    }
}

pub fn my_device_write(value: u8) {
    let state = DEVICE_STATE.lock();
    if let Some(dev) = state.as_ref() {
        unsafe {
            outb(dev.base_port, value);
        }
    }
}
```

### Step 7: Add Syscalls (Optional)

If userland needs access, add syscalls in `kernel/src/sys/syscall.rs`:

```rust
pub fn sys_my_device_read() -> Option<u8> {
    crate::drivers::mydevice::my_device_read()
}

pub fn sys_my_device_write(value: u8) {
    crate::drivers::mydevice::my_device_write(value);
}
```

### Step 8: Test the Driver

Create a userland test program:

```rust
// kernel/userspace/my_device_test/src/main.rs
fn main() {
    println!("Testing my device driver...");
    
    // Write test
    my_device_write(0x42);
    
    // Read test
    if let Some(value) = my_device_read() {
        println!("Read value: 0x{:02X}", value);
    }
    
    println!("Test complete");
}
```

### Driver Development Best Practices

1. **Validate all inputs**: Check device resources before use
2. **Handle errors gracefully**: Return DriverError, don't panic
3. **Log important events**: Use log macros for debugging
4. **Minimize IRQ handler work**: Defer processing to task context
5. **Use proper synchronization**: Protect shared state with locks
6. **Follow lock ordering**: Prevent deadlocks in SMP
7. **Test thoroughly**: Create userland test programs
8. **Document behavior**: Add doc comments to public APIs

### Common Driver Patterns

**Buffered I/O:**
```rust
static BUFFER: Mutex<[u8; 256]> = Mutex::new([0; 256]);
static HEAD: Mutex<usize> = Mutex::new(0);
static TAIL: Mutex<usize> = Mutex::new(0);

fn irq_handler() {
    let data = read_from_device();
    
    let mut head = HEAD.lock();
    let tail = TAIL.lock();
    let mut buffer = BUFFER.lock();
    
    let next_head = (*head + 1) % 256;
    if next_head != *tail {
        buffer[*head] = data;
        *head = next_head;
    }
}

pub fn read() -> Option<u8> {
    let mut head = HEAD.lock();
    let mut tail = TAIL.lock();
    let buffer = BUFFER.lock();
    
    if *head == *tail {
        None
    } else {
        let data = buffer[*tail];
        *tail = (*tail + 1) % 256;
        Some(data)
    }
}
```

**DMA Setup (Future):**
```rust
fn setup_dma_transfer(buffer: &[u8]) -> Result<(), DriverError> {
    // Allocate DMA buffer
    let dma_buf = crate::mm::alloc_dma_buffer(buffer.len())?;
    
    // Copy data to DMA buffer
    dma_buf.copy_from_slice(buffer);
    
    // Program device with physical address
    let phys_addr = crate::mm::virt_to_phys(dma_buf.as_ptr() as usize)?;
    unsafe {
        outl(DEVICE_DMA_ADDR, phys_addr as u32);
        outl(DEVICE_DMA_LEN, buffer.len() as u32);
        outb(DEVICE_DMA_CTRL, DMA_START);
    }
    
    Ok(())
}
```

**MMIO Device:**
```rust
use crate::io::mmio::{mmio_read32, mmio_write32};

struct MmioDevice {
    base_addr: usize,
}

impl MmioDevice {
    fn read_register(&self, offset: usize) -> u32 {
        unsafe { mmio_read32(self.base_addr + offset) }
    }
    
    fn write_register(&self, offset: usize, value: u32) {
        unsafe { mmio_write32(self.base_addr + offset, value) }
    }
    
    fn init(&self) {
        // Reset device
        self.write_register(0x00, 0x01);
        
        // Configure device
        self.write_register(0x04, 0x12345678);
        
        // Enable device
        self.write_register(0x00, 0x03);
    }
}
```

### Debugging Drivers

**Serial Debug Output:**
```rust
use crate::drivers::serial::serial_println;

fn my_driver_function() {
    serial_println!("Driver: entering function");
    serial_println!("Driver: value = {}", value);
    serial_println!("Driver: exiting function");
}
```

**Log Macros:**
```rust
use log::{debug, info, warn, error};

fn my_driver_init() {
    info!("Initializing driver");
    debug!("Device base: 0x{:04X}", base);
    warn!("Device not responding");
    error!("Initialization failed");
}
```

**IRQ Debugging:**
```rust
fn irq_handler() {
    let cpu = crate::arch::cpu_id();
    log::trace!("IRQ on CPU {}", cpu);
    
    // Count interrupts
    static IRQ_COUNT: AtomicUsize = AtomicUsize::new(0);
    let count = IRQ_COUNT.fetch_add(1, Ordering::Relaxed);
    
    if count % 1000 == 0 {
        log::info!("Processed {} interrupts", count);
    }
}
```

## Appendix: Quick Reference

### System Call Template

```rust
pub fn sys_my_call(arg1: usize, arg2: usize) -> Result<usize, Errno> {
    // Validate
    if arg1 == 0 {
        return Err(EINVAL);
    }
    
    // Get context
    let task = current_task();
    
    // Check permissions
    if !has_permission(task) {
        return Err(EPERM);
    }
    
    // Perform operation
    let result = do_work(arg1, arg2)?;
    
    // Log
    log::debug!("my_call({}, {}) = {}", arg1, arg2, result);
    
    Ok(result)
}
```

### /proc File Template

```rust
fn proc_my_file(pid: Pid, buf: &mut [u8]) -> Result<usize, Errno> {
    let task = find_task(pid).ok_or(ESRCH)?;
    
    let seq = task.seqlock.read_begin();
    let data = read_data(task);
    if !task.seqlock.read_retry(seq) {
        return Err(EAGAIN);
    }
    
    let output = format!("Data: {}\n", data);
    let len = output.len().min(buf.len());
    buf[..len].copy_from_slice(&output.as_bytes()[..len]);
    
    Ok(len)
}
```

### Mellobox Utility Template

```rust
pub fn main(args: &[&str]) -> Result<i32> {
    let mut opts = Args::new(args);
    
    while let Some(opt) = opts.next_opt()? {
        match opt {
            "-h" => { print_usage(); return Ok(0); }
            _ => return Err(Error::Usage("Unknown option")),
        }
    }
    
    let files = opts.remaining();
    for file in files {
        process_file(file)?;
    }
    
    Ok(0)
}
```
