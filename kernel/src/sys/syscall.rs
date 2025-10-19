//! System Call Interface
//!
//! This module implements the system call interface for userland-kernel communication.
//! It provides syscall entry point, dispatcher, and handler functions.

use crate::serial_println;
use crate::sys::METRICS;

/// Syscall entry point (naked function)
///
/// This function is called when userland invokes int 0x80.
/// It saves all registers, calls the dispatcher, and restores registers.
///
/// Register mapping (x86-64 System V ABI):
/// - RAX: Syscall number (input), return value (output)
/// - RDI: Argument 1
/// - RSI: Argument 2
/// - RDX: Argument 3
#[unsafe(naked)]
#[no_mangle]
pub extern "C" fn syscall_entry() {
    core::arch::naked_asm!(
        // The CPU has already pushed SS, RSP, RFLAGS, CS, RIP
        // We need to save all other registers
        
        // Save caller-saved registers
        "push rax",      // Syscall number
        "push rcx",
        "push rdx",      // Arg 3
        "push rsi",      // Arg 2
        "push rdi",      // Arg 1
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        
        // Save callee-saved registers
        "push rbx",
        "push rbp",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        
        // Clear direction flag (required by ABI)
        "cld",
        
        // Prepare arguments for syscall_dispatcher
        // Stack layout after all pushes (each register = 8 bytes):
        // [rsp + 0]  = r15
        // [rsp + 8]  = r14
        // [rsp + 16] = r13
        // [rsp + 24] = r12
        // [rsp + 32] = rbp
        // [rsp + 40] = rbx
        // [rsp + 48] = r11
        // [rsp + 56] = r10
        // [rsp + 64] = r9
        // [rsp + 72] = r8
        // [rsp + 80] = rdi (arg1) ← we need this
        // [rsp + 88] = rsi (arg2) ← we need this
        // [rsp + 96] = rdx (arg3) ← we need this
        // [rsp + 104] = rcx
        // [rsp + 112] = rax (syscall_id)
        
        // RDI = syscall_id (from RAX)
        // RSI = arg1 (from original RDI)
        // RDX = arg2 (from original RSI)
        // RCX = arg3 (from original RDX)
        "mov rdi, rax",           // syscall_id
        "mov rsi, [rsp + 80]",    // arg1 (original RDI)
        "mov rdx, [rsp + 88]",    // arg2 (original RSI)
        "mov rcx, [rsp + 96]",    // arg3 (original RDX)
        
        // Call the dispatcher
        "call {dispatcher}",
        
        // RAX now contains the return value
        // We need to preserve it while restoring other registers
        // Use the stack slot where we saved RAX (syscall_id)
        "mov [rsp + 112], rax",   // Save return value to old RAX slot
        
        // Restore callee-saved registers
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbp",
        "pop rbx",
        
        // Restore caller-saved registers (except RAX which has return value)
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",
        
        // Restore return value to RAX (from the slot we saved it to)
        "pop rax",    // This pops the return value we saved earlier
        
        // Return from interrupt (pops RIP, CS, RFLAGS, RSP, SS)
        "iretq",
        
        dispatcher = sym syscall_dispatcher_wrapper,
    )
}

/// Wrapper for syscall_dispatcher to match calling convention
///
/// This function converts the register arguments to Rust function arguments.
#[no_mangle]
extern "C" fn syscall_dispatcher_wrapper(
    syscall_id: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> isize {
    syscall_dispatcher(syscall_id, arg1, arg2, arg3)
}

/// Syscall numbers
pub const SYS_WRITE: usize = 0;
pub const SYS_EXIT: usize = 1;
pub const SYS_SLEEP: usize = 2;
pub const SYS_IPC_SEND: usize = 3;
pub const SYS_IPC_RECV: usize = 4;

/// Syscall dispatcher
///
/// Routes syscall ID to appropriate handler and increments metrics.
///
/// # Arguments
/// * `syscall_id` - Syscall number (from RAX)
/// * `arg1` - First argument (from RDI)
/// * `arg2` - Second argument (from RSI)
/// * `arg3` - Third argument (from RDX)
///
/// # Returns
/// Result value (0 or positive on success, -1 on error)
pub fn syscall_dispatcher(
    syscall_id: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> isize {
    // Get current task ID for logging
    let task_id = match crate::sched::get_current_task_info() {
        Some((id, _)) => id,
        None => 0, // Unknown task
    };
    
    // Get syscall name for logging
    let syscall_name = match syscall_id {
        SYS_WRITE => "SYS_WRITE",
        SYS_EXIT => "SYS_EXIT",
        SYS_SLEEP => "SYS_SLEEP",
        SYS_IPC_SEND => "SYS_IPC_SEND",
        SYS_IPC_RECV => "SYS_IPC_RECV",
        _ => "INVALID",
    };
    
    // Log syscall invocation with task ID and syscall name
    serial_println!(
        "[SYSCALL] Task {} invoked {} (id={})",
        task_id,
        syscall_name,
        syscall_id
    );
    
    // Log syscall arguments at TRACE level (commented out to avoid spam)
    // Uncomment for detailed debugging:
    // serial_println!(
    //     "[SYSCALL] TRACE: {} args: arg1={:#x}, arg2={:#x}, arg3={:#x}",
    //     syscall_name, arg1, arg2, arg3
    // );
    
    // Increment metrics counter for this syscall
    METRICS.increment_syscall(syscall_id);
    
    // Dispatch to appropriate handler
    let result = match syscall_id {
        SYS_WRITE => sys_write(arg1, arg2, arg3),
        SYS_EXIT => sys_exit(arg1),
        SYS_SLEEP => sys_sleep(arg1),
        SYS_IPC_SEND => sys_ipc_send(arg1, arg2, arg3),
        SYS_IPC_RECV => sys_ipc_recv(arg1, arg2, arg3),
        _ => {
            serial_println!("[SYSCALL] ERROR: Invalid syscall ID: {}", syscall_id);
            -1 // Invalid syscall
        }
    };
    
    // Log syscall return value
    if result >= 0 {
        serial_println!(
            "[SYSCALL] Task {} {} returned: {}",
            task_id,
            syscall_name,
            result
        );
    } else {
        serial_println!(
            "[SYSCALL] ERROR: Task {} {} failed with error: {}",
            task_id,
            syscall_name,
            result
        );
    }
    
    result
}

/// sys_write handler - Write data to serial output
///
/// # Arguments
/// * `fd` - File descriptor (only 0/stdout supported in Phase 4)
/// * `buf_ptr` - Pointer to buffer
/// * `len` - Length of data to write
///
/// # Returns
/// Number of bytes written, or -1 on error
fn sys_write(fd: usize, buf_ptr: usize, len: usize) -> isize {
    // Validate file descriptor (only stdout supported)
    if fd != 0 {
        return -1;
    }
    
    // Phase 4: No pointer validation, assume kernel-accessible
    // Phase 5 will add copy_from_user() validation
    
    if buf_ptr == 0 || len == 0 {
        return 0; // Nothing to write
    }
    
    // Convert pointer to slice
    let buffer = unsafe {
        core::slice::from_raw_parts(buf_ptr as *const u8, len)
    };
    
    // Convert to string (lossy for non-UTF8)
    let s = core::str::from_utf8(buffer).unwrap_or("[invalid UTF-8]");
    
    // Write to serial
    serial_println!("[USERLAND] {}", s);
    
    len as isize
}

/// sys_exit handler - Terminate current task
///
/// # Arguments
/// * `code` - Exit code
///
/// # Returns
/// Never returns
fn sys_exit(code: usize) -> ! {
    serial_println!("[SYSCALL] Task exiting with code {}", code);
    
    // TODO: Mark task as terminated and remove from all queues
    // For now, just loop forever
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// sys_sleep handler - Put task to sleep for specified ticks
///
/// # Arguments
/// * `ticks` - Number of ticks to sleep
///
/// # Returns
/// 0 on success, -1 on error
fn sys_sleep(ticks: usize) -> isize {
    // Validate tick count
    if ticks == 0 {
        return 0; // Sleep for 0 ticks is a no-op
    }
    
    // Get current task ID and priority from scheduler
    let (_task_id, priority) = match crate::sched::get_current_task_info() {
        Some(info) => info,
        None => {
            return -1;
        }
    };
    
    // Call scheduler to put task to sleep
    if !crate::sched::sleep_current_task(ticks as u64, priority) {
        return -1;
    }
    
    // Increment sleep counter metric
    use core::sync::atomic::Ordering;
    METRICS.sleep_count.fetch_add(1, Ordering::Relaxed);
    
    // Trigger scheduler to select next task
    // This will context switch away from the current task
    crate::sched::yield_now();
    
    // When we wake up, we return here
    0
}

/// sys_ipc_send handler - Send message to port
///
/// # Arguments
/// * `port_id` - Target port ID
/// * `buf_ptr` - Pointer to message buffer
/// * `len` - Length of message
///
/// # Returns
/// 0 on success, -1 on error
fn sys_ipc_send(port_id: usize, buf_ptr: usize, len: usize) -> isize {
    use crate::sys::port::PORT_MANAGER;
    
    // Validate buffer pointer and length
    if buf_ptr == 0 || len == 0 {
        return -1;
    }
    
    // Phase 4: No pointer validation, assume kernel-accessible
    // Convert pointer to slice
    let buffer = unsafe {
        core::slice::from_raw_parts(buf_ptr as *const u8, len)
    };
    
    // Get PORT_MANAGER and send message
    let mut port_mgr = PORT_MANAGER.lock();
    match port_mgr.send_message(port_id, buffer) {
        Ok(()) => 0,
        Err(_e) => -1,
    }
}

/// sys_ipc_recv handler - Receive message from port (blocking)
///
/// # Arguments
/// * `port_id` - Source port ID
/// * `buf_ptr` - Pointer to receive buffer
/// * `len` - Maximum length to receive
///
/// # Returns
/// Number of bytes received, or -1 on error
fn sys_ipc_recv(port_id: usize, buf_ptr: usize, len: usize) -> isize {
    use crate::sys::port::PORT_MANAGER;
    
    // Validate buffer pointer and length
    if buf_ptr == 0 || len == 0 {
        return -1;
    }
    
    // Get current task ID
    let task_id = match crate::sched::get_current_task_info() {
        Some((id, _)) => id,
        None => {
            return -1;
        }
    };
    
    // Phase 4: No pointer validation, assume kernel-accessible
    // Convert pointer to mutable slice
    let buffer = unsafe {
        core::slice::from_raw_parts_mut(buf_ptr as *mut u8, len)
    };
    
    // Get PORT_MANAGER and receive message
    let mut port_mgr = PORT_MANAGER.lock();
    match port_mgr.recv_message(port_id, task_id, buffer) {
        Ok(bytes_received) => bytes_received as isize,
        Err(_e) => -1,
    }
}
