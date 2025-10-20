//! Page Fault Handler
//!
//! This module implements the page fault handler for memory protection
//! in user-mode processes. It detects user space faults and terminates
//! processes that access invalid memory.

use crate::sched;
use crate::serial_println;
use crate::user::process::{ProcessManager, ProcessState};

/// Page fault error code bits
const PF_PRESENT: u64 = 1 << 0; // Page was present
const PF_WRITE: u64 = 1 << 1; // Fault was caused by write
const PF_USER: u64 = 1 << 2; // Fault occurred in user mode
const PF_RESERVED: u64 = 1 << 3; // Reserved bit was set
const PF_INSTR: u64 = 1 << 4; // Fault was caused by instruction fetch

/// Page fault handler entry point
///
/// This function is called when a page fault occurs. It analyzes the fault
/// and determines the appropriate action:
/// - User space faults: Terminate the process
/// - Kernel space faults: Panic (should not happen in normal operation)
///
/// # Arguments
/// * `error_code` - Page fault error code from CPU
/// * `fault_addr` - Faulting virtual address (from CR2 register)
/// * `rip` - Instruction pointer where fault occurred
///
/// # Safety
/// This function is called from interrupt context and must be interrupt-safe.
#[no_mangle]
pub extern "C" fn page_fault_handler(error_code: u64, fault_addr: u64, rip: u64) {
    let cpu_id = unsafe { crate::arch::x86_64::smp::percpu::percpu_current().id };

    // Read CR2 register to get the faulting address
    let cr2_addr = unsafe {
        let addr: u64;
        core::arch::asm!("mov {}, cr2", out(reg) addr);
        addr
    };

    // Use CR2 if fault_addr is 0 (some calling conventions may not pass it)
    let actual_fault_addr = if fault_addr == 0 {
        cr2_addr
    } else {
        fault_addr
    };

    serial_println!(
        "[FAULT][cpu{}] Page fault at RIP=0x{:x}, fault_addr=0x{:x}, error=0x{:x}",
        cpu_id,
        rip,
        actual_fault_addr,
        error_code
    );

    // Decode error code
    let present = (error_code & PF_PRESENT) != 0;
    let write = (error_code & PF_WRITE) != 0;
    let user_mode = (error_code & PF_USER) != 0;
    let reserved = (error_code & PF_RESERVED) != 0;
    let instruction_fetch = (error_code & PF_INSTR) != 0;

    serial_println!(
        "[FAULT] Error details: present={}, write={}, user={}, reserved={}, instr_fetch={}",
        present,
        write,
        user_mode,
        reserved,
        instruction_fetch
    );

    // Check if this is a user space fault
    if user_mode {
        handle_user_page_fault(actual_fault_addr, error_code, rip);
    } else {
        handle_kernel_page_fault(actual_fault_addr, error_code, rip);
    }
}

/// Handle page fault in user space
///
/// User space page faults indicate that a user process accessed invalid memory.
/// This function terminates the offending process and logs the fault details.
///
/// # Arguments
/// * `fault_addr` - Faulting virtual address
/// * `error_code` - Page fault error code
/// * `rip` - Instruction pointer where fault occurred
fn handle_user_page_fault(fault_addr: u64, error_code: u64, rip: u64) {
    let cpu_id = unsafe { crate::arch::x86_64::smp::percpu::percpu_current().id };

    // Get current task/process information
    let current_task_info = match sched::get_current_task_info() {
        Some(info) => info,
        None => {
            serial_println!(
                "[FAULT][cpu{}] User page fault but no current task!",
                cpu_id
            );
            panic!("[FAULT] User page fault with no current task");
        }
    };

    let current_task_id = current_task_info.0;

    serial_println!(
        "[FAULT][cpu{}] User process {} page fault:",
        cpu_id,
        current_task_id
    );
    serial_println!("[FAULT]   Fault address: 0x{:x}", fault_addr);
    serial_println!("[FAULT]   Instruction pointer: 0x{:x}", rip);
    serial_println!("[FAULT]   Error code: 0x{:x}", error_code);

    // Decode the fault type for logging
    let fault_type = if (error_code & PF_PRESENT) == 0 {
        "Page not present"
    } else if (error_code & PF_WRITE) != 0 {
        "Write to read-only page"
    } else if (error_code & PF_INSTR) != 0 {
        "Instruction fetch from non-executable page"
    } else if (error_code & PF_RESERVED) != 0 {
        "Reserved bit violation"
    } else {
        "Unknown fault type"
    };

    serial_println!("[FAULT]   Fault type: {}", fault_type);

    // Check if the fault address is within valid user space
    use crate::user::process::USER_LIMIT;
    if fault_addr >= USER_LIMIT as u64 {
        serial_println!(
            "[FAULT]   Fault address 0x{:x} is outside user space limit 0x{:x}",
            fault_addr,
            USER_LIMIT
        );
    }

    // Get current task to check memory regions
    if let Some(current_task) = sched::get_task_mut(current_task_id) {
        serial_println!(
            "[FAULT]   Task '{}' has {} memory regions:",
            current_task.name,
            current_task.region_count
        );

        // Log all memory regions for debugging
        for i in 0..current_task.region_count {
            if let Some(region) = &current_task.memory_regions[i] {
                serial_println!(
                    "[FAULT]     Region {}: 0x{:x}-0x{:x} ({:?})",
                    i,
                    region.start,
                    region.end,
                    region.region_type
                );
            }
        }

        // Check if fault address is within any valid region
        let in_valid_region = current_task
            .find_memory_region(fault_addr as usize)
            .is_some();

        if !in_valid_region {
            serial_println!(
                "[FAULT]   Fault address 0x{:x} is not in any valid memory region",
                fault_addr
            );
        } else {
            serial_println!(
                "[FAULT]   Fault address 0x{:x} is within a valid region (permission violation)",
                fault_addr
            );
        }
    }

    // Terminate the process
    serial_println!(
        "[FAULT] Terminating process {} due to page fault",
        current_task_id
    );

    // Mark process as terminated in process table
    if let Some(mut process_guard) = ProcessManager::get_process(current_task_id) {
        if let Some(process) = process_guard.get_mut() {
            process.state = ProcessState::Terminated;
            serial_println!("[FAULT] Process {} marked as terminated", process.pid);
        }
    }

    // Mark task as terminated in scheduler
    if let Some(current_task) = sched::get_task_mut(current_task_id) {
        current_task.state = crate::sched::task::TaskState::Ready; // Will be cleaned up
        serial_println!("[FAULT] Task {} marked for cleanup", current_task_id);
    }

    serial_println!("[FAULT] Yielding to scheduler after process termination");

    // Yield to scheduler - this process should never run again
    sched::yield_now();

    // Should never reach here
    panic!("[FAULT] Returned from yield after process termination");
}

/// Handle page fault in kernel space
///
/// Kernel space page faults are critical errors that indicate bugs in the kernel.
/// This function logs detailed information and panics.
///
/// # Arguments
/// * `fault_addr` - Faulting virtual address
/// * `error_code` - Page fault error code
/// * `rip` - Instruction pointer where fault occurred
fn handle_kernel_page_fault(fault_addr: u64, error_code: u64, rip: u64) -> ! {
    let cpu_id = unsafe { crate::arch::x86_64::smp::percpu::percpu_current().id };

    serial_println!("[FAULT][cpu{}] CRITICAL: Kernel page fault!", cpu_id);
    serial_println!("[FAULT]   Fault address: 0x{:x}", fault_addr);
    serial_println!("[FAULT]   Instruction pointer: 0x{:x}", rip);
    serial_println!("[FAULT]   Error code: 0x{:x}", error_code);

    // Decode error code for detailed logging
    let present = (error_code & PF_PRESENT) != 0;
    let write = (error_code & PF_WRITE) != 0;
    let reserved = (error_code & PF_RESERVED) != 0;
    let instruction_fetch = (error_code & PF_INSTR) != 0;

    serial_println!("[FAULT] Kernel fault details:");
    serial_println!("[FAULT]   Page present: {}", present);
    serial_println!("[FAULT]   Write access: {}", write);
    serial_println!("[FAULT]   Reserved bit: {}", reserved);
    serial_println!("[FAULT]   Instruction fetch: {}", instruction_fetch);

    // Check if this might be a stack overflow
    if fault_addr < 0x1000 {
        serial_println!("[FAULT] Possible null pointer dereference");
    } else if fault_addr >= 0xFFFF_8000_0000_0000 && fault_addr < 0xFFFF_A000_0000_0000 {
        serial_println!("[FAULT] Fault in kernel direct map region");
    } else if fault_addr >= 0xFFFF_C000_0000_0000 {
        serial_println!("[FAULT] Fault in kernel code/data region");
    }

    // Get current task info for context
    if let Some((task_id, _)) = sched::get_current_task_info() {
        serial_println!("[FAULT] Current task: {}", task_id);

        if let Some(task) = sched::get_task_mut(task_id) {
            serial_println!("[FAULT] Task name: '{}'", task.name);
            serial_println!(
                "[FAULT] Task stack: 0x{:x} (size: {})",
                task.stack as u64,
                task.stack_size
            );
        }
    } else {
        serial_println!("[FAULT] No current task (early boot or idle)");
    }

    panic!(
        "[FAULT] Kernel page fault at 0x{:x} from RIP 0x{:x}",
        fault_addr, rip
    );
}

/// Assembly wrapper for page fault handler
///
/// This function is called from the IDT entry and sets up the proper
/// calling convention for the Rust handler.
#[unsafe(naked)]
#[no_mangle]
pub extern "C" fn page_fault_wrapper() {
    core::arch::naked_asm!(
        // Save all registers
        "push rax",
        "push rcx",
        "push rdx",
        "push rbx",
        "push rbp",
        "push rsi",
        "push rdi",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // Get error code (pushed by CPU before calling this handler)
        // It's at [rsp + 15*8] (after we pushed 15 registers)
        "mov rdi, [rsp + 15*8]",  // error_code -> first argument

        // Get CR2 (fault address)
        "mov rax, cr2",
        "mov rsi, rax",           // fault_addr -> second argument

        // Get RIP (return address, pushed by CPU)
        // It's at [rsp + 16*8] (after error code and 15 registers)
        "mov rdx, [rsp + 16*8]",  // rip -> third argument

        // Call the Rust handler
        "call {handler}",

        // Restore all registers
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rdi",
        "pop rsi",
        "pop rbp",
        "pop rbx",
        "pop rdx",
        "pop rcx",
        "pop rax",

        // Remove error code from stack (CPU pushed it)
        "add rsp, 8",

        // Return from interrupt
        "iretq",

        handler = sym page_fault_handler,
    )
}

/// Initialize page fault handler in IDT
///
/// This function should be called during kernel initialization to set up
/// the page fault handler in the Interrupt Descriptor Table.
///
/// # Safety
/// This function modifies the IDT and should only be called during kernel init.
pub unsafe fn init_page_fault_handler() {
    serial_println!("[FAULT] Initializing page fault handler...");

    // TODO: Set up IDT entry for page fault (interrupt 14)
    // This would involve:
    // 1. Getting a reference to the IDT
    // 2. Setting entry 14 to point to page_fault_wrapper
    // 3. Configuring the entry as an interrupt gate with IST if needed

    // For now, we'll just log that the handler is ready
    serial_println!("[FAULT] Page fault handler ready (IDT setup TODO)");
}

/// Test function for page fault handling
///
/// This function can be called to test the page fault handler by
/// deliberately causing a page fault.
///
/// # Safety
/// This function will cause a page fault and should only be used for testing.
#[allow(dead_code)]
pub unsafe fn test_page_fault() {
    serial_println!("[FAULT] Testing page fault handler...");

    // Cause a page fault by accessing a null pointer
    let null_ptr = 0x0 as *mut u8;
    *null_ptr = 42;

    // Should never reach here
    serial_println!("[FAULT] ERROR: Page fault test did not trigger fault!");
}
