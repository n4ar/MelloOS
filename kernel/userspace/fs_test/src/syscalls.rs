//! System call interface for fs_test

/// System call numbers
const SYS_READ: usize = 0;
const SYS_WRITE: usize = 1;
const SYS_OPEN: usize = 2;
const SYS_CLOSE: usize = 3;
const SYS_EXIT: usize = 60;
const SYS_MKDIR: usize = 83;

/// Perform a system call with 1 argument
#[inline]
fn syscall1(n: usize, arg1: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") n,
            in("rdi") arg1,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
    }
    ret
}

/// Perform a system call with 2 arguments
#[inline]
fn syscall2(n: usize, arg1: usize, arg2: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") n,
            in("rdi") arg1,
            in("rsi") arg2,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
    }
    ret
}

/// Perform a system call with 3 arguments
#[inline]
fn syscall3(n: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") n,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
    }
    ret
}

/// Read from a file descriptor
pub fn sys_read(fd: usize, buf: usize, count: usize) -> isize {
    syscall3(SYS_READ, fd, buf, count)
}

/// Write to a file descriptor
pub fn sys_write(fd: usize, buf: usize, count: usize) -> isize {
    syscall3(SYS_WRITE, fd, buf, count)
}

/// Open a file
pub fn sys_open(path: usize, flags: usize) -> isize {
    syscall2(SYS_OPEN, path, flags)
}

/// Close a file descriptor
pub fn sys_close(fd: usize) -> isize {
    syscall1(SYS_CLOSE, fd)
}

/// Exit the process
pub fn sys_exit(code: usize) -> ! {
    syscall1(SYS_EXIT, code);
    loop {}
}

/// Create a directory
pub fn sys_mkdir(path: usize, mode: usize) -> isize {
    syscall2(SYS_MKDIR, path, mode)
}
