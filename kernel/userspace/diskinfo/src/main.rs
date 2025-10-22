#![no_std]
#![no_main]

use core::panic::PanicInfo;

// Syscall numbers
const SYS_WRITE: usize = 0;
const SYS_EXIT: usize = 1;
const SYS_GET_BLOCK_DEVICE_INFO: usize = 31;

// Block device information structure (must match kernel definition)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct BlockDeviceInfo {
    block_count: u64,    // Total number of blocks
    block_size: u32,     // Size of each block in bytes
    capacity_mb: u32,    // Total capacity in megabytes
}

// Syscall wrappers
fn syscall0(n: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") n,
            lateout("rax") ret,
            options(nostack)
        );
    }
    ret
}

fn syscall1(n: usize, arg1: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") n,
            in("rdi") arg1,
            lateout("rax") ret,
            options(nostack)
        );
    }
    ret
}

fn syscall3(n: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") n,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack)
        );
    }
    ret
}

fn sys_write(fd: usize, buf: &[u8]) -> isize {
    syscall3(SYS_WRITE, fd, buf.as_ptr() as usize, buf.len())
}

fn sys_exit(code: usize) -> ! {
    syscall1(SYS_EXIT, code);
    loop {}
}

fn sys_get_block_device_info(info: &mut BlockDeviceInfo) -> isize {
    syscall1(SYS_GET_BLOCK_DEVICE_INFO, info as *mut _ as usize)
}

// Print functions
fn print(s: &str) {
    sys_write(1, s.as_bytes());
}

fn print_u64(n: u64) {
    if n == 0 {
        print("0");
        return;
    }

    let mut buf = [0u8; 20];
    let mut i = 0;
    let mut num = n;

    while num > 0 {
        buf[i] = b'0' + (num % 10) as u8;
        num /= 10;
        i += 1;
    }

    // Reverse the buffer
    for j in 0..i / 2 {
        buf.swap(j, i - 1 - j);
    }

    let s = core::str::from_utf8(&buf[..i]).unwrap_or("?");
    print(s);
}

fn print_u32(n: u32) {
    print_u64(n as u64);
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Print header
    print("Block Device Information:\n");
    print("==========================\n\n");

    // Get block device info
    let mut info = BlockDeviceInfo {
        block_count: 0,
        block_size: 0,
        capacity_mb: 0,
    };

    let result = sys_get_block_device_info(&mut info);

    if result < 0 {
        print("Error: Failed to get block device information\n");
        print("  (Device may not be initialized or not present)\n");
        sys_exit(1);
    }

    // Display information
    print("  Block count:  ");
    print_u64(info.block_count);
    print(" blocks\n");

    print("  Block size:   ");
    print_u32(info.block_size);
    print(" bytes\n");

    print("  Total size:   ");
    print_u32(info.capacity_mb);
    print(" MB\n");

    // Calculate and display total size in GB if >= 1024 MB
    if info.capacity_mb >= 1024 {
        print("                ");
        print_u32(info.capacity_mb / 1024);
        print(" GB\n");
    }

    print("\n");
    sys_exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    print("diskinfo: panic!\n");
    sys_exit(1);
}
