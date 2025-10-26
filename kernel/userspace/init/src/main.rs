#![no_std]
#![no_main]

use core::arch::asm;

// Syscall numbers (legacy int 0x80 interface)
const SYS_WRITE: usize = 0;
const SYS_EXIT: usize = 1;
const SYS_SLEEP: usize = 2;
const SYS_IPC_SEND: usize = 3;
const SYS_IPC_RECV: usize = 4;
const SYS_GETPID: usize = 5;
const SYS_YIELD: usize = 6;
const SYS_FORK: usize = 7;
const SYS_WAIT: usize = 8;
const SYS_EXEC: usize = 9;

/// Raw syscall function using fast syscall instruction
#[inline(always)]
unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    let ret: isize;
    asm!(
        "syscall",
        inout("rax") id => ret,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        out("rcx") _,  // Clobbered by syscall
        out("r11") _,  // Clobbered by syscall
        options(nostack)
    );
    ret
}

/// Get current privilege level (CPL) from CS register
#[inline(always)]
fn get_current_privilege_level() -> u8 {
    let cs: u16;
    unsafe {
        asm!("mov {0:x}, cs", out(reg) cs);
    }
    (cs & 3) as u8
}

/// Write data to stdout (fd=1)
fn sys_write(msg: &str) -> isize {
    unsafe { syscall(SYS_WRITE, 1, msg.as_ptr() as usize, msg.len()) }
}

/// Get current process ID
fn sys_getpid() -> isize {
    unsafe { syscall(SYS_GETPID, 0, 0, 0) }
}

/// Fork current process
fn sys_fork() -> isize {
    unsafe { syscall(SYS_FORK, 0, 0, 0) }
}

/// Yield CPU to scheduler
fn sys_yield() -> isize {
    unsafe { syscall(SYS_YIELD, 0, 0, 0) }
}

/// Sleep for specified number of ticks
fn sys_sleep(ticks: usize) -> isize {
    unsafe { syscall(SYS_SLEEP, ticks, 0, 0) }
}

/// Send message to IPC port
fn sys_ipc_send(port_id: usize, data: &[u8]) -> isize {
    unsafe { syscall(SYS_IPC_SEND, port_id, data.as_ptr() as usize, data.len()) }
}

/// Receive message from IPC port (blocking)
fn sys_ipc_recv(port_id: usize, buf: &mut [u8]) -> isize {
    unsafe { syscall(SYS_IPC_RECV, port_id, buf.as_mut_ptr() as usize, buf.len()) }
}

/// Exit current task
fn sys_exit(code: usize) -> ! {
    unsafe {
        syscall(SYS_EXIT, code, 0, 0);
    }
    loop {}
}

/// Execute a new program
/// 
/// # Arguments
/// * `path` - Null-terminated path to executable
/// * `argv` - Null-terminated array of argument pointers
/// 
/// # Returns
/// * Does not return on success
/// * Negative error code on failure
fn sys_exec(path: *const u8, argv: *const *const u8) -> isize {
    unsafe { syscall(SYS_EXEC, path as usize, argv as usize, 0) }
}

/// Test privilege level validation
fn test_privilege_level() {
    let cpl = get_current_privilege_level();
    sys_write("=== Privilege Level Test ===\n");

    if cpl == 3 {
        sys_write("✓ PASS: Running at privilege level 3 (user mode)\n");
    } else {
        sys_write("✗ FAIL: Not running at privilege level 3, got level ");
        // Simple number to string conversion for single digit
        let level_char = (b'0' + cpl) as char;
        let level_str = [level_char as u8, b'\n', 0];
        let level_msg = core::str::from_utf8(&level_str[..2]).unwrap_or("?\n");
        sys_write(level_msg);
    }
}

/// Test basic syscall functionality
fn test_syscalls() {
    sys_write("=== Syscall Functionality Test ===\n");

    // Test sys_getpid
    let pid = sys_getpid();
    if pid > 0 {
        sys_write("✓ PASS: sys_getpid returned valid PID\n");
    } else {
        sys_write("✗ FAIL: sys_getpid failed\n");
    }

    // Test sys_write (already working if we can see this)
    sys_write("✓ PASS: sys_write working correctly\n");

    // Test sys_yield
    let yield_result = sys_yield();
    if yield_result >= 0 {
        sys_write("✓ PASS: sys_yield completed successfully\n");
    } else {
        sys_write("✗ FAIL: sys_yield failed\n");
    }
}

/// Test fork functionality with a simple fork chain
fn test_fork_chain() {
    sys_write("=== Fork Chain Test ===\n");

    const FORK_CHAIN_LENGTH: usize = 5; // Smaller chain for init process
    let mut forks_created = 0;

    for i in 0..FORK_CHAIN_LENGTH {
        let fork_result = sys_fork();

        if fork_result == 0 {
            // Child process
            sys_write("Child process created in fork chain\n");
            // Child exits immediately
            sys_exit(0);
        } else if fork_result > 0 {
            // Parent process
            forks_created += 1;
            sys_write("Parent: created child process\n");

            // Yield to allow child to run
            sys_yield();
        } else {
            // Fork failed
            sys_write("✗ FAIL: Fork failed\n");
            break;
        }
    }

    if forks_created == FORK_CHAIN_LENGTH {
        sys_write("✓ PASS: Fork chain test completed successfully\n");
    } else {
        sys_write("✗ FAIL: Fork chain incomplete\n");
    }
}

/// Test memory protection boundaries
fn test_memory_protection() {
    sys_write("=== Memory Protection Test ===\n");

    // Test valid user memory access (this message itself)
    let valid_msg = "Valid memory access test\n";
    let valid_result =
        unsafe { syscall(SYS_WRITE, 1, valid_msg.as_ptr() as usize, valid_msg.len()) };

    if valid_result > 0 {
        sys_write("✓ PASS: Valid user memory access succeeded\n");
    } else {
        sys_write("✗ FAIL: Valid user memory access failed\n");
    }

    // Test invalid kernel memory access (should fail safely)
    let kernel_addr = 0xFFFF_8000_0000_0000usize;
    let invalid_result = unsafe { syscall(SYS_WRITE, 1, kernel_addr, 10) };

    if invalid_result < 0 {
        sys_write("✓ PASS: Invalid kernel memory access correctly rejected\n");
    } else {
        sys_write("✗ FAIL: Invalid kernel memory access not rejected\n");
    }

    // Test null pointer access (should fail safely)
    let null_result = unsafe { syscall(SYS_WRITE, 1, 0, 10) };

    if null_result < 0 {
        sys_write("✓ PASS: Null pointer access correctly rejected\n");
    } else {
        sys_write("✗ FAIL: Null pointer access not rejected\n");
    }
}

/// Entry point for init process
#[no_mangle]
pub extern "C" fn _start() -> ! {
    sys_write("MelloOS Init Process Starting...\n");
    sys_write("========================================\n");

    // Quick system validation
    let cpl = get_current_privilege_level();
    if cpl == 3 {
        sys_write("✓ Running in user mode (Ring 3)\n");
    } else {
        sys_write("✗ ERROR: Not in user mode!\n");
    }

    let pid = sys_getpid();
    if pid == 1 {
        sys_write("✓ Init process (PID 1)\n");
    }

    sys_write("========================================\n");
    sys_write("Launching shell...\n\n");

    // Try to exec shell directly
    // Prepare argv array: ["/bin/sh", NULL]
    let shell_path = b"/bin/sh\0";
    let argv: [*const u8; 2] = [
        shell_path.as_ptr(),
        core::ptr::null(),
    ];

    sys_write("[Init] Attempting to exec /bin/sh...\n");
    
    // Call exec - this should not return on success
    let exec_result = sys_exec(shell_path.as_ptr(), argv.as_ptr());
    
    // If we reach here, exec failed
    sys_write("\n");
    sys_write("╔════════════════════════════════════════╗\n");
    sys_write("║         EXEC FAILED                    ║\n");
    sys_write("╚════════════════════════════════════════╝\n");
    sys_write("\n");
    sys_write("ERROR: Failed to exec shell (error code: ");
    
    // Print error code (simple conversion for negative numbers)
    if exec_result < 0 {
        sys_write("-");
        let abs_val = (-exec_result) as usize;
        if abs_val < 10 {
            let digit = (b'0' + abs_val as u8) as char;
            let digit_str = [digit as u8, b'\n', 0];
            let msg = core::str::from_utf8(&digit_str[..2]).unwrap_or("?\n");
            sys_write(msg);
        } else {
            sys_write("??\n");
        }
    } else {
        sys_write("0\n");
    }
    
    sys_write("\nFalling back to test mode...\n");
    sys_write("========================================\n\n");

    // Run tests as fallback
    test_privilege_level();
    sys_write("\n");
    
    test_syscalls();
    sys_write("\n");
    
    test_fork_chain();
    sys_write("\n");
    
    test_memory_protection();
    sys_write("\n");

    sys_write("========================================\n");
    sys_write("All tests completed. System halted.\n");
    sys_write("(Shell exec failed - check /bin/sh exists)\n");

    // Hang forever
    loop {
        sys_sleep(1000);
        sys_yield();
    }
}

// Panic handler for userspace
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
