# MelloOS Architecture

This document provides detailed architecture information about MelloOS kernel components.

## System Overview

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
│  │           Task Scheduler (sched/)                    │  │
│  │  - Round-Robin algorithm                             │  │
│  │  - Context switching (< 1μs)                         │  │
│  │  - Timer interrupts (100 Hz)                         │  │
│  │  - Task Control Blocks                               │  │
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
│                  Hardware Abstraction                       │
│  - x86_64 CPU (registers, instructions)                    │
│  - PIT (Programmable Interval Timer)                       │
│  - PIC (Programmable Interrupt Controller)                 │
│  - Serial Port (COM1)                                      │
│  - Framebuffer (UEFI GOP)                                  │
└─────────────────────────────────────────────────────────────┘
```

## Memory Management Architecture

### 1. Physical Memory Manager (PMM)

**Location:** `kernel/src/mm/pmm.rs`

**Features:**
- Bitmap-based frame allocator (1 bit per 4KB frame)
- O(n) allocation with last_alloc optimization → O(1) average case
- Automatic memory zeroing for security
- Contiguous frame allocation for DMA devices
- Memory statistics (total/free memory in MB)

**API:**
```rust
pub fn alloc_frame() -> Option<PhysAddr>;
pub fn free_frame(phys_addr: PhysAddr);
pub fn alloc_contiguous(count: usize, align: usize) -> Option<PhysAddr>;
```

### 2. Virtual Memory (Paging)

**Location:** `kernel/src/mm/paging.rs`

**Features:**
- 4-level page tables: PML4 → PDPT → PD → PT
- Per-section memory permissions:
  - `.text`: Read + Execute (RX)
  - `.rodata`: Read only (R)
  - `.data/.bss`: Read + Write + No Execute (RW+NX)
- Guard pages for overflow detection
- TLB invalidation with `invlpg` instruction
- Virtual address translation

**Page Table Flags:**
```rust
PRESENT     = 1 << 0   // Page is in memory
WRITABLE    = 1 << 1   // Page is writable
USER        = 1 << 2   // Accessible from user mode
NO_EXECUTE  = 1 << 63  // Page is not executable (NXE bit)
GLOBAL      = 1 << 8   // Not flushed from TLB
```

**API:**
```rust
pub fn map_page(virt: VirtAddr, phys: PhysAddr, flags: u64) -> Result<()>;
pub fn unmap_page(virt: VirtAddr) -> Result<()>;
pub fn translate(virt: VirtAddr) -> Option<PhysAddr>;
```

### 3. Kernel Heap Allocator

**Location:** `kernel/src/mm/allocator.rs`

**Features:**
- Buddy System algorithm for efficient allocation
- Block sizes: 64B, 128B, 256B, ..., 1MB (15 orders)
- O(log n) allocation and deallocation
- Automatic block splitting and coalescing
- Thread-safe with `spin::Mutex`
- 16MB kernel heap at `0xFFFF_A000_0000_0000`

**API:**
```rust
pub fn kmalloc(size: usize) -> *mut u8;
pub fn kfree(ptr: *mut u8, size: usize);
pub fn allocated_bytes() -> usize;
```

## Task Scheduler Architecture

### 1. Scheduler Core

**Location:** `kernel/src/sched/mod.rs`

**Data Structures:**
```rust
// Circular queue for O(1) operations
struct TaskQueue {
    tasks: [TaskId; MAX_TASKS],  // Ring buffer
    head: usize,
    tail: usize,
    count: usize,
}

// Scheduler state (single mutex for atomicity)
struct SchedState {
    runqueue: TaskQueue,         // Ready tasks
    current: Option<TaskId>,     // Currently running task
    next_tid: usize,             // Next task ID to assign
}

// Task table (heap-allocated tasks)
static TASK_TABLE: Mutex<[TaskPtr; MAX_TASKS]>;
```

**Round-Robin Algorithm:**
```
1. Timer interrupt fires (every 10ms)
2. Current task moved to back of runqueue
3. Next task popped from front of runqueue
4. Task states updated (Running → Ready, Ready → Running)
5. Context switch performed
6. Next task resumes execution
```

**API:**
```rust
pub fn init_scheduler();
pub fn spawn_task(name: &'static str, entry: fn() -> !) -> Result<TaskId>;
pub fn tick();  // Called by timer interrupt
```

### 2. Context Switching

**Location:** `kernel/src/sched/context.rs`

**CPU Context Structure:**
```rust
#[repr(C)]
pub struct CpuContext {
    r15: u64,  // Callee-saved registers
    r14: u64,  // (System V ABI)
    r13: u64,
    r12: u64,
    rbp: u64,
    rbx: u64,
    rsp: u64,  // Stack pointer
}
```

**Context Switch Flow:**
```asm
context_switch:
    ; Save current task's registers
    push rbx, rbp, r12, r13, r14, r15
    mov [rdi + 48], rsp          ; Save RSP to current.rsp
    
    ; Load next task's registers
    mov rsp, [rsi + 48]          ; Load RSP from next.rsp
    pop r15, r14, r13, r12, rbp, rbx
    
    ; Return to next task
    ret                          ; Jump to return address on stack
```

**Performance:**
- Context switch time: < 1 microsecond
- Register save/restore: ~50 CPU cycles
- Total overhead: ~1% at 100 Hz

### 3. Task Management

**Location:** `kernel/src/sched/task.rs`

**Task Control Block:**
```rust
pub struct Task {
    id: TaskId,              // Unique identifier
    name: &'static str,      // Human-readable name
    stack: *mut u8,          // Stack base address
    stack_size: usize,       // 8KB per task
    state: TaskState,        // Ready, Running, or Sleeping
    context: CpuContext,     // Saved CPU state
}
```

**Task States:**
```
     spawn()
        ↓
    ┌─────────┐
    │  Ready  │←─────┐
    └────┬────┘      │
         │           │
         │ schedule()│ preempt
         ▼           │
    ┌─────────┐     │
    │ Running │─────┘
    └─────────┘
```

**Stack Layout (8KB per task):**
```
High Address
┌─────────────────┐
│  entry_point    │ ← Pushed by Task::new
├─────────────────┤
│ entry_trampoline│ ← Return address
├─────────────────┤
│  R15 - RBX      │ ← Initial register values (zeros)
├─────────────────┤ ← Initial RSP
│                 │
│   Stack Space   │ 8KB (grows downward)
│                 │
└─────────────────┘
Low Address
```

### 4. Timer Interrupt System

**Location:** `kernel/src/sched/timer.rs`

**Components:**

1. **PIT (Programmable Interval Timer)**
   - Base frequency: 1,193,182 Hz
   - Configured for 100 Hz (10ms intervals)
   - Mode 3: Square wave generator

2. **PIC (Programmable Interrupt Controller)**
   - Master PIC: IRQ 0-7 → Vectors 32-39
   - Slave PIC: IRQ 8-15 → Vectors 40-47
   - Timer (IRQ0) → Vector 32

3. **IDT (Interrupt Descriptor Table)**
   - 256 entries (0-255)
   - Vectors 0-31: CPU exceptions
   - Vector 32: Timer interrupt handler
   - Vectors 33-255: Available for future use

**Interrupt Flow:**
```
1. PIT fires interrupt (IRQ0)
   ↓
2. CPU automatically:
   - Disables interrupts (IF=0)
   - Saves SS, RSP, RFLAGS, CS, RIP to stack
   - Jumps to IDT entry 32
   ↓
3. Timer handler:
   - Sends EOI to PIC
   - Calls scheduler tick()
   ↓
4. Scheduler:
   - Selects next task (Round-Robin)
   - Performs context switch
   ↓
5. Next task resumes
   - Eventually executes iretq
   - Restores RIP, CS, RFLAGS, RSP, SS
   - Re-enables interrupts (IF=1)
```

## Security Architecture

### Memory Protection

1. **NX Bit (No Execute)**
   - Enabled via EFER MSR (bit 11)
   - Data pages marked with NO_EXECUTE flag
   - Prevents code execution in data regions
   - Mitigates buffer overflow attacks

2. **Write Protection**
   - Enabled via CR0 register (bit 16)
   - Kernel respects page-level write permissions
   - Read-only pages cannot be written
   - Protects code and constant data

3. **Memory Zeroing**
   - All allocated frames zeroed before use
   - Prevents information leakage
   - Ensures clean state for new allocations

4. **Guard Pages**
   - Unmapped pages around stack and heap
   - Trigger page faults on overflow/underflow
   - Early detection of memory corruption

5. **Stack Isolation**
   - Each task has separate 8KB stack
   - Stacks allocated from kernel heap
   - No shared stack space between tasks
   - Prevents cross-task stack corruption

## Memory Layout

```
Virtual Address Space:
0x0000_0000_0000_0000 - 0x0000_7FFF_FFFF_FFFF : User space (not used yet)
0xFFFF_8000_0000_0000 - 0xFFFF_9FFF_FFFF_FFFF : HHDM (direct physical mapping)
0xFFFF_A000_0000_0000 - 0xFFFF_A000_00FF_FFFF : Kernel heap (16MB)
0xFFFF_FFFF_8000_0000 - 0xFFFF_FFFF_FFFF_FFFF : Kernel code/data

Task Stacks:
- Each task has an 8KB stack allocated from kernel heap
- Stacks grow downward from high addresses
- Stack pointer (RSP) saved in Task Control Block during context switch
```

## Page Table Flags

- **.text section**: `PRESENT | GLOBAL` (Read + Execute)
- **.rodata section**: `PRESENT | NO_EXECUTE | GLOBAL` (Read only)
- **.data/.bss section**: `PRESENT | WRITABLE | NO_EXECUTE | GLOBAL` (Read + Write)
- **Heap pages**: `PRESENT | WRITABLE | NO_EXECUTE` (Read + Write)

## Buddy Allocator Orders

```
Order 0:  64 bytes   (2^6)
Order 1:  128 bytes  (2^7)
Order 2:  256 bytes  (2^8)
...
Order 14: 1 MB       (2^20)
```

## Interrupt Vector Mapping

```
CPU Exceptions:      0-31   (Reserved by CPU)
Timer (IRQ0):        32     (0x20) - PIT interrupt
Keyboard (IRQ1):     33     (0x21) - Not yet implemented
Other IRQs:          34-47  (0x22-0x2F) - Available for future use
```

## Context Switch Mechanism

1. **Timer Interrupt Fires** (every 10ms at 100 Hz)
2. **CPU Saves State** (RIP, CS, RFLAGS, RSP, SS automatically)
3. **Handler Sends EOI** to PIC (allows next interrupt)
4. **Scheduler Selects Next Task** (Round-Robin from runqueue)
5. **Context Switch**:
   - Save current task's callee-saved registers (RBX, RBP, R12-R15)
   - Save current RSP to current task's context
   - Load next task's RSP from next task's context
   - Restore next task's callee-saved registers
   - Return to next task (ret instruction)
6. **Next Task Resumes** from where it was interrupted

## Performance Characteristics

**Memory Management:**
- **Frame Allocation**: O(n) worst case, O(1) average with last_alloc optimization
- **Heap Allocation**: O(log n) for buddy system operations
- **Page Mapping**: O(1) with existing page tables, O(4) when creating new tables
- **TLB Invalidation**: Single page invalidation with `invlpg`

**Task Scheduler:**
- **Context Switch**: < 1 microsecond (assembly-optimized)
- **Task Selection**: O(1) with circular queue
- **Timer Frequency**: 100 Hz (10ms time slices)
- **Scheduling Overhead**: ~1% CPU time at 100 Hz
