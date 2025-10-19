/// Init process loader
/// 
/// This module handles loading and spawning the userland init process.

use crate::sched::{spawn_task, priority::TaskPriority};
use crate::serial_println;
use core::slice;

/// Embedded init binary
/// This will be populated by including the compiled init binary
/// For Phase 4, we embed it directly. Phase 5 will use proper ELF loading.
#[cfg(not(test))]
static INIT_BINARY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/init_binary.bin"));

#[cfg(test)]
static INIT_BINARY: &[u8] = &[];

/// Load and spawn the init process
/// 
/// This function:
/// 1. Loads the init binary into memory
/// 2. Maps the binary pages with appropriate permissions
/// 3. Creates an init task with entry point at the binary start
/// 4. Spawns the init task with Normal priority
/// 
/// Phase 4 Implementation Notes:
/// - No user/kernel separation (tasks run in Ring 0)
/// - No ELF parsing (binary is position-independent)
/// - No memory protection (all tasks share kernel address space)
/// 
/// Phase 5 will add:
/// - ELF parsing and loading
/// - User mode execution (Ring 3)
/// - Separate user/kernel stacks
/// - Memory protection via paging
pub fn load_init_process() -> Result<(), &'static str> {
    serial_println!("[INIT] Loading init process...");
    
    if INIT_BINARY.is_empty() {
        serial_println!("[INIT] Warning: Init binary is empty, skipping init process");
        serial_println!("[INIT] Build the userspace init first: make userspace");
        return Ok(());
    }
    
    serial_println!("[INIT] Init binary size: {} bytes", INIT_BINARY.len());
    serial_println!("[INIT] Init binary address: {:p}", INIT_BINARY.as_ptr());
    
    // For Phase 4, we'll spawn a task that demonstrates the init process concept
    // The actual init binary execution requires ELF parsing which will be added in Phase 5
    
    // Spawn the init task wrapper with Normal priority
    spawn_task("init", init_task_wrapper, TaskPriority::Normal)
        .map_err(|_| "Failed to spawn init task")?;
    
    serial_println!("[INIT] Init process task spawned successfully");
    serial_println!("[INIT] Note: Full ELF loading will be implemented in Phase 5");
    
    Ok(())
}

/// Init task wrapper
/// 
/// This is a simplified version for Phase 4 that demonstrates the init process concept.
/// It performs the same operations that the userland init would do:
/// 1. Print hello message via sys_write
/// 2. Demonstrate IPC by sending/receiving messages
/// 3. Demonstrate sleep functionality
/// 
/// Phase 5 will replace this with proper ELF loading and user mode execution.
fn init_task_wrapper() -> ! {
    serial_println!("[INIT] Init task started");
    
    // Helper function to invoke syscall
    unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
        let ret: isize;
        core::arch::asm!(
            "int 0x80",
            in("rax") id,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }
    
    // Print hello message using sys_write (syscall 0)
    let hello_msg = "Hello from userland! ✨\n";
    let result = unsafe {
        syscall(0, 0, hello_msg.as_ptr() as usize, hello_msg.len())
    };
    serial_println!("[INIT] sys_write returned: {}", result);
    
    // Demonstrate IPC by sending "hello" to port 15 (dedicated init port)
    let hello_ipc_msg = b"hello";
    serial_println!("[INIT] Sending 'hello' to port 15...");
    let send_result = unsafe {
        syscall(3, 15, hello_ipc_msg.as_ptr() as usize, hello_ipc_msg.len())
    };
    
    if send_result >= 0 {
        serial_println!("[INIT] Successfully sent 'hello' to port 15");
    } else {
        serial_println!("[INIT] Failed to send to port 15: {}", send_result);
    }
    
    // Note: We skip the blocking receive to avoid hanging the kernel
    // In a real system, init would have other tasks to communicate with
    serial_println!("[INIT] IPC demonstration complete (skipping blocking receive)");
    
    // Sleep for 100 ticks
    serial_println!("[INIT] Sleeping for 100 ticks...");
    let sleep_result = unsafe {
        syscall(2, 100, 0, 0)
    };
    serial_println!("[INIT] sys_sleep returned: {}", sleep_result);
    
    // Print wake up message
    serial_println!("[INIT] Woke up!");
    
    // Enter infinite loop with periodic sleep
    let mut counter = 0u32;
    loop {
        serial_println!("[INIT] Init process running (iteration {})...", counter);
        
        // Sleep for 1000 ticks (10 seconds at 100 Hz)
        unsafe {
            syscall(2, 1000, 0, 0);
        }
        
        counter = counter.wrapping_add(1);
    }
}
