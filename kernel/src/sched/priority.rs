//! Priority-Based Task Scheduler
//!
//! This module implements a three-level priority scheduler with sleep/wake support.
//! Tasks are organized into separate ready queues based on priority (High, Normal, Low),
//! and the scheduler always selects the highest priority ready task.
//!
//! # Features
//!
//! - **Three Priority Levels**: High, Normal, Low
//! - **O(1) Task Selection**: Using priority bitmap for fast queue lookup
//! - **Round-Robin within Priority**: Tasks at same priority scheduled fairly
//! - **Sleep/Wake Mechanism**: Timer-based task suspension
//! - **Preemption Control**: Critical sections can disable preemption
//!
//! # Architecture
//!
//! ```text
//! High Priority Queue:    [Task 5] -> [Task 8] -> NULL
//! Normal Priority Queue:  [Task 1] -> [Task 2] -> [Task 3] -> NULL
//! Low Priority Queue:     [Task 4] -> [Task 6] -> NULL
//!
//! Sleeping Tasks:
//! [
//!     { task_id: 3, wake_tick: 1050, priority: Normal },
//!     { task_id: 7, wake_tick: 1200, priority: High },
//! ]
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use crate::sched::priority::{PriorityScheduler, TaskPriority};
//!
//! let mut sched = PriorityScheduler::new();
//!
//! // Enqueue tasks with different priorities
//! sched.enqueue_task(1, TaskPriority::Normal);
//! sched.enqueue_task(2, TaskPriority::High);
//! sched.enqueue_task(3, TaskPriority::Low);
//!
//! // Select next task (will be task 2 - highest priority)
//! let next = sched.select_next(); // Returns Some(2)
//!
//! // Put task to sleep
//! sched.sleep_task(1, 100, TaskPriority::Normal);
//!
//! // Update tick and wake sleeping tasks
//! for _ in 0..100 {
//!     sched.tick();
//! }
//! sched.wake_sleeping_tasks(); // Task 1 wakes up
//! ```

use super::task::TaskId;

/// Maximum number of tasks per queue
const MAX_TASKS: usize = 64;

/// Task priority levels
///
/// Defines three priority levels for task scheduling. Higher priority tasks
/// are always selected before lower priority tasks.
///
/// # Priority Order
///
/// High > Normal > Low
///
/// # Usage
///
/// ```rust,no_run
/// use crate::sched::priority::TaskPriority;
///
/// let priority = TaskPriority::High;
/// assert_eq!(priority.as_index(), 2);
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum TaskPriority {
    /// Low priority (value 0)
    Low = 0,
    /// Normal priority (value 1, default)
    Normal = 1,
    /// High priority (value 2)
    High = 2,
}

impl TaskPriority {
    /// Convert priority to queue index
    pub const fn as_index(self) -> usize {
        self as usize
    }
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

/// Simple circular queue for task IDs (reused from mod.rs)
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
}

/// Sleeping task entry
#[derive(Copy, Clone)]
struct SleepingTask {
    task_id: TaskId,
    wake_tick: u64,
    priority: TaskPriority,
    valid: bool, // Whether this slot is occupied
}

impl SleepingTask {
    const fn empty() -> Self {
        Self {
            task_id: 0,
            wake_tick: 0,
            priority: TaskPriority::Normal,
            valid: false,
        }
    }
}

/// Priority scheduler with three ready queues
///
/// Manages task scheduling across three priority levels with sleep/wake support.
/// Uses a bitmap for O(1) priority queue selection.
///
/// # Fields
///
/// - **ready_queues**: Three circular queues (one per priority level)
/// - **non_empty_queues**: Bitmap tracking which queues have tasks (bits 0-2)
/// - **sleeping_tasks**: Fixed-size array of sleeping tasks
/// - **current_tick**: Current timer tick count
/// - **preempt_disable_count**: Counter for preemption control (0 = enabled)
///
/// # Performance
///
/// - **enqueue_task**: O(1)
/// - **select_next**: O(1) with bitmap
/// - **sleep_task**: O(n) to find empty slot
/// - **wake_sleeping_tasks**: O(n) linear scan
///
/// # Future Optimizations
///
/// Phase 5 will replace the sleeping_tasks array with a BinaryHeap for O(log n)
/// wake operations.
pub struct PriorityScheduler {
    /// Ready queues for each priority level [Low, Normal, High]
    ready_queues: [TaskQueue; 3],
    
    /// Bitmap tracking non-empty queues for O(1) selection
    /// Bits 0-2 correspond to Low/Normal/High priorities
    non_empty_queues: u8,
    
    /// Array of sleeping tasks (fixed size for no_std)
    sleeping_tasks: [SleepingTask; MAX_TASKS],
    
    /// Current tick count
    current_tick: u64,
    
    /// Preemption disable counter (0 = preemption enabled)
    preempt_disable_count: usize,
}

impl PriorityScheduler {
    /// Create a new priority scheduler
    pub const fn new() -> Self {
        Self {
            ready_queues: [TaskQueue::new(), TaskQueue::new(), TaskQueue::new()],
            non_empty_queues: 0,
            sleeping_tasks: [SleepingTask::empty(); MAX_TASKS],
            current_tick: 0,
            preempt_disable_count: 0,
        }
    }
    
    /// Add task to appropriate priority queue
    ///
    /// # Arguments
    /// * `task_id` - Task identifier
    /// * `priority` - Task priority level
    ///
    /// # Returns
    /// `true` if task was enqueued successfully, `false` if queue is full
    pub fn enqueue_task(&mut self, task_id: TaskId, priority: TaskPriority) -> bool {
        let index = priority.as_index();
        let success = self.ready_queues[index].push_back(task_id);
        
        if success {
            // Set the bit for this priority level
            self.non_empty_queues |= 1 << index;
        }
        
        success
    }
    
    /// Select next task to run (highest priority first)
    ///
    /// Checks queues from highest to lowest priority and returns the first
    /// available task. Uses bitmap for O(1) queue selection.
    ///
    /// # Returns
    /// `Some(task_id)` if a task is available, `None` if all queues are empty
    pub fn select_next(&mut self) -> Option<TaskId> {
        // Check queues from highest to lowest priority
        // High = 2, Normal = 1, Low = 0
        for priority_index in (0..=2).rev() {
            // Check if this queue has tasks using bitmap
            if (self.non_empty_queues & (1 << priority_index)) != 0 {
                if let Some(task_id) = self.ready_queues[priority_index].pop_front() {
                    // Update bitmap if queue is now empty
                    if self.ready_queues[priority_index].is_empty() {
                        self.non_empty_queues &= !(1 << priority_index);
                    }
                    return Some(task_id);
                } else {
                    // Queue was marked as non-empty but pop failed - clear the bit
                    self.non_empty_queues &= !(1 << priority_index);
                }
            }
        }
        
        None
    }
    
    /// Check if all queues are empty
    pub fn is_empty(&self) -> bool {
        self.non_empty_queues == 0
    }
    
    /// Get total number of tasks across all queues
    pub fn len(&self) -> usize {
        self.ready_queues[0].len() + self.ready_queues[1].len() + self.ready_queues[2].len()
    }
    
    /// Put task to sleep for specified ticks
    ///
    /// Task will be removed from ready queue and added to sleeping list.
    /// The task will wake up after `ticks` timer interrupts.
    ///
    /// # Arguments
    /// * `task_id` - Task identifier
    /// * `ticks` - Number of ticks to sleep
    /// * `priority` - Task priority (for re-enqueuing when woken)
    ///
    /// # Returns
    /// `true` if task was put to sleep successfully, `false` if no slots available
    pub fn sleep_task(&mut self, task_id: TaskId, ticks: u64, priority: TaskPriority) -> bool {
        use crate::serial_println;
        
        let wake_tick = self.current_tick + ticks;
        
        // Find an empty slot in sleeping_tasks array
        for slot in &mut self.sleeping_tasks {
            if !slot.valid {
                *slot = SleepingTask {
                    task_id,
                    wake_tick,
                    priority,
                    valid: true,
                };
                
                // Log sleep operation
                serial_println!(
                    "[SCHED] Task {} sleeping for {} ticks (wake at tick {})",
                    task_id,
                    ticks,
                    wake_tick
                );
                
                return true;
            }
        }
        
        // No empty slots available
        false
    }
    
    /// Wake tasks whose sleep time has elapsed
    ///
    /// Scans the sleeping tasks array and wakes any tasks whose wake_tick
    /// is less than or equal to the current tick. Woken tasks are re-enqueued
    /// to their appropriate priority queue.
    ///
    /// # Returns
    /// Number of tasks woken
    pub fn wake_sleeping_tasks(&mut self) -> usize {
        use crate::serial_println;
        
        let mut woken_count = 0;
        let current_tick = self.current_tick;
        
        // First pass: collect tasks to wake
        let mut tasks_to_wake = [(0usize, TaskPriority::Normal); MAX_TASKS];
        let mut wake_index = 0;
        
        for slot in &mut self.sleeping_tasks {
            if slot.valid && slot.wake_tick <= current_tick {
                if wake_index < MAX_TASKS {
                    tasks_to_wake[wake_index] = (slot.task_id, slot.priority);
                    wake_index += 1;
                }
                slot.valid = false;
                woken_count += 1;
            }
        }
        
        // Second pass: re-enqueue woken tasks and log
        for i in 0..wake_index {
            let (task_id, priority) = tasks_to_wake[i];
            self.enqueue_task(task_id, priority);
            
            // Log wake operation
            serial_println!(
                "[SCHED] Task {} woke up at tick {} (priority: {:?})",
                task_id,
                current_tick,
                priority
            );
        }
        
        woken_count
    }
    
    /// Update tick counter and wake tasks
    pub fn tick(&mut self) {
        self.current_tick += 1;
    }
    
    /// Get current tick count
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }
    
    /// Disable preemption (for critical sections)
    pub fn preempt_disable(&mut self) {
        self.preempt_disable_count += 1;
    }
    
    /// Enable preemption
    pub fn preempt_enable(&mut self) {
        if self.preempt_disable_count > 0 {
            self.preempt_disable_count -= 1;
        }
    }
    
    /// Check if preemption is allowed
    pub fn can_preempt(&self) -> bool {
        self.preempt_disable_count == 0
    }
}

/// Counter for preemption operations (for throttling logs)
static PREEMPT_OP_COUNT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// Global preemption disable function
/// 
/// Disables preemption by disabling interrupts.
/// Must be called before acquiring spinlocks in IPC operations.
/// 
/// Note: In SMP mode, preemption control is handled by interrupt disable/enable
/// rather than a counter, since each CPU manages its own scheduling independently.
pub fn preempt_disable() {
    use crate::serial_println;
    use core::sync::atomic::Ordering;
    
    // Disable interrupts to prevent preemption
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
    }
    
    // Log preemption disable with throttling (every 100th operation)
    let count = PREEMPT_OP_COUNT.fetch_add(1, Ordering::Relaxed);
    if count % 100 == 0 {
        serial_println!("[SCHED] Preemption disabled");
    }
}

/// Global preemption enable function
/// 
/// Enables preemption by enabling interrupts.
/// Must be called after releasing spinlocks in IPC operations.
/// 
/// Note: In SMP mode, preemption control is handled by interrupt disable/enable
/// rather than a counter, since each CPU manages its own scheduling independently.
pub fn preempt_enable() {
    use crate::serial_println;
    use core::sync::atomic::Ordering;
    
    // Enable interrupts to allow preemption
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }
    
    // Log preemption enable with throttling (every 100th operation)
    let count = PREEMPT_OP_COUNT.load(Ordering::Relaxed);
    if count % 100 == 0 {
        serial_println!("[SCHED] Preemption enabled");
    }
}
