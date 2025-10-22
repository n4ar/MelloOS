use core::arch::asm;

// Syscall numbers
const SYS_EXIT: usize = 1;
const SYS_WRITE: usize = 2;
const SYS_GET_DEVICE_LIST: usize = 30;

/// Device information structure (must match kernel definition)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DeviceInfo {
    pub name: [u8; 32],      // Device name (null-terminated)
    pub bus_type: u32,       // Bus type (0=Platform, 1=PS2, 2=PCI, 3=Virtio)
    pub io_base: u64,        // I/O base address
    pub irq: u32,            // IRQ number (0xFFFFFFFF if none)
    pub state: u32,          // Device state
    pub has_driver: u32,     // 1 if driver is loaded, 0 otherwise
}

/// Raw syscall with 1 argument
unsafe fn syscall1(n: usize, arg1: usize) -> isize {
    let ret: isize;
    asm!(
        "syscall",
        in("rax") n,
        in("rdi") arg1,
        lateout("rax") ret,
        options(nostack)
    );
    ret
}

/// Raw syscall with 2 arguments
unsafe fn syscall2(n: usize, arg1: usize, arg2: usize) -> isize {
    let ret: isize;
    asm!(
        "syscall",
        in("rax") n,
        in("rdi") arg1,
        in("rsi") arg2,
        lateout("rax") ret,
        options(nostack)
    );
    ret
}

/// Exit the process
pub fn exit(code: i32) -> ! {
    unsafe {
        syscall1(SYS_EXIT, code as usize);
    }
    loop {}
}

/// Write to file descriptor
pub fn write(fd: usize, buf: &[u8]) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            in("rax") SYS_WRITE,
            in("rdi") fd,
            in("rsi") buf.as_ptr() as usize,
            in("rdx") buf.len(),
            lateout("rax") ret,
            options(nostack)
        );
    }
    ret
}

/// Get list of devices
pub fn get_device_list(devices: &mut [DeviceInfo]) -> isize {
    unsafe { syscall2(SYS_GET_DEVICE_LIST, devices.as_mut_ptr() as usize, devices.len()) }
}
