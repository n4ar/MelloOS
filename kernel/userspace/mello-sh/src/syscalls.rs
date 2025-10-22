//! System call wrappers for mello-sh

use core::arch::asm;

// System call numbers (must match kernel)
const SYS_READ: usize = 0;
const SYS_WRITE: usize = 1;
const SYS_OPEN: usize = 2;
const SYS_CLOSE: usize = 3;
const SYS_EXIT: usize = 60;
const SYS_FORK: usize = 57;
const SYS_EXECVE: usize = 59;
const SYS_WAIT4: usize = 61;
const SYS_PIPE: usize = 22;
const SYS_DUP2: usize = 33;
const SYS_SETPGID: usize = 109;
const SYS_GETPGRP: usize = 111;
const SYS_TCSETPGRP: usize = 136;
const SYS_TCGETPGRP: usize = 137;
const SYS_KILL: usize = 62;
const SYS_SIGACTION: usize = 13;
const SYS_GETCWD: usize = 79;
const SYS_CHDIR: usize = 80;

/// Raw system call with 0 arguments
#[inline]
unsafe fn syscall0(n: usize) -> isize {
    let ret: isize;
    asm!(
        "syscall",
        in("rax") n,
        lateout("rax") ret,
        options(nostack)
    );
    ret
}

/// Raw system call with 1 argument
#[inline]
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

/// Raw system call with 2 arguments
#[inline]
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

/// Raw system call with 3 arguments
#[inline]
unsafe fn syscall3(n: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    let ret: isize;
    asm!(
        "syscall",
        in("rax") n,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        lateout("rax") ret,
        options(nostack)
    );
    ret
}

/// Raw system call with 4 arguments
#[inline]
unsafe fn syscall4(n: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize) -> isize {
    let ret: isize;
    asm!(
        "syscall",
        in("rax") n,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        lateout("rax") ret,
        options(nostack)
    );
    ret
}

/// Read from file descriptor
pub fn read(fd: i32, buf: &mut [u8]) -> isize {
    unsafe { syscall3(SYS_READ, fd as usize, buf.as_mut_ptr() as usize, buf.len()) }
}

/// Write to file descriptor
pub fn write(fd: i32, buf: &[u8]) -> isize {
    unsafe { syscall3(SYS_WRITE, fd as usize, buf.as_ptr() as usize, buf.len()) }
}

/// Exit process
pub fn exit(code: i32) -> ! {
    unsafe {
        syscall1(SYS_EXIT, code as usize);
    }
    loop {}
}

/// Fork process
pub fn fork() -> isize {
    unsafe { syscall0(SYS_FORK) }
}

/// Execute program
pub fn execve(path: &[u8], argv: &[*const u8], envp: &[*const u8]) -> isize {
    unsafe {
        syscall3(
            SYS_EXECVE,
            path.as_ptr() as usize,
            argv.as_ptr() as usize,
            envp.as_ptr() as usize,
        )
    }
}

/// Wait for child process
pub fn wait4(pid: i32, status: &mut i32, options: i32) -> isize {
    unsafe {
        syscall4(
            SYS_WAIT4,
            pid as usize,
            status as *mut i32 as usize,
            options as usize,
            0,
        )
    }
}

/// Create pipe
pub fn pipe(fds: &mut [i32; 2]) -> isize {
    unsafe { syscall1(SYS_PIPE, fds.as_mut_ptr() as usize) }
}

/// Duplicate file descriptor
pub fn dup2(oldfd: i32, newfd: i32) -> isize {
    unsafe { syscall2(SYS_DUP2, oldfd as usize, newfd as usize) }
}

/// Set process group ID
pub fn setpgid(pid: i32, pgid: i32) -> isize {
    unsafe { syscall2(SYS_SETPGID, pid as usize, pgid as usize) }
}

/// Get process group ID
pub fn getpgrp() -> isize {
    unsafe { syscall0(SYS_GETPGRP) }
}

/// Set foreground process group
pub fn tcsetpgrp(fd: i32, pgid: i32) -> isize {
    unsafe { syscall2(SYS_TCSETPGRP, fd as usize, pgid as usize) }
}

/// Get foreground process group
pub fn tcgetpgrp(fd: i32) -> isize {
    unsafe { syscall1(SYS_TCGETPGRP, fd as usize) }
}

/// Send signal to process
pub fn kill(pid: i32, sig: i32) -> isize {
    unsafe { syscall2(SYS_KILL, pid as usize, sig as usize) }
}

/// Get current working directory
pub fn getcwd(buf: &mut [u8]) -> isize {
    unsafe { syscall2(SYS_GETCWD, buf.as_mut_ptr() as usize, buf.len()) }
}

/// Change directory
pub fn chdir(path: &[u8]) -> isize {
    unsafe { syscall1(SYS_CHDIR, path.as_ptr() as usize) }
}

/// Get process ID
pub fn getpid() -> isize {
    unsafe { syscall0(39) } // SYS_GETPID = 39
}

/// Open file
pub fn open(path: &[u8], flags: i32, mode: i32) -> isize {
    unsafe { syscall3(SYS_OPEN, path.as_ptr() as usize, flags as usize, mode as usize) }
}

/// Close file descriptor
pub fn close(fd: i32) -> isize {
    unsafe { syscall1(SYS_CLOSE, fd as usize) }
}

// Wait options
pub const WNOHANG: i32 = 1;
pub const WUNTRACED: i32 = 2;
pub const WCONTINUED: i32 = 8;

// Signal numbers
pub const SIGINT: i32 = 2;
pub const SIGTSTP: i32 = 20;
pub const SIGCONT: i32 = 18;
pub const SIGCHLD: i32 = 17;

// Open flags
pub const O_RDONLY: i32 = 0;
pub const O_WRONLY: i32 = 1;
pub const O_RDWR: i32 = 2;
pub const O_CREAT: i32 = 0x40;
pub const O_TRUNC: i32 = 0x200;
pub const O_APPEND: i32 = 0x400;
