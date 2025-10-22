#![no_std]
#![no_main]

use core::arch::asm;

// Syscall numbers
const SYS_WRITE: usize = 0;
const SYS_EXIT: usize = 1;
const SYS_READ_STDIN: usize = 25;

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

/// Read from keyboard (stdin)
fn sys_read_stdin(buf: &mut [u8]) -> isize {
    unsafe { syscall(SYS_READ_STDIN, buf.as_mut_ptr() as usize, buf.len(), 0) }
}

/// Exit current task
fn sys_exit(code: usize) -> ! {
    unsafe {
        syscall(SYS_EXIT, code, 0, 0);
    }
    loop {}
}

/// Entry point for kbd_test program
///
/// This program reads keyboard input and echoes it back to stdout.
/// Press Ctrl+C (ASCII 3) to exit.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    sys_write("=== Keyboard Test Program ===\n");
    sys_write("Type characters to see them echoed back.\n");
    sys_write("Press Ctrl+C to exit.\n");
    sys_write("\n");

    let mut buffer = [0u8; 64];
    let mut line_buffer = [0u8; 256];
    let mut line_pos = 0;

    loop {
        // Read from keyboard
        let bytes_read = sys_read_stdin(&mut buffer);
        
        if bytes_read > 0 {
            // Process each character
            for i in 0..(bytes_read as usize) {
                let ch = buffer[i];
                
                // Check for Ctrl+C (ASCII 3)
                if ch == 3 {
                    sys_write("\n^C\n");
                    sys_write("Exiting kbd_test...\n");
                    sys_exit(0);
                }
                
                // Check for backspace (ASCII 8 or 127)
                if ch == 8 || ch == 127 {
                    if line_pos > 0 {
                        line_pos -= 1;
                        // Echo backspace sequence: backspace, space, backspace
                        sys_write("\x08 \x08");
                    }
                    continue;
                }
                
                // Check for newline (ASCII 10 or 13)
                if ch == b'\n' || ch == b'\r' {
                    sys_write("\n");
                    
                    // Echo the complete line
                    if line_pos > 0 {
                        sys_write("You typed: ");
                        let line_str = core::str::from_utf8(&line_buffer[..line_pos])
                            .unwrap_or("[invalid UTF-8]");
                        sys_write(line_str);
                        sys_write("\n");
                        line_pos = 0;
                    }
                    continue;
                }
                
                // Add to line buffer if printable
                if ch >= 32 && ch < 127 {
                    if line_pos < line_buffer.len() {
                        line_buffer[line_pos] = ch;
                        line_pos += 1;
                    }
                    
                    // Echo the character
                    let echo_buf = [ch];
                    let echo_str = core::str::from_utf8(&echo_buf).unwrap_or("?");
                    sys_write(echo_str);
                }
            }
        }
        
        // Yield CPU to avoid busy-waiting
        // Small delay to reduce CPU usage
        for _ in 0..1000 {
            unsafe {
                asm!("pause");
            }
        }
    }
}

// Panic handler for userspace
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
