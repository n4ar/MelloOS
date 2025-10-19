# MelloOS API Guide

This document provides comprehensive API usage examples for MelloOS kernel development.

## Memory Management APIs

### Allocating Memory

```rust
use crate::mm::allocator::{kmalloc, kfree};

// Allocate 1KB of memory
let ptr = kmalloc(1024);

if !ptr.is_null() {
    // Memory is automatically zeroed
    unsafe {
        // Use the memory
        *ptr = 0x42;
        *(ptr.offset(1)) = 0x43;
    }
    
    // Free when done (must pass same size)
    kfree(ptr, 1024);
} else {
    // Out of memory - handle error
    serial_println!("Failed to allocate memory");
}
```

**Important Notes:**
- ✅ Always check if `kmalloc()` returns null
- ✅ Always call `kfree()` with the same size used in `kmalloc()`
- ✅ Memory is automatically zeroed for security
- ✅ All allocations are thread-safe (Mutex protected)
- ❌ Don't use after free
- ❌ Don't double free

### Physical Memory

```rust
use crate::mm::pmm::PhysicalMemoryManager;

// Allocate a 4KB physical frame
let frame = pmm.alloc_frame();

if let Some(phys_addr) = frame {
    // Use the frame
    serial_println!("Allocated frame at 0x{:x}", phys_addr);
    
    // Free when done
    pmm.free_frame(phys_addr);
}
```

### Virtual Memory

```rust
use crate::mm::paging::{PageMapper, PageTableFlags};

let mut mapper = PageMapper::new();

// Map a virtual page to a physical frame
let virt_addr = 0xFFFF_B000_0000_0000;
let phys_addr = pmm.alloc_frame().unwrap();

mapper.map_page(
    virt_addr,
    phys_addr,
    PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    &mut pmm,
).expect("Failed to map page");

// Translate virtual to physical
if let Some(phys) = mapper.translate(virt_addr) {
    serial_println!("Virtual 0x{:x} → Physical 0x{:x}", virt_addr, phys);
}

// Unmap when done
mapper.unmap_page(virt_addr).expect("Failed to unmap");
```

## Task Scheduler APIs

### Spawning Tasks

```rust
use crate::sched::spawn_task;

// Define a task function (must never return)
fn my_task() -> ! {
    loop {
        serial_println!("Task is running!");
        
        // Do some work
        for _ in 0..1_000_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

// Spawn the task
match spawn_task("my_task", my_task) {
    Ok(task_id) => {
        serial_println!("Spawned task with ID: {}", task_id);
    }
    Err(e) => {
        serial_println!("Failed to spawn task: {:?}", e);
    }
}
```

**Task Requirements:**
- ✅ Must have signature `fn() -> !` (never returns)
- ✅ Must contain an infinite loop
- ✅ Can use up to 8KB of stack
- ✅ Can call `kmalloc`/`kfree` for dynamic memory
- ❌ Don't return from the function
- ❌ Don't use more than 8KB stack (no deep recursion)

### Initializing the Scheduler

```rust
use crate::sched::{init_scheduler, spawn_task};
use crate::sched::timer::init_timer;

// 1. Initialize scheduler (creates idle task)
init_scheduler();

// 2. Spawn your tasks
spawn_task("task_a", task_a).expect("Failed to spawn task_a");
spawn_task("task_b", task_b).expect("Failed to spawn task_b");

// 3. Initialize timer at 100 Hz
unsafe {
    init_timer(100);
}

// 4. Enable interrupts
unsafe {
    core::arch::asm!("sti");
}

// 5. Idle loop (scheduler will preempt this)
loop {
    unsafe { core::arch::asm!("hlt"); }
}
```

## Logging APIs

### Memory Management Logging

```rust
use crate::{mm_log, mm_info, mm_error, mm_test_ok};

mm_log!("Initializing subsystem...");
mm_info!("Total memory: {} MB", total_mb);
mm_error!("Out of memory");
mm_test_ok!("Test passed");

// Format addresses in hexadecimal
let addr = 0x1000;
mm_log!("Allocated frame at 0x{:x}", addr);

// Format sizes with appropriate units
use crate::mm::log::format_size;
let (value, unit) = format_size(16 * 1024 * 1024);
mm_log!("Heap size: {} {}", value, unit);  // "Heap size: 16 MB"
```

### Scheduler Logging

```rust
use crate::{sched_log, sched_info, sched_warn, sched_error};

sched_log!("Context switch to task {}", task_id);
sched_info!("Spawned task: {}", name);
sched_warn!("Runqueue is empty");
sched_error!("Failed to allocate stack");
```

### Serial Output

```rust
use crate::{serial_print, serial_println};

serial_print!("Hello, ");
serial_println!("world!");
serial_println!("Value: {}", 42);
```

## Adding New Features

### 1. Create a New Module

```rust
// kernel/src/mymodule.rs
pub fn my_function() {
    serial_println!("Hello from my module!");
}
```

```rust
// kernel/src/main.rs
mod mymodule;

fn _start() -> ! {
    // ...
    mymodule::my_function();
    // ...
}
```

### 2. Add a New Task

```rust
fn my_new_task() -> ! {
    loop {
        // Your task logic here
        serial_println!("My task is running");
        
        // Yield CPU time
        for _ in 0..1_000_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

// In main.rs
spawn_task("my_new_task", my_new_task).expect("Failed to spawn task");
```

### 3. Add a New Interrupt Handler

```rust
// In timer.rs or new interrupt module
extern "C" fn my_interrupt_handler() {
    // Handle the interrupt
    serial_println!("Interrupt received!");
    
    // Send EOI if needed
    unsafe {
        send_eoi();
    }
}

// Register in IDT
unsafe {
    IDT.entries[33].set_handler(
        my_interrupt_handler as usize,
        code_selector
    );
}
```

## Debugging Tips

### Serial Console Debugging

```rust
// Add debug output anywhere in the kernel
serial_println!("[DEBUG] Variable value: {}", value);
serial_println!("[DEBUG] Address: 0x{:x}", addr);
serial_println!("[DEBUG] Entering function: {}", function_name);
```

### QEMU Monitor

Start QEMU with monitor access:
```bash
qemu-system-x86_64 -monitor stdio -cdrom mellos.iso ...
```

Useful monitor commands:
```
info registers    # Show CPU registers
info mem          # Show memory mappings
info tlb          # Show TLB entries
info pic          # Show PIC state
info irq          # Show interrupt statistics
x /10x 0x1000     # Examine memory at address
```

### Memory Debugging

```rust
// Check memory statistics
let total_mb = pmm.total_memory_mb();
let free_mb = pmm.free_memory_mb();
serial_println!("Memory: {} MB total, {} MB free", total_mb, free_mb);

// Check heap usage
let allocated = allocator::allocated_bytes();
serial_println!("Heap allocated: {} bytes", allocated);

// Validate pointers
if ptr.is_null() {
    serial_println!("ERROR: Null pointer!");
}

// Check alignment
if addr % 4096 != 0 {
    serial_println!("ERROR: Address not page-aligned!");
}
```

### Scheduler Debugging

```rust
// Check task count
let sched = SCHED.lock();
serial_println!("Runqueue length: {}", sched.runqueue.len());

// Check current task
if let Some(id) = sched.current {
    serial_println!("Current task: {}", id);
}

// Check context switch count
let switches = SWITCH_COUNT.load(Ordering::Relaxed);
serial_println!("Total context switches: {}", switches);
```

### Build Verification

```bash
# Run automated build verification
./tools/verify_build.sh

# Check kernel binary
file kernel/target/x86_64-unknown-none/release/mellos-kernel

# Check ISO structure
xorriso -indev mellos.iso -find

# Disassemble kernel
objdump -d kernel/target/x86_64-unknown-none/release/mellos-kernel | less
```

## Common Issues and Solutions

### Issue: Kernel Hangs After `sti`

**Cause:** IDT not properly initialized or timer not configured

**Solution:**
```rust
// Ensure proper initialization order
init_idt();           // First
remap_pic();          // Second
init_pit_timer(100);  // Third
core::arch::asm!("sti");  // Finally
```

### Issue: Triple Fault / Reboot Loop

**Cause:** Stack overflow or invalid memory access

**Solution:**
```rust
// Add stack validation
if task.context.rsp == 0 {
    panic!("Task has null RSP!");
}

// Check stack bounds
let stack_bottom = task.stack as u64;
let stack_top = stack_bottom + 8192;
if task.context.rsp < stack_bottom || task.context.rsp >= stack_top {
    panic!("RSP outside stack bounds!");
}
```

### Issue: Out of Memory

**Cause:** Too many allocations or memory leak

**Solution:**
```rust
// Check available memory
let free_mb = pmm.free_memory_mb();
if free_mb < 10 {
    serial_println!("WARNING: Low memory! {} MB free", free_mb);
}

// Always free allocated memory
let ptr = kmalloc(1024);
// ... use ptr ...
kfree(ptr, 1024);  // Don't forget!
```

### Issue: Tasks Not Switching

**Cause:** Timer not firing or runqueue empty

**Solution:**
```rust
// Check timer ticks
let ticks = get_tick_count();
serial_println!("Timer ticks: {}", ticks);

// Check runqueue
let sched = SCHED.lock();
if sched.runqueue.is_empty() {
    serial_println!("WARNING: Runqueue is empty!");
}
```
