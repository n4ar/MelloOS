//! Task Management
//!
//! This module defines the Task Control Block (TCB) and task-related structures.
//! It handles task creation, state management, and stack allocation.

use super::context::CpuContext;

/// Task identifier type
pub type TaskId = usize;

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
    
    /// Task is sleeping (for future use)
    Sleeping,
}

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
    /// 
    /// # Returns
    /// A Result containing the new Task with Ready state, or an error if stack allocation fails
    pub fn new(id: TaskId, name: &'static str, entry_point: fn() -> !) -> SchedulerResult<Self> {
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
            rsp = rsp.offset(-1); *rsp = 0; // R15
            rsp = rsp.offset(-1); *rsp = 0; // R14
            rsp = rsp.offset(-1); *rsp = 0; // R13
            rsp = rsp.offset(-1); *rsp = 0; // R12
            rsp = rsp.offset(-1); *rsp = 0; // RBP
            rsp = rsp.offset(-1); *rsp = 0; // RBX
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
        
        Ok(Self {
            id,
            name,
            stack,
            stack_size: STACK_SIZE,
            state: TaskState::Ready,
            context,
        })
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
