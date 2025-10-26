# Stack Overflow Prevention - Best Practices

## Overview

This document covers best practices for preventing stack overflow issues in MelloOS kernel development, based on real issues encountered during development.

## Background

Stack overflow occurs when a function tries to use more stack space than is available. In kernel development, this is particularly dangerous because:

1. **Kernel stacks are limited** - Typically 16KB-64KB per task
2. **No guard pages in early boot** - Stack overflow can silently corrupt memory
3. **Hard to debug** - Often manifests as mysterious hangs or crashes
4. **SMP complications** - Each CPU has its own stack

## Real-World Case Study: IPC Port Creation

### The Problem

During Phase 4 implementation, the system hung during IPC initialization:

```
[IPC] create_port: Calling Port::new(0)
<system hangs>
```

### Root Cause

The `Port` struct was too large for the kernel stack:

```rust
struct MessageQueue {
    messages: [Message; 16],  // 16 * 4096 bytes = 65KB!
    head: usize,
    tail: usize,
    count: usize,
}

struct Port {
    id: usize,
    queue: MessageQueue,      // 65KB
    blocked_tasks: TaskQueue,
    lock: Mutex<()>,
}

// This caused stack overflow:
let port = Port::new(port_id);  // Tries to allocate 65KB+ on stack!
```

### The Solution

Reduce the size to fit within stack limits:

```rust
// Reduced from 16 to 4 messages
const MAX_MESSAGES_PER_PORT: usize = 4;  // 4 * 4096 = 16KB

// Now Port struct is ~16KB, which fits on stack
```

## Stack Size Guidelines

### Kernel Stack Sizes

- **x86_64 kernel stack**: Typically 16KB-32KB
- **Per-CPU stacks**: 16KB each
- **Interrupt stacks (IST)**: 4KB-8KB each
- **User stacks**: 8MB (in user space)

### Safe Allocation Limits

**Rule of thumb:** Keep stack allocations under 4KB per function

```rust
// ✅ SAFE: Small stack allocation
fn process_data() {
    let buffer: [u8; 1024] = [0; 1024];  // 1KB - OK
    // ...
}

// ⚠️ RISKY: Medium stack allocation
fn process_large_data() {
    let buffer: [u8; 4096] = [0; 4096];  // 4KB - Borderline
    // ...
}

// ❌ DANGEROUS: Large stack allocation
fn process_huge_data() {
    let buffer: [u8; 16384] = [0; 16384];  // 16KB - TOO LARGE!
    // ...
}
```

## Prevention Strategies

### 1. Use Heap Allocation for Large Data

```rust
// ❌ WRONG: Large array on stack
fn bad_example() {
    let data: [u8; 65536] = [0; 65536];  // 64KB on stack!
}

// ✅ CORRECT: Use Box for heap allocation
fn good_example() {
    let data: Box<[u8; 65536]> = Box::new([0; 65536]);  // On heap
}

// ✅ CORRECT: Use Vec for dynamic allocation
fn better_example() {
    let mut data = vec![0u8; 65536];  // On heap
}
```

### 2. Use Box::new_uninit() for Large Structs

When you need to create large structs, use `Box::new_uninit()` to avoid stack allocation:

```rust
// ❌ WRONG: Creates struct on stack first
fn create_port_bad(id: usize) -> Box<Port> {
    Box::new(Port::new(id))  // Port created on stack, then moved to heap
}

// ✅ CORRECT: Allocate uninitialized memory on heap first
fn create_port_good(id: usize) -> Box<Port> {
    let mut uninit = Box::<Port>::new_uninit();
    unsafe {
        uninit.as_mut_ptr().write(Port::new(id));
        uninit.assume_init()
    }
}
```

### 3. Reduce Array Sizes

```rust
// ❌ WRONG: Unnecessarily large array
const MAX_MESSAGES: usize = 256;  // 256 * 4KB = 1MB!

struct MessageQueue {
    messages: [Message; MAX_MESSAGES],
}

// ✅ CORRECT: Reasonable size
const MAX_MESSAGES: usize = 4;  // 4 * 4KB = 16KB

struct MessageQueue {
    messages: [Message; MAX_MESSAGES],
}
```

### 4. Use References Instead of Copying

```rust
// ❌ WRONG: Copies large struct on stack
fn process_data(data: LargeStruct) {  // Copies entire struct
    // ...
}

// ✅ CORRECT: Use reference
fn process_data(data: &LargeStruct) {  // Just a pointer
    // ...
}

// ✅ CORRECT: Use mutable reference if needed
fn process_data_mut(data: &mut LargeStruct) {
    // ...
}
```

### 5. Split Large Structs

```rust
// ❌ WRONG: Monolithic large struct
struct HugeStruct {
    buffer1: [u8; 16384],
    buffer2: [u8; 16384],
    buffer3: [u8; 16384],
}

// ✅ CORRECT: Split into smaller pieces
struct SmallStruct {
    buffer1: Box<[u8; 16384]>,
    buffer2: Box<[u8; 16384]>,
    buffer3: Box<[u8; 16384]>,
}
```

## Detection and Debugging

### 1. Check Struct Sizes at Compile Time

```rust
use core::mem::size_of;

// Add compile-time size checks
const _: () = {
    assert!(size_of::<Port>() < 16384, "Port struct too large for stack!");
};

// Or use static_assertions crate
use static_assertions::const_assert;
const_assert!(size_of::<Port>() < 16384);
```

### 2. Add Debug Logging

```rust
fn create_port(id: usize) -> Result<Box<Port>, Error> {
    serial_println!("[DEBUG] Port size: {} bytes", size_of::<Port>());
    serial_println!("[DEBUG] Creating port {}...", id);
    
    let port = create_port_safely(id)?;
    
    serial_println!("[DEBUG] Port {} created successfully", id);
    Ok(port)
}
```

### 3. Monitor Stack Usage

```rust
// Check remaining stack space (x86_64)
fn check_stack_usage() {
    let stack_var: u64 = 0;
    let stack_ptr = &stack_var as *const u64 as usize;
    
    serial_println!("[DEBUG] Current stack pointer: 0x{:x}", stack_ptr);
    
    // Compare with known stack base to estimate usage
}
```

### 4. Use Compiler Warnings

Enable stack size warnings in your build configuration:

```toml
# In Cargo.toml or .cargo/config.toml
[build]
rustflags = [
    "-C", "inline-threshold=0",  # Disable inlining to see real stack usage
    "-Z", "stack-size-section",  # Add stack size information
]
```

## Common Patterns and Solutions

### Pattern 1: Large Message Buffers

```rust
// ❌ PROBLEM
struct Message {
    data: [u8; 4096],  // 4KB per message
}

struct MessageQueue {
    messages: [Message; 16],  // 64KB total!
}

// ✅ SOLUTION 1: Reduce queue size
const MAX_MESSAGES: usize = 4;  // 16KB total

// ✅ SOLUTION 2: Use heap allocation
struct MessageQueue {
    messages: Box<[Message; 16]>,
}

// ✅ SOLUTION 3: Use Vec
struct MessageQueue {
    messages: Vec<Message>,
}
```

### Pattern 2: Nested Large Structs

```rust
// ❌ PROBLEM
struct Inner {
    data: [u8; 8192],
}

struct Outer {
    inner1: Inner,  // 8KB
    inner2: Inner,  // 8KB
    inner3: Inner,  // 8KB
}  // Total: 24KB!

// ✅ SOLUTION: Box the inner structs
struct Outer {
    inner1: Box<Inner>,
    inner2: Box<Inner>,
    inner3: Box<Inner>,
}  // Total: 24 bytes (3 pointers)
```

### Pattern 3: Temporary Buffers

```rust
// ❌ PROBLEM
fn process_file() {
    let buffer: [u8; 65536] = [0; 65536];  // 64KB on stack!
    read_file(&mut buffer);
}

// ✅ SOLUTION: Use heap allocation
fn process_file() {
    let mut buffer = vec![0u8; 65536];  // On heap
    read_file(&mut buffer);
}
```

## Architecture-Specific Considerations

### x86_64

- Default kernel stack: 16KB
- IST stacks: 4KB each (3 stacks)
- Guard pages: Not available in early boot

```rust
// In kernel/src/arch/x86_64/gdt.rs
const KERNEL_STACK_SIZE: usize = 16384;  // 16KB
const IST_STACK_SIZE: usize = 4096;      // 4KB
```

### SMP Considerations

Each CPU has its own stack:

```rust
// Per-CPU stacks
static AP_STACKS: [[u8; 16384]; MAX_CPUS] = [[0; 16384]; MAX_CPUS];

// Be extra careful with stack usage in SMP code
fn smp_init() {
    // This runs on AP stack, which is only 16KB
    // Avoid large stack allocations here!
}
```

## Testing for Stack Issues

### 1. Stress Testing

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_stack_usage() {
        // Create many nested calls to test stack depth
        fn recursive(depth: usize) {
            if depth > 0 {
                let _buffer: [u8; 1024] = [0; 1024];
                recursive(depth - 1);
            }
        }
        
        recursive(10);  // Should not overflow
    }
}
```

### 2. Size Assertions

```rust
#[cfg(test)]
mod tests {
    use core::mem::size_of;
    
    #[test]
    fn test_struct_sizes() {
        assert!(size_of::<Port>() < 16384, "Port too large");
        assert!(size_of::<MessageQueue>() < 8192, "MessageQueue too large");
        assert!(size_of::<Message>() == 4096, "Message size changed");
    }
}
```

## Summary

**Key Takeaways:**

1. **Keep stack allocations under 4KB** per function
2. **Use heap allocation** for large data structures
3. **Use `Box::new_uninit()`** for large structs
4. **Check struct sizes** at compile time
5. **Monitor stack usage** during development
6. **Test with stress tests** to catch issues early
7. **Be extra careful in SMP code** - each CPU has limited stack

**Warning Signs:**

- Mysterious hangs during initialization
- System crashes with no clear error
- Functions with large local variables
- Nested structs with large arrays
- Deep recursion

**When in doubt:** If a struct is larger than 4KB, allocate it on the heap!

## Related Documents

- [SMP Boot and Deadlock Prevention](smp-boot-and-deadlock-prevention.md)
- [Memory Management Best Practices](../architecture/memory-management.md)
- [Debugging Guide](../troubleshooting/README.md)
