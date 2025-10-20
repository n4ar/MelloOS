/// Per-CPU data structures
///
/// This module provides per-CPU data structures that allow each CPU core
/// to maintain its own state without requiring locks for access.
///
/// Each CPU core has its own PerCpu structure that is cache-line aligned
/// to prevent false sharing between cores.
use crate::config::MAX_CPUS;
use crate::sched::task::TaskId;
use crate::sync::SpinLock;
use core::sync::atomic::AtomicU64;

/// Maximum number of tasks per CPU runqueue
const MAX_RUNQUEUE_SIZE: usize = 64;

/// Simple circular queue for task IDs in per-CPU runqueue
///
/// This is a fixed-size queue that uses a circular buffer to store task IDs.
/// It's more efficient than a dynamic collection for kernel use.
pub struct RunQueue {
    tasks: [TaskId; MAX_RUNQUEUE_SIZE],
    head: usize,
    tail: usize,
    count: usize,
}

impl RunQueue {
    /// Create a new empty runqueue
    pub const fn new() -> Self {
        Self {
            tasks: [0; MAX_RUNQUEUE_SIZE],
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    /// Add a task to the back of the queue
    ///
    /// Returns true if successful, false if queue is full
    pub fn push_back(&mut self, task_id: TaskId) -> bool {
        if self.count >= MAX_RUNQUEUE_SIZE {
            return false;
        }

        self.tasks[self.tail] = task_id;
        self.tail = (self.tail + 1) % MAX_RUNQUEUE_SIZE;
        self.count += 1;
        true
    }

    /// Remove and return the task from the front of the queue
    ///
    /// Returns None if queue is empty
    pub fn pop_front(&mut self) -> Option<TaskId> {
        if self.count == 0 {
            return None;
        }

        let task_id = self.tasks[self.head];
        self.head = (self.head + 1) % MAX_RUNQUEUE_SIZE;
        self.count -= 1;
        Some(task_id)
    }

    /// Get the number of tasks in the queue
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Check if the queue is full
    pub fn is_full(&self) -> bool {
        self.count >= MAX_RUNQUEUE_SIZE
    }
}

/// Per-CPU data structure
///
/// This structure contains all data that is specific to a single CPU core.
/// It is cache-line aligned (64 bytes) to prevent false sharing between cores.
///
/// # Fields
/// * `id` - Logical CPU ID (0 for BSP, 1..N for APs)
/// * `apic_id` - APIC ID from MADT (may not be sequential)
/// * `node_id` - NUMA node ID (for future NUMA support)
/// * `runqueue` - Queue of tasks ready to execute on this core
/// * `current_task` - Currently executing task (None if idle)
/// * `idle_task` - Idle task for this core (runs when no other tasks are ready)
/// * `lapic_timer_hz` - Calibrated LAPIC timer frequency in Hz
/// * `ticks` - Number of timer ticks since boot
/// * `in_interrupt` - True if currently executing an interrupt handler
#[repr(C, align(64))]
pub struct PerCpu {
    /// Logical CPU ID (0 for BSP, 1..N for APs)
    pub id: usize,

    /// APIC ID from MADT
    pub apic_id: u8,

    /// NUMA node ID (for future NUMA support)
    pub node_id: u8,

    /// Runqueue for this CPU core
    pub runqueue: SpinLock<RunQueue>,

    /// Currently executing task (None if idle)
    pub current_task: Option<TaskId>,

    /// Idle task for this core
    pub idle_task: TaskId,

    /// Calibrated LAPIC timer frequency in Hz
    pub lapic_timer_hz: u64,

    /// Number of timer ticks since boot
    pub ticks: AtomicU64,

    /// True if currently executing an interrupt handler
    pub in_interrupt: bool,
}

impl PerCpu {
    /// Create a new uninitialized PerCpu structure
    ///
    /// This is used to initialize the static PERCPU_ARRAY.
    /// The actual initialization is done by init_percpu().
    const fn new_uninit() -> Self {
        PerCpu {
            id: 0,
            apic_id: 0,
            node_id: 0,
            runqueue: SpinLock::new(RunQueue::new()),
            current_task: None,
            idle_task: 0,
            lapic_timer_hz: 0,
            ticks: AtomicU64::new(0),
            in_interrupt: false,
        }
    }
}

/// Static array of per-CPU data structures
///
/// This array contains one PerCpu structure for each CPU core.
/// Each core accesses its own PerCpu structure using the GS.BASE MSR.
static mut PERCPU_ARRAY: [PerCpu; MAX_CPUS] = {
    const INIT: PerCpu = PerCpu::new_uninit();
    [INIT; MAX_CPUS]
};

/// Initialize a PerCpu structure for a specific CPU
///
/// This function initializes the PerCpu structure for the given CPU ID
/// with the provided APIC ID. It should be called once for each CPU
/// during the boot sequence.
///
/// # Arguments
/// * `cpu_id` - Logical CPU ID (0 for BSP, 1..N for APs)
/// * `apic_id` - APIC ID from MADT
///
/// # Safety
/// This function must be called exactly once per CPU during initialization.
/// It accesses the mutable static PERCPU_ARRAY.
pub unsafe fn init_percpu(cpu_id: usize, apic_id: u8) {
    // Debug: 'A' at start of init_percpu
    core::arch::asm!(
        "mov al, 'A'",
        "mov dx, 0x3F8",
        "out dx, al",
        options(nostack, nomem)
    );

    if cpu_id >= MAX_CPUS {
        panic!("[PERCPU] Invalid CPU ID: {}", cpu_id);
    }

    // Debug: 'B' after bounds check
    core::arch::asm!(
        "mov al, 'B'",
        "mov dx, 0x3F8",
        "out dx, al",
        options(nostack, nomem)
    );

    let percpu = &mut PERCPU_ARRAY[cpu_id];

    // Debug: 'C' after getting percpu reference
    core::arch::asm!(
        "mov al, 'C'",
        "mov dx, 0x3F8",
        "out dx, al",
        options(nostack, nomem)
    );

    // Test: Try reading the current value first
    let _test_read = percpu.id;
    core::arch::asm!(
        "mov al, 'X'",
        "mov dx, 0x3F8",
        "out dx, al",
        options(nostack, nomem)
    );

    // Test: Try a simple write to a local variable on stack
    let mut test_var: usize = 0;
    test_var = cpu_id;
    core::arch::asm!(
        "mov al, 'Y'",
        "mov dx, 0x3F8",
        "out dx, al",
        options(nostack, nomem)
    );

    // Now try the actual write
    percpu.id = cpu_id;
    core::arch::asm!(
        "mov al, 'E'",
        "mov dx, 0x3F8",
        "out dx, al",
        options(nostack, nomem)
    );

    percpu.apic_id = apic_id;
    core::arch::asm!(
        "mov al, 'F'",
        "mov dx, 0x3F8",
        "out dx, al",
        options(nostack, nomem)
    );

    percpu.node_id = 0;
    core::arch::asm!(
        "mov al, 'G'",
        "mov dx, 0x3F8",
        "out dx, al",
        options(nostack, nomem)
    );

    percpu.current_task = None;
    core::arch::asm!(
        "mov al, 'H'",
        "mov dx, 0x3F8",
        "out dx, al",
        options(nostack, nomem)
    );

    percpu.idle_task = 0;
    core::arch::asm!(
        "mov al, 'I'",
        "mov dx, 0x3F8",
        "out dx, al",
        options(nostack, nomem)
    );

    percpu.lapic_timer_hz = 0;
    core::arch::asm!(
        "mov al, 'J'",
        "mov dx, 0x3F8",
        "out dx, al",
        options(nostack, nomem)
    );

    percpu.ticks = AtomicU64::new(0);
    core::arch::asm!(
        "mov al, 'K'",
        "mov dx, 0x3F8",
        "out dx, al",
        options(nostack, nomem)
    );

    percpu.in_interrupt = false;
    core::arch::asm!(
        "mov al, 'L'",
        "mov dx, 0x3F8",
        "out dx, al",
        options(nostack, nomem)
    );

    // Debug: 'D' after setting all fields
    core::arch::asm!(
        "mov al, 'D'",
        "mov dx, 0x3F8",
        "out dx, al",
        options(nostack, nomem)
    );

    // Runqueue is already initialized by new_uninit()
}

/// Get a reference to the PerCpu structure for a specific CPU
///
/// This function returns a reference to the PerCpu structure for the
/// given CPU ID. It can be used to access another CPU's data.
///
/// # Arguments
/// * `cpu_id` - Logical CPU ID
///
/// # Returns
/// A reference to the PerCpu structure for the given CPU
///
/// # Panics
/// Panics if cpu_id >= MAX_CPUS
pub fn percpu_for(cpu_id: usize) -> &'static PerCpu {
    if cpu_id >= MAX_CPUS {
        panic!("[PERCPU] Invalid CPU ID: {}", cpu_id);
    }

    unsafe { &PERCPU_ARRAY[cpu_id] }
}

/// Get a mutable reference to the PerCpu structure for a specific CPU
///
/// This function returns a mutable reference to the PerCpu structure for
/// the given CPU ID. It should only be used during initialization or when
/// the caller has exclusive access to the CPU's data.
///
/// # Arguments
/// * `cpu_id` - Logical CPU ID
///
/// # Returns
/// A mutable reference to the PerCpu structure for the given CPU
///
/// # Safety
/// The caller must ensure that no other code is accessing this PerCpu
/// structure concurrently.
///
/// # Panics
/// Panics if cpu_id >= MAX_CPUS
pub unsafe fn percpu_for_mut(cpu_id: usize) -> &'static mut PerCpu {
    if cpu_id >= MAX_CPUS {
        panic!("[PERCPU] Invalid CPU ID: {}", cpu_id);
    }

    &mut PERCPU_ARRAY[cpu_id]
}

/// MSR number for GS.BASE
///
/// The GS.BASE MSR (0xC0000101) is used to store a pointer to the current
/// CPU's PerCpu structure. This allows fast access to per-CPU data without
/// requiring locks or atomic operations.
const MSR_GS_BASE: u32 = 0xC0000101;

/// Write a value to a Model-Specific Register (MSR)
///
/// # Arguments
/// * `msr` - MSR number
/// * `value` - 64-bit value to write
///
/// # Safety
/// This function writes to an MSR, which can affect system behavior.
/// The caller must ensure the MSR number is valid and the value is appropriate.
#[inline]
unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;

    core::arch::asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nostack, preserves_flags)
    );
}

/// Read a value from a Model-Specific Register (MSR)
///
/// # Arguments
/// * `msr` - MSR number
///
/// # Returns
/// The 64-bit value read from the MSR
///
/// # Safety
/// This function reads from an MSR. The caller must ensure the MSR number is valid.
#[inline]
unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;

    core::arch::asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nostack, preserves_flags)
    );

    ((high as u64) << 32) | (low as u64)
}

/// Configure GS.BASE MSR to point to the current CPU's PerCpu structure
///
/// This function sets the GS.BASE MSR to point to the PerCpu structure
/// for the given CPU ID. This allows percpu_current() to quickly access
/// the current CPU's data.
///
/// # Arguments
/// * `cpu_id` - Logical CPU ID
///
/// # Safety
/// This function must be called exactly once per CPU during initialization.
/// It writes to the GS.BASE MSR and accesses the mutable static PERCPU_ARRAY.
pub unsafe fn setup_gs_base(cpu_id: usize) {
    if cpu_id >= MAX_CPUS {
        panic!("[PERCPU] Invalid CPU ID: {}", cpu_id);
    }

    let percpu_ptr = &PERCPU_ARRAY[cpu_id] as *const PerCpu as u64;
    wrmsr(MSR_GS_BASE, percpu_ptr);
}

/// Get a reference to the current CPU's PerCpu structure
///
/// This function reads the GS.BASE MSR to get a pointer to the current
/// CPU's PerCpu structure. This is the fastest way to access per-CPU data.
///
/// # Returns
/// A reference to the current CPU's PerCpu structure
///
/// # Safety
/// This function assumes that GS.BASE has been properly initialized by
/// setup_gs_base(). If GS.BASE is not initialized, this will return an
/// invalid reference.
pub fn percpu_current() -> &'static PerCpu {
    unsafe {
        let percpu_ptr = rdmsr(MSR_GS_BASE) as *const PerCpu;
        &*percpu_ptr
    }
}

/// Get a mutable reference to the current CPU's PerCpu structure
///
/// This function reads the GS.BASE MSR to get a pointer to the current
/// CPU's PerCpu structure and returns a mutable reference.
///
/// # Returns
/// A mutable reference to the current CPU's PerCpu structure
///
/// # Safety
/// This function assumes that GS.BASE has been properly initialized by
/// setup_gs_base(). The caller must ensure that no other code is accessing
/// this PerCpu structure concurrently (typically by disabling interrupts).
pub unsafe fn percpu_current_mut() -> &'static mut PerCpu {
    let percpu_ptr = rdmsr(MSR_GS_BASE) as *mut PerCpu;
    &mut *percpu_ptr
}
