//! Port management for IPC
//! Provides port structures and message queues
//!
//! # SMP Safety and Lock Ordering
//!
//! The IPC system uses a two-level locking strategy for SMP safety:
//!
//! 1. **PORT_MANAGER.table_lock**: Protects port creation/deletion (rarely held)
//! 2. **Per-port locks**: Protect individual port operations (frequently held)
//!
//! ## Lock Ordering Rules
//!
//! - PORT_MANAGER.table_lock must be acquired before any per-port lock
//! - Never hold more than one per-port lock at a time
//! - Preemption must be disabled before acquiring per-port locks
//! - Port locks must be released before calling scheduler functions
//!
//! ## Cross-Core IPC
//!
//! When a task on CPU A sends a message to a task blocked on CPU B:
//! 1. Sender acquires port lock
//! 2. Message is enqueued
//! 3. Receiver task is marked Ready and enqueued to a CPU
//! 4. If receiver is enqueued to a remote CPU, RESCHEDULE_IPI is sent
//! 5. Port lock is released
//! 6. Remote CPU receives IPI and schedules the receiver task
//!
//! See `kernel/src/sync/lock_ordering.rs` for complete lock ordering documentation.

use super::ipc::{IpcError, Message};
use crate::sched::task::TaskId;
use spin::Mutex;

/// Maximum messages per port queue
/// Reduced from 16 to 4 to avoid stack overflow (each Message is 4KB)
const MAX_MESSAGES_PER_PORT: usize = 4;

/// Maximum blocked tasks per port
const MAX_BLOCKED_TASKS: usize = 64;

/// Simple circular queue for messages
struct MessageQueue {
    messages: [Message; MAX_MESSAGES_PER_PORT],
    head: usize,
    tail: usize,
    count: usize,
}

impl MessageQueue {
    const fn new() -> Self {
        Self {
            messages: [Message::new(); MAX_MESSAGES_PER_PORT],
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    fn push_back(&mut self, message: Message) -> bool {
        if self.count >= MAX_MESSAGES_PER_PORT {
            return false;
        }

        self.messages[self.tail] = message;
        self.tail = (self.tail + 1) % MAX_MESSAGES_PER_PORT;
        self.count += 1;
        true
    }

    fn pop_front(&mut self) -> Option<Message> {
        if self.count == 0 {
            return None;
        }

        let message = self.messages[self.head];
        self.head = (self.head + 1) % MAX_MESSAGES_PER_PORT;
        self.count -= 1;
        Some(message)
    }

    fn len(&self) -> usize {
        self.count
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Simple circular queue for task IDs
struct TaskQueue {
    tasks: [TaskId; MAX_BLOCKED_TASKS],
    head: usize,
    tail: usize,
    count: usize,
}

impl TaskQueue {
    const fn new() -> Self {
        Self {
            tasks: [0; MAX_BLOCKED_TASKS],
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    fn push_back(&mut self, task_id: TaskId) -> bool {
        if self.count >= MAX_BLOCKED_TASKS {
            return false;
        }

        self.tasks[self.tail] = task_id;
        self.tail = (self.tail + 1) % MAX_BLOCKED_TASKS;
        self.count += 1;
        true
    }

    fn pop_front(&mut self) -> Option<TaskId> {
        if self.count == 0 {
            return None;
        }

        let task_id = self.tasks[self.head];
        self.head = (self.head + 1) % MAX_BLOCKED_TASKS;
        self.count -= 1;
        Some(task_id)
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Port structure for IPC message passing
///
/// A port is a communication endpoint that maintains a FIFO queue of messages
/// and a list of tasks blocked waiting for messages.
///
/// # SMP Safety
/// Each port has its own spinlock to protect concurrent access from multiple CPUs.
/// The lock protects both the message queue and the blocked tasks queue.
pub struct Port {
    /// Port identifier (0-255)
    pub id: usize,

    /// Message queue (max 16 messages)
    pub queue: MessageQueue,

    /// Tasks blocked waiting for messages (FIFO wake policy)
    pub blocked_tasks: TaskQueue,

    /// Spinlock protecting port operations
    pub lock: Mutex<()>,
}

impl Port {
    /// Create a new port with the given ID
    ///
    /// # Arguments
    /// * `id` - Port identifier (must be 0-255)
    ///
    /// # Returns
    /// A new Port with empty queues
    pub const fn new(id: usize) -> Self {
        Self {
            id,
            queue: MessageQueue::new(),
            blocked_tasks: TaskQueue::new(),
            lock: Mutex::new(()),
        }
    }

    /// Check if the message queue is full (16 messages)
    pub fn is_queue_full(&self) -> bool {
        self.queue.len() >= MAX_MESSAGES_PER_PORT
    }

    /// Check if there are any messages in the queue
    pub fn has_messages(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Check if there are any blocked tasks
    pub fn has_blocked_tasks(&self) -> bool {
        !self.blocked_tasks.is_empty()
    }
}

/// Port Manager
///
/// Manages all ports in the system (max 256 ports).
/// Provides send and receive operations with proper synchronization.
///
/// # SMP Safety
/// The PortManager uses a two-level locking strategy:
/// 1. table_lock: Protects port creation/deletion (coarse-grained)
/// 2. per-port locks: Protect individual port operations (fine-grained)
/// This allows multiple CPUs to access different ports concurrently.
pub struct PortManager {
    /// Array of optional ports (256 max)
    /// Using Box to avoid stack overflow from large Port structures
    pub ports: [Option<alloc::boxed::Box<Port>>; 256],

    /// Lock for port creation/deletion
    pub table_lock: Mutex<()>,
}

impl PortManager {
    /// Create a new PortManager with no ports initialized
    pub const fn new() -> Self {
        const NONE_PORT: Option<alloc::boxed::Box<Port>> = None;
        Self {
            ports: [NONE_PORT; 256],
            table_lock: Mutex::new(()),
        }
    }

    /// Create a port at the specified ID
    ///
    /// # Arguments
    /// * `port_id` - Port identifier (must be 0-255)
    ///
    /// # Returns
    /// Ok(()) on success, or IpcError on failure
    ///
    /// # Errors
    /// - `IpcError::InvalidPort` if port_id >= 256
    ///
    /// # Note
    /// This function assumes the caller already holds the PORT_MANAGER lock.
    /// It does NOT acquire table_lock internally to avoid deadlock.
    pub fn create_port(&mut self, port_id: usize) -> Result<(), IpcError> {
        use crate::serial_println;

        serial_println!("[IPC] create_port: Validating port_id {}", port_id);

        // Validate port ID
        if port_id >= 256 {
            return Err(IpcError::InvalidPort);
        }

        serial_println!(
            "[IPC] create_port: Allocating uninitialized Port {} on heap...",
            port_id
        );

        // Allocate uninitialized memory on heap first (doesn't require stack space)
        let mut uninit_port = alloc::boxed::Box::<Port>::new_uninit();

        serial_println!(
            "[IPC] create_port: Initializing Port {} in-place...",
            port_id
        );

        // Initialize the port in-place on the heap
        let new_port = unsafe {
            // Write the initialized Port directly into the heap allocation
            uninit_port.as_mut_ptr().write(Port::new(port_id));
            // Assume initialization is complete
            uninit_port.assume_init()
        };

        serial_println!("[IPC] create_port: Port {} created successfully", port_id);

        self.ports[port_id] = Some(new_port);

        serial_println!("[IPC] create_port: Port {} assigned successfully", port_id);

        Ok(())
    }

    /// Send a message to a port
    ///
    /// This function:
    /// 1. Validates port ID and message size
    /// 2. Acquires port lock with preempt_disable()
    /// 3. Checks queue capacity
    /// 4. Enqueues message
    /// 5. Wakes one blocked task (FIFO) if any
    /// 6. Releases port lock with preempt_enable()
    /// 7. Increments ipc_sends metric
    ///
    /// # Arguments
    /// * `port_id` - Target port ID
    /// * `data` - Message data to send
    ///
    /// # Returns
    /// Ok(()) on success, or IpcError on failure
    ///
    /// # Errors
    /// - `IpcError::InvalidPort` if port_id >= 256
    /// - `IpcError::PortNotFound` if port doesn't exist
    /// - `IpcError::MessageTooLarge` if data.len() > 4096
    /// - `IpcError::QueueFull` if port queue is full (16 messages)
    ///
    /// # SMP Safety
    /// This function handles cross-core IPC correctly:
    /// - Per-port locks prevent concurrent access to the same port
    /// - Task wakeup uses enqueue_task which sends RESCHEDULE_IPI to remote CPUs
    /// - Preemption is disabled while holding port locks to prevent deadlocks
    pub fn send_message(&mut self, port_id: usize, data: &[u8]) -> Result<(), IpcError> {
        use crate::serial_println;
        use core::sync::atomic::Ordering;

        // Validate port ID
        if port_id >= 256 {
            return Err(IpcError::InvalidPort);
        }

        // Validate message size (max 4096 bytes)
        if data.len() > 4096 {
            return Err(IpcError::MessageTooLarge);
        }

        // Get port reference
        let port = match &mut self.ports[port_id] {
            Some(p) => p,
            None => return Err(IpcError::PortNotFound),
        };

        // Disable preemption before acquiring port lock
        crate::sched::priority::preempt_disable();

        // Acquire port lock
        let _lock = port.lock.lock();

        // Check queue capacity (max 16 messages)
        if port.is_queue_full() {
            // Increment queue full metric
            crate::sys::METRICS
                .ipc_queue_full
                .fetch_add(1, Ordering::Relaxed);

            // Release lock and re-enable preemption
            drop(_lock);
            crate::sched::priority::preempt_enable();

            serial_println!("[IPC] Port {} queue full", port_id);
            return Err(IpcError::QueueFull);
        }

        // Create message and enqueue
        let message = Message::from_slice(data);
        if !port.queue.push_back(message) {
            // This shouldn't happen since we checked is_queue_full above
            drop(_lock);
            crate::sched::priority::preempt_enable();
            return Err(IpcError::QueueFull);
        }

        serial_println!("[IPC] Sent {} bytes to port {}", data.len(), port_id);

        // Wake one blocked task (FIFO) if any
        if let Some(task_id) = port.blocked_tasks.pop_front() {
            serial_println!("[IPC] Waking task {} blocked on port {}", task_id, port_id);

            // Update task state to Ready and add to scheduler
            // We need to get the task's priority first
            if let Some((_, _priority)) = crate::sched::get_task_priority(task_id) {
                // Mark task as Ready
                if let Some(task) = crate::sched::get_task_mut(task_id) {
                    task.state = crate::sched::task::TaskState::Ready;
                    task.blocked_on_port = None;
                }

                // Add task back to scheduler (will select CPU with smallest runqueue)
                // enqueue_task will automatically send RESCHEDULE_IPI if the task
                // is enqueued to a remote CPU
                crate::sched::enqueue_task(task_id, None);
            }
        }

        // Release lock and re-enable preemption
        drop(_lock);
        crate::sched::priority::preempt_enable();

        // Increment ipc_sends metric
        crate::sys::METRICS
            .ipc_sends
            .fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Receive a message from a port (blocking)
    ///
    /// This function:
    /// 1. Validates port ID and buffer
    /// 2. Acquires port lock with preempt_disable()
    /// 3. If message available: dequeues, copies to buffer, returns bytes received
    /// 4. If no message: adds task to blocked_tasks queue, marks task as Blocked, triggers scheduler
    /// 5. Releases port lock with preempt_enable()
    /// 6. Increments ipc_recvs metric
    ///
    /// # Arguments
    /// * `port_id` - Source port ID
    /// * `task_id` - ID of the receiving task
    /// * `buf` - Buffer to receive message into
    ///
    /// # Returns
    /// Ok(bytes_received) on success, or IpcError on failure
    ///
    /// # Errors
    /// - `IpcError::InvalidPort` if port_id >= 256
    /// - `IpcError::PortNotFound` if port doesn't exist
    /// - `IpcError::InvalidBuffer` if buffer is too small or invalid
    ///
    /// # SMP Safety
    /// This function handles cross-core IPC correctly:
    /// - Per-port locks prevent concurrent access to the same port
    /// - Task blocking uses proper task state locks
    /// - yield_now() operates on current core's runqueue
    /// - Recursive call after wakeup is safe because message is guaranteed to be available
    pub fn recv_message(
        &mut self,
        port_id: usize,
        task_id: TaskId,
        buf: &mut [u8],
    ) -> Result<usize, IpcError> {
        use crate::serial_println;
        use core::sync::atomic::Ordering;

        // Validate port ID
        if port_id >= 256 {
            return Err(IpcError::InvalidPort);
        }

        // Validate buffer
        if buf.is_empty() {
            return Err(IpcError::InvalidBuffer);
        }

        // Get port reference
        let port = match &mut self.ports[port_id] {
            Some(p) => p,
            None => return Err(IpcError::PortNotFound),
        };

        // Disable preemption before acquiring port lock
        crate::sched::priority::preempt_disable();

        // Acquire port lock
        let _lock = port.lock.lock();

        // Check if message is available
        if let Some(message) = port.queue.pop_front() {
            // Message available - copy to buffer
            let bytes_to_copy = core::cmp::min(message.len(), buf.len());
            buf[..bytes_to_copy].copy_from_slice(&message.as_slice()[..bytes_to_copy]);

            serial_println!(
                "[IPC] Received {} bytes from port {}",
                bytes_to_copy,
                port_id
            );

            // Release lock and re-enable preemption
            drop(_lock);
            crate::sched::priority::preempt_enable();

            // Increment ipc_recvs metric
            crate::sys::METRICS
                .ipc_recvs
                .fetch_add(1, Ordering::Relaxed);

            return Ok(bytes_to_copy);
        }

        // No message available - block the task
        serial_println!(
            "[IPC] Task {} blocking on port {} (no messages)",
            task_id,
            port_id
        );

        // Add task to blocked queue
        if !port.blocked_tasks.push_back(task_id) {
            // Blocked tasks queue is full - return error instead of blocking
            drop(_lock);
            crate::sched::priority::preempt_enable();
            serial_println!("[IPC] Port {} blocked tasks queue full", port_id);
            return Err(IpcError::QueueFull);
        }

        // Release lock and re-enable preemption
        drop(_lock);
        crate::sched::priority::preempt_enable();

        // Mark task as Blocked and update blocked_on_port
        if let Some(task) = crate::sched::get_task_mut(task_id) {
            task.state = crate::sched::task::TaskState::Blocked;
            task.blocked_on_port = Some(port_id);
        }

        // Trigger scheduler to select next task
        // This will context switch away from the current task
        crate::sched::yield_now();

        // When we wake up (after a message arrives), we need to try receiving again
        // This is a recursive call, but it should succeed immediately since we were woken
        // because a message arrived
        self.recv_message(port_id, task_id, buf)
    }
}

/// Global PORT_MANAGER instance
///
/// This is initialized at boot and provides access to all IPC ports.
/// Protected by Mutex for thread-safe access.
pub static PORT_MANAGER: Mutex<PortManager> = Mutex::new(PortManager::new());

/// Initialize IPC subsystem
///
/// Creates system ports (0-15) for kernel use.
/// Should be called during kernel initialization.
pub fn init_ipc() {
    use crate::serial_println;

    serial_println!("[IPC] Initializing IPC subsystem...");
    serial_println!("[IPC] Attempting to acquire PORT_MANAGER lock...");

    let mut port_mgr = PORT_MANAGER.lock();

    serial_println!("[IPC] PORT_MANAGER lock acquired");

    // Create system ports 0-15
    for port_id in 0..16 {
        serial_println!("[IPC] About to create port {}...", port_id);

        // Create port with detailed logging
        match port_mgr.create_port(port_id) {
            Ok(()) => {
                serial_println!("[IPC] Port {} created successfully", port_id);
            }
            Err(e) => {
                serial_println!("[IPC] Failed to create port {}: {:?}", port_id, e);
            }
        }
    }

    serial_println!("[IPC] Created 16 system ports (0-15)");
    serial_println!("[IPC] IPC subsystem initialized!");
}
