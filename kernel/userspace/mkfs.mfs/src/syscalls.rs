//! System call wrappers

pub const O_RDONLY: usize = 0;
pub const O_WRONLY: usize = 1;
pub const O_RDWR: usize = 2;

pub const SEEK_SET: usize = 0;
pub const SEEK_CUR: usize = 1;
pub const SEEK_END: usize = 2;

#[inline(always)]
fn syscall0(n: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") n,
            lateout("rax") ret,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
fn syscall1(n: usize, arg1: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") n,
            in("rdi") arg1,
            lateout("rax") ret,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
fn syscall2(n: usize, arg1: usize, arg2: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") n,
            in("rdi") arg1,
            in("rsi") arg2,
            lateout("rax") ret,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
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
            options(nostack)
        );
    }
    ret
}

pub fn exit(code: i32) -> ! {
    syscall1(6, code as usize);
    loop {}
}

pub fn open(path: &str, flags: usize) -> Result<usize, isize> {
    let ret = syscall2(10, path.as_ptr() as usize, flags);
    if ret < 0 {
        Err(ret)
    } else {
        Ok(ret as usize)
    }
}

pub fn close(fd: usize) {
    syscall1(11, fd);
}

pub fn read(fd: usize, buf: &mut [u8]) -> Result<usize, isize> {
    let ret = syscall3(12, fd, buf.as_mut_ptr() as usize, buf.len());
    if ret < 0 {
        Err(ret)
    } else {
        Ok(ret as usize)
    }
}

pub fn write(fd: usize, buf: &[u8]) -> Result<usize, isize> {
    let ret = syscall3(13, fd, buf.as_ptr() as usize, buf.len());
    if ret < 0 {
        Err(ret)
    } else {
        Ok(ret as usize)
    }
}

pub fn lseek(fd: usize, offset: i64, whence: usize) -> Result<i64, isize> {
    let ret = syscall3(14, fd, offset as usize, whence);
    if ret < 0 {
        Err(ret)
    } else {
        Ok(ret as i64)
    }
}

pub fn fsync(fd: usize) -> Result<(), isize> {
    let ret = syscall1(15, fd);
    if ret < 0 {
        Err(ret)
    } else {
        Ok(())
    }
}

pub fn get_args() -> &'static [&'static str] {
    // Placeholder - return empty args for now
    &["mkfs.mfs"]
}
