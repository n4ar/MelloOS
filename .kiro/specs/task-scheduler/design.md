# Task Scheduler Design Document

## Overview

ระบบ Task Scheduler สำหรับ MelloOS เป็นระบบ Multitasking แบบ Preemptive ที่ใช้ Round-Robin Scheduling Algorithm ร่วมกับ Timer Interrupt เพื่อสลับการทำงานระหว่าง Task ต่างๆ อย่างอัตโนมัติ

ระบบนี้ประกอบด้วย 4 ส่วนหลัก:
1. **Task Management** - จัดการ Task Control Block และ Task State
2. **Context Switching** - บันทึกและกลับคืน CPU register
3. **Scheduler** - เลือก Task ถัดไปด้วย Round-Robin algorithm
4. **Timer Interrupt** - สร้าง periodic interrupt เพื่อ preempt Task

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Kernel Main                          │
│  - เรียก sched::init_scheduler()                        │
│  - สร้าง demo tasks                                     │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│              Scheduler Module (sched/)                  │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Scheduler Core (mod.rs)                         │  │
│  │  - RUNQUEUE: VecDeque<Task>                      │  │
│  │  - CURRENT_TASK: Option<TaskId>                  │  │
│  │  - init_scheduler()                              │  │
│  │  - tick() → schedule_next()                      │  │
│  │  - spawn_task(fn_ptr) → Task                     │  │
│  └───────────────────────────────────────────────────┘  │
│                                                          │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Task Management (task.rs)                       │  │
│  │  - struct Task { id, name, stack, state, ctx }  │  │
│  │  - enum TaskState { Ready, Running, Sleeping }  │  │
│  │  - Task::new(fn_ptr) → Task                      │  │
│  └───────────────────────────────────────────────────┘  │
│                                                          │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Context Switch (context.rs)                     │  │
│  │  - struct CpuContext { rsp, rbp, r12-r15, ... }  │  │
│  │  - context_switch(curr, next) [Assembly]        │  │
│  └───────────────────────────────────────────────────┘  │
│                                                          │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Timer Interrupt (timer.rs)                      │  │
│  │  - init_timer(frequency)                         │  │
│  │  - timer_interrupt_handler()                     │  │
│  │  - setup_idt()                                   │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│           Hardware (Timer + CPU)                        │
│  - APIC Timer / PIT                                     │
│  - Generates periodic interrupts                        │
│  - CPU saves/restores registers                         │
└─────────────────────────────────────────────────────────┘
```

### Module Structure

```
kernel/src/
├── sched/
│   ├── mod.rs          # Scheduler core และ runqueue management
│   ├── task.rs         # Task structure และ task management
│   ├── context.rs      # Context switching (Assembly + Rust wrapper)
│   └── timer.rs        # Timer interrupt setup และ IDT configuration
└── main.rs             # เรียก init_scheduler() และสร้าง demo tasks
```


## Components and Interfaces

### 1. Task Structure (task.rs)

#### Task Control Block (TCB)

```rust
pub struct Task {
    pub id: TaskId,
    pub name: &'static str,
    pub stack: *mut u8,
    pub stack_size: usize,
    pub state: TaskState,
    pub context: CpuContext,
}

pub type TaskId = usize;

pub enum TaskState {
    Ready,      // พร้อมทำงาน รออยู่ใน runqueue
    Running,    // กำลังทำงานอยู่
    Sleeping,   // หยุดชั่วคราว (สำหรับอนาคต)
}
```

#### Task Creation

```rust
impl Task {
    /// สร้าง Task ใหม่จาก function pointer
    /// - จอง stack ขนาด 8KB จาก kernel heap
    /// - ตั้งค่า initial context (rsp, rip)
    /// - กำหนด state เป็น Ready
    pub fn new(id: TaskId, name: &'static str, entry_point: fn() -> !) -> Self;
    
    /// ทำลาย Task และคืน stack memory
    pub fn destroy(self);
}
```

### 2. CPU Context (context.rs)

#### Context Structure

```rust
#[repr(C)]
pub struct CpuContext {
    // Callee-saved registers (ต้องบันทึกตาม x86_64 calling convention)
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,
    
    // Stack pointer
    // RSP จะชี้ไปยัง stack ที่มี return address อยู่ด้านบน
    pub rsp: u64,
}

impl CpuContext {
    /// สร้าง initial context สำหรับ task ใหม่
    /// - ตั้ง RSP ให้ชี้ไปยัง prepared stack
    /// - Stack จะมี entry_trampoline เป็น return address
    pub fn new_for_task(stack_top: u64, entry_point: fn() -> !) -> Self;
}
```


#### Context Switch Implementation

```rust
/// Context switch จาก current task ไป next task
/// 
/// Assembly implementation:
/// 1. บันทึก callee-saved registers ของ current task ลง stack
/// 2. บันทึก rsp ของ current task ลง CpuContext
/// 3. โหลด rsp ของ next task จาก CpuContext
/// 4. กลับคืน callee-saved registers ของ next task จาก stack
/// 5. ret (กระโดดไปที่ return address บน stack)
/// 
/// สำหรับ task ใหม่: return address จะเป็น entry_trampoline
/// สำหรับ task ที่ถูก preempt: return address จะเป็นจุดที่หยุดไว้
pub unsafe fn context_switch(current: &mut CpuContext, next: &CpuContext);
```

Assembly implementation:
```asm
.global context_switch
context_switch:
    ; RDI = &mut current.rsp (first argument)
    ; RSI = &next.rsp (second argument)
    
    ; Save current context to stack
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15
    
    ; Save current RSP to current.rsp
    mov [rdi], rsp
    
    ; Load next RSP from next.rsp
    mov rsp, [rsi]
    
    ; Restore next context from stack
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx
    
    ; Return to next task
    ; - สำหรับ task ใหม่: จะกระโดดไป entry_trampoline
    ; - สำหรับ task ที่ถูก preempt: จะกลับไปจุดที่หยุดไว้
    ret
```

#### Entry Trampoline

```rust
/// Entry trampoline สำหรับ task ใหม่
/// - เรียก entry_point ของ task
/// - ถ้า entry_point return (ไม่ควรเกิด): panic
extern "C" fn entry_trampoline() -> ! {
    // ดึง entry_point จาก register หรือ stack
    // (จะถูกเตรียมไว้ตอน Task::new)
    let entry_point: fn() -> ! = unsafe { 
        // Implementation detail: อาจเก็บใน R12 หรือ stack
        core::mem::transmute(/* ... */)
    };
    
    entry_point();
    
    // ไม่ควรถึงจุดนี้
    panic!("[SCHED] Task returned from entry point!");
}
```

### 3. Scheduler Core (mod.rs)

#### Global State

```rust
use alloc::collections::VecDeque;
use alloc::boxed::Box;
use alloc::vec::Vec;
use spin::Mutex;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Task table - เก็บ Task objects ที่ตำแหน่งคงที่ในหน่วยความจำ
/// ใช้ Box เพื่อไม่ให้ Task ถูกย้ายเมื่อ vector grow
static TASK_TABLE: Mutex<Vec<Option<Box<Task>>>> = Mutex::new(Vec::new());

/// Scheduler state - รวม runqueue และ current task ไว้ด้วยกัน
/// เพื่อลด critical sections และป้องกัน deadlock
struct SchedState {
    runqueue: VecDeque<TaskId>,  // เก็บแค่ TaskId แทน Task object
    current: Option<TaskId>,
    next_tid: usize,
}

static SCHED: Mutex<SchedState> = Mutex::new(SchedState {
    runqueue: VecDeque::new(),
    current: None,
    next_tid: 1,
});
```


#### Scheduler Functions

```rust
/// เริ่มต้น scheduler
/// - สร้าง idle task (task id 0)
/// - เริ่มต้น timer interrupt
pub fn init_scheduler();

/// สร้าง task ใหม่และเพิ่มเข้า runqueue
/// - จอง Task ใน TASK_TABLE (ใช้ Box เพื่อไม่ให้ถูกย้าย)
/// - เพิ่ม TaskId เข้า runqueue
/// Returns: TaskId ของ task ที่สร้าง
pub fn spawn_task(name: &'static str, entry_point: fn() -> !) -> TaskId;

/// Scheduler tick - เรียกจาก timer interrupt
/// - เลือก task ถัดไปจาก runqueue (Round-Robin)
/// - ทำ context switch
/// - ไม่ return กลับมาที่ task เดิม (tail-switch)
pub fn tick();

/// เลือก task ถัดไปด้วย Round-Robin algorithm
/// - ย้าย current TaskId ไปท้าย runqueue
/// - เอา TaskId แรกจาก runqueue มาเป็น current
/// - ดึง Task object จาก TASK_TABLE ด้วย TaskId
fn schedule_next() -> Option<&'static mut Task>;

/// ดึง Task จาก TASK_TABLE ด้วย TaskId
/// Returns: mutable reference ไปยัง Task (ปลอดภัยเพราะใช้ Box)
fn get_task(id: TaskId) -> Option<&'static mut Task>;
```

#### Round-Robin Algorithm

```
1. เมื่อ tick() ถูกเรียก (จาก timer interrupt):
   - Lock SCHED state (single critical section)
   - ถ้ามี current TaskId:
     - ดึง Task จาก TASK_TABLE
     - เปลี่ยน state จาก Running → Ready
     - push current TaskId ไปท้าย runqueue
   
2. เลือก task ถัดไป:
   - pop_front TaskId จาก runqueue
   - ดึง Task จาก TASK_TABLE ด้วย TaskId
   - เปลี่ยน state เป็น Running
   - ตั้งเป็น current TaskId
   - Unlock SCHED state
   
3. ทำ context switch (tail-switch):
   - context_switch(&mut old_ctx, &new_ctx)
   - ไม่ return กลับมาที่ handler เดิม
   - next task จะเริ่มทำงานต่อจากจุดที่หยุดไว้
```

### 4. Timer Interrupt (timer.rs)

#### IDT (Interrupt Descriptor Table)

```rust
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

/// ตั้งค่า IDT และ register timer interrupt handler
pub fn init_idt() {
    unsafe {
        IDT.timer.set_handler_fn(timer_interrupt_handler);
        IDT.load();
    }
}
```


#### Timer Configuration

```rust
/// เริ่มต้น timer (PIT หรือ APIC)
/// frequency: จำนวน interrupts ต่อวินาที (Hz)
pub fn init_timer(frequency: u32);

/// Timer interrupt handler
/// - เรียก scheduler tick()
/// - ส่ง EOI (End of Interrupt) ไปยัง PIC
/// 
/// หมายเหตุ: CPU จะปิด interrupts (IF=0) อัตโนมัติเมื่อเข้า handler
/// ดังนั้นไม่มี race condition กับ interrupt อื่น
extern "x86-interrupt" fn timer_interrupt_handler(
    _stack_frame: InterruptStackFrame
) {
    // ส่ง EOI ก่อน (เพื่อให้ PIC พร้อมรับ interrupt ถัดไป)
    unsafe {
        send_eoi();
    }
    
    // เรียก scheduler (tail-switch: ไม่ return กลับมา)
    crate::sched::tick();
    
    // หมายเหตุ: tick() จะทำ context switch และไม่ return
    // ดังนั้นโค้ดหลังจากนี้จะไม่ถูกเรียก
}
```

#### PIT (Programmable Interval Timer) Configuration

เนื่องจาก APIC ซับซ้อนกว่า เราจะใช้ PIT ก่อนใน MVP:

```rust
const PIT_FREQUENCY: u32 = 1193182; // PIT base frequency
const PIT_COMMAND: u16 = 0x43;
const PIT_CHANNEL_0: u16 = 0x40;
const PIC1_COMMAND: u16 = 0x20;

pub fn init_pit_timer(frequency: u32) {
    let divisor = PIT_FREQUENCY / frequency;
    
    unsafe {
        // Set PIT to mode 3 (square wave generator)
        Port::new(PIT_COMMAND).write(0x36u8);
        
        // Set frequency divisor
        Port::new(PIT_CHANNEL_0).write((divisor & 0xFF) as u8);
        Port::new(PIT_CHANNEL_0).write(((divisor >> 8) & 0xFF) as u8);
    }
}

unsafe fn send_eoi() {
    // Send End of Interrupt to PIC Master (0x20)
    // IRQ0 (timer) อยู่ที่ master PIC ดังนั้นส่ง EOI ไปที่ master เท่านั้น
    // ถ้าใช้ IRQ >= 8 (slave PIC) ต้องส่ง EOI ไปทั้ง slave และ master
    Port::new(PIC1_COMMAND).write(0x20u8);
}
```


## Data Models

### Task Lifecycle

```
     ┌─────────┐
     │  spawn  │
     └────┬────┘
          │
          ▼
     ┌─────────┐
     │  Ready  │◄─────────┐
     └────┬────┘          │
          │               │
          │ schedule()    │ preempt/yield
          ▼               │
     ┌─────────┐          │
     │ Running │──────────┘
     └────┬────┘
          │
          │ exit (future)
          ▼
     ┌─────────┐
     │  Dead   │
     └─────────┘
```

### Memory Layout

#### Task Stack Layout

```
High Address
┌─────────────────┐
│  Guard Page     │ (optional, สำหรับ stack overflow detection)
├─────────────────┤
│                 │
│   Stack Space   │ 8KB
│   (grows down)  │
│                 │
├─────────────────┤
│      R15        │ ← Initial stack frame (prepared by Task::new)
│      R14        │
│      R13        │
│      R12        │    (เก็บ entry_point pointer)
│      RBP        │
│      RBX        │
├─────────────────┤ ← Initial RSP (context.rsp)
│ entry_trampoline│    (return address)
└─────────────────┘
Low Address

เมื่อ context_switch() ทำ ret ครั้งแรก:
- จะกระโดดไป entry_trampoline
- entry_trampoline จะดึง entry_point จาก R12
- แล้วเรียก entry_point()
```

#### Context on Stack (during context switch)

```
High Address
┌─────────────────┐
│   Task Data     │
├─────────────────┤
│      R15        │
│      R14        │
│      R13        │
│      R12        │
│      RBP        │
│      RBX        │
├─────────────────┤ ← RSP after save
│  Return Addr    │
└─────────────────┘
Low Address
```


## Error Handling

### Error Types

```rust
pub enum SchedulerError {
    OutOfMemory,           // ไม่สามารถจอง stack สำหรับ task ใหม่
    NoTasksAvailable,      // runqueue ว่าง
    InvalidTaskId,         // task id ไม่ถูกต้อง
    ContextSwitchFailed,   // context switch ล้มเหลว
}
```

### Error Handling Strategy

1. **Out of Memory**
   - เกิดเมื่อ kmalloc() ล้มเหลวในการจอง stack
   - การจัดการ: return error จาก spawn_task(), ไม่เพิ่ม task เข้า runqueue
   - Logging: `[SCHED] ERROR: Failed to allocate stack for task`

2. **No Tasks Available**
   - เกิดเมื่อ runqueue ว่างใน schedule_next()
   - การจัดการ: รัน idle task (task id 0) แทน
   - Logging: `[SCHED] WARNING: No tasks in runqueue, running idle task`

3. **Context Switch Failed**
   - เกิดเมื่อ context pointer เป็น null หรือไม่ valid
   - การจัดการ: panic! เพราะเป็น critical error
   - Logging: `[SCHED] PANIC: Context switch failed`

4. **Timer Interrupt Issues**
   - เกิดเมื่อ timer interrupt ไม่ fire หรือ fire บ่อยเกินไป
   - การจัดการ: ตรวจสอบ frequency configuration
   - Logging: `[SCHED] WARNING: Timer frequency may be incorrect`

### Panic Conditions

ระบบจะ panic ในกรณีต่อไปนี้:
- Context switch ล้มเหลว (critical error)
- IDT setup ล้มเหลว
- Stack pointer corruption detected
- Double fault ใน interrupt handler


## Testing Strategy

### Unit Tests

#### 1. Task Creation Tests
```rust
#[test]
fn test_task_creation() {
    // Test: สร้าง task ใหม่
    // Verify: task มี id, name, stack ที่ถูกต้อง
    // Verify: state เป็น Ready
}

#[test]
fn test_task_stack_allocation() {
    // Test: stack ถูกจองจาก heap
    // Verify: stack pointer ไม่เป็น null
    // Verify: stack size = 8KB
}
```

#### 2. Context Switch Tests
```rust
#[test]
fn test_context_save_restore() {
    // Test: บันทึกและกลับคืน context
    // Verify: register values ถูกต้องหลัง context switch
}
```

#### 3. Scheduler Tests
```rust
#[test]
fn test_spawn_task() {
    // Test: spawn task ใหม่
    // Verify: task ถูกเพิ่มเข้า runqueue
    // Verify: task id ถูก increment
}

#[test]
fn test_round_robin() {
    // Test: spawn 3 tasks และ tick หลายครั้ง
    // Verify: tasks ถูกเลือกตามลำดับ Round-Robin
}
```

### Integration Tests

#### 1. Timer Interrupt Test
```rust
#[test]
fn test_timer_interrupt() {
    // Test: ตั้งค่า timer และรอ interrupt
    // Verify: interrupt handler ถูกเรียก
    // Verify: tick counter เพิ่มขึ้น
}
```

#### 2. Multi-Task Test
```rust
#[test]
fn test_two_tasks_switching() {
    // Test: สร้าง 2 tasks ที่ print ข้อความต่างกัน
    // Verify: เห็นข้อความสลับกัน
    // Verify: ไม่ crash หลัง 100 context switches
}
```


### Manual Testing

#### Test Scenario 1: Basic Task Switching

**Setup:**
```rust
fn task_a() -> ! {
    loop {
        serial_println!("A");
        // Busy wait
        for _ in 0..1000000 { unsafe { core::arch::asm!("nop"); } }
    }
}

fn task_b() -> ! {
    loop {
        serial_println!("B");
        for _ in 0..1000000 { unsafe { core::arch::asm!("nop"); } }
    }
}

// In main.rs
sched::init_scheduler();
sched::spawn_task("Task A", task_a);
sched::spawn_task("Task B", task_b);
```

**Expected Output:**
```
[SCHED] Initializing scheduler...
[SCHED] Spawned task 1: Task A
[SCHED] Spawned task 2: Task B
[SCHED] Timer initialized at 100 Hz
[SCHED] Switch → Task 1 (Task A)
A
[SCHED] Switch → Task 2 (Task B)
B
[SCHED] Switch → Task 1 (Task A)
A
[SCHED] Switch → Task 2 (Task B)
B
...
```

#### Test Scenario 2: Stress Test

**Setup:**
- สร้าง 5 tasks
- รัน 1000 context switches
- ตรวจสอบว่าไม่ crash

**Expected:**
- ระบบเสถียร
- ไม่มี stack overflow
- ไม่มี memory leak

#### Test Scenario 3: Timer Frequency Test

**Setup:**
- ทดสอบ frequency ต่างๆ: 10 Hz, 100 Hz, 1000 Hz
- วัดจำนวน context switches ต่อวินาที

**Expected:**
- Context switches ตรงกับ frequency ที่ตั้ง (±5%)


## Design Decisions and Rationales

### 1. Round-Robin Scheduling

**Decision:** ใช้ Round-Robin algorithm สำหรับ MVP

**Rationale:**
- เรียบง่าย ง่ายต่อการ implement และ debug
- ให้ความเป็นธรรม (fairness) - ทุก task ได้เวลา CPU เท่ากัน
- ไม่ต้องมี priority system ซึ่งซับซ้อนกว่า
- เหมาะสำหรับ MVP ที่มี task น้อย
- สามารถ upgrade เป็น priority-based scheduler ได้ในอนาคต

**Trade-offs:**
- ไม่มี priority - task สำคัญไม่ได้รับ CPU มากกว่า
- ไม่เหมาะกับ real-time tasks
- Context switch overhead เท่ากันทุก task

### 2. PIT vs APIC Timer

**Decision:** ใช้ PIT (Programmable Interval Timer) สำหรับ MVP

**Rationale:**
- PIT เรียบง่ายกว่า APIC มาก
- มี documentation และ example code มากกว่า
- รองรับทุก x86_64 system
- เพียงพอสำหรับ MVP (100 Hz)
- สามารถ migrate ไป APIC ได้ในอนาคต

**Trade-offs:**
- PIT เก่ากว่า APIC
- Frequency resolution ต่ำกว่า
- ไม่ support per-CPU timer (สำหรับ SMP)

### 3. Stack Size (8KB)

**Decision:** ใช้ stack ขนาด 8KB ต่อ task

**Rationale:**
- เพียงพอสำหรับ kernel tasks ทั่วไป
- ไม่ใหญ่เกินไป (ประหยัด memory)
- เป็น 2 pages (4KB × 2) - ง่ายต่อการจัดการ
- ตรงกับ Linux kernel default stack size

**Trade-offs:**
- อาจไม่พอสำหรับ deep recursion
- ต้องระวัง stack overflow


### 4. Context Switch in Assembly

**Decision:** เขียน context switch ด้วย inline assembly

**Rationale:**
- ต้องการ control register manipulation โดยตรง
- Rust compiler ไม่สามารถ optimize context switch ได้ดีพอ
- ต้องการ performance สูงสุด (context switch เกิดบ่อย)
- ตรงตาม x86_64 calling convention

**Trade-offs:**
- Code ซับซ้อนกว่า pure Rust
- ยากต่อการ debug
- Platform-specific (ต้องเขียนใหม่สำหรับ architecture อื่น)

### 5. VecDeque for Runqueue + Task Table

**Decision:** ใช้ `VecDeque<TaskId>` สำหรับ runqueue และ `Vec<Option<Box<Task>>>` สำหรับ task table

**Rationale:**
- **Runqueue เก็บแค่ TaskId:**
  - ไม่ต้องย้าย Task object เมื่อ push/pop
  - ป้องกัน pointer invalidation
  - เตรียมพร้อมสำหรับ sleep queues และ wait queues
- **Task Table ใช้ Box:**
  - Task object อยู่ที่ตำแหน่งคงที่ในหน่วยความจำ
  - ปลอดภัยกับ context pointer และ borrow
  - สามารถเก็บ reference ไปยัง Task ได้
- **O(1) operations:**
  - push_back/pop_front: O(1)
  - Task lookup by ID: O(1)

**Trade-offs:**
- ต้องใช้ heap allocation มากขึ้น (Box + Vec)
- Task table อาจมี holes (Option::None) เมื่อ task ถูกทำลาย
- ในอนาคตอาจต้อง implement slab allocator สำหรับ Task

### 6. Unified Scheduler State with Mutex

**Decision:** รวม runqueue, current task, และ next_tid ไว้ใน `SchedState` struct เดียว และใช้ `spin::Mutex` ป้องกัน

**Rationale:**
- **Single critical section:**
  - ลด lock/unlock operations
  - ป้องกัน deadlock (ไม่ต้องล็อกหลาย mutex)
  - ง่ายต่อการ reason about concurrency
- **Interrupt safety:**
  - CPU ปิด interrupts อัตโนมัติใน handler
  - Mutex ป้องกัน race condition จาก nested interrupts (ถ้ามี)
- **spin::Mutex:**
  - ไม่ต้องการ OS support
  - เหมาะกับ kernel space
  - API เหมือน std::sync::Mutex

**Trade-offs:**
- Spinlock อาจ waste CPU cycles (แต่ critical section สั้นมาก)
- ไม่เหมาะกับ long critical sections (แต่เราทำให้สั้นที่สุด)
- ในอนาคตอาจต้องใช้ lock-free data structures สำหรับ SMP


## Implementation Notes

### Dependencies

จะต้องเพิ่ม dependencies ใน `Cargo.toml`:

```toml
[dependencies]
limine = "0.5"
spin = "0.9"          # มีอยู่แล้ว
x86_64 = "0.15"       # มีอยู่แล้ว
```

ไม่ต้องเพิ่ม dependency ใหม่ เพราะมีครบแล้ว!

### Integration with Existing Code

#### 1. Memory Management Integration

Scheduler จะใช้ memory management ที่มีอยู่:
- `allocator::kmalloc()` สำหรับจอง task stack
- `allocator::kfree()` สำหรับคืน stack เมื่อ task จบ

```rust
// In task.rs
use crate::mm::allocator::{kmalloc, kfree};

impl Task {
    pub fn new(id: TaskId, name: &'static str, entry_point: fn() -> !) -> Self {
        // Allocate 8KB stack
        let stack = kmalloc(8192);
        // ...
    }
}
```

#### 2. Serial Logging Integration

Scheduler จะใช้ serial logging ที่มีอยู่:

```rust
// In mod.rs
use crate::serial_println;

pub fn tick() {
    serial_println!("[SCHED] Tick");
    // ...
}
```

#### 3. Main.rs Integration

เพิ่มการเรียก scheduler ใน `main.rs`:

```rust
// In main.rs
mod sched;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // ... existing initialization ...
    
    serial_println!("[KERNEL] Initializing scheduler...");
    sched::init_scheduler();
    
    // Spawn demo tasks
    sched::spawn_task("Task A", task_a);
    sched::spawn_task("Task B", task_b);
    
    serial_println!("[KERNEL] Scheduler initialized!");
    
    // Enable interrupts
    unsafe { core::arch::asm!("sti"); }
    
    // Idle loop
    loop {
        unsafe { core::arch::asm!("hlt"); }
    }
}

fn task_a() -> ! {
    loop {
        serial_println!("A");
    }
}

fn task_b() -> ! {
    loop {
        serial_println!("B");
    }
}
```


### x86_64 Calling Convention

Context switch ต้องเป็นไปตาม System V AMD64 ABI calling convention:

**Callee-saved registers** (ต้องบันทึก):
- RBX, RBP, R12, R13, R14, R15
- RSP (stack pointer)

**Caller-saved registers** (ไม่ต้องบันทึก):
- RAX, RCX, RDX, RSI, RDI, R8-R11
- เพราะ caller จะบันทึกเองก่อนเรียก function

**Return value:**
- RAX (ไม่ต้องบันทึกใน context switch)

### Interrupt Handling Flow

```
1. Timer interrupt fires (IRQ0)
   ↓
2. CPU automatically:
   - Disables interrupts (IF=0)
   - Saves SS, RSP, RFLAGS, CS, RIP to current task's kernel stack
   - Loads interrupt handler from IDT
   ↓
3. CPU jumps to timer_interrupt_handler
   ↓
4. Handler sends EOI to PIC (ให้ PIC พร้อมรับ interrupt ถัดไป)
   ↓
5. Handler calls sched::tick()
   ↓
6. tick() locks SCHED state (single critical section)
   ↓
7. tick() updates runqueue (current → back, next → front)
   ↓
8. tick() unlocks SCHED state
   ↓
9. tick() calls context_switch(&mut old_ctx, &new_ctx)
   ↓
10. context_switch() (tail-switch):
    - Saves old task's registers to stack
    - Saves old task's RSP to old_ctx
    - Loads new task's RSP from new_ctx
    - Restores new task's registers from stack
    - ret (jumps to new task's return address)
   ↓
11. New task continues execution
    - ถ้าเป็น task ใหม่: จะไปที่ entry_trampoline → entry_point
    - ถ้าเป็น task ที่ถูก preempt: จะกลับไปจุดที่หยุดไว้

หมายเหตุ: เราไม่ return กลับไปที่ handler เดิม (tail-switch)
ดังนั้น iretq จะเกิดใน context ของ next task แทน
```

### Critical Sections

ต้องระวัง race conditions ในส่วนต่อไปนี้:

1. **Scheduler state access**
   - ใช้ single Mutex (SCHED) ป้องกัน concurrent access
   - Lock ครั้งเดียวใน tick() เพื่อ update runqueue และ current task
   - ลด critical section ให้สั้นที่สุด

2. **Task table access**
   - ใช้ Mutex (TASK_TABLE) ป้องกัน
   - Lock เฉพาะตอน spawn/destroy task
   - ไม่ lock ใน context switch (ใช้ TaskId lookup แทน)

3. **Interrupt handler safety**
   - CPU ปิด interrupts (IF=0) อัตโนมัติเมื่อเข้า handler
   - ห้าม enable interrupts ใน handler
   - ทำงานให้เร็วที่สุด (send EOI → tick → context switch)
   - ไม่ควร allocate memory ใน handler (ใช้ pre-allocated structures)

4. **Deadlock prevention**
   - ใช้ single lock (SCHED) แทนหลาย locks
   - ไม่ nested locking
   - Critical section สั้นมาก (< 100 instructions)


## Future Enhancements

### Phase 4: Advanced Scheduling

1. **Priority-based Scheduling**
   - เพิ่ม priority field ใน Task
   - ใช้ priority queue แทน VecDeque
   - Implement priority inheritance

2. **Sleep/Wake Mechanism**
   - เพิ่ม sleep_until(time)
   - เพิ่ม wake_task(task_id)
   - Implement wait queues

3. **CPU Affinity**
   - เพิ่ม cpu_affinity field
   - Pin task ไปยัง specific CPU
   - Load balancing

### Phase 5: SMP Support

1. **Per-CPU Runqueues**
   - แยก runqueue ต่อ CPU
   - Implement work stealing

2. **APIC Timer**
   - Migrate จาก PIT ไป APIC
   - Per-CPU timer interrupts

3. **Spinlock Improvements**
   - Implement ticket locks
   - Add lock debugging

### Phase 6: Process Management

1. **User Space Tasks**
   - แยก kernel tasks และ user tasks
   - Implement privilege level switching

2. **Process Creation**
   - fork() system call
   - exec() system call

3. **IPC (Inter-Process Communication)**
   - Message passing
   - Shared memory


## Performance Considerations

### Context Switch Overhead

**Target:** < 1 microsecond per context switch

**Optimization strategies:**
1. Minimize register saves (เฉพาะ callee-saved)
2. ใช้ inline assembly (ไม่มี function call overhead)
3. Cache-friendly data structures
4. Avoid memory allocation ใน critical path

### Timer Frequency

**Recommended:** 100 Hz (10ms per tick)

**Trade-offs:**
- **Higher frequency (1000 Hz):**
  - ✅ Better responsiveness
  - ❌ More context switch overhead
  - ❌ More CPU time spent in scheduler

- **Lower frequency (10 Hz):**
  - ✅ Less overhead
  - ❌ Worse responsiveness
  - ❌ Tasks wait longer for CPU

### Memory Usage

**Per Task:**
- Task struct: ~64 bytes
- Stack: 8KB
- Total: ~8KB per task

**For 100 tasks:** ~800KB

**Optimization:**
- ใช้ smaller stack สำหรับ simple tasks
- Implement stack growth on demand
- Share read-only data between tasks

### Scalability

**Current design scales to:**
- ~100 tasks (VecDeque performance)
- Single CPU only
- No priority support

**For better scalability:**
- Migrate to intrusive linked list
- Implement per-CPU runqueues
- Add priority queues


## Security Considerations

### Stack Overflow Protection

**Risk:** Task stack overflow อาจ corrupt kernel memory

**Mitigation:**
1. จอง guard page ก่อนและหลัง stack
2. ตรวจสอบ RSP ใน context switch
3. Implement stack canaries (future)

```rust
// Guard page implementation
pub fn allocate_task_stack() -> *mut u8 {
    // Allocate 3 pages: guard + stack + guard
    let total_size = 3 * 4096;
    let base = kmalloc(total_size);
    
    // Mark first and last page as guard
    // (requires paging support)
    
    // Return middle page
    base + 4096
}
```

### Interrupt Safety

**Risk:** Race condition ใน interrupt handler

**Mitigation:**
1. ใช้ Mutex สำหรับ shared state
2. Disable interrupts ใน critical sections
3. Keep interrupt handler short

```rust
pub fn tick() {
    // Interrupts already disabled by CPU
    let mut runqueue = RUNQUEUE.lock();
    // ... critical section ...
    // Mutex automatically unlocked
}
```

### Task Isolation

**Current:** ไม่มี isolation (ทุก task รันใน kernel mode)

**Future:**
- Implement user mode tasks
- Separate address spaces
- System call interface


## References

### Technical Documentation

1. **x86_64 Architecture**
   - Intel 64 and IA-32 Architectures Software Developer's Manual
   - AMD64 Architecture Programmer's Manual
   - System V AMD64 ABI Calling Convention

2. **Interrupt Handling**
   - OSDev Wiki: Interrupts
   - OSDev Wiki: 8259 PIC
   - OSDev Wiki: APIC

3. **Scheduling Algorithms**
   - Operating Systems: Three Easy Pieces (OSTEP) - Scheduling Chapter
   - Linux Kernel Development by Robert Love
   - The Design and Implementation of the FreeBSD Operating System

### Code References

1. **Rust OS Development**
   - Writing an OS in Rust by Philipp Oppermann
   - Redox OS source code
   - Theseus OS source code

2. **Context Switching**
   - xv6 (MIT) - context switch implementation
   - Linux kernel - __switch_to() function
   - SerenityOS - context switch code

### Related MelloOS Documentation

- Memory Management Design (`.kiro/specs/memory-management/design.md`)
- Memory Management Logging (`.kiro/specs/memory-management/logging.md`)
- Kernel Boot Process (`README.md`)


## Additional Design Clarifications

### IRQ Vector Mapping

**PIC Remapping:**
- Master PIC (IRQ 0-7) → Vectors 32-39 (0x20-0x27)
- Slave PIC (IRQ 8-15) → Vectors 40-47 (0x28-0x2F)
- Timer (IRQ0) → Vector 32 (0x20)

**IDT Setup:**
```rust
pub fn init_idt() {
    unsafe {
        // Timer interrupt at vector 32
        IDT[32].set_handler_fn(timer_interrupt_handler);
        IDT.load();
    }
}
```

### Logging Strategy

**Log Levels:**
- `[SCHED]` - ข้อความทั่วไป
- `[SCHED] INFO:` - ข้อมูลสำคัญ
- `[SCHED] WARNING:` - คำเตือน
- `[SCHED] ERROR:` - ข้อผิดพลาด

**Throttling:**
- Log context switch เฉพาะ 10 ครั้งแรก (เพื่อไม่ให้ log เยอะเกินไป)
- หลังจากนั้น log ทุก 100 switches
- Demo tasks (A/B) log ทุกครั้ง (เพื่อแสดงการสลับ)

```rust
static SWITCH_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn tick() {
    let count = SWITCH_COUNT.fetch_add(1, Ordering::Relaxed);
    
    // Log first 10 switches, then every 100
    if count < 10 || count % 100 == 0 {
        serial_println!("[SCHED] Switch #{} → Task {} ({})", 
            count, next_id, next_name);
    }
    
    // ... context switch ...
}
```

### Task::new() Implementation Details

```rust
impl Task {
    pub fn new(id: TaskId, name: &'static str, entry_point: fn() -> !) -> Self {
        // 1. Allocate 8KB stack
        let stack = kmalloc(8192);
        if stack.is_null() {
            panic!("[SCHED] Failed to allocate stack for task {}", id);
        }
        
        // 2. Calculate stack top (stack grows down)
        let stack_top = (stack as usize) + 8192;
        
        // 3. Prepare initial stack frame
        let mut rsp = stack_top as *mut u64;
        unsafe {
            // Push entry_trampoline as return address
            rsp = rsp.offset(-1);
            *rsp = entry_trampoline as u64;
            
            // Push callee-saved registers (will be popped by context_switch)
            rsp = rsp.offset(-1); *rsp = 0; // RBX
            rsp = rsp.offset(-1); *rsp = 0; // RBP
            rsp = rsp.offset(-1); *rsp = entry_point as u64; // R12 (entry_point)
            rsp = rsp.offset(-1); *rsp = 0; // R13
            rsp = rsp.offset(-1); *rsp = 0; // R14
            rsp = rsp.offset(-1); *rsp = 0; // R15
        }
        
        // 4. Create context
        let context = CpuContext {
            rsp: rsp as u64,
            rbx: 0,
            rbp: 0,
            r12: entry_point as u64,
            r13: 0,
            r14: 0,
            r15: 0,
        };
        
        Self {
            id,
            name,
            stack,
            stack_size: 8192,
            state: TaskState::Ready,
            context,
        }
    }
}
```

