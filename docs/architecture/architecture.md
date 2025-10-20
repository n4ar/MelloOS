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
│  │           System Call Interface (sys/)               │  │
│  │  - Syscall dispatcher (int 0x80)                     │  │
│  │  - 5 syscalls: write, exit, sleep, ipc_send/recv    │  │
│  │  - Kernel metrics collection                         │  │
│  │  - SMP-safe with per-object locking                  │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │           IPC Subsystem (sys/ipc.rs)                 │  │
│  │  - Port-based message passing                        │  │
│  │  - 256 ports with 16-message queues                  │  │
│  │  - Blocking receive with FIFO wake policy            │  │
│  │  - Cross-core IPC with reschedule IPIs               │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │           SMP Multi-Core Scheduler (sched/)          │  │
│  │  - Per-core runqueues with load balancing            │  │
│  │  - Priority-based scheduling (High/Normal/Low)       │  │
│  │  - Sleep/wake mechanism                              │  │
│  │  - Context switching (< 1μs)                         │  │
│  │  - Per-core APIC timers (100 Hz)                     │  │
│  │  - Inter-processor interrupts (IPIs)                 │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │        Memory Management (mm/)                       │  │
│  │  ┌────────────┐ ┌──────────┐ ┌──────────────────┐   │  │
│  │  │    PMM     │ │  Paging  │ │  Heap Allocator  │   │  │
│  │  │  (Bitmap)  │ │ (4-level)│ │ (Buddy System)   │   │  │
│  │  └────────────┘ └──────────┘ └──────────────────┘   │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │           SMP Infrastructure (arch/x86_64/smp/)      │  │
│  │  ┌────────────┐ ┌──────────┐ ┌──────────────────┐   │  │
│  │  │ ACPI/MADT  │ │ Per-CPU  │ │ Synchronization  │   │  │
│  │  │  Parser    │ │   Data   │ │   (SpinLocks)    │   │  │
│  │  └────────────┘ └──────────┘ └──────────────────┘   │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  Hardware Abstraction                       │
│  - x86_64 Multi-Core CPUs (BSP + APs)                      │
│  - Local APIC (per-core timers, IPIs)                      │
│  - I/O APIC (external interrupt routing)                   │
│  - PIT (Programmable Interval Timer)                       │
│  - Serial Port (COM1)                                      │
│  - Framebuffer (UEFI GOP)                                  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  Userland Processes                         │
│  - Init process (PID 1)                                    │
│  - Syscall wrappers for kernel services                    │
│  - Tasks distributed across multiple CPU cores             │
└─────────────────────────────────────────────────────────────┘
```

## SMP (Symmetric Multi-Processing) Architecture

MelloOS supports symmetric multi-processing with up to 8 CPU cores. The SMP implementation provides:

- **CPU Discovery**: ACPI MADT parsing to detect available cores
- **AP Bootstrap**: Bringing Application Processors online via INIT/SIPI sequence
- **Per-Core Data**: Isolated data structures for each CPU core
- **Load Balancing**: Automatic task distribution across cores
- **Synchronization**: SpinLocks and atomic operations for thread safety
- **Inter-Processor Communication**: IPIs for cross-core coordination

### SMP Boot Sequence

```
BSP (Bootstrap Processor):
1. Parse ACPI MADT table → Discover CPU cores
2. Initialize BSP Local APIC
3. Setup AP trampoline code at 0x8000
4. For each AP:
   - Send INIT IPI
   - Send SIPI with trampoline address
   - Wait for AP to signal online
5. Initialize per-core timers
6. Start multi-core scheduler

APs (Application Processors):
1. Wake up in real mode at trampoline
2. Transition: Real → Protected → Long mode
3. Initialize per-CPU data structures
4. Configure Local APIC and timer
5. Signal BSP that AP is online
6. Enter scheduler loop
```

### Multi-Core Task Distribution

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│   Core 0    │  │   Core 1    │  │   Core 2    │  │   Core 3    │
│    (BSP)    │  │    (AP)     │  │    (AP)     │  │    (AP)     │
├─────────────┤  ├─────────────┤  ├─────────────┤  ├─────────────┤
│ Runqueue    │  │ Runqueue    │  │ Runqueue    │  │ Runqueue    │
│ [Task A]    │  │ [Task C]    │  │ [Task B]    │  │ [Task D]    │
│ [Idle]      │  │ [Idle]      │  │ [Idle]      │  │ [Idle]      │
├─────────────┤  ├─────────────┤  ├─────────────┤  ├─────────────┤
│ LAPIC Timer │  │ LAPIC Timer │  │ LAPIC Timer │  │ LAPIC Timer │
│ 100 Hz      │  │ 100 Hz      │  │ 100 Hz      │  │ 100 Hz      │
└─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘
       │                │                │                │
       └────────────────┼────────────────┼────────────────┘
                        │                │
              ┌─────────┴────────────────┴─────────┐
              │     Load Balancer (100ms)          │
              │  - Migrate tasks between cores     │
              │  - Send RESCHEDULE_IPI to target   │
              └────────────────────────────────────┘
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
    priority_sched: PriorityScheduler,  // Priority-based scheduler
    current: Option<TaskId>,            // Currently running task
    next_tid: usize,                    // Next task ID to assign
}

// Task table (heap-allocated tasks)
static TASK_TABLE: Mutex<[TaskPtr; MAX_TASKS]>;
```

**Priority-Based Scheduling Algorithm:**
```
1. Timer interrupt fires (every 10ms)
2. Scheduler wakes sleeping tasks whose time has elapsed
3. Scheduler selects highest priority ready task
4. If higher priority task is ready, preempt current task
5. Context switch performed
6. Selected task resumes execution
```

**API:**
```rust
pub fn init_scheduler();
pub fn spawn_task(name: &'static str, entry: fn() -> !, priority: TaskPriority) -> Result<TaskId>;
pub fn tick();  // Called by timer interrupt
pub fn sleep_current(ticks: u64);  // Put current task to sleep
```

### 1.1 Priority Scheduler

**Location:** `kernel/src/sched/priority.rs`

**Features:**
- Three priority levels: High, Normal, Low
- Separate ready queue for each priority level
- O(1) task selection using priority bitmap
- Round-robin scheduling within same priority
- Sleep/wake mechanism with tick-based timers
- Preemption control for critical sections

**Data Structures:**
```rust
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
}

struct SleepingTask {
    task_id: TaskId,
    wake_tick: u64,
}

pub struct PriorityScheduler {
    ready_queues: [TaskQueue; 3],      // One queue per priority
    sleeping_tasks: Vec<SleepingTask>, // Tasks waiting to wake
    current_tick: u64,                 // Current timer tick
    preempt_disable_count: usize,      // Preemption disable counter
    non_empty_queues: u8,              // Bitmap for O(1) selection
}
```

**Priority Selection Algorithm:**
```
1. Check non_empty_queues bitmap
2. Select highest priority non-empty queue (High > Normal > Low)
3. Pop task from front of selected queue
4. Return task ID for execution
```

**Sleep/Wake Mechanism:**
```
Sleep:
1. Task calls sys_sleep(ticks)
2. Calculate wake_tick = current_tick + ticks
3. Remove task from ready queue
4. Add to sleeping_tasks list
5. Mark task as Sleeping
6. Trigger scheduler to select next task

Wake:
1. Timer interrupt increments current_tick
2. Scan sleeping_tasks for wake_tick <= current_tick
3. Move eligible tasks back to appropriate priority queue
4. Mark tasks as Ready
5. Remove from sleeping_tasks list
```

**API:**
```rust
impl PriorityScheduler {
    pub fn enqueue_task(&mut self, task_id: TaskId, priority: TaskPriority);
    pub fn select_next(&mut self) -> Option<TaskId>;
    pub fn sleep_task(&mut self, task_id: TaskId, ticks: u64);
    pub fn wake_sleeping_tasks(&mut self);
    pub fn tick(&mut self);
    pub fn preempt_disable(&mut self);
    pub fn preempt_enable(&mut self);
    pub fn can_preempt(&self) -> bool;
}
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
    id: TaskId,                      // Unique identifier
    name: &'static str,              // Human-readable name
    stack: *mut u8,                  // Stack base address
    stack_size: usize,               // 8KB per task
    state: TaskState,                // Ready, Running, Sleeping, or Blocked
    context: CpuContext,             // Saved CPU state
    priority: TaskPriority,          // Task priority (High/Normal/Low)
    wake_tick: Option<u64>,          // Wake time for sleeping tasks
    blocked_on_port: Option<usize>,  // Port ID if blocked on IPC
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
    │ Running │─────┤
    └────┬────┘     │
         │          │
         ├──────────┘
         │
         ├─ sys_sleep() ──→ ┌──────────┐
         │                   │ Sleeping │
         │                   └────┬─────┘
         │                        │ wake_tick elapsed
         │                        └──────────┐
         │                                   │
         └─ sys_ipc_recv() ─→ ┌─────────┐   │
           (no message)        │ Blocked │   │
                               └────┬────┘   │
                                    │ message arrives
                                    └────────┘
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

## System Call Interface

### Overview

**Location:** `kernel/src/sys/syscall.rs`

The system call interface provides a controlled mechanism for userland code to request kernel services. MelloOS uses the x86 `int 0x80` instruction for syscall invocation in Phase 4.

### Syscall ABI (x86-64 System V)

**Register Mapping:**
- `RAX`: Syscall number (input), return value (output)
- `RDI`: Argument 1
- `RSI`: Argument 2
- `RDX`: Argument 3
- `RCX`, `R8-R11`: Caller-saved (clobbered)
- `RBX`, `RBP`, `R12-R15`: Callee-saved (preserved)

**Return Values:**
- Success: Non-negative value (0, bytes written, bytes received, etc.)
- Error: -1

### Syscall Table

| ID | Name | Arguments | Description | Return |
|----|------|-----------|-------------|--------|
| 0 | SYS_WRITE | (fd, buf, len) | Write data to serial output | bytes written or -1 |
| 1 | SYS_EXIT | (code) | Terminate current task | does not return |
| 2 | SYS_SLEEP | (ticks) | Sleep for specified ticks | 0 or -1 |
| 3 | SYS_IPC_SEND | (port_id, buf, len) | Send message to port | 0 or -1 |
| 4 | SYS_IPC_RECV | (port_id, buf, len) | Receive message (blocking) | bytes received or -1 |

### Syscall Flow

```
Userland Task
    |
    | 1. int 0x80 (syscall instruction)
    v
Syscall Entry Point (ASM)
    |
    | 2. Save registers (RAX, RBX, RCX, RDX, RSI, RDI, R8-R15)
    | 3. Clear direction flag (DF = 0)
    v
Syscall Dispatcher
    |
    | 4. Validate syscall ID
    | 5. Route to appropriate handler
    v
Syscall Handler (sys_write, sys_sleep, etc.)
    |
    | 6. Execute kernel operation
    | 7. Update metrics
    v
Syscall Return (ASM)
    |
    | 8. Restore registers
    | 9. iretq (return to userland)
    v
Userland Task (continues)
```

### IDT Configuration

- **Vector**: 0x80 (128)
- **Gate Type**: Interrupt Gate (0xE)
- **DPL**: 3 (user-accessible)
- **Present**: 1
- **IST**: 0 (use current stack)

### Kernel Metrics

The syscall subsystem tracks the following metrics:

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

### Userland Syscall Wrappers

**Location:** `kernel/userspace/init/src/main.rs`

```rust
// Raw syscall invocation
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

// High-level wrappers
pub fn sys_write(msg: &str) {
    syscall(0, 0, msg.as_ptr() as usize, msg.len());
}

pub fn sys_sleep(ticks: usize) {
    syscall(2, ticks, 0, 0);
}

pub fn sys_ipc_send(port: usize, data: &[u8]) -> isize {
    syscall(3, port, data.as_ptr() as usize, data.len())
}

pub fn sys_ipc_recv(port: usize, buf: &mut [u8]) -> isize {
    syscall(4, port, buf.as_mut_ptr() as usize, buf.len())
}
```

## IPC (Inter-Process Communication)

### Overview

**Location:** `kernel/src/sys/ipc.rs`, `kernel/src/sys/port.rs`

MelloOS implements port-based message passing for inter-task communication. Tasks send and receive messages through numbered ports (0-255).

### Architecture

```
Task A                    Port Manager                    Task B
  |                            |                            |
  | sys_ipc_send(port, msg)    |                            |
  |--------------------------->|                            |
  |                            | Acquire port lock          |
  |                            | Enqueue message            |
  |                            | Check blocked tasks        |
  |                            |--------------------------->|
  |                            | Wake Task B (FIFO)         |
  |                            |                            | Dequeue message
  |                            |                            | Copy to buffer
  |                            |                            | Return bytes
```

### Data Structures

```rust
// Message structure (max 4096 bytes)
pub struct Message {
    data: Vec<u8>,
}

// Port structure
pub struct Port {
    id: usize,
    queue: VecDeque<Message>,         // Max 16 messages
    blocked_tasks: VecDeque<TaskId>,  // FIFO wake order
    lock: Spinlock<()>,
}

// Port manager (256 ports)
pub struct PortManager {
    ports: [Option<Port>; 256],
    table_lock: Spinlock<()>,
}
```

### IPC Semantics

**Message Passing:**
- **Send**: Non-blocking if queue has space, returns -1 if full
- **Receive**: Blocking if no messages available, wakes when message arrives
- **Wake Policy**: FIFO (first blocked task woken first)
- **Message Size**: Maximum 4096 bytes per message
- **Queue Size**: Maximum 16 messages per port

**Synchronization:**
- Each port has a spinlock protecting queue operations
- Lock hierarchy: PortManager::table_lock → Port::lock → Scheduler lock
- Preemption disabled while holding port lock
- No memory allocation while holding locks

### IPC Flow

**Send Message:**
```
1. Validate port ID and message size
2. Acquire port lock (with preempt_disable)
3. Check queue capacity (max 16 messages)
4. Enqueue message to port queue
5. If tasks blocked on port:
   - Wake one task (FIFO order)
   - Move task to ready queue
6. Release port lock (with preempt_enable)
7. Increment ipc_sends metric
8. Return 0 (success) or -1 (error)
```

**Receive Message:**
```
1. Validate port ID and buffer
2. Acquire port lock (with preempt_disable)
3. If message available:
   - Dequeue message
   - Copy to user buffer
   - Release lock
   - Return bytes received
4. If no message:
   - Add task to blocked_tasks queue
   - Mark task as Blocked
   - Release lock
   - Trigger scheduler (task sleeps)
   - (Task wakes when message arrives)
   - Retry receive operation
```

### Error Handling

```rust
pub enum IpcError {
    InvalidPort,        // Port ID >= 256
    QueueFull,          // 16 messages already queued
    InvalidBuffer,      // NULL or invalid buffer pointer
    PortNotFound,       // Port not initialized
    MessageTooLarge,    // Message > 4096 bytes
}
```

### API

```rust
impl PortManager {
    // Create port (called at boot)
    pub fn create_port(&mut self, port_id: usize) -> Result<(), IpcError>;
    
    // Send message to port
    pub fn send_message(&mut self, port_id: usize, data: &[u8]) 
        -> Result<(), IpcError>;
    
    // Receive message from port (blocking)
    pub fn recv_message(&mut self, port_id: usize, task_id: TaskId, 
        buf: &mut [u8]) -> Result<usize, IpcError>;
}
```

## Userland Processes

### Init Process

**Location:** `kernel/userspace/init/`

The init process is the first userland program launched by the kernel after boot. It demonstrates syscall and IPC functionality.

**Features:**
- Compiled as separate `no_std` binary
- Linked at fixed address (0x400000)
- Embedded into kernel image
- Runs with Normal priority
- Uses syscall wrappers for kernel services

**Example Code:**
```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    sys_write("Hello from userland! ✨\n");
    
    // IPC demo
    sys_ipc_send(2, b"ping");
    
    let mut buf = [0u8; 64];
    let bytes = sys_ipc_recv(1, &mut buf);
    
    sys_write("Got reply: pong\n");
    
    // Sleep demo
    sys_sleep(100);
    sys_write("Woke up!\n");
    
    loop {
        sys_sleep(1000);
    }
}
```

**Build Process:**
1. Compile init with `cargo build --release`
2. Extract binary with `objcopy`
3. Embed into kernel image
4. Kernel loads init at boot
5. Kernel spawns init task with entry point

## SMP Synchronization Architecture

### SpinLock Implementation

**Location:** `kernel/src/sync/spin.rs`

MelloOS uses spinlocks for protecting shared data structures in the multi-core environment:

```rust
pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

pub struct IrqSpinLock<T> {
    inner: SpinLock<T>,
}
```

**Features:**
- **Atomic Operations**: Uses `compare_exchange` with Acquire/Release ordering
- **Exponential Backoff**: Reduces bus contention during lock acquisition
- **IRQ-Safe Variant**: Disables interrupts while holding lock
- **RAII Guards**: Automatic unlock on scope exit
- **Deadlock Prevention**: Documented lock ordering rules

**Lock Hierarchy (to prevent deadlocks):**
1. Global locks before per-object locks
2. Task locks before port locks  
3. Runqueue locks ordered by CPU ID (lower ID first)
4. Never hold multiple runqueue locks unless migrating tasks

### Per-CPU Data Structures

**Location:** `kernel/src/arch/x86_64/smp/percpu.rs`

Each CPU core maintains its own data structures to minimize lock contention:

```rust
#[repr(C, align(64))]  // Cache line aligned
pub struct PerCpu {
    pub id: usize,
    pub apic_id: u8,
    pub runqueue: SpinLock<VecDeque<TaskId>>,
    pub current_task: Option<TaskId>,
    pub idle_task: TaskId,
    pub lapic_timer_hz: u64,
    pub ticks: AtomicU64,
    pub in_interrupt: bool,
}
```

**Access Methods:**
- **Current CPU**: `percpu_current()` using GS.BASE MSR
- **Remote CPU**: `percpu_for(cpu_id)` for cross-core access
- **Cache Alignment**: 64-byte alignment prevents false sharing

### Inter-Processor Interrupts (IPIs)

**Location:** `kernel/src/arch/x86_64/apic/ipi.rs`

IPIs enable cores to coordinate operations:

**IPI Types:**
- `RESCHEDULE_IPI (0x30)`: Trigger scheduler on target core
- `TLB_FLUSH_IPI (0x31)`: Flush TLB on target core (future)
- `HALT_IPI (0x32)`: Halt target core (future)

**Usage Examples:**
```rust
// Wake up remote core after task migration
send_reschedule_ipi(target_cpu);

// Broadcast to all cores except self
broadcast_ipi(RESCHEDULE_IPI_VECTOR, true);
```

### SMP-Safe Syscall Infrastructure

All Phase 4 syscalls are made SMP-safe through fine-grained locking:

**Synchronization Strategy:**
- **No Global Syscall Lock**: Each syscall uses appropriate per-object locks
- **Task Operations**: Protected by per-task spinlocks
- **IPC Operations**: Port queues protected by per-port spinlocks
- **Scheduler Operations**: Per-core runqueues with per-CPU spinlocks

**Cross-Core IPC Flow:**
```
Task on Core 0 → sys_ipc_send(port, msg) → Port Queue → Wake Task on Core 1
                                              ↓
                                    Send RESCHEDULE_IPI → Core 1
                                              ↓
                                    Core 1 scheduler runs → Task receives message
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
0x0000_0000_0040_0000 - 0x0000_0000_004F_FFFF : Init process (1MB)
0xFFFF_8000_0000_0000 - 0xFFFF_9FFF_FFFF_FFFF : HHDM (direct physical mapping)
0xFFFF_A000_0000_0000 - 0xFFFF_A000_00FF_FFFF : Kernel heap (16MB)
0xFFFF_FFFF_8000_0000 - 0xFFFF_FFFF_FFFF_FFFF : Kernel code/data

Task Stacks:
- Each task has an 8KB stack allocated from kernel heap
- Stacks grow downward from high addresses
- Stack pointer (RSP) saved in Task Control Block during context switch

IPC Message Queues:
- 256 ports, each with up to 16 messages
- Messages allocated from kernel heap (max 4096 bytes each)
- Total max IPC memory: 256 * 16 * 4096 = 16MB
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
Syscall:             128    (0x80) - System call interface
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
- **Task Selection**: O(1) with priority bitmap
- **Sleep/Wake**: O(n) linear scan in Phase 4 (O(log n) with BinaryHeap in Phase 5)
- **Timer Frequency**: 100 Hz (10ms time slices)
- **Scheduling Overhead**: ~1% CPU time at 100 Hz

**System Calls:**
- **Syscall Overhead**: ~100-200 cycles (int 0x80)
- **Register Save/Restore**: ~50 cycles
- **Dispatcher Routing**: ~10 cycles
- **Total Latency**: ~1-2 microseconds

**IPC:**
- **Message Send**: O(1) enqueue + O(1) wake
- **Message Receive**: O(1) dequeue (or block if empty)
- **Lock Acquisition**: Spinlock with preemption disabled
- **Message Copy**: O(n) where n = message size (max 4096 bytes)
