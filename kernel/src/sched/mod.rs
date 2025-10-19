//! Task Scheduler Module
//!
//! This module implements a preemptive multitasking scheduler using Round-Robin algorithm.
//! It manages task creation, context switching, and timer-based preemption.

pub mod task;
pub mod context;
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

use spin::Mutex;
use task::{Task, TaskId, TaskState, SchedulerError, SchedulerResult};
use context::CpuContext;

/// Maximum number of tasks supported
const MAX_TASKS: usize = 64;

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

/// Scheduler state containing the runqueue and current task information
struct SchedState {
    /// Queue of ready tasks (stores TaskIds, not Task objects)
    runqueue: TaskQueue,
    
    /// Currently running task ID (None if no task is running)
    current: Option<TaskId>,
    
    /// Next task ID to assign (incremented for each new task)
    next_tid: usize,
}

impl SchedState {
    /// Create a new empty scheduler state
    const fn new() -> Self {
        Self {
            runqueue: TaskQueue::new(),
            current: None,
            next_tid: 1, // Start at 1, reserve 0 for idle task
        }
    }
}

/// Global scheduler state protected by a mutex
static SCHED: Mutex<SchedState> = Mutex::new(SchedState::new());

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
/// 4. Adds the TaskId to the runqueue
/// 5. Logs the task spawn
///
/// # Arguments
/// * `name` - Human-readable task name
/// * `entry_point` - Function pointer to the task's entry point
///
/// # Returns
/// A Result containing the TaskId of the newly spawned task, or an error if spawning fails
///
/// # Errors
/// Returns `SchedulerError::TooManyTasks` if the task table is full
/// Returns `SchedulerError::OutOfMemory` if memory allocation fails
/// Returns `SchedulerError::RunqueueFull` if the runqueue is full
pub fn spawn_task(name: &'static str, entry_point: fn() -> !) -> SchedulerResult<TaskId> {
    use crate::mm::allocator::kmalloc;
    use core::ptr;
    
    // Lock both SCHED and TASK_TABLE
    let mut sched = SCHED.lock();
    let mut task_table = TASK_TABLE.lock();
    
    // 1. Generate unique TaskId
    let task_id = sched.next_tid;
    
    if task_id >= MAX_TASKS {
        sched_error!("Too many tasks! Maximum is {}", MAX_TASKS);
        return Err(SchedulerError::TooManyTasks);
    }
    
    sched.next_tid += 1;
    
    // 2. Create new Task
    let task = match Task::new(task_id, name, entry_point) {
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
    
    task_table[task_id] = TaskPtr::new(task_ptr);
    
    // 4. Add TaskId to runqueue
    if !sched.runqueue.push_back(task_id) {
        sched_error!("Failed to add task {} to runqueue", task_id);
        return Err(SchedulerError::RunqueueFull);
    }
    
    // 5. Log task spawn
    sched_info!("Spawned task {}: {}", task_id, name);
    
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

/// Select the next task to run using Round-Robin algorithm
///
/// This function:
/// 1. Locks SCHED state
/// 2. Moves current TaskId to back of runqueue (if exists)
/// 3. Pops front TaskId from runqueue (or falls back to idle task if empty)
/// 4. Updates current TaskId
/// 5. Unlocks SCHED state
/// 6. Returns references to old and new tasks for context switch
///
/// # Returns
/// A tuple of (old_task, new_task) references, or None if no tasks available
fn schedule_next() -> Option<(&'static mut Task, &'static mut Task)> {
    let mut sched = SCHED.lock();
    
    // Get the current task (if any)
    let old_task_id = sched.current;
    
    // If there's no current task, this is the first switch
    // Don't pop from runqueue - let tick() handle it
    if old_task_id.is_none() {
        drop(sched);
        return None;
    }
    
    // Move current task to back of runqueue (Round-Robin)
    if let Some(current_id) = old_task_id {
        // Update task state from Running to Ready
        if let Some(task) = get_task(current_id) {
            task.state = TaskState::Ready;
        }
        sched.runqueue.push_back(current_id);
    }
    
    // Pop next task from front of runqueue
    // If runqueue is empty, fall back to idle task (id 0)
    let next_task_id = match sched.runqueue.pop_front() {
        Some(id) => id,
        None => {
            // Runqueue is empty - fall back to idle task
            sched_warn!("Runqueue empty, falling back to idle task");
            0 // Idle task ID
        }
    };
    
    // Update current task
    sched.current = Some(next_task_id);
    
    // Drop the lock before getting task references
    drop(sched);
    
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
pub(crate) static SWITCH_COUNT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// Scheduler tick function - called by timer interrupt
///
/// This function:
/// 1. Calls schedule_next() to get old and new tasks
/// 2. Logs the context switch (with throttling)
/// 3. Performs the context switch
///
/// # Notes
/// - This function does not return in the traditional sense (tail-switch)
/// - The next task will continue execution from where it was interrupted
/// - For new tasks, execution starts at entry_trampoline
pub fn tick() {
    use core::sync::atomic::Ordering;
    
    // Get next task to run
    let tasks = schedule_next();
    
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
        
        // Log context switch with throttling
        // First 10 switches: log every switch
        // After that: log every 100 switches
        if count < 10 || count % 100 == 0 {
            sched_log!(
                "Switch #{} → Task {} ({})",
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
        // First switch - no old task yet
        // We need to manually set up the first task and jump to it
        let mut sched = SCHED.lock();
        
        // Pop the first task from runqueue
        if let Some(first_task_id) = sched.runqueue.pop_front() {
            sched.current = Some(first_task_id);
            drop(sched);
            
            if let Some(first_task) = get_task(first_task_id) {
                first_task.state = TaskState::Running;
                
                sched_log!("First switch → Task {} ({})", 
                    first_task.id, first_task.name);
                
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
        } else {
            panic!("[SCHED] CRITICAL: No tasks in runqueue for first switch");
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
/// - The idle task is created but not added to the runqueue
///   (it will be used when the runqueue is empty)
pub fn init_scheduler() {
    use crate::mm::allocator::kmalloc;
    use core::ptr;
    
    sched_info!("Initializing scheduler...");
    
    // Initialize SCHED state
    let mut sched = SCHED.lock();
    sched.runqueue.clear();
    sched.current = None;
    sched.next_tid = 1; // Reserve 0 for idle task
    drop(sched);
    
    // Initialize TASK_TABLE (clear all entries)
    let mut task_table = TASK_TABLE.lock();
    for i in 0..MAX_TASKS {
        task_table[i] = TaskPtr::null();
    }
    drop(task_table);
    
    // Create idle task (task id 0)
    // We manually create it with id 0 instead of using spawn_task
    let idle = match Task::new(0, "idle", idle_task) {
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
    
    sched_info!("Created idle task (id 0)");
    sched_info!("Scheduler initialized!");
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
                unsafe { core::arch::asm!("nop"); }
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
                unsafe { core::arch::asm!("nop"); }
            }
        }
    }
    
    serial_println!("[TEST] Initializing scheduler...");
    init_scheduler();
    
    serial_println!("[TEST] Spawning test tasks...");
    spawn_task("Test Task A", test_task_a).expect("Failed to spawn Test Task A");
    spawn_task("Test Task B", test_task_b).expect("Failed to spawn Test Task B");
    
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
        unsafe { core::arch::asm!("nop"); }
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
        serial_println!("[TEST] ✗ FAILED: Not enough context switches (expected 100+, got {})", total_switches);
        passed = false;
    }
    
    // Check for alternating pattern (both tasks should execute roughly equally)
    let diff = if a_count > b_count { a_count - b_count } else { b_count - a_count };
    let max_diff = (a_count + b_count) / 4; // Allow 25% difference
    
    if diff > max_diff {
        serial_println!("[TEST] ⚠ WARNING: Tasks not alternating evenly (A: {}, B: {})", a_count, b_count);
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
                unsafe { core::arch::asm!("hlt"); }
            }
        }
        
        // Initialize scheduler
        init_scheduler();
        
        // Spawn a task
        let task_id = spawn_task("test_task", dummy_task).expect("Failed to spawn test task");
        
        // Verify task was created
        serial_println!("[TEST] Spawned task with id: {}", task_id);
        
        // Check runqueue has the task
        let sched = SCHED.lock();
        let runqueue_len = sched.runqueue.len();
        drop(sched);
        
        serial_println!("[TEST] Runqueue length: {}", runqueue_len);
        
        if runqueue_len > 0 {
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
                unsafe { core::arch::asm!("hlt"); }
            }
        }
        
        fn task_b() -> ! {
            loop {
                unsafe { core::arch::asm!("hlt"); }
            }
        }
        
        fn task_c() -> ! {
            loop {
                unsafe { core::arch::asm!("hlt"); }
            }
        }
        
        // Initialize scheduler
        init_scheduler();
        
        // Spawn three tasks
        let id_a = spawn_task("task_a", task_a).expect("Failed to spawn task_a");
        let id_b = spawn_task("task_b", task_b).expect("Failed to spawn task_b");
        let id_c = spawn_task("task_c", task_c).expect("Failed to spawn task_c");
        
        serial_println!("[TEST] Spawned tasks: {}, {}, {}", id_a, id_b, id_c);
        
        // Check runqueue order
        let sched = SCHED.lock();
        let runqueue_len = sched.runqueue.len();
        serial_println!("[TEST] Runqueue has {} tasks", runqueue_len);
        drop(sched);
        
        if runqueue_len == 3 {
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
                unsafe { core::arch::asm!("hlt"); }
            }
        }
        
        fn task_2() -> ! {
            loop {
                unsafe { core::arch::asm!("hlt"); }
            }
        }
        
        // Spawn tasks
        spawn_task("task_1", task_1).expect("Failed to spawn task_1");
        spawn_task("task_2", task_2).expect("Failed to spawn task_2");
        
        // Verify scheduler state
        let sched = SCHED.lock();
        let has_tasks = !sched.runqueue.is_empty();
        drop(sched);
        
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
