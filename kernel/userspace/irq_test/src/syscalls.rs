use core::arch::asm;

// Syscall numbers
const SYS_EXIT: usize = 1;
const SYS_WRITE: usize = 0;
const SYS_SLEEP: usize = 2;
const SYS_GET_IRQ_STATS: usize = 33;

/// IRQ statistics entry structure (must match kernel definition)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IrqStatsEntry {
    pub irq: u8,
    pub _padding: [u8; 7],
    pub cpu_counts: [u64; 8],
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

/// Sleep for specified milliseconds
pub fn sleep(ms: usize) -> isize {
    unsafe { syscall1(SYS_SLEEP, ms) }
}

/// Get IRQ statistics
pub fn get_irq_stats(stats: &mut [IrqStatsEntry]) -> isize {
    unsafe { syscall2(SYS_GET_IRQ_STATS, stats.as_mut_ptr() as usize, stats.len()) }
}
