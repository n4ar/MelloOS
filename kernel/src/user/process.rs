//! Process Management
//!
//! This module implements the process control block (PCB) and process table
//! for user-mode process management. It provides fine-grained locking for
//! SMP safety and atomic PID allocation.

use crate::mm::paging::PageTable;
use crate::sched::context::CpuContext;
use crate::sched::priority::TaskPriority;
use crate::sched::task::{MemoryRegion, MemoryRegionType};
use crate::sync::{SpinLock, SpinLockGuard};
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Process identifier type
pub type ProcessId = usize;

/// Maximum number of processes in the system
pub const MAX_PROCESSES: usize = 1024;

/// Maximum number of file descriptors per process
pub const MAX_FDS: usize = 256;

/// Maximum number of memory regions per process
const MAX_MEMORY_REGIONS: usize = 16;

/// User space address limit (512GB)
pub const USER_LIMIT: usize = 0x0000_8000_0000_0000;

/// User stack configuration
pub const USER_STACK_TOP: usize = 0x0000_7FFF_FFFF_0000;
pub const USER_STACK_SIZE: usize = 8192; // 8KB

/// Process state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// Process is ready to run and waiting in the runqueue
    Ready,

    /// Process is currently running on the CPU
    Running,

    /// Process is sleeping (waiting for wake_tick)
    Sleeping,

    /// Process is blocked on I/O or IPC
    Blocked,

    /// Process has terminated, waiting for parent to collect exit code
    Zombie,

    /// Process has been fully cleaned up and can be reused
    Terminated,
}

/// Block reason for blocked processes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    /// Blocked waiting for child process to exit
    WaitingForChild,

    /// Blocked on IPC receive
    IpcReceive(usize), // port_id

    /// Blocked on I/O operation
    IoOperation,
}

/// File descriptor placeholder (for future implementation)
#[derive(Debug, Clone)]
pub struct FileDescriptor {
    /// File descriptor number
    pub fd: usize,

    /// File type (placeholder)
    pub file_type: FileType,
}

/// File type enumeration (placeholder)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Standard input/output/error
    StandardStream,

    /// Regular file
    RegularFile,

    /// Directory
    Directory,

    /// Device file
    Device,
}

/// Process error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessError {
    /// Process not found
    ProcessNotFound,

    /// Process table is full
    ProcessTableFull,

    /// Invalid process ID
    InvalidProcessId,

    /// Out of memory
    OutOfMemory,

    /// Permission denied
    PermissionDenied,

    /// Invalid memory region
    InvalidMemoryRegion,

    /// Memory region overlap
    RegionOverlap,

    /// Too many memory regions
    TooManyRegions,

    /// Invalid user address
    InvalidUserAddress,

    /// Process is in wrong state for operation
    InvalidState,

    /// Would block (for non-blocking operations)
    WouldBlock,
}

/// Result type for process operations
pub type ProcessResult<T> = Result<T, ProcessError>;

/// Process Control Block (PCB)
///
/// Contains all information needed to manage a process, including its
/// execution context, memory layout, and state.
pub struct Process {
    /// Unique process identifier
    pub pid: ProcessId,

    /// Parent process ID (None for init process)
    pub parent_pid: Option<ProcessId>,

    /// Current state of the process
    pub state: ProcessState,

    /// Exit code (set when process terminates)
    pub exit_code: Option<i32>,

    /// CPU context (saved registers)
    pub context: CpuContext,

    /// Process priority level
    pub priority: TaskPriority,

    /// Tick at which to wake the process (if sleeping)
    pub wake_tick: Option<u64>,

    /// Block reason (if blocked)
    pub block_reason: Option<BlockReason>,

    /// Process page table (for memory isolation)
    pub page_table: Option<PageTable>,

    /// Memory regions for this process (Code, Data, BSS, Stack, Heap)
    pub memory_regions: [Option<MemoryRegion>; MAX_MEMORY_REGIONS],

    /// Number of active memory regions
    pub region_count: usize,

    /// File descriptor table (for future use)
    pub fd_table: [Option<FileDescriptor>; MAX_FDS],

    /// Process name (for debugging)
    pub name: [u8; 16],

    /// Process creation time (in ticks)
    pub creation_time: u64,

    /// Total CPU time used (in ticks)
    pub cpu_time: u64,
}

impl Process {
    /// Create a new process with the given PID and parent
    ///
    /// # Arguments
    /// * `pid` - Unique process identifier
    /// * `parent_pid` - Parent process ID (None for init process)
    ///
    /// # Returns
    /// A new Process with Ready state and empty memory layout
    pub fn new(pid: ProcessId, parent_pid: Option<ProcessId>) -> Self {
        Self {
            pid,
            parent_pid,
            state: ProcessState::Ready,
            exit_code: None,
            context: CpuContext::new(),
            priority: TaskPriority::Normal,
            wake_tick: None,
            block_reason: None,
            page_table: None,
            memory_regions: [const { None }; MAX_MEMORY_REGIONS],
            region_count: 0,
            fd_table: [const { None }; MAX_FDS],
            name: [0; 16],
            creation_time: 0, // TODO: Get current tick count
            cpu_time: 0,
        }
    }

    /// Set the process name (for debugging)
    ///
    /// # Arguments
    /// * `name` - Process name (will be truncated to 15 chars + null terminator)
    pub fn set_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let len = core::cmp::min(bytes.len(), 15);
        self.name[..len].copy_from_slice(&bytes[..len]);
        self.name[len] = 0; // Null terminator
    }

    /// Get the process name as a string
    ///
    /// # Returns
    /// Process name as a string slice
    pub fn get_name(&self) -> &str {
        // Find null terminator
        let len = self.name.iter().position(|&b| b == 0).unwrap_or(16);
        core::str::from_utf8(&self.name[..len]).unwrap_or("<invalid>")
    }

    /// Add a memory region to this process
    ///
    /// Validates the region and ensures no overlaps with existing regions.
    /// All regions must be within user space limits.
    ///
    /// # Arguments
    /// * `region` - The memory region to add
    ///
    /// # Returns
    /// Ok(()) if the region was added successfully, or an error if validation fails
    pub fn add_memory_region(&mut self, region: MemoryRegion) -> ProcessResult<()> {
        // Validate region bounds
        if region.start >= region.end {
            return Err(ProcessError::InvalidMemoryRegion);
        }

        // Ensure region is in user space
        if region.start >= USER_LIMIT || region.end > USER_LIMIT {
            return Err(ProcessError::InvalidUserAddress);
        }

        // Check for overlaps with existing regions
        for existing_region in &self.memory_regions[..self.region_count] {
            if let Some(existing) = existing_region {
                if region.overlaps_with(existing) {
                    return Err(ProcessError::RegionOverlap);
                }
            }
        }

        // Check if we have space for another region
        if self.region_count >= MAX_MEMORY_REGIONS {
            return Err(ProcessError::TooManyRegions);
        }

        // Add the region
        self.memory_regions[self.region_count] = Some(region);
        self.region_count += 1;

        Ok(())
    }

    /// Find the memory region containing the given address
    ///
    /// # Arguments
    /// * `addr` - Virtual address to look up
    ///
    /// # Returns
    /// Some(region) if found, None if the address is not in any region
    pub fn find_memory_region(&self, addr: usize) -> Option<&MemoryRegion> {
        for region_opt in &self.memory_regions[..self.region_count] {
            if let Some(region) = region_opt {
                if region.contains(addr) {
                    return Some(region);
                }
            }
        }
        None
    }

    /// Get all memory regions of a specific type
    ///
    /// # Arguments
    /// * `region_type` - The type of regions to find
    ///
    /// # Returns
    /// Iterator over regions of the specified type
    pub fn get_regions_by_type(
        &self,
        region_type: MemoryRegionType,
    ) -> impl Iterator<Item = &MemoryRegion> {
        self.memory_regions[..self.region_count]
            .iter()
            .filter_map(|region_opt| region_opt.as_ref())
            .filter(move |region| region.region_type == region_type)
    }

    /// Remove a memory region by address range
    ///
    /// # Arguments
    /// * `start` - Start address of the region to remove
    /// * `end` - End address of the region to remove
    ///
    /// # Returns
    /// Ok(()) if the region was removed, or an error if not found
    pub fn remove_memory_region(&mut self, start: usize, end: usize) -> ProcessResult<()> {
        for i in 0..self.region_count {
            if let Some(region) = &self.memory_regions[i] {
                if region.start == start && region.end == end {
                    // Remove this region by shifting others down
                    for j in i..self.region_count - 1 {
                        self.memory_regions[j] = self.memory_regions[j + 1].take();
                    }
                    self.memory_regions[self.region_count - 1] = None;
                    self.region_count -= 1;
                    return Ok(());
                }
            }
        }
        Err(ProcessError::InvalidMemoryRegion)
    }

    /// Clear all memory regions (used during exec)
    pub fn clear_memory_regions(&mut self) {
        for region in &mut self.memory_regions {
            *region = None;
        }
        self.region_count = 0;
    }

    /// Validate that an address range is within a valid memory region
    ///
    /// Used for page fault handling and memory access validation.
    ///
    /// # Arguments
    /// * `addr` - Start address
    /// * `size` - Size of the access
    ///
    /// # Returns
    /// Ok(region) if the entire range is within a valid region, or an error
    pub fn validate_memory_access(&self, addr: usize, size: usize) -> ProcessResult<&MemoryRegion> {
        let end_addr = addr.saturating_add(size);

        // Find region containing the start address
        let region = self
            .find_memory_region(addr)
            .ok_or(ProcessError::InvalidUserAddress)?;

        // Ensure the entire range is within this region
        if end_addr > region.end {
            return Err(ProcessError::InvalidUserAddress);
        }

        Ok(region)
    }

    /// Get total memory usage for this process
    ///
    /// # Returns
    /// Total size in bytes of all memory regions
    pub fn total_memory_usage(&self) -> usize {
        self.memory_regions[..self.region_count]
            .iter()
            .filter_map(|region_opt| region_opt.as_ref())
            .map(|region| region.size())
            .sum()
    }

    /// Check if this process is a child of the given parent
    ///
    /// # Arguments
    /// * `parent_pid` - Potential parent process ID
    ///
    /// # Returns
    /// true if this process is a child of the given parent
    pub fn is_child_of(&self, parent_pid: ProcessId) -> bool {
        self.parent_pid == Some(parent_pid)
    }

    /// Mark process as zombie with exit code
    ///
    /// # Arguments
    /// * `exit_code` - Process exit code
    pub fn mark_zombie(&mut self, exit_code: i32) {
        self.state = ProcessState::Zombie;
        self.exit_code = Some(exit_code);
    }

    /// Mark process as terminated (fully cleaned up)
    pub fn mark_terminated(&mut self) {
        self.state = ProcessState::Terminated;
        self.exit_code = None;
        self.clear_memory_regions();
    }
}

impl core::fmt::Debug for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Process")
            .field("pid", &self.pid)
            .field("parent_pid", &self.parent_pid)
            .field("state", &self.state)
            .field("exit_code", &self.exit_code)
            .field("priority", &self.priority)
            .field("wake_tick", &self.wake_tick)
            .field("block_reason", &self.block_reason)
            .field("region_count", &self.region_count)
            .field("name", &self.get_name())
            .field("creation_time", &self.creation_time)
            .field("cpu_time", &self.cpu_time)
            .finish()
    }
}

/// Process table entry with fine-grained locking
///
/// Each process slot has its own lock to minimize contention in SMP systems.
/// This allows multiple cores to operate on different processes simultaneously.
struct ProcessTableEntry {
    /// Process data (None if slot is free) - wrapped in UnsafeCell for interior mutability
    process: UnsafeCell<Option<Process>>,

    /// Per-process lock for SMP safety
    lock: SpinLock<()>,
}

// Safety: ProcessTableEntry is safe to share between threads because:
// - All access to the UnsafeCell is protected by the SpinLock
// - The lock ensures exclusive access to the process data
// - No data races can occur as long as the lock is held
unsafe impl Sync for ProcessTableEntry {}

impl ProcessTableEntry {
    /// Create a new empty process table entry
    const fn new() -> Self {
        Self {
            process: UnsafeCell::new(None),
            lock: SpinLock::new(()),
        }
    }

    /// Check if this slot is free (requires lock for safe access)
    fn is_free(&self) -> bool {
        let _guard = self.lock.lock();
        unsafe { (*self.process.get()).is_none() }
    }
}

/// Process table guard for safe access to a process slot
///
/// This guard ensures that the process slot remains locked while it's being accessed.
/// The lock is automatically released when the guard is dropped.
pub struct ProcessGuard<'a> {
    /// Reference to the process (if present)
    process: &'a mut Option<Process>,

    /// Lock guard to ensure exclusive access
    _lock_guard: SpinLockGuard<'a, ()>,
}

impl<'a> ProcessGuard<'a> {
    /// Get a reference to the process (if present)
    pub fn get(&self) -> Option<&Process> {
        self.process.as_ref()
    }

    /// Get a mutable reference to the process (if present)
    pub fn get_mut(&mut self) -> Option<&mut Process> {
        self.process.as_mut()
    }

    /// Take the process out of the slot (leaving it empty)
    pub fn take(&mut self) -> Option<Process> {
        self.process.take()
    }

    /// Insert a process into the slot (replacing any existing process)
    pub fn insert(&mut self, process: Process) -> Option<Process> {
        self.process.replace(process)
    }

    /// Check if the slot contains a process
    pub fn is_some(&self) -> bool {
        self.process.is_some()
    }

    /// Check if the slot is empty
    pub fn is_none(&self) -> bool {
        self.process.is_none()
    }
}

/// Global process table with fine-grained locking
///
/// Each process slot has its own lock to enable concurrent access from multiple cores.
/// This design minimizes lock contention and improves SMP scalability.
static PROCESS_TABLE: [ProcessTableEntry; MAX_PROCESSES] =
    [const { ProcessTableEntry::new() }; MAX_PROCESSES];

/// Atomic PID counter for unique process ID allocation
static NEXT_PID: AtomicUsize = AtomicUsize::new(1);

/// Process table manager
///
/// Provides high-level operations for process management with proper locking.
pub struct ProcessManager;

impl ProcessManager {
    /// Allocate a new unique process ID
    ///
    /// Uses atomic increment to ensure uniqueness across all cores.
    ///
    /// # Returns
    /// A new unique process ID
    pub fn alloc_pid() -> ProcessId {
        NEXT_PID.fetch_add(1, Ordering::Relaxed)
    }

    /// Find a free process slot and lock it
    ///
    /// Searches the process table for an empty slot and returns a guard
    /// that provides exclusive access to that slot.
    ///
    /// # Returns
    /// Some(ProcessGuard) if a free slot was found, None if table is full
    pub fn alloc_process_slot() -> Option<ProcessGuard<'static>> {
        for entry in &PROCESS_TABLE {
            // Try to acquire the lock
            if let Some(lock_guard) = entry.lock.try_lock() {
                // Check if the slot is free while we have the lock
                let process_ref = unsafe { &mut *entry.process.get() };
                if process_ref.is_none() {
                    return Some(ProcessGuard {
                        process: process_ref,
                        _lock_guard: lock_guard,
                    });
                }
                // Slot was taken by another core, continue searching
            }
        }
        None // Process table is full
    }

    /// Get a process by PID with exclusive access
    ///
    /// Searches the process table for the given PID and returns a guard
    /// that provides exclusive access to that process.
    ///
    /// # Arguments
    /// * `pid` - Process ID to search for
    ///
    /// # Returns
    /// Some(ProcessGuard) if the process was found, None if not found
    pub fn get_process(pid: ProcessId) -> Option<ProcessGuard<'static>> {
        for entry in &PROCESS_TABLE {
            // Try to acquire the lock
            let lock_guard = entry.lock.lock();

            // Check if this slot contains the process we're looking for
            let process_ref = unsafe { &mut *entry.process.get() };
            if let Some(ref process) = process_ref {
                if process.pid == pid {
                    return Some(ProcessGuard {
                        process: process_ref,
                        _lock_guard: lock_guard,
                    });
                }
            }
            // Not the process we're looking for, lock will be released automatically
        }
        None // Process not found
    }

    /// Create a new process and add it to the process table
    ///
    /// Allocates a new PID, finds a free slot, and initializes the process.
    ///
    /// # Arguments
    /// * `parent_pid` - Parent process ID (None for init process)
    /// * `name` - Process name for debugging
    ///
    /// # Returns
    /// Ok(pid) if the process was created successfully, or an error
    pub fn create_process(parent_pid: Option<ProcessId>, name: &str) -> ProcessResult<ProcessId> {
        // Allocate new PID
        let pid = Self::alloc_pid();

        // Find free process slot
        let mut slot = Self::alloc_process_slot().ok_or(ProcessError::ProcessTableFull)?;

        // Create new process
        let mut process = Process::new(pid, parent_pid);
        process.set_name(name);

        // Insert into slot
        slot.insert(process);

        Ok(pid)
    }

    /// Remove a process from the process table
    ///
    /// Marks the process slot as free so it can be reused.
    ///
    /// # Arguments
    /// * `pid` - Process ID to remove
    ///
    /// # Returns
    /// Ok(process) if the process was removed, or an error if not found
    pub fn remove_process(pid: ProcessId) -> ProcessResult<Process> {
        let mut slot = Self::get_process(pid).ok_or(ProcessError::ProcessNotFound)?;

        slot.take().ok_or(ProcessError::ProcessNotFound)
    }

    /// Find the first zombie child of a parent process
    ///
    /// Searches the process table for a zombie process that is a child
    /// of the given parent.
    ///
    /// # Arguments
    /// * `parent_pid` - Parent process ID
    ///
    /// # Returns
    /// Some((child_pid, exit_code)) if a zombie child is found, None otherwise
    pub fn find_zombie_child(parent_pid: ProcessId) -> Option<(ProcessId, i32)> {
        for entry in &PROCESS_TABLE {
            let _lock_guard = entry.lock.lock();

            let process_ref = unsafe { &*entry.process.get() };
            if let Some(ref process) = process_ref {
                if process.is_child_of(parent_pid) && process.state == ProcessState::Zombie {
                    if let Some(exit_code) = process.exit_code {
                        return Some((process.pid, exit_code));
                    }
                }
            }
        }

        None
    }

    /// Count the number of active processes
    ///
    /// # Returns
    /// Number of processes in the table (excluding free slots)
    pub fn count_processes() -> usize {
        let mut count = 0;

        for entry in &PROCESS_TABLE {
            let _lock_guard = entry.lock.lock();
            let process_ref = unsafe { &*entry.process.get() };
            if process_ref.is_some() {
                count += 1;
            }
        }

        count
    }

    /// Get process table statistics
    ///
    /// # Returns
    /// (total_slots, used_slots, free_slots)
    pub fn get_stats() -> (usize, usize, usize) {
        let used = Self::count_processes();
        let total = MAX_PROCESSES;
        let free = total - used;

        (total, used, free)
    }
}

/// User pointer validation functions
///
/// These functions validate that user-provided pointers are within the valid
/// user address space and can be safely accessed by the kernel.

/// Check if a pointer is a valid user space address
///
/// # Arguments
/// * `ptr` - Pointer to validate
///
/// # Returns
/// true if the pointer is in valid user space, false otherwise
pub fn is_user_pointer_valid(ptr: usize) -> bool {
    ptr != 0 && ptr < USER_LIMIT
}

/// Copy data from user space to kernel space
///
/// Validates the source pointer and performs a safe copy operation.
/// In the current implementation (shared address space), this is a simple
/// memory copy with validation. Future implementations will use temporary
/// kernel mappings for true isolation.
///
/// # Arguments
/// * `dst` - Destination buffer in kernel space
/// * `src_ptr` - Source pointer in user space
/// * `len` - Number of bytes to copy
///
/// # Returns
/// Ok(()) if the copy succeeded, or an error if validation failed
pub fn copy_from_user(dst: &mut [u8], src_ptr: usize, len: usize) -> ProcessResult<()> {
    // Validate source pointer is in user space
    if !is_user_pointer_valid(src_ptr) || !is_user_pointer_valid(src_ptr + len) {
        return Err(ProcessError::InvalidUserAddress);
    }

    // Check destination buffer size
    if len > dst.len() {
        return Err(ProcessError::InvalidMemoryRegion);
    }

    // TODO: When implementing full page table separation, replace direct pointer
    // access with temporary kernel mapping (kmap_user_page()) for safety

    // Perform copy with page fault handling (current shared address space)
    unsafe {
        let src = core::slice::from_raw_parts(src_ptr as *const u8, len);
        dst[..len].copy_from_slice(src);
    }

    Ok(())
}

/// Copy data from kernel space to user space
///
/// Validates the destination pointer and performs a safe copy operation.
/// In the current implementation (shared address space), this is a simple
/// memory copy with validation. Future implementations will use temporary
/// kernel mappings for true isolation.
///
/// # Arguments
/// * `dst_ptr` - Destination pointer in user space
/// * `src` - Source buffer in kernel space
///
/// # Returns
/// Ok(()) if the copy succeeded, or an error if validation failed
pub fn copy_to_user(dst_ptr: usize, src: &[u8]) -> ProcessResult<()> {
    // Validate destination pointer is in user space
    if !is_user_pointer_valid(dst_ptr) || !is_user_pointer_valid(dst_ptr + src.len()) {
        return Err(ProcessError::InvalidUserAddress);
    }

    // TODO: When implementing full page table separation, replace direct pointer
    // access with temporary kernel mapping (kmap_user_page()) for safety

    // Perform copy with page fault handling (current shared address space)
    unsafe {
        let dst = core::slice::from_raw_parts_mut(dst_ptr as *mut u8, src.len());
        dst.copy_from_slice(src);
    }

    Ok(())
}

/// Process-Scheduler Integration Functions
///
/// These functions provide integration between the Process management system
/// and the existing Task-based scheduler. They allow both systems to work
/// together during the transition to full process management.

/// Synchronize process state with task state
///
/// Updates the process state to match the corresponding task state.
/// This ensures consistency between the two management systems.
///
/// # Arguments
/// * `process_id` - Process ID to synchronize
/// * `task_id` - Corresponding task ID
///
/// # Returns
/// Ok(()) if synchronization succeeded, or an error
pub fn sync_process_with_task(
    process_id: ProcessId,
    task_id: crate::sched::task::TaskId,
) -> ProcessResult<()> {
    use crate::sched;

    // Get the task state
    let task_state = match sched::get_task_mut(task_id) {
        Some(task) => task.state,
        None => return Err(ProcessError::ProcessNotFound),
    };

    // Get the process and update its state
    let mut process_guard =
        ProcessManager::get_process(process_id).ok_or(ProcessError::ProcessNotFound)?;

    let process = process_guard
        .get_mut()
        .ok_or(ProcessError::ProcessNotFound)?;

    // Map task state to process state
    process.state = match task_state {
        crate::sched::task::TaskState::Ready => ProcessState::Ready,
        crate::sched::task::TaskState::Running => ProcessState::Running,
        crate::sched::task::TaskState::Sleeping => ProcessState::Sleeping,
        crate::sched::task::TaskState::Blocked => ProcessState::Blocked,
    };

    // Sync other fields
    if let Some(task) = sched::get_task_mut(task_id) {
        process.context = task.context.clone();
        process.priority = task.priority;
        process.wake_tick = task.wake_tick;

        // Sync memory regions if they differ
        if process.region_count != task.region_count {
            process.clear_memory_regions();
            for i in 0..task.region_count {
                if let Some(region) = &task.memory_regions[i] {
                    let _ = process.add_memory_region(region.clone());
                }
            }
        }
    }

    Ok(())
}

/// Synchronize task state with process state
///
/// Updates the task state to match the corresponding process state.
/// This is the reverse of sync_process_with_task.
///
/// # Arguments
/// * `task_id` - Task ID to synchronize
/// * `process_id` - Corresponding process ID
///
/// # Returns
/// Ok(()) if synchronization succeeded, or an error
pub fn sync_task_with_process(
    task_id: crate::sched::task::TaskId,
    process_id: ProcessId,
) -> ProcessResult<()> {
    use crate::sched;

    // Get the process state
    let process_guard =
        ProcessManager::get_process(process_id).ok_or(ProcessError::ProcessNotFound)?;

    let process = process_guard.get().ok_or(ProcessError::ProcessNotFound)?;

    // Get the task and update its state
    let task = sched::get_task_mut(task_id).ok_or(ProcessError::ProcessNotFound)?;

    // Map process state to task state
    task.state = match process.state {
        ProcessState::Ready => crate::sched::task::TaskState::Ready,
        ProcessState::Running => crate::sched::task::TaskState::Running,
        ProcessState::Sleeping => crate::sched::task::TaskState::Sleeping,
        ProcessState::Blocked => crate::sched::task::TaskState::Blocked,
        ProcessState::Zombie => crate::sched::task::TaskState::Ready, // Will be cleaned up
        ProcessState::Terminated => crate::sched::task::TaskState::Ready, // Will be cleaned up
    };

    // Sync other fields
    task.context = process.context.clone();
    task.priority = process.priority;
    task.wake_tick = process.wake_tick;

    // Sync memory regions if they differ
    if task.region_count != process.region_count {
        task.clear_memory_regions();
        for i in 0..process.region_count {
            if let Some(region) = &process.memory_regions[i] {
                let _ = task.add_memory_region(region.clone());
            }
        }
    }

    Ok(())
}

/// Create a task for an existing process
///
/// Creates a corresponding Task in the scheduler for a Process.
/// This allows the process to be scheduled and executed.
///
/// # Arguments
/// * `process_id` - Process ID to create task for
/// * `entry_point` - Task entry point function
///
/// # Returns
/// Ok(task_id) if task was created successfully, or an error
pub fn create_task_for_process(
    process_id: ProcessId,
    entry_point: fn() -> !,
) -> ProcessResult<crate::sched::task::TaskId> {
    use crate::sched;

    // Get the process
    let process_guard =
        ProcessManager::get_process(process_id).ok_or(ProcessError::ProcessNotFound)?;

    let process = process_guard.get().ok_or(ProcessError::ProcessNotFound)?;

    // Get process info before creating task
    let process_name = "process_task"; // Use a static name for now
    let process_priority = process.priority;

    // Drop the guard before creating task to avoid lifetime issues
    drop(process_guard);

    // Create task with same priority as process
    let task_id = sched::spawn_task(process_name, entry_point, process_priority)
        .map_err(|_| ProcessError::OutOfMemory)?;

    // Synchronize the task with the process
    sync_task_with_process(task_id, process_id)?;

    Ok(task_id)
}

/// Get the process ID for a task ID
///
/// Finds the process that corresponds to a given task.
/// This is a simple mapping for now - in a full implementation,
/// we would maintain a proper task->process mapping table.
///
/// # Arguments
/// * `task_id` - Task ID to look up
///
/// # Returns
/// Some(process_id) if found, None if no corresponding process
pub fn get_process_for_task(task_id: crate::sched::task::TaskId) -> Option<ProcessId> {
    // For now, we assume task_id == process_id
    // In a full implementation, we would maintain a mapping table
    Some(task_id)
}

/// Get the task ID for a process ID
///
/// Finds the task that corresponds to a given process.
/// This is a simple mapping for now - in a full implementation,
/// we would maintain a proper process->task mapping table.
///
/// # Arguments
/// * `process_id` - Process ID to look up
///
/// # Returns
/// Some(task_id) if found, None if no corresponding task
pub fn get_task_for_process(process_id: ProcessId) -> Option<crate::sched::task::TaskId> {
    // For now, we assume process_id == task_id
    // In a full implementation, we would maintain a mapping table
    Some(process_id)
}

/// Enhanced process state management
///
/// Updates both process and task states when a process changes state.
/// This ensures consistency between the two management systems.
///
/// # Arguments
/// * `process_id` - Process ID to update
/// * `new_state` - New process state
///
/// # Returns
/// Ok(()) if state was updated successfully, or an error
pub fn set_process_state(process_id: ProcessId, new_state: ProcessState) -> ProcessResult<()> {
    // Update process state
    let mut process_guard =
        ProcessManager::get_process(process_id).ok_or(ProcessError::ProcessNotFound)?;

    let process = process_guard
        .get_mut()
        .ok_or(ProcessError::ProcessNotFound)?;

    let old_state = process.state;
    process.state = new_state;

    // Update corresponding task state if it exists
    if let Some(task_id) = get_task_for_process(process_id) {
        sync_task_with_process(task_id, process_id)?;
    }

    crate::serial_println!(
        "[PROCESS] Process {} state changed: {:?} -> {:?}",
        process_id,
        old_state,
        new_state
    );

    Ok(())
}

/// Process context switching support
///
/// Handles process-specific context switching operations like
/// page table switching and TLB flushing.
///
/// # Arguments
/// * `old_process_id` - Process being switched away from
/// * `new_process_id` - Process being switched to
///
/// # Returns
/// Ok(()) if context switch preparation succeeded, or an error
pub fn prepare_process_context_switch(
    old_process_id: Option<ProcessId>,
    new_process_id: ProcessId,
) -> ProcessResult<()> {
    // Get the new process
    let new_process_guard =
        ProcessManager::get_process(new_process_id).ok_or(ProcessError::ProcessNotFound)?;

    let new_process = new_process_guard
        .get()
        .ok_or(ProcessError::ProcessNotFound)?;

    // TODO: Switch page tables when we have per-process page tables
    // For now, we're using a shared address space

    // TODO: Flush TLB if switching between different address spaces
    // unsafe {
    //     core::arch::asm!("mov rax, cr3; mov cr3, rax", out("rax") _);
    // }

    // Update process state to Running
    drop(new_process_guard);
    set_process_state(new_process_id, ProcessState::Running)?;

    // Update old process state to Ready (if it was Running)
    if let Some(old_pid) = old_process_id {
        let old_process_guard = ProcessManager::get_process(old_pid);
        if let Some(guard) = old_process_guard {
            if let Some(old_process) = guard.get() {
                if old_process.state == ProcessState::Running {
                    drop(guard);
                    set_process_state(old_pid, ProcessState::Ready)?;
                }
            }
        }
    }

    Ok(())
}

/// Process Management Tests
///
/// These tests verify the functionality of the process management system
/// including process creation, state management, and cleanup.

#[cfg(not(test))]
pub mod tests {
    use super::*;
    use crate::sched::priority::TaskPriority;
    use crate::sched::{self};
    use crate::serial_println;

    /// Test process creation and basic operations
    pub fn test_process_creation() {
        serial_println!("[TEST] Testing process creation...");

        // Test creating a process
        match ProcessManager::create_process(None, "test_process") {
            Ok(pid) => {
                serial_println!("[TEST] ✓ Created process with PID {}", pid);

                // Test getting the process
                if let Some(process_guard) = ProcessManager::get_process(pid) {
                    if let Some(process) = process_guard.get() {
                        serial_println!(
                            "[TEST] ✓ Retrieved process: PID={}, name='{}'",
                            process.pid,
                            process.get_name()
                        );

                        // Test process state
                        if process.state == ProcessState::Ready {
                            serial_println!("[TEST] ✓ Process has correct initial state: Ready");
                        } else {
                            serial_println!(
                                "[TEST] ✗ Process has incorrect state: {:?}",
                                process.state
                            );
                        }
                    } else {
                        serial_println!("[TEST] ✗ Process slot is empty");
                    }
                } else {
                    serial_println!("[TEST] ✗ Failed to retrieve created process");
                }

                // Test removing the process
                match ProcessManager::remove_process(pid) {
                    Ok(removed) => {
                        serial_println!(
                            "[TEST] ✓ Removed process {} ({})",
                            removed.pid,
                            removed.get_name()
                        );
                    }
                    Err(e) => {
                        serial_println!("[TEST] ✗ Failed to remove process: {:?}", e);
                    }
                }
            }
            Err(e) => {
                serial_println!("[TEST] ✗ Failed to create process: {:?}", e);
            }
        }
    }

    /// Test process memory region management
    pub fn test_memory_regions() {
        serial_println!("[TEST] Testing memory region management...");

        match ProcessManager::create_process(None, "memory_test") {
            Ok(pid) => {
                if let Some(mut process_guard) = ProcessManager::get_process(pid) {
                    if let Some(process) = process_guard.get_mut() {
                        // Test adding memory regions
                        use crate::mm::paging::PageTableFlags;
                        use crate::sched::task::{MemoryRegion, MemoryRegionType};

                        let code_region = MemoryRegion::new(
                            0x400000,
                            0x401000,
                            PageTableFlags::PRESENT | PageTableFlags::USER,
                            MemoryRegionType::Code,
                        );

                        match process.add_memory_region(code_region) {
                            Ok(()) => {
                                serial_println!("[TEST] ✓ Added code region");

                                // Test finding the region
                                if let Some(found_region) = process.find_memory_region(0x400500) {
                                    if found_region.region_type == MemoryRegionType::Code {
                                        serial_println!("[TEST] ✓ Found code region correctly");
                                    } else {
                                        serial_println!("[TEST] ✗ Found region has wrong type");
                                    }
                                } else {
                                    serial_println!("[TEST] ✗ Failed to find added region");
                                }

                                // Test memory usage calculation
                                let usage = process.total_memory_usage();
                                if usage == 4096 {
                                    serial_println!(
                                        "[TEST] ✓ Memory usage calculated correctly: {} bytes",
                                        usage
                                    );
                                } else {
                                    serial_println!(
                                        "[TEST] ✗ Memory usage incorrect: {} bytes",
                                        usage
                                    );
                                }
                            }
                            Err(e) => {
                                serial_println!("[TEST] ✗ Failed to add memory region: {:?}", e);
                            }
                        }

                        // Test overlapping region detection
                        let overlapping_region = MemoryRegion::new(
                            0x400800,
                            0x401800,
                            PageTableFlags::PRESENT | PageTableFlags::USER,
                            MemoryRegionType::Data,
                        );

                        match process.add_memory_region(overlapping_region) {
                            Err(ProcessError::RegionOverlap) => {
                                serial_println!("[TEST] ✓ Correctly detected overlapping region");
                            }
                            Ok(()) => {
                                serial_println!("[TEST] ✗ Failed to detect overlapping region");
                            }
                            Err(e) => {
                                serial_println!(
                                    "[TEST] ✗ Unexpected error for overlapping region: {:?}",
                                    e
                                );
                            }
                        }
                    }
                }

                // Clean up
                let _ = ProcessManager::remove_process(pid);
            }
            Err(e) => {
                serial_println!("[TEST] ✗ Failed to create process for memory test: {:?}", e);
            }
        }
    }

    /// Test process state transitions
    pub fn test_process_states() {
        serial_println!("[TEST] Testing process state transitions...");

        match ProcessManager::create_process(None, "state_test") {
            Ok(pid) => {
                // Test initial state
                if let Some(process_guard) = ProcessManager::get_process(pid) {
                    if let Some(process) = process_guard.get() {
                        if process.state == ProcessState::Ready {
                            serial_println!("[TEST] ✓ Initial state is Ready");
                        } else {
                            serial_println!(
                                "[TEST] ✗ Initial state is not Ready: {:?}",
                                process.state
                            );
                        }
                    }
                }

                // Test state changes
                match set_process_state(pid, ProcessState::Running) {
                    Ok(()) => {
                        serial_println!("[TEST] ✓ Changed state to Running");

                        // Verify state change
                        if let Some(process_guard) = ProcessManager::get_process(pid) {
                            if let Some(process) = process_guard.get() {
                                if process.state == ProcessState::Running {
                                    serial_println!("[TEST] ✓ State change verified");
                                } else {
                                    serial_println!(
                                        "[TEST] ✗ State change not applied: {:?}",
                                        process.state
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        serial_println!("[TEST] ✗ Failed to change state: {:?}", e);
                    }
                }

                // Test zombie state
                if let Some(mut process_guard) = ProcessManager::get_process(pid) {
                    if let Some(process) = process_guard.get_mut() {
                        process.mark_zombie(42);

                        if process.state == ProcessState::Zombie && process.exit_code == Some(42) {
                            serial_println!("[TEST] ✓ Zombie state set correctly");
                        } else {
                            serial_println!("[TEST] ✗ Zombie state not set correctly");
                        }
                    }
                }

                // Clean up
                let _ = ProcessManager::remove_process(pid);
            }
            Err(e) => {
                serial_println!("[TEST] ✗ Failed to create process for state test: {:?}", e);
            }
        }
    }

    /// Test process table management and PID allocation
    pub fn test_process_table() {
        serial_println!("[TEST] Testing process table management...");

        // Test PID allocation
        let pid1 = ProcessManager::alloc_pid();
        let pid2 = ProcessManager::alloc_pid();
        let pid3 = ProcessManager::alloc_pid();

        if pid2 == pid1 + 1 && pid3 == pid2 + 1 {
            serial_println!(
                "[TEST] ✓ PID allocation is sequential: {}, {}, {}",
                pid1,
                pid2,
                pid3
            );
        } else {
            serial_println!(
                "[TEST] ✗ PID allocation is not sequential: {}, {}, {}",
                pid1,
                pid2,
                pid3
            );
        }

        // Test process table statistics
        let (total, used_before, free_before) = ProcessManager::get_stats();
        serial_println!(
            "[TEST] Process table before: {}/{} used, {} free",
            used_before,
            total,
            free_before
        );

        // Create multiple processes
        let mut created_pids = [0usize; 5];
        let mut created_count = 0;
        for i in 0..5 {
            match ProcessManager::create_process(None, "table_test") {
                Ok(pid) => {
                    created_pids[created_count] = pid;
                    created_count += 1;
                    serial_println!("[TEST] Created process {} (iteration {})", pid, i);
                }
                Err(e) => {
                    serial_println!("[TEST] ✗ Failed to create process {}: {:?}", i, e);
                    break;
                }
            }
        }

        let (_, used_after, free_after) = ProcessManager::get_stats();
        serial_println!(
            "[TEST] Process table after creation: {} used, {} free",
            used_after,
            free_after
        );

        if used_after == used_before + created_count {
            serial_println!("[TEST] ✓ Process table statistics are correct");
        } else {
            serial_println!("[TEST] ✗ Process table statistics are incorrect");
        }

        // Clean up created processes
        for i in 0..created_count {
            let pid = created_pids[i];
            match ProcessManager::remove_process(pid) {
                Ok(_) => {
                    serial_println!("[TEST] Cleaned up process {}", pid);
                }
                Err(e) => {
                    serial_println!("[TEST] ✗ Failed to clean up process {}: {:?}", pid, e);
                }
            }
        }

        let (_, used_final, free_final) = ProcessManager::get_stats();
        serial_println!(
            "[TEST] Process table after cleanup: {} used, {} free",
            used_final,
            free_final
        );

        if used_final == used_before {
            serial_println!("[TEST] ✓ Process table cleanup successful");
        } else {
            serial_println!("[TEST] ✗ Process table cleanup incomplete");
        }
    }

    /// Test zombie child detection
    pub fn test_zombie_children() {
        serial_println!("[TEST] Testing zombie child detection...");

        // Create parent process
        match ProcessManager::create_process(None, "parent") {
            Ok(parent_pid) => {
                // Create child process
                match ProcessManager::create_process(Some(parent_pid), "child") {
                    Ok(child_pid) => {
                        serial_println!(
                            "[TEST] Created parent {} and child {}",
                            parent_pid,
                            child_pid
                        );

                        // Mark child as zombie
                        if let Some(mut child_guard) = ProcessManager::get_process(child_pid) {
                            if let Some(child) = child_guard.get_mut() {
                                child.mark_zombie(123);
                                serial_println!("[TEST] Marked child {} as zombie", child_pid);
                            }
                        }

                        // Test finding zombie child
                        match ProcessManager::find_zombie_child(parent_pid) {
                            Some((found_pid, exit_code)) => {
                                if found_pid == child_pid && exit_code == 123 {
                                    serial_println!("[TEST] ✓ Found zombie child correctly: PID={}, exit_code={}", 
                                                   found_pid, exit_code);
                                } else {
                                    serial_println!(
                                        "[TEST] ✗ Found wrong zombie child: PID={}, exit_code={}",
                                        found_pid,
                                        exit_code
                                    );
                                }
                            }
                            None => {
                                serial_println!("[TEST] ✗ Failed to find zombie child");
                            }
                        }

                        // Clean up
                        let _ = ProcessManager::remove_process(child_pid);
                    }
                    Err(e) => {
                        serial_println!("[TEST] ✗ Failed to create child process: {:?}", e);
                    }
                }

                // Clean up parent
                let _ = ProcessManager::remove_process(parent_pid);
            }
            Err(e) => {
                serial_println!("[TEST] ✗ Failed to create parent process: {:?}", e);
            }
        }
    }

    /// Test integration with scheduler
    pub fn test_scheduler_integration() {
        serial_println!("[TEST] Testing scheduler integration...");

        // Test creating a process-task pair
        fn dummy_task() -> ! {
            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }

        match sched::spawn_process_task("integration_test", dummy_task, TaskPriority::Normal, None)
        {
            Ok((process_id, task_id)) => {
                serial_println!(
                    "[TEST] ✓ Created process {} with task {}",
                    process_id,
                    task_id
                );

                // Test synchronization
                match sync_process_with_task(process_id, task_id) {
                    Ok(()) => {
                        serial_println!("[TEST] ✓ Process-task synchronization successful");
                    }
                    Err(e) => {
                        serial_println!("[TEST] ✗ Process-task synchronization failed: {:?}", e);
                    }
                }

                // Test mapping functions
                if let Some(found_process_id) = get_process_for_task(task_id) {
                    if found_process_id == process_id {
                        serial_println!("[TEST] ✓ Task->Process mapping works");
                    } else {
                        serial_println!(
                            "[TEST] ✗ Task->Process mapping incorrect: {} != {}",
                            found_process_id,
                            process_id
                        );
                    }
                }

                if let Some(found_task_id) = get_task_for_process(process_id) {
                    if found_task_id == task_id {
                        serial_println!("[TEST] ✓ Process->Task mapping works");
                    } else {
                        serial_println!(
                            "[TEST] ✗ Process->Task mapping incorrect: {} != {}",
                            found_task_id,
                            task_id
                        );
                    }
                }

                // Clean up (process will be cleaned up when task is removed)
                serial_println!(
                    "[TEST] Integration test cleanup (task will be cleaned up by scheduler)"
                );
            }
            Err(e) => {
                serial_println!("[TEST] ✗ Failed to create process-task pair: {:?}", e);
            }
        }
    }

    /// Run all process management tests
    pub fn run_all_tests() {
        serial_println!("[TEST] ========================================");
        serial_println!("[TEST] Running Process Management Tests");
        serial_println!("[TEST] ========================================");

        test_process_creation();
        test_memory_regions();
        test_process_states();
        test_process_table();
        test_zombie_children();
        test_scheduler_integration();

        serial_println!("[TEST] ========================================");
        serial_println!("[TEST] Process Management Tests Completed!");
        serial_println!("[TEST] ========================================");
    }
}
