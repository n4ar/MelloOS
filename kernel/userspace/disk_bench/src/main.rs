#![no_std]
#![no_main]

use core::panic::PanicInfo;

// Syscall numbers
const SYS_WRITE: usize = 0;
const SYS_EXIT: usize = 1;
const SYS_BLOCK_READ: usize = 28;

// Syscall wrappers
#[inline(always)]
unsafe fn syscall0(n: usize) -> isize {
    let ret: isize;
    core::arch::asm!(
        "int 0x80",
        in("rax") n,
        lateout("rax") ret,
        options(nostack)
    );
    ret
}

#[inline(always)]
unsafe fn syscall3(n: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    let ret: isize;
    core::arch::asm!(
        "int 0x80",
        in("rax") n,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        lateout("rax") ret,
        options(nostack)
    );
    ret
}

fn write_stdout(s: &str) {
    unsafe {
        syscall3(SYS_WRITE, 1, s.as_ptr() as usize, s.len());
    }
}

fn exit(_code: usize) -> ! {
    unsafe {
        syscall0(SYS_EXIT);
    }
    loop {}
}

fn block_read(lba: usize, buf: &mut [u8], count: usize) -> isize {
    unsafe {
        syscall3(SYS_BLOCK_READ, lba, buf.as_mut_ptr() as usize, count)
    }
}

// Simple integer to string conversion
fn u64_to_str(mut n: u64, buf: &mut [u8]) -> &str {
    if n == 0 {
        buf[0] = b'0';
        return core::str::from_utf8(&buf[0..1]).unwrap();
    }

    let mut i = 0;
    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }

    // Reverse the digits
    buf[0..i].reverse();
    core::str::from_utf8(&buf[0..i]).unwrap()
}

// Simple timer using CPU timestamp counter
fn rdtsc() -> u64 {
    unsafe {
        let lo: u32;
        let hi: u32;
        core::arch::asm!(
            "rdtsc",
            out("eax") lo,
            out("edx") hi,
            options(nomem, nostack)
        );
        ((hi as u64) << 32) | (lo as u64)
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    write_stdout("Disk benchmark - reading sector 0 (MBR)\n");

    // Allocate buffer for sector (512 bytes)
    let mut buf = [0u8; 512];

    // Measure read time
    let start = rdtsc();
    let result = block_read(0, &mut buf, 1);
    let end = rdtsc();

    if result < 0 {
        write_stdout("ERROR: Failed to read sector 0\n");
        exit(1);
    }

    if result == 0 {
        write_stdout("ERROR: No blocks read\n");
        exit(1);
    }

    write_stdout("Successfully read sector 0\n");

    // Check MBR signature (0x55AA at offset 510-511)
    let sig_byte1 = buf[510];
    let sig_byte2 = buf[511];

    write_stdout("MBR signature: 0x");
    
    // Convert bytes to hex
    let hex_chars = b"0123456789ABCDEF";
    let mut hex_buf = [0u8; 4];
    hex_buf[0] = hex_chars[(sig_byte1 >> 4) as usize];
    hex_buf[1] = hex_chars[(sig_byte1 & 0x0F) as usize];
    hex_buf[2] = hex_chars[(sig_byte2 >> 4) as usize];
    hex_buf[3] = hex_chars[(sig_byte2 & 0x0F) as usize];
    
    write_stdout(core::str::from_utf8(&hex_buf).unwrap());
    write_stdout("\n");

    if sig_byte1 == 0x55 && sig_byte2 == 0xAA {
        write_stdout("✓ MBR signature valid (0x55AA)\n");
    } else {
        write_stdout("✗ MBR signature invalid (expected 0x55AA)\n");
    }

    // Calculate elapsed cycles
    let elapsed = end - start;

    // Convert to approximate microseconds (assuming 2 GHz CPU)
    // cycles / (2 * 10^9 cycles/sec) * 10^6 us/sec = cycles / 2000
    let elapsed_us = elapsed / 2000;

    write_stdout("Read time: ");
    let mut num_buf = [0u8; 20];
    let num_str = u64_to_str(elapsed_us, &mut num_buf);
    write_stdout(num_str);
    write_stdout(" us (approx)\n");

    write_stdout("CPU cycles: ");
    let cycles_str = u64_to_str(elapsed, &mut num_buf);
    write_stdout(cycles_str);
    write_stdout("\n");

    // Display first 16 bytes of MBR for verification
    write_stdout("\nFirst 16 bytes of sector 0:\n");
    for i in 0..16 {
        if i > 0 && i % 8 == 0 {
            write_stdout("\n");
        }
        write_stdout("0x");
        let byte = buf[i];
        let mut hex = [0u8; 2];
        hex[0] = hex_chars[(byte >> 4) as usize];
        hex[1] = hex_chars[(byte & 0x0F) as usize];
        write_stdout(core::str::from_utf8(&hex).unwrap());
        write_stdout(" ");
    }
    write_stdout("\n");

    write_stdout("\nDisk benchmark complete\n");
    exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    write_stdout("PANIC in disk_bench\n");
    exit(1);
}
