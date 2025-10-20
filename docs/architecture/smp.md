# SMP (Symmetric Multi-Processing) Implementation

This document provides detailed implementation notes for MelloOS SMP support, covering CPU discovery, AP bootstrap, synchronization, and multi-core scheduling.

## Overview

MelloOS SMP implementation enables the kernel to utilize multiple CPU cores simultaneously. The design follows a symmetric multi-processing model where all cores are treated equally after initialization, with the Bootstrap Processor (BSP) handling initial system setup.

**Key Features:**
- Support for up to 8 CPU cores
- ACPI MADT-based CPU discovery
- INIT/SIPI AP bootstrap sequence
- Per-core runqueues with load balancing
- SpinLock-based synchronization
- Inter-processor interrupts (IPIs)
- SMP-safe syscall infrastructure

## CPU Discovery and ACPI Integration

### MADT (Multiple APIC Description Table) Parsing

**Location:** `kernel/src/arch/x86_64/acpi/mod.rs`

The ACPI MADT table contains information about system processors and interrupt controllers:

```rust
pub struct MadtInfo {
    pub lapic_address: u64,           // Local APIC base address
    pub cpus: Vec<CpuInfo>,           // Detected CPU cores
    pub ioapics: Vec<IoApicInfo>,     // I/O APIC controllers
}

pub struct CpuInfo {
    pub apic_id: u8,                  // APIC ID (unique per core)
    pub processor_id: u8,             // ACPI processor ID
    pub enabled: bool,                // CPU is enabled and usable
}
```

**Parsing Process:**
1. Locate RSDP (Root System Description Pointer) from bootloader
2. Follow RSDT/XSDT chain to find MADT
3. Parse MADT entries:
   - **Type 0**: Local APIC (CPU core information)
   - **Type 1**: I/O APIC (external interrupt routing)
4. Validate checksums and table signatures
5. Filter enabled CPUs only
6. Store LAPIC base address (typically 0xFEE00000)

**Expected Boot Log:**
```
[ACPI] RSDP found at 0x...
[ACPI] MADT found at 0x...
[SMP] CPUs detected: 4 (apic_ids=[0,1,2,3])
```

## Local APIC Management

### APIC Initialization

**Location:** `kernel/src/arch/x86_64/apic/mod.rs`

Each CPU core has a Local APIC for interrupt handling and inter-processor communication:

```rust
pub struct LocalApic {
    base_addr: *mut u32,              // Memory-mapped I/O base
}

// Key APIC registers (offsets from base)
const LAPIC_ID: u32 = 0x20;          // APIC ID register
const LAPIC_EOI: u32 = 0xB0;         // End of Interrupt
const LAPIC_SPURIOUS: u32 = 0xF0;    // Spurious interrupt vector
const LAPIC_ICR_LOW: u32 = 0x300;    // Interrupt Command Register (low)
const LAPIC_ICR_HIGH: u32 = 0x310;   // Interrupt Command Register (high)
const LAPIC_TIMER_LVT: u32 = 0x320;  // Timer Local Vector Table
```

**Initialization Sequence:**
1. Map LAPIC base address (from MADT) into kernel virtual memory
2. Enable LAPIC by setting bit 8 in spurious interrupt vector register
3. Set spurious vector to 0xFF (unused vector)
4. Verify LAPIC functionality by reading LAPIC ID register
5. Configure timer for periodic interrupts at 100 Hz

**BSP vs AP Initialization:**
- **BSP**: Initializes first during early boot, before AP bringup
- **APs**: Initialize their own LAPIC during `ap_entry64()` function

### APIC Timer Calibration

**Location:** `kernel/src/arch/x86_64/apic/mod.rs`

Each core's APIC timer must be calibrated to determine its frequency:

```rust
pub fn calibrate_timer(&mut self) -> u64 {
    // 1. Program PIT channel 2 for 10ms one-shot
    // 2. Set LAPIC timer to maximum count (0xFFFFFFFF)
    // 3. Wait for PIT interrupt
    // 4. Read LAPIC current count
    // 5. Calculate frequency: (0xFFFFFFFF - current) * 100
}
```

**Timer Configuration:**
- **Mode**: Periodic (generates interrupts at regular intervals)
- **Divide Value**: 16 (reduces timer frequency)
- **Vector**: 0x20 (timer interrupt vector)
- **Frequency**: 100 Hz (10ms intervals for scheduler)

**Expected Boot Log:**
```
[APIC] BSP LAPIC initialized at 0xFEE00000
[APIC] core0 timer @1000000Hz
[APIC] core1 timer @1000000Hz
[APIC] core2 timer @1000000Hz
[APIC] core3 timer @1000000Hz
```

## Application Processor Bootstrap

### Trampoline Code

**Location:** `kernel/src/arch/x86_64/smp/boot_ap.S`

APs start in 16-bit real mode and must transition to 64-bit long mode:

**Memory Layout (at 0x8000):**
```
0x8000: trampoline_start     (16-bit real mode entry)
0x8100: gdt32               (temporary 32-bit GDT)
0x8200: gdt64               (64-bit GDT)
0x8300: ap_stack_ptr        (pointer to AP stack)
0x8308: ap_entry_ptr        (pointer to ap_entry64)
0x8310: ap_cr3              (kernel page table)
0x8318: ap_cpu_id           (assigned CPU ID)
```

**Transition Sequence:**
1. **Real Mode (16-bit)**:
   - Enable A20 line for >1MB memory access
   - Load temporary GDT with 32-bit segments
   - Set CR0.PE = 1 (enter protected mode)

2. **Protected Mode (32-bit)**:
   - Load 64-bit GDT
   - Set CR4.PAE = 1 (enable Physical Address Extension)
   - Load CR3 with kernel page table
   - Set EFER.LME = 1 (enable long mode)
   - Set CR0.PG = 1 (enable paging)

3. **Long Mode (64-bit)**:
   - Jump to `ap_entry64` function
   - Initialize per-CPU data structures
   - Configure GS.BASE MSR
   - Initialize Local APIC
   - Enter scheduler loop

### AP Initialization Process

**Location:** `kernel/src/arch/x86_64/smp/mod.rs`

The BSP brings up each AP using the INIT/SIPI sequence:

```rust
pub fn init_smp(madt_info: &MadtInfo) -> Result<usize, SmpError> {
    // 1. Setup trampoline code at 0x8000
    setup_trampoline();
    
    // 2. Identity map 0x0000-0x9FFF for real mode access
    identity_map_low_memory();
    
    // 3. For each AP in MADT:
    for cpu in &madt_info.cpus {
        if cpu.apic_id == bsp_apic_id { continue; }  // Skip BSP
        
        // 4. Allocate stack for AP (16KB)
        let stack = allocate_ap_stack();
        
        // 5. Write AP-specific data to trampoline
        write_trampoline_data(cpu.apic_id, stack, ap_entry64);
        
        // 6. Send INIT IPI to AP
        lapic.send_init_ipi(cpu.apic_id);
        sleep_ms(10);  // Wait 10ms
        
        // 7. Send SIPI twice (startup vector = 0x08 for address 0x8000)
        lapic.send_sipi(cpu.apic_id, 0x08);
        sleep_us(200);  // Wait 200μs
        lapic.send_sipi(cpu.apic_id, 0x08);
        
        // 8. Wait for AP to signal online (up to 100ms)
        wait_for_ap_online(cpu.apic_id, 100);
    }
}
```

**INIT/SIPI Sequence Details:**
- **INIT IPI**: Resets the target AP to known state
- **SIPI (Startup IPI)**: Provides starting address (0x8000 = vector 0x08)
- **Timing**: INIT → 10ms delay → SIPI → 200μs delay → SIPI
- **Retry Logic**: Send SIPI twice for reliability

**Expected Boot Log:**
```
[SMP] Trampoline copied to 0x8000
[SMP] Sending INIT to AP#1 (apic_id=1)
[SMP] Sending SIPI to AP#1 (vector=0x08)
[SMP] AP#1 online
[SMP] Sending INIT to AP#2 (apic_id=2)
[SMP] Sending SIPI to AP#2 (vector=0x08)
[SMP] AP#2 online
```

## Per-CPU Data Structures

### PerCpu Structure

**Location:** `kernel/src/arch/x86_64/smp/percpu.rs`

Each CPU core maintains its own data to minimize lock contention:

```rust
#[repr(C, align(64))]  // Cache line aligned to prevent false sharing
pub struct PerCpu {
    pub id: usize,                           // CPU ID (0, 1, 2, ...)
    pub apic_id: u8,                         // APIC ID from MADT
    pub node_id: u8,                         // NUMA node (future)
    pub runqueue: SpinLock<VecDeque<TaskId>>, // Per-core task queue
    pub current_task: Option<TaskId>,         // Currently running task
    pub idle_task: TaskId,                   // Idle task for this core
    pub lapic_timer_hz: u64,                 // Calibrated timer frequency
    pub ticks: AtomicU64,                    // Timer tick counter
    pub in_interrupt: bool,                  // Interrupt nesting flag
}

// Global array of per-CPU structures
static PERCPU_ARRAY: [PerCpu; MAX_CPUS] = ...;
```

### GS.BASE Setup

Each CPU uses the GS.BASE MSR to quickly access its PerCpu structure:

```rust
// MSR numbers
const MSR_GS_BASE: u32 = 0xC0000101;

// Set GS.BASE to point to this CPU's PerCpu structure
pub fn init_percpu(cpu_id: usize, apic_id: u8) {
    let percpu_ptr = &PERCPU_ARRAY[cpu_id] as *const PerCpu as u64;
    unsafe {
        wrmsr(MSR_GS_BASE, percpu_ptr);
    }
}

// Fast access to current CPU's data
pub fn percpu_current() -> &'static mut PerCpu {
    unsafe {
        let ptr: u64;
        asm!("mov {}, gs:0", out(reg) ptr);
        &mut *(ptr as *mut PerCpu)
    }
}
```

**Benefits:**
- **O(1) Access**: No array indexing or CPU ID lookup needed
- **Cache Friendly**: Each core accesses its own cache line
- **Lock-Free**: Most per-CPU data can be accessed without locks

## Multi-Core Scheduler

### Per-Core Runqueues

**Location:** `kernel/src/sched/mod.rs`

The scheduler maintains separate runqueues for each CPU core:

```rust
// Global scheduler state (shared data)
pub struct Scheduler {
    pub tasks: SpinLock<BTreeMap<TaskId, Task>>,  // Task table
    pub next_tid: AtomicUsize,                    // Next task ID
}

// Per-core runqueue (in PerCpu structure)
pub runqueue: SpinLock<VecDeque<TaskId>>
```

**Task Assignment Strategy:**
1. **New Tasks**: Assign to CPU with smallest runqueue
2. **Load Balancing**: Migrate tasks every 100ms if imbalance > 2 tasks
3. **Task Migration**: Move lowest-priority task from busy to idle CPU
4. **IPI Notification**: Send RESCHEDULE_IPI after migration

### Load Balancing Algorithm

**Location:** `kernel/src/sched/mod.rs`

```rust
pub fn balance_load() {
    let cpu_count = get_cpu_count();
    let mut queue_sizes = Vec::new();
    
    // 1. Collect runqueue sizes from all CPUs
    for cpu_id in 0..cpu_count {
        let percpu = percpu_for(cpu_id);
        let size = percpu.runqueue.lock().len();
        queue_sizes.push((cpu_id, size));
    }
    
    // 2. Find busiest and least busy CPUs
    queue_sizes.sort_by_key(|&(_, size)| size);
    let (idle_cpu, min_tasks) = queue_sizes[0];
    let (busy_cpu, max_tasks) = queue_sizes[cpu_count - 1];
    
    // 3. Migrate if imbalance > 2 tasks
    if max_tasks > min_tasks + 2 {
        migrate_task_between_cpus(busy_cpu, idle_cpu);
        send_reschedule_ipi(idle_cpu);
    }
}
```

**Migration Process:**
1. **Lock Ordering**: Always acquire locks in CPU ID order (lower first)
2. **Task Selection**: Move lowest-priority task from busy CPU
3. **Atomic Transfer**: Remove from source, add to destination
4. **IPI Notification**: Wake up target CPU to schedule new task

**Expected Scheduler Log:**
```
[SCHED] Created task A (priority=10)
[SCHED] Created task B (priority=5)
[SCHED] Created task C (priority=8)
[SCHED] Created task D (priority=3)
[SCHED][core0] run A
[SCHED][core1] run C
[SCHED][core2] run B
[SCHED][core3] run D
[SCHED] send RESCHED IPI → core1
[SCHED][core1] preempt C → run A
```

## Synchronization Primitives

### SpinLock Implementation

**Location:** `kernel/src/sync/spin.rs`

```rust
pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T> SpinLock<T> {
    pub fn lock(&self) -> SpinLockGuard<T> {
        let mut backoff = 1;
        
        // Exponential backoff to reduce bus contention
        while self.locked.compare_exchange_weak(
            false, true, Ordering::Acquire, Ordering::Relaxed
        ).is_err() {
            // Pause loop with exponential backoff
            for _ in 0..backoff {
                core::hint::spin_loop();
            }
            backoff = (backoff * 2).min(256);
        }
        
        SpinLockGuard { lock: self }
    }
}

// RAII guard for automatic unlock
pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}
```

### IRQ-Safe SpinLock

For data accessed from interrupt handlers:

```rust
pub struct IrqSpinLock<T> {
    inner: SpinLock<T>,
}

impl<T> IrqSpinLock<T> {
    pub fn lock(&self) -> IrqSpinLockGuard<T> {
        let flags = save_and_disable_interrupts();
        let guard = self.inner.lock();
        IrqSpinLockGuard { guard, flags }
    }
}

// Restores interrupt state on drop
impl<T> Drop for IrqSpinLockGuard<'_, T> {
    fn drop(&mut self) {
        restore_interrupts(self.flags);
    }
}
```

### Lock Ordering Rules

To prevent deadlocks, locks must be acquired in a specific order:

1. **Global before Local**: Global locks before per-object locks
2. **Task before Port**: Task locks before IPC port locks
3. **CPU ID Ordering**: Runqueue locks by CPU ID (lower first)
4. **No Multiple Runqueues**: Never hold multiple runqueue locks except during migration

**Example Safe Migration:**
```rust
fn migrate_task(task_id: TaskId, from_cpu: usize, to_cpu: usize) {
    let (first_cpu, second_cpu) = if from_cpu < to_cpu {
        (from_cpu, to_cpu)
    } else {
        (to_cpu, from_cpu)
    };
    
    // Always lock in CPU ID order
    let first_lock = percpu_for(first_cpu).runqueue.lock();
    let second_lock = percpu_for(second_cpu).runqueue.lock();
    
    // Perform migration...
}
```

## Inter-Processor Interrupts (IPIs)

### IPI Implementation

**Location:** `kernel/src/arch/x86_64/apic/ipi.rs`

IPIs enable cores to send interrupts to each other:

```rust
pub fn send_ipi(target_apic_id: u8, vector: u8) {
    let lapic = percpu_current().lapic;
    
    // 1. Write destination APIC ID to ICR high register
    lapic.write_register(LAPIC_ICR_HIGH, (target_apic_id as u32) << 24);
    
    // 2. Write vector and delivery mode to ICR low register
    let icr_low = vector as u32 | (0b000 << 8);  // Fixed delivery mode
    lapic.write_register(LAPIC_ICR_LOW, icr_low);
    
    // 3. Wait for delivery status to clear
    while (lapic.read_register(LAPIC_ICR_LOW) & (1 << 12)) != 0 {
        core::hint::spin_loop();
    }
}

pub fn broadcast_ipi(vector: u8, exclude_self: bool) {
    let current_apic_id = percpu_current().apic_id;
    
    for cpu_id in 0..get_cpu_count() {
        let target_apic_id = percpu_for(cpu_id).apic_id;
        
        if exclude_self && target_apic_id == current_apic_id {
            continue;
        }
        
        send_ipi(target_apic_id, vector);
    }
}
```

### IPI Vectors and Handlers

**Defined Vectors:**
- `RESCHEDULE_IPI (0x30)`: Trigger scheduler on target core
- `TLB_FLUSH_IPI (0x31)`: Flush TLB on target core (future)
- `HALT_IPI (0x32)`: Halt target core for shutdown (future)

**RESCHEDULE_IPI Handler:**
```rust
pub extern "x86-interrupt" fn reschedule_ipi_handler(_frame: InterruptStackFrame) {
    let cpu = percpu_current();
    
    // Send EOI to acknowledge interrupt
    cpu.lapic.eoi();
    
    // Trigger scheduler on this core
    schedule_on_core(cpu.id);
}
```

**Usage Examples:**
```rust
// Wake up specific core after task migration
send_reschedule_ipi(target_cpu_id);

// Emergency halt all cores
broadcast_ipi(HALT_IPI_VECTOR, true);
```

## SMP-Safe System Calls

### Syscall Synchronization Strategy

All Phase 4 syscalls are made SMP-safe without a global syscall lock:

**Per-Object Locking:**
- **Task Operations**: Each task has its own spinlock
- **IPC Operations**: Each port has its own spinlock  
- **Scheduler Operations**: Each CPU has its own runqueue lock
- **Memory Operations**: Page table operations use per-process locks

**No Global Bottlenecks:**
- **Syscall Dispatcher**: Read-only syscall table, no locking needed
- **Task Table**: Protected by fine-grained per-task locks
- **Port Manager**: Protected by per-port locks

### Cross-Core IPC

When tasks on different cores communicate via IPC:

```rust
// Task on Core 0 sends message to Task on Core 1
pub fn sys_ipc_send(port_id: usize, data: &[u8]) -> Result<usize, SyscallError> {
    let port = get_port(port_id)?;
    
    // 1. Acquire port lock (may be on different core)
    let mut port_guard = port.lock();
    
    // 2. Enqueue message
    port_guard.queue.push_back(Message::new(data));
    
    // 3. If tasks are blocked on this port, wake one
    if let Some(blocked_task_id) = port_guard.blocked_tasks.pop_front() {
        let task_cpu = get_task_cpu(blocked_task_id);
        
        // 4. Wake task on its assigned CPU
        wake_task_on_cpu(blocked_task_id, task_cpu);
        
        // 5. Send IPI to target CPU to run scheduler
        if task_cpu != current_cpu_id() {
            send_reschedule_ipi(task_cpu);
        }
    }
    
    Ok(0)
}
```

## Testing and Verification

### QEMU Test Configurations

**Basic SMP Test (2 CPUs):**
```bash
./tools/qemu-test-smp2.sh
# or
./tools/qemu.sh -smp 2 -enable-kvm
```

**Full SMP Test (4 CPUs):**
```bash
./tools/qemu-test-smp4.sh  
# or
./tools/qemu.sh -smp 4 -enable-kvm
```

**Automated Boot Test:**
```bash
./tools/test_boot.sh -smp 4 -timeout 10
```

### Expected Test Output

**Successful SMP Boot:**
```
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
[SCHED] Created task A (priority=10)
[SCHED] Created task B (priority=5)
[SCHED] Created task C (priority=8)
[SCHED] Created task D (priority=3)
[SCHED][core0] run A
[SCHED][core1] run C
[SCHED][core2] run B
[SCHED][core3] run D
```

### Verification Checklist

**CPU Detection:**
- [ ] All CPUs detected from MADT
- [ ] BSP APIC ID correctly identified
- [ ] AP APIC IDs match MADT entries

**AP Bootstrap:**
- [ ] All APs successfully brought online
- [ ] Each AP initializes its Local APIC
- [ ] Per-CPU data structures properly initialized
- [ ] GS.BASE MSR configured on each core

**Multi-Core Scheduling:**
- [ ] Tasks distributed across multiple cores
- [ ] Each core logs with correct core ID
- [ ] Load balancing occurs periodically
- [ ] RESCHEDULE_IPI sent and received

**Synchronization:**
- [ ] No deadlocks during concurrent operations
- [ ] SpinLocks properly protect shared data
- [ ] Cross-core IPC works correctly
- [ ] System remains stable under load

**Timer System:**
- [ ] Each core receives timer interrupts
- [ ] Tick counters increment independently
- [ ] Scheduler called on each timer tick
- [ ] Timer frequencies properly calibrated

## Performance Considerations

### Cache Line Alignment

```rust
#[repr(C, align(64))]  // 64-byte cache line alignment
pub struct PerCpu {
    // Frequently accessed fields first
    pub id: usize,
    pub current_task: Option<TaskId>,
    
    // Less frequently accessed fields
    pub apic_id: u8,
    pub node_id: u8,
    
    // Separate cache line for runqueue
    pub runqueue: SpinLock<VecDeque<TaskId>>,
}
```

**Benefits:**
- **False Sharing Prevention**: Each CPU's data in separate cache lines
- **Cache Locality**: Related data grouped together
- **Memory Bandwidth**: Reduced cache line bouncing between cores

### Lock Contention Reduction

**Strategies:**
- **Per-Core Data**: Minimize shared data structures
- **Fine-Grained Locking**: Use per-object locks instead of global locks
- **Lock-Free Algorithms**: Use atomics where possible
- **Exponential Backoff**: Reduce bus contention during lock acquisition

**Hot Path Optimization:**
- **Fast Path**: Common operations avoid locks when possible
- **Batch Operations**: Group multiple operations under single lock
- **IPI Batching**: Send multiple IPIs together when possible

### Memory Ordering

**Guidelines:**
- **Acquire/Release**: Use for most synchronization operations
- **SeqCst**: Only when strict ordering required (rare)
- **Relaxed**: For counters and statistics (non-critical)

```rust
// Typical spinlock memory ordering
self.locked.compare_exchange_weak(
    false, true, 
    Ordering::Acquire,    // Acquire semantics on success
    Ordering::Relaxed     // Relaxed on failure (retry)
)
```

## Troubleshooting

### Common Issues

**AP Boot Failures:**
- **Symptom**: APs don't come online, timeout errors
- **Causes**: Incorrect trampoline setup, identity mapping issues
- **Debug**: Check QEMU logs, verify trampoline code at 0x8000

**Triple Faults:**
- **Symptom**: System resets during AP boot
- **Causes**: Page table corruption, stack overflow
- **Debug**: Use `tools/qemu-debug-smp.sh` with QEMU monitor

**Deadlocks:**
- **Symptom**: System hangs, no progress
- **Causes**: Lock ordering violations, interrupt issues
- **Debug**: Add lock ordering assertions, check interrupt state

**Timer Issues:**
- **Symptom**: Scheduler not running, no task switches
- **Causes**: APIC timer misconfiguration, interrupt routing
- **Debug**: Verify timer interrupts, check LAPIC registers

### Debug Tools

**QEMU Monitor Commands:**
```bash
# Connect to QEMU monitor
telnet localhost 55555

# Useful commands
info registers -a          # Show all CPU registers
info mem                   # Show page table mappings  
x/10i $rip                 # Disassemble at RIP
info lapic                 # Show LAPIC state
info ioapic                # Show I/O APIC state
```

**Kernel Debug Output:**
```rust
// Add debug prints to track SMP progress
serial_println!("[DEBUG] CPU {} at checkpoint {}", cpu_id, checkpoint);
```

**Lock Debug Assertions:**
```rust
// Add to debug builds
debug_assert!(from_cpu < to_cpu, "Lock ordering violation");
```

## Future Enhancements

### Planned Features

1. **x2APIC Support**: Use MSR-based APIC for better performance
2. **NUMA Awareness**: Prefer local memory for per-CPU data
3. **CPU Hotplug**: Support adding/removing CPUs at runtime
4. **Advanced Load Balancing**: Consider task priority and CPU affinity
5. **TLB Shootdown**: Implement cross-core TLB invalidation
6. **Power Management**: Support CPU idle states (C-states)

### Performance Optimizations

1. **Lock-Free Runqueues**: Use atomic operations for task enqueue/dequeue
2. **Work Stealing**: Allow idle CPUs to steal tasks from busy CPUs
3. **CPU Affinity**: Pin tasks to specific CPUs for cache locality
4. **Interrupt Coalescing**: Batch timer interrupts to reduce overhead
5. **APIC Virtualization**: Use hardware virtualization features

### Scalability Improvements

1. **Per-NUMA Node Scheduling**: Separate schedulers for each NUMA node
2. **Hierarchical Load Balancing**: Balance within nodes first, then across nodes
3. **Distributed Synchronization**: Replace global locks with distributed algorithms
4. **Memory Pools**: Per-CPU memory allocators to reduce contention