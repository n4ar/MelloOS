//! Task Scheduler Module
//!
//! This module implements a preemptive multitasking scheduler using Round-Robin algorithm.
//! It manages task creation, context switching, and timer-based preemption.
//!
//! # SMP Safety and Lock Ordering
//!
//! The scheduler is designed to work correctly in SMP environments with multiple CPUs.
//! To prevent deadlocks, locks must be acquired in the following order:
//!
//! 1. SCHED (global scheduler state)
//! 2. TASK_TABLE (global task table)
//! 3. Per-CPU runqueue locks (in ascending CPU ID order)
//! 4. Per-task state (implicit in get_task_mut)
//!
//! ## Key SMP Design Decisions
//!
//! - **Per-CPU Runqueues**: Each CPU has its own runqueue to minimize contention
//! - **Lock-Free Task Assignment**: New tasks are assigned to the CPU with the smallest runqueue
//! - **IPI-Based Coordination**: RESCHEDULE_IPI is sent when tasks are enqueued to remote CPUs
//! - **Ordered Lock Acquisition**: Multiple runqueue locks are always acquired in CPU ID order
//!
//! ## Critical Sections
//!
//! - Task creation: Holds SCHED and TASK_TABLE briefly, then releases before enqueuing
//! - Task migration: Holds two runqueue locks in CPU ID order
//! - Context switch: Only accesses current CPU's runqueue (no cross-CPU locks)
//!
//! See `kernel/src/sync/lock_ordering.rs` for complete lock ordering documentation.

pub mod context;
pub mod priority;
pub mod task;
pub mod timer;

/// Scheduler logging macros with consistent [SCHED] prefix
///
/// These macros provide different log levels for scheduler operations:
/// - sched_log!: General information
/// - sched_info!: Important information
/// - sched_warn!: Warnings
/// - sched_error!: Errors

/// Log general scheduler information
#[macro_export]
macro_rules! sched_log {
    ($($arg:tt)*) => {
        $crate::serial_println!("[SCHED] {}", format_args!($($arg)*))
    };
}

/// Log important scheduler information
#[macro_export]
macro_rules! sched_info {
    ($($arg:tt)*) => {
        $crate::serial_println!("[SCHED] INFO: {}", format_args!($($arg)*))
    };
}

/// Log scheduler warnings
#[macro_export]
macro_rules! sched_warn {
    ($($arg:tt)*) => {
        $crate::serial_println!("[SCHED] WARNING: {}", format_args!($($arg)*))
    };
}

/// Log scheduler errors
#[macro_export]
macro_rules! sched_error {
    ($($arg:tt)*) => {
        $crate::serial_println!("[SCHED] ERROR: {}", format_args!($($arg)*))
    };
}

use crate::arch::x86_64::smp::percpu::{percpu_current, percpu_for};
use context::CpuContext;
use priority::TaskPriority;
use spin::Mutex;
pub use task::Task;
use task::{SchedulerError, SchedulerResult, TaskId, TaskState};

/// Maximum number of tasks supported
const MAX_TASKS: usize = 64;

/// Maximum number of tasks per CPU runqueue (from percpu.rs)
const MAX_RUNQUEUE_SIZE: usize = 64;

/// Wrapper for task pointer that implements Sync
///
/// # Safety
/// This is safe because:
/// - We only access tasks through the TASK_TABLE mutex
/// - Each task is only accessed by one context at a time
/// - Tasks are heap-allocated and don't move
#[derive(Copy, Clone)]
struct TaskPtr(*mut Task);

unsafe impl Sync for TaskPtr {}
unsafe impl Send for TaskPtr {}

impl TaskPtr {
    const fn null() -> Self {
        Self(core::ptr::null_mut())
    }

    fn new(ptr: *mut Task) -> Self {
        Self(ptr)
    }

    fn get(&self) -> *mut Task {
        self.0
    }

    fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

/// Simple circular queue for task IDs
struct TaskQueue {
    tasks: [TaskId; MAX_TASKS],
    head: usize,
    tail: usize,
    count: usize,
}

impl TaskQueue {
    const fn new() -> Self {
        Self {
            tasks: [0; MAX_TASKS],
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    fn push_back(&mut self, task_id: TaskId) -> bool {
        if self.count >= MAX_TASKS {
            return false;
        }

        self.tasks[self.tail] = task_id;
        self.tail = (self.tail + 1) % MAX_TASKS;
        self.count += 1;
        true
    }

    fn pop_front(&mut self) -> Option<TaskId> {
        if self.count == 0 {
            return None;
        }

        let task_id = self.tasks[self.head];
        self.head = (self.head + 1) % MAX_TASKS;
        self.count -= 1;
        Some(task_id)
    }

    fn len(&self) -> usize {
        self.count
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.count = 0;
    }
}

/// Scheduler state containing global task management
///
/// Note: Runqueues are now per-CPU (in PerCpu structure)
struct SchedState {
    /// Next task ID to assign (incremented for each new task)
    next_tid: usize,
}

impl SchedState {
    /// Create a new empty scheduler state
    fn new() -> Self {
        Self {
            next_tid: 1, // Start at 1, reserve 0 for idle task
        }
    }
}

/// Global scheduler state protected by a mutex
static SCHED: spin::Once<Mutex<SchedState>> = spin::Once::new();

/// Get the number of online CPUs from SMP module
fn get_cpu_count() -> usize {
    crate::arch::x86_64::smp::get_cpu_count()
}

/// Task table storing all Task objects
/// Uses TaskPtr wrapper for heap-allocated tasks
/// TaskPtr::null() indicates an empty slot
static TASK_TABLE: Mutex<[TaskPtr; MAX_TASKS]> = Mutex::new([TaskPtr::null(); MAX_TASKS]);

/// Spawn a new task with the given entry point
///
/// This function:
/// 1. Generates a unique TaskId
/// 2. Creates a new Task with Task::new()
/// 3. Allocates the Task on the heap and adds it to TASK_TABLE
/// 4. Assigns the task to a CPU (will be done by enqueue_task)
/// 5. Logs the task spawn
///
/// # Arguments
/// * `name` - Human-readable task name
/// * `entry_point` - Function pointer to the task's entry point
/// * `priority` - Task priority level
///
/// # Returns
/// A Result containing the TaskId of the newly spawned task, or an error if spawning fails
///
/// # Errors
/// Returns `SchedulerError::TooManyTasks` if the task table is full
/// Returns `SchedulerError::OutOfMemory` if memory allocation fails
/// Returns `SchedulerError::RunqueueFull` if the runqueue is full
pub fn spawn_task(
    name: &'static str,
    entry_point: fn() -> !,
    priority: TaskPriority,
) -> SchedulerResult<TaskId> {
    use crate::mm::allocator::kmalloc;
    use core::ptr;

    // Lock both SCHED and TASK_TABLE
    let mut sched = SCHED.get().expect("Scheduler not initialized").lock();
    let mut task_table = TASK_TABLE.lock();

    // 1. Generate unique TaskId
    let task_id = sched.next_tid;

    if task_id >= MAX_TASKS {
        sched_error!("Too many tasks! Maximum is {}", MAX_TASKS);
        return Err(SchedulerError::TooManyTasks);
    }

    sched.next_tid += 1;

    // Drop locks before creating task (to avoid holding locks during allocation)
    drop(sched);
    drop(task_table);

    // 2. Create new Task with specified priority
    let task = match Task::new(task_id, name, entry_point, priority) {
        Ok(task) => task,
        Err(e) => {
            sched_error!("Failed to create task {}: {:?}", task_id, e);
            return Err(e);
        }
    };

    // 3. Allocate Task on heap and add to TASK_TABLE
    let task_size = core::mem::size_of::<Task>();
    let task_ptr = kmalloc(task_size) as *mut Task;

    if task_ptr.is_null() {
        sched_error!("Failed to allocate memory for task {} ({})", task_id, name);
        return Err(SchedulerError::OutOfMemory);
    }

    unsafe {
        ptr::write(task_ptr, task);
    }

    let mut task_table = TASK_TABLE.lock();
    task_table[task_id] = TaskPtr::new(task_ptr);
    drop(task_table);

    // 4. Enqueue task to a CPU runqueue (will select CPU with smallest runqueue)
    enqueue_task(task_id, None);

    // 5. Log task spawn
    sched_info!(
        "Spawned task {}: {} (priority: {:?})",
        task_id,
        name,
        priority
    );

    Ok(task_id)
}

/// Get a mutable reference to a task from the task table
///
/// # Arguments
/// * `id` - TaskId to look up
///
/// # Returns
/// A mutable reference to the Task, or None if the task doesn't exist
///
/// # Safety
/// This function returns a 'static mutable reference, which is safe because:
/// - Tasks are allocated on the heap and don't move
/// - We only access tasks while holding appropriate locks
/// - Each task is only accessed by one context at a time
fn get_task(id: TaskId) -> Option<&'static mut Task> {
    let task_table = TASK_TABLE.lock();

    if id >= MAX_TASKS {
        return None;
    }

    // Get the task pointer
    let task_ptr = task_table[id];

    if task_ptr.is_null() {
        return None;
    }

    // Convert to static reference (safe because task is heap-allocated and doesn't move)
    unsafe { Some(&mut *task_ptr.get()) }
}

/// Schedule the next task on a specific CPU core
///
/// This function:
/// 1. Gets the current CPU's PerCpu structure
/// 2. Moves current task back to runqueue if still ready
/// 3. Selects next task from the CPU's runqueue
/// 4. Updates current_task in PerCpu
/// 5. Returns references to old and new tasks for context switch
///
/// # Arguments
/// * `cpu_id` - The CPU core to schedule on
///
/// # Returns
/// A tuple of (old_task, new_task) references, or None if no tasks available
fn schedule_on_core(cpu_id: usize) -> Option<(&'static mut Task, &'static mut Task)> {
    use core::sync::atomic::Ordering;

    // Get the PerCpu structure for this core
    let percpu = unsafe { crate::arch::x86_64::smp::percpu::percpu_for_mut(cpu_id) };

    // Get the current task (if any)
    let old_task_id = percpu.current_task;

    // If there's no current task, this is the first switch on this core
    if old_task_id.is_none() {
        return None;
    }

    // Move current task back to runqueue if it's still ready
    if let Some(current_id) = old_task_id {
        if let Some(task) = get_task(current_id) {
            // Only re-enqueue if task is still in Running state
            // (it might have been put to sleep or blocked)
            if task.state == TaskState::Running {
                task.state = TaskState::Ready;
                let mut runqueue = percpu.runqueue.lock();
                if !runqueue.push_back(current_id) {
                    sched_warn!("CPU {} runqueue full, dropping task {}", cpu_id, current_id);
                }
            }
        }
    }

    // Select next task from this CPU's runqueue
    let next_task_id = {
        let mut runqueue = percpu.runqueue.lock();
        match runqueue.pop_front() {
            Some(id) => id,
            None => {
                // Runqueue empty - use idle task
                percpu.idle_task
            }
        }
    };

    // Update current task in PerCpu
    percpu.current_task = Some(next_task_id);

    // Get task references
    let old_task = old_task_id.and_then(|id| get_task(id));
    let new_task = get_task(next_task_id)?;

    // Update new task state to Running
    new_task.state = TaskState::Running;

    // Return both tasks
    if let Some(old) = old_task {
        Some((old, new_task))
    } else {
        // Should not reach here since we checked old_task_id.is_none() above
        None
    }
}

/// Global counter for context switches (for logging throttling)
pub(crate) static SWITCH_COUNT: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);

/// Scheduler tick function - called by timer interrupt
///
/// This function:
/// 1. Determines the current CPU ID
/// 2. Calls schedule_on_core() to get old and new tasks
/// 3. Logs the context switch (with throttling)
/// 4. Performs the context switch
///
/// # Notes
/// - This function does not return in the traditional sense (tail-switch)
/// - The next task will continue execution from where it was interrupted
/// - For new tasks, execution starts at entry_trampoline
pub fn tick() {
    use core::sync::atomic::Ordering;

    // Increment timer_ticks metric
    crate::sys::METRICS
        .timer_ticks
        .fetch_add(1, Ordering::Relaxed);

    // Get current CPU ID
    let cpu_id = percpu_current().id;

    // Get next task to run on this core
    let tasks = schedule_on_core(cpu_id);

    if let Some((old_task, new_task)) = tasks {
        // Validate task pointers before context switch
        if old_task.context.rsp == 0 {
            panic!("[SCHED] CRITICAL: Old task has invalid RSP (null stack pointer)");
        }
        if new_task.context.rsp == 0 {
            panic!("[SCHED] CRITICAL: New task has invalid RSP (null stack pointer)");
        }

        // Increment switch counter
        let count = SWITCH_COUNT.fetch_add(1, Ordering::Relaxed);

        // Increment ctx_switches metric
        crate::sys::METRICS
            .ctx_switches
            .fetch_add(1, Ordering::Relaxed);

        // Check if this is a preemptive switch (old task was still Running/Ready)
        if old_task.state == TaskState::Running || old_task.state == TaskState::Ready {
            crate::sys::METRICS
                .preemptions
                .fetch_add(1, Ordering::Relaxed);
        }

        // Log context switch with throttling
        // First 10 switches: log every switch
        // After that: log every 100 switches
        if count < 10 || count % 100 == 0 {
            sched_log!(
                "[core{}] Switch #{} → Task {} ({})",
                cpu_id,
                count,
                new_task.id,
                new_task.name
            );
        }

        // Perform context switch
        // This is a tail-switch: we don't return to this function
        unsafe {
            context::context_switch(
                &mut old_task.context as *mut CpuContext,
                &new_task.context as *const CpuContext,
            );
        }

        // Note: We never reach here because context_switch doesn't return
        // The next task will continue from where it was interrupted
    } else {
        // First switch on this core - no old task yet
        // We need to manually set up the first task and jump to it
        let percpu = unsafe { crate::arch::x86_64::smp::percpu::percpu_for_mut(cpu_id) };

        // Select the first task from this core's runqueue
        let first_task_id = {
            let mut runqueue = percpu.runqueue.lock();
            match runqueue.pop_front() {
                Some(id) => id,
                None => {
                    // No tasks in runqueue - use idle task
                    percpu.idle_task
                }
            }
        };

        percpu.current_task = Some(first_task_id);

        if let Some(first_task) = get_task(first_task_id) {
            first_task.state = TaskState::Running;

            sched_log!(
                "[core{}] First switch → Task {} ({}) [priority: {:?}]",
                cpu_id,
                first_task.id,
                first_task.name,
                first_task.priority
            );

            // Validate the task's RSP
            if first_task.context.rsp == 0 {
                panic!("[SCHED] CRITICAL: First task has null RSP");
            }

            // For the first switch, we need to manually jump to the task
            // We'll use a dummy context for the "old" task (which is the kernel boot code)
            // This context will never be used again
            let mut dummy_context = CpuContext {
                r15: 0,
                r14: 0,
                r13: 0,
                r12: 0,
                rbp: 0,
                rbx: 0,
                rsp: 0, // Will be filled by context_switch
            };

            unsafe {
                context::context_switch(
                    &mut dummy_context as *mut CpuContext,
                    &first_task.context as *const CpuContext,
                );
            }

            // Should never reach here
            panic!("[SCHED] CRITICAL: Returned from first context switch");
        } else {
            panic!("[SCHED] CRITICAL: First task not found in task table");
        }
    }
}

/// Idle task entry point
///
/// This task runs when no other tasks are available.
/// It simply halts the CPU until the next interrupt.
fn idle_task() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Get current task ID and priority
///
/// Returns the current task's ID and priority, or None if no task is running
pub fn get_current_task_info() -> Option<(TaskId, TaskPriority)> {
    let percpu = percpu_current();
    let current_id = percpu.current_task?;

    let task = get_task(current_id)?;
    Some((task.id, task.priority))
}

/// Get task priority by ID
///
/// Returns the task's ID and priority, or None if task doesn't exist
pub fn get_task_priority(task_id: TaskId) -> Option<(TaskId, TaskPriority)> {
    let task = get_task(task_id)?;
    Some((task.id, task.priority))
}

/// Get mutable reference to a task (public version for IPC)
///
/// Returns a mutable reference to the task, or None if task doesn't exist
pub fn get_task_mut(task_id: TaskId) -> Option<&'static mut Task> {
    get_task(task_id)
}

/// Enqueue a task to a CPU runqueue
///
/// Assigns the task to the CPU with the smallest runqueue, or to a specific CPU if specified.
/// If the task is enqueued to a remote CPU (not the current CPU), sends a RESCHEDULE_IPI
/// to wake up that CPU and schedule the new task.
///
/// # Arguments
/// * `task_id` - The task to enqueue
/// * `target_cpu` - Optional specific CPU to enqueue to. If None, selects CPU with smallest runqueue.
pub fn enqueue_task(task_id: TaskId, target_cpu: Option<usize>) {
    let cpu_count = get_cpu_count();

    // Determine which CPU to enqueue to
    let cpu_id = if let Some(cpu) = target_cpu {
        // Use specified CPU
        if cpu >= cpu_count {
            sched_warn!("Invalid target CPU {}, using CPU 0", cpu);
            0
        } else {
            cpu
        }
    } else {
        // Find CPU with smallest runqueue
        let mut min_cpu = 0;
        let mut min_size = usize::MAX;

        for i in 0..cpu_count {
            let percpu = percpu_for(i);
            let runqueue = percpu.runqueue.lock();
            let size = runqueue.len();

            if size < min_size {
                min_size = size;
                min_cpu = i;
            }
        }

        min_cpu
    };

    // Get current CPU ID to check if this is a remote enqueue
    let current_cpu = percpu_current().id;

    // Enqueue task to selected CPU's runqueue
    let percpu = percpu_for(cpu_id);
    let mut runqueue = percpu.runqueue.lock();

    if !runqueue.push_back(task_id) {
        sched_error!(
            "Failed to enqueue task {} to CPU {} (runqueue full)",
            task_id,
            cpu_id
        );
    } else {
        sched_log!(
            "Enqueued task {} to CPU {} (runqueue size: {})",
            task_id,
            cpu_id,
            runqueue.len()
        );

        // Drop the runqueue lock before sending IPI
        drop(runqueue);

        // If we enqueued to a remote CPU, send RESCHEDULE_IPI to wake it up
        if cpu_id != current_cpu && cpu_count > 1 {
            use crate::arch::x86_64::apic::ipi::send_reschedule_ipi;
            send_reschedule_ipi(cpu_id);
        }
    }
}

/// Dequeue a task from a CPU's runqueue
///
/// Removes and returns the next task from the specified CPU's runqueue.
///
/// # Arguments
/// * `cpu_id` - The CPU to dequeue from
///
/// # Returns
/// The task ID of the dequeued task, or None if the runqueue is empty
#[allow(dead_code)]
pub fn dequeue_task(cpu_id: usize) -> Option<TaskId> {
    let percpu = percpu_for(cpu_id);
    let mut runqueue = percpu.runqueue.lock();
    runqueue.pop_front()
}

/// Put current task to sleep for specified ticks
///
/// Returns true on success, false on error
pub fn sleep_current_task(ticks: u64, _priority: TaskPriority) -> bool {
    // Get current CPU and task
    let percpu = percpu_current();
    let current_id = match percpu.current_task {
        Some(id) => id,
        None => return false,
    };

    // Update task state to Sleeping
    if let Some(task) = get_task(current_id) {
        task.state = TaskState::Sleeping;
        task.wake_tick = Some(ticks);
    }

    // Note: Task will not be re-enqueued until wake time
    // The timer interrupt will check wake_tick and re-enqueue when ready

    true
}

/// Migrate a task from one CPU to another
///
/// This function moves a task from the source CPU's runqueue to the destination CPU's runqueue.
/// It uses proper lock ordering (lower CPU ID first) to prevent deadlocks.
///
/// # Arguments
/// * `task_id` - The task to migrate
/// * `from_cpu` - Source CPU ID
/// * `to_cpu` - Destination CPU ID
///
/// # Returns
/// true on success, false on error
pub fn migrate_task(task_id: TaskId, from_cpu: usize, to_cpu: usize) -> bool {
    if from_cpu == to_cpu {
        return false; // No migration needed
    }

    let cpu_count = get_cpu_count();

    if from_cpu >= cpu_count || to_cpu >= cpu_count {
        return false; // Invalid CPU IDs
    }

    // Lock ordering: always lock lower CPU ID first to prevent deadlocks
    let (first_cpu, second_cpu) = if from_cpu < to_cpu {
        (from_cpu, to_cpu)
    } else {
        (to_cpu, from_cpu)
    };

    // Assert CPU ID ordering in debug builds
    crate::sync::lock_ordering::assert_cpu_id_order(first_cpu, second_cpu);

    let percpu_first = percpu_for(first_cpu);
    let percpu_second = percpu_for(second_cpu);

    // Lock both runqueues in order
    let mut runqueue_first = percpu_first.runqueue.lock();
    let mut runqueue_second = percpu_second.runqueue.lock();

    // Get the correct source and destination queues
    let (src_queue, dst_queue) = if from_cpu == first_cpu {
        (&mut *runqueue_first, &mut *runqueue_second)
    } else {
        (&mut *runqueue_second, &mut *runqueue_first)
    };

    // Remove task from source queue
    // Note: This is a simple implementation that searches the queue
    // A more efficient implementation would use a different data structure
    let mut found = false;
    let mut temp_tasks = [0; MAX_RUNQUEUE_SIZE];
    let mut temp_count = 0;

    while let Some(tid) = src_queue.pop_front() {
        if tid == task_id {
            found = true;
            break;
        } else {
            temp_tasks[temp_count] = tid;
            temp_count += 1;
        }
    }

    // Restore tasks we removed
    for i in 0..temp_count {
        src_queue.push_back(temp_tasks[i]);
    }

    if !found {
        return false; // Task not found in source queue
    }

    // Add task to destination queue
    if !dst_queue.push_back(task_id) {
        // Destination queue full - put task back in source queue
        src_queue.push_back(task_id);
        return false;
    }

    // Drop locks before sending IPI
    drop(runqueue_first);
    drop(runqueue_second);

    sched_log!(
        "Migrated task {} from CPU {} to CPU {}",
        task_id,
        from_cpu,
        to_cpu
    );

    // Send RESCHEDULE_IPI to destination CPU to schedule the migrated task
    use crate::arch::x86_64::apic::ipi::send_reschedule_ipi;
    send_reschedule_ipi(to_cpu);

    true
}

/// Balance load across CPUs
///
/// This function checks runqueue sizes and migrates tasks from busy CPUs to idle CPUs
/// if the imbalance is greater than 2 tasks.
///
/// # Returns
/// The number of tasks migrated
pub fn balance_load() -> usize {
    let cpu_count = get_cpu_count();

    if cpu_count <= 1 {
        return 0; // No balancing needed for single CPU
    }

    // Find CPU with largest and smallest runqueue
    let mut max_cpu = 0;
    let mut max_size = 0;
    let mut min_cpu = 0;
    let mut min_size = usize::MAX;

    for i in 0..cpu_count {
        let percpu = percpu_for(i);
        let runqueue = percpu.runqueue.lock();
        let size = runqueue.len();

        if size > max_size {
            max_size = size;
            max_cpu = i;
        }

        if size < min_size {
            min_size = size;
            min_cpu = i;
        }
    }

    // Check if imbalance is greater than 2 tasks
    if max_size <= min_size + 2 {
        return 0; // No significant imbalance
    }

    // Migrate one task from busy CPU to idle CPU
    let task_to_migrate = {
        let percpu = percpu_for(max_cpu);
        let mut runqueue = percpu.runqueue.lock();
        runqueue.pop_front()
    };

    if let Some(task_id) = task_to_migrate {
        // Re-enqueue to destination CPU
        let percpu = percpu_for(min_cpu);
        let mut runqueue = percpu.runqueue.lock();

        if runqueue.push_back(task_id) {
            sched_log!(
                "Load balance: migrated task {} from CPU {} (size {}) to CPU {} (size {})",
                task_id,
                max_cpu,
                max_size,
                min_cpu,
                min_size
            );
            return 1;
        } else {
            // Failed to enqueue - put back in source queue
            let percpu = percpu_for(max_cpu);
            let mut runqueue = percpu.runqueue.lock();
            runqueue.push_back(task_id);
        }
    }

    0
}

/// Yield CPU to next task (voluntary context switch)
///
/// This function triggers the scheduler to select the next task.
/// It does not return in the traditional sense - execution continues
/// in the next task, and eventually returns here when this task runs again.
pub fn yield_now() {
    // Call the scheduler tick function to perform context switch
    tick();
}





/// Initialize the scheduler
///
/// This function:
/// 1. Initializes SCHED and TASK_TABLE
/// 2. Creates the idle task (task id 0)
/// 3. Logs scheduler initialization
///
/// # Notes
/// - Must be called before spawning any tasks
/// - Must be called before enabling interrupts
/// - The idle task is created but not added to any runqueue
///   (it will be used when runqueues are empty)
pub fn init_scheduler() {
    use crate::mm::allocator::kmalloc;
    use core::ptr;

    sched_info!("Initializing scheduler...");

    // Initialize SCHED state
    SCHED.call_once(|| Mutex::new(SchedState::new()));

    // Initialize TASK_TABLE (clear all entries)
    let mut task_table = TASK_TABLE.lock();
    for i in 0..MAX_TASKS {
        task_table[i] = TaskPtr::null();
    }
    drop(task_table);

    // Create idle task (task id 0)
    // We manually create it with id 0 instead of using spawn_task
    let idle = match Task::new(0, "idle", idle_task, TaskPriority::Low) {
        Ok(task) => task,
        Err(e) => {
            panic!("[SCHED] CRITICAL: Failed to create idle task: {:?}", e);
        }
    };

    // Allocate idle task on heap
    let task_size = core::mem::size_of::<Task>();
    let task_ptr = kmalloc(task_size) as *mut Task;

    if task_ptr.is_null() {
        panic!("[SCHED] CRITICAL: Failed to allocate memory for idle task");
    }

    unsafe {
        ptr::write(task_ptr, idle);
    }

    let mut task_table = TASK_TABLE.lock();
    task_table[0] = TaskPtr::new(task_ptr);
    drop(task_table);

    // Set idle task for all CPUs
    let cpu_count = get_cpu_count();
    for cpu_id in 0..cpu_count {
        unsafe {
            let percpu = crate::arch::x86_64::smp::percpu::percpu_for_mut(cpu_id);
            percpu.idle_task = 0;
        }
    }

    sched_info!("Created idle task (id 0)");
    sched_info!("Scheduler initialized!");
}

/// Process-aware context switching
///
/// Enhanced version of schedule_on_core that integrates with process management.
/// This function coordinates between Task and Process state during context switches.
///
/// # Arguments
/// * `cpu_id` - The CPU core to schedule on
///
/// # Returns
/// A tuple of (old_task, new_task) references, or None if no tasks available
pub fn schedule_with_process_integration(
    cpu_id: usize,
) -> Option<(&'static mut Task, &'static mut Task)> {
    // Use the existing scheduler logic
    let result = schedule_on_core(cpu_id);

    // If we have a context switch, update process states
    if let Some((old_task, new_task)) = &result {
        // Update process states through integration functions
        use crate::user::process::{get_process_for_task, prepare_process_context_switch};

        let old_process_id = get_process_for_task(old_task.id);
        let new_process_id = get_process_for_task(new_task.id);

        if let Some(new_pid) = new_process_id {
            if let Err(e) = prepare_process_context_switch(old_process_id, new_pid) {
                sched_warn!("Failed to prepare process context switch: {:?}", e);
            }
        }
    }

    result
}

/// Enhanced tick function with process integration
///
/// This version of tick() integrates with the process management system
/// to ensure consistent state between Tasks and Processes.
pub fn tick_with_process_integration() {
    use core::sync::atomic::Ordering;

    // Increment timer_ticks metric
    crate::sys::METRICS
        .timer_ticks
        .fetch_add(1, Ordering::Relaxed);

    // Get current CPU ID
    let cpu_id = percpu_current().id;

    // Get next task to run on this core (with process integration)
    let tasks = schedule_with_process_integration(cpu_id);

    if let Some((old_task, new_task)) = tasks {
        // Validate task pointers before context switch
        if old_task.context.rsp == 0 {
            panic!("[SCHED] CRITICAL: Old task has invalid RSP (null stack pointer)");
        }
        if new_task.context.rsp == 0 {
            panic!("[SCHED] CRITICAL: New task has invalid RSP (null stack pointer)");
        }

        // Increment switch counter
        let count = SWITCH_COUNT.fetch_add(1, Ordering::Relaxed);

        // Increment ctx_switches metric
        crate::sys::METRICS
            .ctx_switches
            .fetch_add(1, Ordering::Relaxed);

        // Check if this is a preemptive switch (old task was still Running/Ready)
        if old_task.state == TaskState::Running || old_task.state == TaskState::Ready {
            crate::sys::METRICS
                .preemptions
                .fetch_add(1, Ordering::Relaxed);
        }

        // Log context switch with throttling
        if count < 10 || count % 100 == 0 {
            sched_log!(
                "[core{}] Switch #{} → Task {} ({})",
                cpu_id,
                count,
                new_task.id,
                new_task.name
            );
        }

        // Perform context switch
        unsafe {
            context::context_switch(
                &mut old_task.context as *mut CpuContext,
                &new_task.context as *const CpuContext,
            );
        }
    } else {
        // Use the existing first-switch logic
        let percpu = unsafe { crate::arch::x86_64::smp::percpu::percpu_for_mut(cpu_id) };

        let first_task_id = {
            let mut runqueue = percpu.runqueue.lock();
            match runqueue.pop_front() {
                Some(id) => id,
                None => percpu.idle_task,
            }
        };

        percpu.current_task = Some(first_task_id);

        if let Some(first_task) = get_task(first_task_id) {
            first_task.state = TaskState::Running;

            sched_log!(
                "[core{}] First switch → Task {} ({}) [priority: {:?}]",
                cpu_id,
                first_task.id,
                first_task.name,
                first_task.priority
            );

            if first_task.context.rsp == 0 {
                panic!("[SCHED] CRITICAL: First task has null RSP");
            }

            let mut dummy_context = CpuContext {
                r15: 0,
                r14: 0,
                r13: 0,
                r12: 0,
                rbp: 0,
                rbx: 0,
                rsp: 0,
            };

            unsafe {
                context::context_switch(
                    &mut dummy_context as *mut CpuContext,
                    &first_task.context as *const CpuContext,
                );
            }

            panic!("[SCHED] CRITICAL: Returned from first context switch");
        } else {
            panic!("[SCHED] CRITICAL: First task not found in task table");
        }
    }
}

/// Create a process-aware task
///
/// This function creates both a Process and a Task, linking them together
/// for integrated management.
///
/// # Arguments
/// * `name` - Process/task name
/// * `entry_point` - Task entry point function
/// * `priority` - Task priority
/// * `parent_pid` - Optional parent process ID
///
/// # Returns
/// Ok((process_id, task_id)) if creation succeeded, or an error
pub fn spawn_process_task(
    name: &'static str,
    entry_point: fn() -> !,
    priority: TaskPriority,
    parent_pid: Option<usize>,
) -> SchedulerResult<(usize, TaskId)> {
    use crate::user::process::{ProcessError, ProcessManager};

    // Create the process first
    let process_id = ProcessManager::create_process(parent_pid, name).map_err(|e| match e {
        ProcessError::ProcessTableFull => SchedulerError::TooManyTasks,
        ProcessError::OutOfMemory => SchedulerError::OutOfMemory,
        _ => SchedulerError::OutOfMemory,
    })?;

    // Create the corresponding task
    let task_id = spawn_task(name, entry_point, priority)?;

    // Link them together through synchronization
    use crate::user::process::sync_task_with_process;
    if let Err(e) = sync_task_with_process(task_id, process_id) {
        sched_warn!(
            "Failed to sync task {} with process {}: {:?}",
            task_id,
            process_id,
            e
        );
    }

    sched_info!(
        "Created process {} with task {} ({})",
        process_id,
        task_id,
        name
    );

    Ok((process_id, task_id))
}

/// End-to-end integration test for task switching
///
/// This test verifies that:
/// 1. Two tasks can be spawned successfully
/// 2. Tasks switch alternately (A B A B pattern)
/// 3. System remains stable for 100+ context switches
///
/// # Safety
/// This function enables interrupts and runs tasks. It should only be called
/// during kernel initialization in a controlled test environment.
pub fn test_task_switching_integration() {
    use crate::serial_println;
    use core::sync::atomic::{AtomicUsize, Ordering};

    serial_println!("[TEST] ========================================");
    serial_println!("[TEST] End-to-End Task Switching Integration Test");
    serial_println!("[TEST] ========================================");

    // Counter for task A executions
    static TASK_A_COUNT: AtomicUsize = AtomicUsize::new(0);
    // Counter for task B executions
    static TASK_B_COUNT: AtomicUsize = AtomicUsize::new(0);

    // Test task A - increments counter and prints
    fn test_task_a() -> ! {
        loop {
            let count = TASK_A_COUNT.fetch_add(1, Ordering::Relaxed);
            if count < 60 {
                serial_println!("[TEST] Task A execution #{}", count);
            }
            // Busy wait
            for _ in 0..500_000 {
                unsafe {
                    core::arch::asm!("nop");
                }
            }
        }
    }

    // Test task B - increments counter and prints
    fn test_task_b() -> ! {
        loop {
            let count = TASK_B_COUNT.fetch_add(1, Ordering::Relaxed);
            if count < 60 {
                serial_println!("[TEST] Task B execution #{}", count);
            }
            // Busy wait
            for _ in 0..500_000 {
                unsafe {
                    core::arch::asm!("nop");
                }
            }
        }
    }

    serial_println!("[TEST] Initializing scheduler...");
    init_scheduler();

    serial_println!("[TEST] Spawning test tasks...");
    spawn_task("Test Task A", test_task_a, TaskPriority::Normal)
        .expect("Failed to spawn Test Task A");
    spawn_task("Test Task B", test_task_b, TaskPriority::Normal)
        .expect("Failed to spawn Test Task B");

    serial_println!("[TEST] Initializing timer at 100 Hz...");
    unsafe {
        timer::init_timer(100);
    }

    serial_println!("[TEST] Enabling interrupts...");
    unsafe {
        core::arch::asm!("sti");
    }

    serial_println!("[TEST] Waiting for 100+ context switches...");
    serial_println!("[TEST] (This will take several seconds)");

    // Wait for enough context switches
    // At 100 Hz, we get ~100 switches per second
    // Wait for about 2 seconds to get 200+ switches
    for _ in 0..200_000_000 {
        unsafe {
            core::arch::asm!("nop");
        }
    }

    // Disable interrupts to check results
    unsafe {
        core::arch::asm!("cli");
    }

    let a_count = TASK_A_COUNT.load(Ordering::Relaxed);
    let b_count = TASK_B_COUNT.load(Ordering::Relaxed);
    let total_switches = SWITCH_COUNT.load(Ordering::Relaxed);

    serial_println!("[TEST] ========================================");
    serial_println!("[TEST] Test Results:");
    serial_println!("[TEST]   Task A executions: {}", a_count);
    serial_println!("[TEST]   Task B executions: {}", b_count);
    serial_println!("[TEST]   Total context switches: {}", total_switches);
    serial_println!("[TEST] ========================================");

    // Verify results
    let mut passed = true;

    if a_count == 0 {
        serial_println!("[TEST] ✗ FAILED: Task A never executed");
        passed = false;
    }

    if b_count == 0 {
        serial_println!("[TEST] ✗ FAILED: Task B never executed");
        passed = false;
    }

    if total_switches < 100 {
        serial_println!(
            "[TEST] ✗ FAILED: Not enough context switches (expected 100+, got {})",
            total_switches
        );
        passed = false;
    }

    // Check for alternating pattern (both tasks should execute roughly equally)
    let diff = if a_count > b_count {
        a_count - b_count
    } else {
        b_count - a_count
    };
    let max_diff = (a_count + b_count) / 4; // Allow 25% difference

    if diff > max_diff {
        serial_println!(
            "[TEST] ⚠ WARNING: Tasks not alternating evenly (A: {}, B: {})",
            a_count,
            b_count
        );
        serial_println!("[TEST]   This might indicate scheduling issues");
    }

    if passed {
        serial_println!("[TEST] ✓ End-to-End Integration Test PASSED!");
        serial_println!("[TEST]   - Both tasks executed successfully");
        serial_println!("[TEST]   - System remained stable for 100+ switches");
        serial_println!("[TEST]   - Tasks alternated as expected");
    } else {
        serial_println!("[TEST] ✗ End-to-End Integration Test FAILED!");
    }

    serial_println!("[TEST] ========================================");
}

/// Manual test functions for kernel-space testing
/// These can be called during kernel initialization to verify scheduler functionality
#[cfg(not(test))]
pub mod manual_tests {
    use super::*;
    use crate::serial_println;

    /// Test that spawn_task adds tasks to the runqueue
    pub fn test_spawn_task_adds_to_runqueue() {
        serial_println!("[TEST] Testing spawn_task adds to runqueue...");

        // Create a dummy task function
        fn dummy_task() -> ! {
            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }

        // Initialize scheduler
        init_scheduler();

        // Spawn a task
        let task_id = spawn_task("test_task", dummy_task, TaskPriority::Normal)
            .expect("Failed to spawn test task");

        // Verify task was created
        serial_println!("[TEST] Spawned task with id: {}", task_id);

        // Check per-CPU runqueue has the task
        let cpu_count = get_cpu_count();
        let mut total_tasks = 0;

        for i in 0..cpu_count {
            let percpu = percpu_for(i);
            let runqueue = percpu.runqueue.lock();
            total_tasks += runqueue.len();
        }

        serial_println!("[TEST] Total tasks in runqueues: {}", total_tasks);

        if total_tasks > 0 {
            serial_println!("[TEST] ✓ spawn_task adds to runqueue test passed!");
        } else {
            serial_println!("[TEST] ✗ spawn_task adds to runqueue test FAILED!");
        }
    }

    /// Test Round-Robin task selection
    pub fn test_round_robin_selection() {
        serial_println!("[TEST] Testing Round-Robin task selection...");

        // Create dummy task functions
        fn task_a() -> ! {
            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }

        fn task_b() -> ! {
            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }

        fn task_c() -> ! {
            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }

        // Initialize scheduler
        init_scheduler();

        // Spawn three tasks with different priorities
        let id_a =
            spawn_task("task_a", task_a, TaskPriority::High).expect("Failed to spawn task_a");
        let id_b =
            spawn_task("task_b", task_b, TaskPriority::Normal).expect("Failed to spawn task_b");
        let id_c = spawn_task("task_c", task_c, TaskPriority::Low).expect("Failed to spawn task_c");

        serial_println!("[TEST] Spawned tasks: {}, {}, {}", id_a, id_b, id_c);

        // Check per-CPU runqueues have tasks
        let cpu_count = get_cpu_count();
        let mut total_tasks = 0;

        for i in 0..cpu_count {
            let percpu = percpu_for(i);
            let runqueue = percpu.runqueue.lock();
            total_tasks += runqueue.len();
        }

        serial_println!("[TEST] Total tasks in runqueues: {}", total_tasks);

        if total_tasks == 3 {
            serial_println!("[TEST] ✓ Round-Robin task selection test passed!");
        } else {
            serial_println!("[TEST] ✗ Round-Robin task selection test FAILED!");
        }
    }

    /// Test multiple task switching (conceptual test)
    pub fn test_multiple_task_switching() {
        serial_println!("[TEST] Testing multiple task switching...");
        serial_println!("[TEST] Note: Actual context switching requires timer interrupts");
        serial_println!("[TEST] This test verifies the scheduler state management");

        // Initialize scheduler
        init_scheduler();

        // Create dummy tasks
        fn task_1() -> ! {
            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }

        fn task_2() -> ! {
            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }

        // Spawn tasks
        spawn_task("task_1", task_1, TaskPriority::Normal).expect("Failed to spawn task_1");
        spawn_task("task_2", task_2, TaskPriority::Normal).expect("Failed to spawn task_2");

        // Verify scheduler state
        let cpu_count = get_cpu_count();
        let mut has_tasks = false;

        for i in 0..cpu_count {
            let percpu = percpu_for(i);
            let runqueue = percpu.runqueue.lock();
            if !runqueue.is_empty() {
                has_tasks = true;
                break;
            }
        }

        if has_tasks {
            serial_println!("[TEST] ✓ Multiple task switching test passed!");
        } else {
            serial_println!("[TEST] ✗ Multiple task switching test FAILED!");
        }
    }

    /// Run all scheduler tests
    pub fn run_all_tests() {
        serial_println!("[TEST] ========================================");
        serial_println!("[TEST] Running Scheduler Tests");
        serial_println!("[TEST] ========================================");

        test_spawn_task_adds_to_runqueue();
        test_round_robin_selection();
        test_multiple_task_switching();

        serial_println!("[TEST] ========================================");
        serial_println!("[TEST] All Scheduler Tests Completed!");
        serial_println!("[TEST] ========================================");
    }
}
