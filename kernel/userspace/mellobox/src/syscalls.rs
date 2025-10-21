//! System call wrappers for mellobox

#![allow(dead_code)]

use core::arch::asm;

// System call numbers (must match kernel)
const SYS_READ: usize = 0;
const SYS_WRITE: usize = 1;
const SYS_OPEN: usize = 2;
const SYS_CLOSE: usize = 3;
const SYS_EXIT: usize = 60;
const SYS_OPENAT: usize = 257;
const SYS_GETDENTS: usize = 78;
const SYS_FSTAT: usize = 5;
const SYS_LSTAT: usize = 6;
const SYS_UNLINK: usize = 87;
const SYS_RENAME: usize = 82;
const SYS_MKDIR: usize = 83;
const SYS_RMDIR: usize = 84;
const SYS_GETCWD: usize = 79;
const SYS_KILL: usize = 62;

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

/// Open file
pub fn open(path: &[u8], flags: i32, mode: i32) -> isize {
    unsafe { syscall3(SYS_OPEN, path.as_ptr() as usize, flags as usize, mode as usize) }
}

/// Close file descriptor
pub fn close(fd: i32) -> isize {
    unsafe { syscall1(SYS_CLOSE, fd as usize) }
}

/// Open file relative to directory
pub fn openat(dirfd: i32, path: &[u8], flags: i32, mode: i32) -> isize {
    unsafe {
        syscall4(
            SYS_OPENAT,
            dirfd as usize,
            path.as_ptr() as usize,
            flags as usize,
            mode as usize,
        )
    }
}

/// Get directory entries
pub fn getdents(fd: i32, buf: &mut [u8]) -> isize {
    unsafe { syscall3(SYS_GETDENTS, fd as usize, buf.as_mut_ptr() as usize, buf.len()) }
}

/// Get file status
pub fn fstat(fd: i32, statbuf: *mut u8) -> isize {
    unsafe { syscall2(SYS_FSTAT, fd as usize, statbuf as usize) }
}

/// Get file status (don't follow symlinks)
pub fn lstat(path: &[u8], statbuf: *mut u8) -> isize {
    unsafe { syscall2(SYS_LSTAT, path.as_ptr() as usize, statbuf as usize) }
}

/// Remove file
pub fn unlink(path: &[u8]) -> isize {
    unsafe { syscall1(SYS_UNLINK, path.as_ptr() as usize) }
}

/// Rename file
pub fn rename(oldpath: &[u8], newpath: &[u8]) -> isize {
    unsafe { syscall2(SYS_RENAME, oldpath.as_ptr() as usize, newpath.as_ptr() as usize) }
}

/// Create directory
pub fn mkdir(path: &[u8], mode: i32) -> isize {
    unsafe { syscall2(SYS_MKDIR, path.as_ptr() as usize, mode as usize) }
}

/// Remove directory
pub fn rmdir(path: &[u8]) -> isize {
    unsafe { syscall1(SYS_RMDIR, path.as_ptr() as usize) }
}

/// Get current working directory
pub fn getcwd(buf: &mut [u8]) -> isize {
    unsafe { syscall2(SYS_GETCWD, buf.as_mut_ptr() as usize, buf.len()) }
}

/// Send signal to process
pub fn kill(pid: i32, sig: i32) -> isize {
    unsafe { syscall2(SYS_KILL, pid as usize, sig as usize) }
}

// Open flags
pub const O_RDONLY: i32 = 0;
pub const O_WRONLY: i32 = 1;
pub const O_RDWR: i32 = 2;
pub const O_CREAT: i32 = 0x40;
pub const O_TRUNC: i32 = 0x200;
pub const O_APPEND: i32 = 0x400;
pub const O_DIRECTORY: i32 = 0x10000;

// AT constants
pub const AT_FDCWD: i32 = -100;

// File modes
pub const S_IRWXU: i32 = 0o700;
pub const S_IRUSR: i32 = 0o400;
pub const S_IWUSR: i32 = 0o200;
pub const S_IXUSR: i32 = 0o100;
