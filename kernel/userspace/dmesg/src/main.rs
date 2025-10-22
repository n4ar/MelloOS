#![no_std]
#![no_main]

use core::arch::asm;

// Syscall numbers
const SYS_WRITE: usize = 0;
const SYS_EXIT: usize = 1;
const SYS_READ_KERNEL_LOG: usize = 32;

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

/// Write data to stdout (fd=1)
fn sys_write(msg: &str) -> isize {
    unsafe { syscall(SYS_WRITE, 1, msg.as_ptr() as usize, msg.len()) }
}

/// Read kernel log buffer
fn sys_read_kernel_log(buf: &mut [u8]) -> isize {
    unsafe { syscall(SYS_READ_KERNEL_LOG, buf.as_mut_ptr() as usize, buf.len(), 0) }
}

/// Exit current task
fn sys_exit(code: usize) -> ! {
    unsafe {
        syscall(SYS_EXIT, code, 0, 0);
    }
    loop {}
}

/// Entry point for dmesg program
///
/// This program displays the kernel log buffer, showing driver lifecycle events
/// and other kernel messages.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Allocate a large buffer for the kernel log
    // Using a static buffer to avoid heap allocation
    static mut LOG_BUFFER: [u8; 65536] = [0u8; 65536];

    // Read the kernel log
    let bytes_read = unsafe {
        let log_buf_ptr = core::ptr::addr_of_mut!(LOG_BUFFER);
        sys_read_kernel_log(&mut *log_buf_ptr)
    };

    if bytes_read < 0 {
        sys_write("Error: Failed to read kernel log\n");
        sys_exit(1);
    }

    if bytes_read == 0 {
        sys_write("Kernel log is empty\n");
        sys_exit(0);
    }

    // Display the log
    sys_write("=== Kernel Log (dmesg) ===\n\n");
    
    // Convert the buffer to a string and write it
    let log_str = unsafe {
        core::str::from_utf8(&LOG_BUFFER[..(bytes_read as usize)])
            .unwrap_or("[invalid UTF-8 in log]")
    };
    
    sys_write(log_str);
    sys_write("\n");
    
    // Exit successfully
    sys_exit(0);
}

// Panic handler for userspace
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
