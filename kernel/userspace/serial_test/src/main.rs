#![no_std]
#![no_main]

use core::panic::PanicInfo;

// Syscall numbers
const SYS_WRITE: usize = 0;
const SYS_EXIT: usize = 1;
const SYS_SERIAL_WRITE: usize = 26;
const SYS_SERIAL_READ: usize = 27;

/// Syscall wrapper
#[inline(always)]
fn syscall3(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") id,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack)
        );
    }
    ret
}

/// Write to stdout
fn write_stdout(s: &str) {
    syscall3(SYS_WRITE, 1, s.as_ptr() as usize, s.len());
}

/// Write to serial port
fn serial_write(data: &[u8]) -> isize {
    syscall3(SYS_SERIAL_WRITE, data.as_ptr() as usize, data.len(), 0)
}

/// Read from serial port
fn serial_read(buf: &mut [u8]) -> isize {
    syscall3(SYS_SERIAL_READ, buf.as_ptr() as usize, buf.len(), 0)
}

/// Exit program
fn exit(code: usize) -> ! {
    syscall3(SYS_EXIT, code, 0, 0);
    loop {}
}

/// Entry point
#[no_mangle]
pub extern "C" fn _start() -> ! {
    write_stdout("Serial Port Loopback Test\n");
    write_stdout("==========================\n\n");

    // Test data patterns
    let test_patterns: &[&[u8]] = &[
        b"Hello, Serial!",
        b"0123456789",
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        b"abcdefghijklmnopqrstuvwxyz",
        b"!@#$%^&*()_+-=[]{}|;:',.<>?/",
    ];

    let mut total_tests = 0;
    let mut passed_tests = 0;

    for (i, pattern) in test_patterns.iter().enumerate() {
        total_tests += 1;

        write_stdout("Test ");
        write_number(i + 1);
        write_stdout(": Sending \"");
        write_stdout(core::str::from_utf8(pattern).unwrap_or("[invalid UTF-8]"));
        write_stdout("\"...\n");

        // Send test data
        let bytes_written = serial_write(pattern);
        if bytes_written < 0 {
            write_stdout("  ERROR: Failed to write to serial port\n");
            continue;
        }

        write_stdout("  Sent ");
        write_number(bytes_written as usize);
        write_stdout(" bytes\n");

        // Small delay to allow data to be transmitted
        for _ in 0..100000 {
            unsafe { core::arch::asm!("nop") };
        }

        // Read back data
        let mut read_buf = [0u8; 256];
        let bytes_read = serial_read(&mut read_buf[..pattern.len()]);
        
        if bytes_read < 0 {
            write_stdout("  ERROR: Failed to read from serial port\n");
            continue;
        }

        write_stdout("  Received ");
        write_number(bytes_read as usize);
        write_stdout(" bytes\n");

        // Verify data integrity
        if bytes_read as usize == pattern.len() {
            let mut match_ok = true;
            for j in 0..pattern.len() {
                if read_buf[j] != pattern[j] {
                    match_ok = false;
                    break;
                }
            }

            if match_ok {
                write_stdout("  PASS: Data integrity verified\n");
                passed_tests += 1;
            } else {
                write_stdout("  FAIL: Data mismatch\n");
                write_stdout("  Expected: ");
                write_hex_dump(pattern);
                write_stdout("\n  Received: ");
                write_hex_dump(&read_buf[..bytes_read as usize]);
                write_stdout("\n");
            }
        } else {
            write_stdout("  FAIL: Length mismatch (expected ");
            write_number(pattern.len());
            write_stdout(", got ");
            write_number(bytes_read as usize);
            write_stdout(")\n");
        }

        write_stdout("\n");
    }

    // Print summary
    write_stdout("==========================\n");
    write_stdout("Test Summary:\n");
    write_stdout("  Total:  ");
    write_number(total_tests);
    write_stdout("\n  Passed: ");
    write_number(passed_tests);
    write_stdout("\n  Failed: ");
    write_number(total_tests - passed_tests);
    write_stdout("\n");

    if passed_tests == total_tests {
        write_stdout("\nAll tests PASSED!\n");
        exit(0);
    } else {
        write_stdout("\nSome tests FAILED!\n");
        exit(1);
    }
}

/// Write a number to stdout
fn write_number(mut n: usize) {
    if n == 0 {
        write_stdout("0");
        return;
    }

    let mut buf = [0u8; 20];
    let mut i = 0;

    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }

    // Reverse the digits
    for j in 0..i / 2 {
        buf.swap(j, i - 1 - j);
    }

    write_stdout(core::str::from_utf8(&buf[..i]).unwrap());
}

/// Write hex dump of data
fn write_hex_dump(data: &[u8]) {
    const HEX_CHARS: &[u8] = b"0123456789ABCDEF";
    
    for &byte in data {
        let high = (byte >> 4) as usize;
        let low = (byte & 0x0F) as usize;
        let hex = [HEX_CHARS[high], HEX_CHARS[low], b' '];
        write_stdout(core::str::from_utf8(&hex).unwrap());
    }
}

/// Panic handler
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    write_stdout("\nPANIC in serial_test!\n");
    exit(1);
}
