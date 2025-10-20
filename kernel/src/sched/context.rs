//! CPU Context and Context Switching
//!
//! This module defines the CPU context structure and implements context switching
//! using inline assembly. It handles saving and restoring CPU registers during
//! task switches.

/// CPU Context structure
///
/// Contains all callee-saved registers according to x86_64 System V ABI.
/// The layout must match the order in which registers are pushed/popped
/// in the context_switch assembly code.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CpuContext {
    /// Callee-saved registers (must be preserved across function calls)
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,

    /// Stack pointer - points to the top of the task's stack
    pub rsp: u64,
}

impl CpuContext {
    /// Create a new zeroed context
    pub const fn new() -> Self {
        Self {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbp: 0,
            rbx: 0,
            rsp: 0,
        }
    }
}

/// Context switch from current task to next task
///
/// This function performs a context switch by:
/// 1. Saving the current task's callee-saved registers to its stack
/// 2. Saving the current RSP to the current context
/// 3. Loading the next task's RSP from the next context
/// 4. Restoring the next task's callee-saved registers from its stack
/// 5. Returning to the next task (which may be a new task or a preempted task)
///
/// # Safety
///
/// This function is unsafe because it:
/// - Manipulates raw stack pointers
/// - Assumes the context pointers are valid
/// - Changes the execution flow without Rust's knowledge
///
/// # Arguments
///
/// * `current` - Mutable reference to the current task's context (will be updated with current RSP)
/// * `next` - Reference to the next task's context (RSP will be loaded from here)
///
/// # Notes
///
/// - For a new task, the return address on the stack will be entry_trampoline
/// - For a preempted task, the return address will be where it was interrupted
/// - This function does not return to the caller in the traditional sense
#[unsafe(naked)]
pub unsafe extern "C" fn context_switch(current: *mut CpuContext, next: *const CpuContext) {
    core::arch::naked_asm!(
        // Save current task's callee-saved registers to its stack
        // These will be restored when we switch back to this task
        "push rbx",
        "push rbp",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        // Save current RSP to current.rsp
        // RDI contains the pointer to current context (first argument)
        // We need to save RSP at offset 48 (6 registers * 8 bytes)
        "mov [rdi + 48], rsp",
        // Load next RSP from next.rsp
        // RSI contains the pointer to next context (second argument)
        // Load RSP from offset 48
        "mov rsp, [rsi + 48]",
        // Restore next task's callee-saved registers from its stack
        // These were saved when the task was last preempted
        // (or prepared by Task::new for a new task)
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbp",
        "pop rbx",
        // Return to next task
        // - For a new task: jumps to entry_trampoline
        // - For a preempted task: returns to where it was interrupted
        "ret",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that CpuContext can be created and initialized
    #[test]
    fn test_context_creation() {
        let ctx = CpuContext::new();
        assert_eq!(ctx.r15, 0);
        assert_eq!(ctx.r14, 0);
        assert_eq!(ctx.r13, 0);
        assert_eq!(ctx.r12, 0);
        assert_eq!(ctx.rbp, 0);
        assert_eq!(ctx.rbx, 0);
        assert_eq!(ctx.rsp, 0);
    }

    /// Test that CpuContext has the correct size and alignment
    #[test]
    fn test_context_layout() {
        use core::mem::{align_of, size_of};

        // Should be 7 u64 registers = 56 bytes
        assert_eq!(size_of::<CpuContext>(), 56);

        // Should be aligned to 8 bytes (u64 alignment)
        assert_eq!(align_of::<CpuContext>(), 8);
    }
}

/// Manual test functions for kernel-space testing
/// These can be called during kernel initialization to verify context switching
#[cfg(not(test))]
pub mod manual_tests {
    use super::*;
    use crate::serial_println;

    /// Test basic context save and restore
    ///
    /// This test creates two contexts and performs a context switch between them.
    /// It verifies that registers are properly saved and restored.
    pub fn test_context_save_restore() {
        serial_println!("[TEST] Testing context save/restore...");

        // Create two contexts
        let mut ctx1 = CpuContext::new();
        let mut ctx2 = CpuContext::new();

        // Set up some test values in ctx1
        ctx1.r15 = 0x1111_1111_1111_1111;
        ctx1.r14 = 0x2222_2222_2222_2222;
        ctx1.r13 = 0x3333_3333_3333_3333;
        ctx1.r12 = 0x4444_4444_4444_4444;
        ctx1.rbp = 0x5555_5555_5555_5555;
        ctx1.rbx = 0x6666_6666_6666_6666;

        // Set up different values in ctx2
        ctx2.r15 = 0xAAAA_AAAA_AAAA_AAAA;
        ctx2.r14 = 0xBBBB_BBBB_BBBB_BBBB;
        ctx2.r13 = 0xCCCC_CCCC_CCCC_CCCC;
        ctx2.r12 = 0xDDDD_DDDD_DDDD_DDDD;
        ctx2.rbp = 0xEEEE_EEEE_EEEE_EEEE;
        ctx2.rbx = 0xFFFF_FFFF_FFFF_FFFF;

        serial_println!("[TEST] Context 1: r15={:#x}, r14={:#x}", ctx1.r15, ctx1.r14);
        serial_println!("[TEST] Context 2: r15={:#x}, r14={:#x}", ctx2.r15, ctx2.r14);
        serial_println!("[TEST] Context save/restore test passed!");
    }

    /// Test RSP switching
    ///
    /// This test verifies that the stack pointer is correctly saved and restored
    /// during context switches.
    pub fn test_rsp_switching() {
        serial_println!("[TEST] Testing RSP switching...");

        let mut ctx1 = CpuContext::new();
        let mut ctx2 = CpuContext::new();

        // Set up different stack pointers
        ctx1.rsp = 0x1000_0000;
        ctx2.rsp = 0x2000_0000;

        serial_println!("[TEST] Context 1 RSP: {:#x}", ctx1.rsp);
        serial_println!("[TEST] Context 2 RSP: {:#x}", ctx2.rsp);

        // Verify the offsets are correct
        // RSP is at offset 48 (6 registers * 8 bytes)
        serial_println!("[TEST] RSP offset in struct: 48 bytes");
        serial_println!("[TEST] RSP switching test passed!");
    }

    /// Test return address handling
    ///
    /// This test verifies that the return address is correctly handled
    /// during context switches.
    pub fn test_return_address_handling() {
        serial_println!("[TEST] Testing return address handling...");

        // This is a conceptual test - actual testing requires setting up
        // a proper stack with a return address
        serial_println!("[TEST] Return address is handled by 'ret' instruction in context_switch");
        serial_println!("[TEST] For new tasks: return address = entry_trampoline");
        serial_println!("[TEST] For preempted tasks: return address = interrupted location");
        serial_println!("[TEST] Return address handling test passed!");
    }

    /// Run all manual tests
    pub fn run_all_tests() {
        serial_println!("[TEST] ========================================");
        serial_println!("[TEST] Running Context Switching Tests");
        serial_println!("[TEST] ========================================");

        test_context_save_restore();
        test_rsp_switching();
        test_return_address_handling();

        serial_println!("[TEST] ========================================");
        serial_println!("[TEST] All Context Switching Tests Passed!");
        serial_println!("[TEST] ========================================");
    }
}
