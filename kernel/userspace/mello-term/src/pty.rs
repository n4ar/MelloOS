//! PTY master interaction module
//!
//! Handles communication with the kernel PTY subsystem.

use core::arch::asm;

/// Syscall numbers
const SYS_OPEN: usize = 10;
const SYS_READ: usize = 11;
const SYS_WRITE: usize = 0;
const SYS_CLOSE: usize = 12;
const SYS_IOCTL: usize = 13;
const SYS_FCNTL: usize = 22;

/// ioctl commands
const TIOCGPTN: usize = 0x80045430; // Get PTY number
const TIOCGWINSZ: usize = 0x5413; // Get window size
const TIOCSWINSZ: usize = 0x5414; // Set window size

/// File open flags
const O_RDWR: usize = 0x0002;
const O_NONBLOCK: usize = 0x0800;

/// fcntl commands
const F_GETFL: usize = 3;
const F_SETFL: usize = 4;

/// Raw syscall function
#[inline(always)]
unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    let ret: isize;
    asm!(
        "syscall",
        inout("rax") id => ret,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        out("rcx") _,
        out("r11") _,
        options(nostack)
    );
    ret
}

/// Open a file
fn sys_open(path: &str, _flags: usize) -> Result<i32, &'static str> {
    let result = unsafe { syscall(SYS_OPEN, path.as_ptr() as usize, _flags, 0) };
    if result < 0 {
        Err("Failed to open file")
    } else {
        Ok(result as i32)
    }
}

/// Read from file descriptor
fn sys_read(fd: i32, buf: &mut [u8]) -> Result<usize, &'static str> {
    let result = unsafe { syscall(SYS_READ, fd as usize, buf.as_mut_ptr() as usize, buf.len()) };
    if result < 0 {
        Err("Failed to read")
    } else {
        Ok(result as usize)
    }
}

/// Write to file descriptor
fn sys_write(fd: i32, data: &[u8]) -> Result<usize, &'static str> {
    let result = unsafe { syscall(SYS_WRITE, fd as usize, data.as_ptr() as usize, data.len()) };
    if result < 0 {
        Err("Failed to write")
    } else {
        Ok(result as usize)
    }
}

/// Close file descriptor
fn sys_close(fd: i32) -> Result<(), &'static str> {
    let result = unsafe { syscall(SYS_CLOSE, fd as usize, 0, 0) };
    if result < 0 {
        Err("Failed to close")
    } else {
        Ok(())
    }
}

/// ioctl system call
fn sys_ioctl(fd: i32, cmd: usize, arg: usize) -> Result<i32, &'static str> {
    let result = unsafe { syscall(SYS_IOCTL, fd as usize, cmd, arg) };
    if result < 0 {
        Err("ioctl failed")
    } else {
        Ok(result as i32)
    }
}

/// File descriptor control
fn sys_fcntl(fd: i32, cmd: usize, arg: usize) -> Result<usize, &'static str> {
    let result = unsafe { syscall(SYS_FCNTL, fd as usize, cmd, arg) };
    if result < 0 {
        Err("fcntl failed")
    } else {
        Ok(result as usize)
    }
}

/// PTY master file descriptor wrapper
pub struct PtyMaster {
    master_fd: i32,
    slave_number: u32,
}

impl PtyMaster {
    /// Open /dev/ptmx and allocate a new PTY pair
    pub fn new() -> Result<Self, &'static str> {
        // Open /dev/ptmx to allocate a new PTY pair
        let master_fd = sys_open("/dev/ptmx\0", O_RDWR)?;

        // Get the slave number using ioctl(TIOCGPTN)
        let mut slave_number: u32 = 0;
        sys_ioctl(master_fd, TIOCGPTN, &mut slave_number as *mut u32 as usize)?;

        // Enable non-blocking mode so reads don't stall the UI loop
        let current_flags = sys_fcntl(master_fd, F_GETFL, 0)?;
        sys_fcntl(master_fd, F_SETFL, current_flags | O_NONBLOCK)?;

        Ok(Self {
            master_fd,
            slave_number,
        })
    }

    /// Get the master file descriptor
    pub fn master_fd(&self) -> i32 {
        self.master_fd
    }

    /// Get the slave device number
    pub fn slave_number(&self) -> u32 {
        self.slave_number
    }

    /// Read data from PTY master (reads from slave output)
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, &'static str> {
        sys_read(self.master_fd, buf)
    }

    /// Write data to PTY master (writes to slave input)
    pub fn write(&self, data: &[u8]) -> Result<usize, &'static str> {
        sys_write(self.master_fd, data)
    }

    /// Get window size
    pub fn get_winsize(&self) -> Result<Winsize, &'static str> {
        let mut winsize = Winsize::default();
        sys_ioctl(
            self.master_fd,
            TIOCGWINSZ,
            &mut winsize as *mut Winsize as usize,
        )?;
        Ok(winsize)
    }

    /// Set window size
    pub fn set_winsize(&self, winsize: &Winsize) -> Result<(), &'static str> {
        sys_ioctl(
            self.master_fd,
            TIOCSWINSZ,
            winsize as *const Winsize as usize,
        )?;
        Ok(())
    }
}

impl Drop for PtyMaster {
    fn drop(&mut self) {
        // Close the master FD when dropped
        let _ = sys_close(self.master_fd);
    }
}

/// Window size structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Winsize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}
