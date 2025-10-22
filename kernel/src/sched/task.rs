//! Task Management
//!
//! This module defines the Task Control Block (TCB) and task-related structures.
//! It handles task creation, state management, and stack allocation.

use super::context::CpuContext;
use super::priority::TaskPriority;
use super::process_group::{Pid, Pgid, Sid, DeviceId};
use crate::mm::paging::PageTableFlags;
use crate::signal::{SigAction, signals};
use core::sync::atomic::{AtomicU64, Ordering};

/// Task identifier type
pub type TaskId = usize;

/// Memory region types for process memory tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// Code segment (.text)
    Code,
    /// Data segment (.data)
    Data,
    /// BSS segment (uninitialized data)
    Bss,
    /// Stack segment
    Stack,
    /// Heap segment (future use)
    Heap,
}

/// Memory region descriptor for process memory tracking
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// Start virtual address (inclusive)
    pub start: usize,
    /// End virtual address (exclusive)
    pub end: usize,
    /// Page table flags for this region
    pub flags: PageTableFlags,
    /// Type of memory region
    pub region_type: MemoryRegionType,
}

impl MemoryRegion {
    /// Create a new memory region
    pub fn new(
        start: usize,
        end: usize,
        flags: PageTableFlags,
        region_type: MemoryRegionType,
    ) -> Self {
        Self {
            start,
            end,
            flags,
            region_type,
        }
    }

    /// Check if this region contains the given address
    pub fn contains(&self, addr: usize) -> bool {
        addr >= self.start && addr < self.end
    }

    /// Check if this region overlaps with another region
    pub fn overlaps_with(&self, other: &MemoryRegion) -> bool {
        !(self.end <= other.start || other.end <= self.start)
    }

    /// Get the size of this region in bytes
    pub fn size(&self) -> usize {
        self.end - self.start
    }

    /// Check if the region is page-aligned
    pub fn is_page_aligned(&self) -> bool {
        (self.start % 4096 == 0) && (self.end % 4096 == 0)
    }
}

/// Scheduler error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerError {
    /// Failed to allocate memory for task stack
    OutOfMemory,
    /// Task table is full (maximum tasks reached)
    TooManyTasks,
    /// Invalid task ID
    InvalidTaskId,
    /// Runqueue is full
    RunqueueFull,
    /// Memory region overlap detected
    RegionOverlap,
    /// Invalid memory region (start >= end)
    InvalidRegion,
    /// Address not in user space
    InvalidUserAddress,
    /// Too many memory regions
    TooManyRegions,
}

/// Result type for scheduler operations
pub type SchedulerResult<T> = Result<T, SchedulerError>;

/// Task state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is ready to run and waiting in the runqueue
    Ready,

    /// Task is currently running on the CPU
    Running,

    /// Task is sleeping (waiting for wake_tick)
    Sleeping,

    /// Task is blocked on IPC
    Blocked,
}

/// Maximum number of memory regions per task
const MAX_MEMORY_REGIONS: usize = 16;

/// Maximum number of signals (64 signals, 0-63)
const MAX_SIGNALS: usize = 64;

/// User space address limit (512GB)
pub const USER_LIMIT: usize = 0x0000_8000_0000_0000;

/// Task Control Block (TCB)
///
/// Contains all information needed to manage a task, including its
/// execution context, stack, and state.
#[derive(Debug)]
pub struct Task {
    /// Unique task identifier
    pub id: TaskId,

    /// Human-readable task name
    pub name: &'static str,

    /// Pointer to the task's stack (base address)
    pub stack: *mut u8,

    /// Size of the task's stack in bytes
    pub stack_size: usize,

    /// Current state of the task
    pub state: TaskState,

    /// CPU context (saved registers)
    pub context: CpuContext,

    /// Task priority level
    pub priority: TaskPriority,

    /// Tick at which to wake the task (if sleeping)
    pub wake_tick: Option<u64>,

    /// Port ID the task is blocked on (if blocked on IPC)
    pub blocked_on_port: Option<usize>,

    /// Memory regions for this task (Code, Data, BSS, Stack)
    pub memory_regions: [Option<MemoryRegion>; MAX_MEMORY_REGIONS],

    /// Number of active memory regions
    pub region_count: usize,

    /// Signal handlers for each signal (indexed by signal number)
    pub signal_handlers: [SigAction; MAX_SIGNALS],

    /// Pending signals bitset (bit N = signal N is pending)
    /// Uses atomic operations for race-free signal delivery in SMP
    pub pending_signals: AtomicU64,

    /// Signal mask (bit N = signal N is blocked)
    /// Uses atomic operations for race-free mask updates
    pub signal_mask: AtomicU64,

    /// Process ID (same as task ID for now)
    pub pid: Pid,

    /// Parent process ID
    pub ppid: Pid,

    /// Process group ID
    pub pgid: Pgid,

    /// Session ID
    pub sid: Sid,

    /// Controlling terminal device (if any)
    pub tty: Option<DeviceId>,

    /// Last syscall number executed (for debugging/panic dumps)
    pub last_syscall: Option<usize>,
}

impl Task {
    /// Create a new task with the given entry point
    ///
    /// This function:
    /// 1. Allocates an 8KB stack from the kernel heap
    /// 2. Prepares the initial stack frame with entry_trampoline as return address
    /// 3. Sets up callee-saved registers (R12 holds the entry_point)
    /// 4. Initializes the CPU context with the prepared stack pointer
    ///
    /// # Arguments
    /// * `id` - Unique task identifier
    /// * `name` - Human-readable task name
    /// * `entry_point` - Function pointer to the task's entry point
    /// * `priority` - Task priority level
    ///
    /// # Returns
    /// A Result containing the new Task with Ready state, or an error if stack allocation fails
    pub fn new(
        id: TaskId,
        name: &'static str,
        entry_point: fn() -> !,
        priority: TaskPriority,
    ) -> SchedulerResult<Self> {
        use crate::mm::allocator::kmalloc;

        // 1. Allocate 8KB stack
        const STACK_SIZE: usize = 8192;
        let stack = kmalloc(STACK_SIZE);

        if stack.is_null() {
            return Err(SchedulerError::OutOfMemory);
        }

        // 2. Calculate stack top (stack grows downward)
        let stack_top = (stack as usize) + STACK_SIZE;

        // 3. Prepare initial stack frame
        // The stack will be set up so that when context_switch does 'ret',
        // it will jump to entry_trampoline
        let mut rsp = stack_top as *mut u64;

        unsafe {
            // Push entry_point as an argument (will be below the registers)
            // This will be accessible from entry_trampoline
            rsp = rsp.offset(-1);
            *rsp = entry_point as u64;

            // Push entry_trampoline as return address
            rsp = rsp.offset(-1);
            *rsp = entry_trampoline as u64;

            // Push callee-saved registers (will be popped by context_switch)
            // These are pushed in reverse order of how they'll be popped
            rsp = rsp.offset(-1);
            *rsp = 0; // R15
            rsp = rsp.offset(-1);
            *rsp = 0; // R14
            rsp = rsp.offset(-1);
            *rsp = 0; // R13
            rsp = rsp.offset(-1);
            *rsp = 0; // R12
            rsp = rsp.offset(-1);
            *rsp = 0; // RBP
            rsp = rsp.offset(-1);
            *rsp = 0; // RBX
        }

        // 4. Create CPU context
        let context = CpuContext {
            rsp: rsp as u64,
            rbx: 0,
            rbp: 0,
            r12: entry_point as u64,
            r13: 0,
            r14: 0,
            r15: 0,
        };

        // Initialize signal handlers with defaults
        let signal_handlers = Self::init_default_signal_handlers();

        Ok(Self {
            id,
            name,
            stack,
            stack_size: STACK_SIZE,
            state: TaskState::Ready,
            context,
            priority,
            wake_tick: None,
            blocked_on_port: None,
            memory_regions: [const { None }; MAX_MEMORY_REGIONS],
            region_count: 0,
            signal_handlers,
            pending_signals: AtomicU64::new(0),
            signal_mask: AtomicU64::new(0),
            pid: id,        // PID = task ID
            ppid: 0,        // Will be set by parent
            pgid: id,       // Initially, pgid = pid
            sid: id,        // Initially, sid = pid (for init process)
            tty: None,      // No controlling terminal initially
            last_syscall: None, // No syscall executed yet
        })
    }

    /// Add a memory region to this task
    ///
    /// Validates the region and ensures no overlaps with existing regions.
    /// All regions must be within user space limits.
    ///
    /// # Arguments
    /// * `region` - The memory region to add
    ///
    /// # Returns
    /// Ok(()) if the region was added successfully, or an error if validation fails
    pub fn add_memory_region(&mut self, region: MemoryRegion) -> SchedulerResult<()> {
        // Validate region bounds
        if region.start >= region.end {
            return Err(SchedulerError::InvalidRegion);
        }

        // Ensure region is in user space
        if region.start >= USER_LIMIT || region.end > USER_LIMIT {
            return Err(SchedulerError::InvalidUserAddress);
        }

        // Check for overlaps with existing regions
        for existing_region in &self.memory_regions[..self.region_count] {
            if let Some(existing) = existing_region {
                if region.overlaps_with(existing) {
                    return Err(SchedulerError::RegionOverlap);
                }
            }
        }

        // Check if we have space for another region
        if self.region_count >= MAX_MEMORY_REGIONS {
            return Err(SchedulerError::TooManyRegions);
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
    pub fn remove_memory_region(&mut self, start: usize, end: usize) -> SchedulerResult<()> {
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
        Err(SchedulerError::InvalidRegion)
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
    pub fn validate_memory_access(
        &self,
        addr: usize,
        size: usize,
    ) -> SchedulerResult<&MemoryRegion> {
        let end_addr = addr.saturating_add(size);

        // Find region containing the start address
        let region = self
            .find_memory_region(addr)
            .ok_or(SchedulerError::InvalidUserAddress)?;

        // Ensure the entire range is within this region
        if end_addr > region.end {
            return Err(SchedulerError::InvalidUserAddress);
        }

        Ok(region)
    }

    /// Get total memory usage for this task
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

    /// Clear all memory regions (used during exec)
    pub fn clear_memory_regions(&mut self) {
        for region in &mut self.memory_regions {
            *region = None;
        }
        self.region_count = 0;
    }

    /// Initialize default signal handlers for a new task
    ///
    /// Sets up the default signal actions according to POSIX semantics:
    /// - Most signals terminate the process
    /// - Some signals are ignored by default (SIGCHLD, SIGURG, SIGWINCH)
    /// - Some signals stop the process (SIGSTOP, SIGTSTP, SIGTTIN, SIGTTOU)
    /// - SIGCONT continues a stopped process
    ///
    /// # Returns
    /// Array of SigAction structures with default handlers
    fn init_default_signal_handlers() -> [SigAction; MAX_SIGNALS] {
        let mut handlers = [SigAction::default(); MAX_SIGNALS];

        // Set ignored signals
        handlers[signals::SIGCHLD as usize] = SigAction::ignore();
        handlers[signals::SIGURG as usize] = SigAction::ignore();
        handlers[signals::SIGWINCH as usize] = SigAction::ignore();

        // All other signals use default action (handled by kernel)
        // SIGKILL and SIGSTOP cannot be caught or ignored (enforced elsewhere)

        handlers
    }

    /// Reset signal handlers to default (used during exec)
    ///
    /// After exec, all signal handlers are reset to their default actions,
    /// except for signals that were set to SIG_IGN which remain ignored.
    pub fn reset_signal_handlers(&mut self) {
        for (sig_num, handler) in self.signal_handlers.iter_mut().enumerate() {
            // Keep ignored signals ignored, reset everything else to default
            if !matches!(handler.handler, crate::signal::SigHandler::Ignore) {
                *handler = SigAction::default();
            }
        }
        // Clear pending signals atomically
        self.pending_signals.store(0, Ordering::Release);
        // Keep signal mask (inherited across exec)
    }

    /// Check if a signal is pending and not blocked
    ///
    /// # Arguments
    /// * `signal` - Signal number to check
    ///
    /// # Returns
    /// true if the signal is pending and not blocked
    pub fn has_pending_signal(&self, signal: u32) -> bool {
        if signal >= MAX_SIGNALS as u32 {
            return false;
        }
        let mask = 1u64 << signal;
        let pending = self.pending_signals.load(Ordering::Acquire);
        let blocked = self.signal_mask.load(Ordering::Acquire);
        (pending & mask) != 0 && (blocked & mask) == 0
    }

    /// Get the next pending unblocked signal
    ///
    /// # Returns
    /// Some(signal_number) if there's a pending unblocked signal, None otherwise
    pub fn next_pending_signal(&self) -> Option<u32> {
        let pending = self.pending_signals.load(Ordering::Acquire);
        let blocked = self.signal_mask.load(Ordering::Acquire);
        let unblocked_pending = pending & !blocked;
        if unblocked_pending == 0 {
            return None;
        }
        // Find the lowest set bit (lowest signal number)
        Some(unblocked_pending.trailing_zeros())
    }

    /// Clear a pending signal (atomically)
    ///
    /// # Arguments
    /// * `signal` - Signal number to clear
    pub fn clear_pending_signal(&self, signal: u32) {
        if signal < MAX_SIGNALS as u32 {
            let mask = 1u64 << signal;
            // Use fetch_and with inverted mask to clear the bit atomically
            self.pending_signals.fetch_and(!mask, Ordering::Release);
        }
    }

    /// Add a signal to the pending set (atomically)
    ///
    /// This is the core function for signal delivery. It uses atomic
    /// fetch_or to ensure race-free signal delivery in SMP environments.
    ///
    /// # Arguments
    /// * `signal` - Signal number to add
    ///
    /// # Returns
    /// true if the signal was added, false if invalid signal number
    pub fn add_pending_signal(&self, signal: u32) -> bool {
        if signal >= MAX_SIGNALS as u32 {
            return false;
        }
        let mask = 1u64 << signal;
        // Use fetch_or to set the bit atomically
        self.pending_signals.fetch_or(mask, Ordering::Release);
        true
    }

    /// Set signal mask (atomically)
    ///
    /// # Arguments
    /// * `mask` - New signal mask value
    pub fn set_signal_mask(&self, mask: u64) {
        self.signal_mask.store(mask, Ordering::Release);
    }

    /// Get signal mask (atomically)
    ///
    /// # Returns
    /// Current signal mask value
    pub fn get_signal_mask(&self) -> u64 {
        self.signal_mask.load(Ordering::Acquire)
    }

    /// Block signals (add to mask atomically)
    ///
    /// # Arguments
    /// * `mask` - Signals to block (bit N = block signal N)
    pub fn block_signals(&self, mask: u64) {
        self.signal_mask.fetch_or(mask, Ordering::Release);
    }

    /// Unblock signals (remove from mask atomically)
    ///
    /// # Arguments
    /// * `mask` - Signals to unblock (bit N = unblock signal N)
    pub fn unblock_signals(&self, mask: u64) {
        self.signal_mask.fetch_and(!mask, Ordering::Release);
    }
}

/// Entry trampoline for new tasks
///
/// This function is called when a new task is first scheduled.
/// The entry_point function pointer is on the stack (pushed by Task::new).
/// If the entry_point ever returns (which it shouldn't), we panic.
///
/// # Safety
/// This function uses inline assembly to extract the entry point from the stack.
/// It must only be called through the context switch mechanism.
#[unsafe(naked)]
#[no_mangle]
pub extern "C" fn entry_trampoline() -> ! {
    core::arch::naked_asm!(
        // Pop the entry_point from the stack (it was pushed by Task::new)
        "pop rax",

        // Save entry_point in a callee-saved register
        "mov r12, rax",

        // Enable interrupts before calling the task
        // (they were disabled during the interrupt handler)
        "sti",

        // Align stack to 16 bytes (required by System V ABI)
        // The stack should be 16-byte aligned before a call instruction
        "and rsp, -16",

        // R12 contains the entry_point function pointer
        // Call the entry point
        "call r12",

        // If we reach here, the task returned (which shouldn't happen)
        // We need to panic, but we can't call panic directly from naked functions
        // So we'll call a helper function
        "call {task_returned_panic}",

        // Infinite loop as fallback (should never reach here)
        "2:",
        "hlt",
        "jmp 2b",

        task_returned_panic = sym task_returned_panic,
    )
}

/// Helper function called when a task returns unexpectedly
///
/// This is called from entry_trampoline if the task's entry point returns.
/// Tasks should never return, so this is a critical error.
#[inline(never)]
fn task_returned_panic() -> ! {
    panic!("[SCHED] CRITICAL: Task returned from entry point!");
}
