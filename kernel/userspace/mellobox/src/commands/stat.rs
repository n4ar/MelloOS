//! stat - display file status

use crate::args::Args;
use crate::error::{Error, Result};
use crate::syscalls;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

// File stat structure (must match kernel)
#[repr(C)]
struct Stat {
    st_dev: u64,
    st_ino: u64,
    st_mode: u32,
    st_nlink: u32,
    st_uid: u32,
    st_gid: u32,
    st_rdev: u64,
    st_size: i64,
    st_blksize: i64,
    st_blocks: i64,
    st_atime: i64,
    st_mtime: i64,
    st_ctime: i64,
}

// Mode constants
const S_IFMT: u32 = 0o170000;
const S_IFSOCK: u32 = 0o140000;
const S_IFLNK: u32 = 0o120000;
const S_IFREG: u32 = 0o100000;
const S_IFBLK: u32 = 0o060000;
const S_IFDIR: u32 = 0o040000;
const S_IFCHR: u32 = 0o020000;
const S_IFIFO: u32 = 0o010000;

pub fn main(argv: &'static [&'static str]) -> Result<i32> {
    let args = Args::parse(argv, "L")?;
    
    let follow_symlinks = !args.has_option('L');
    
    if args.positional_count() == 0 {
        syscalls::write(2, b"stat: missing operand\n");
        return Ok(1);
    }
    
    for i in 0..args.positional_count() {
        let path = args.get_positional(i).unwrap();
        
        // Prepare path with null terminator
        let mut path_bytes = Vec::new();
        path_bytes.extend_from_slice(path.as_bytes());
        path_bytes.push(0);
        
        // Get file stats
        let mut statbuf = [0u8; core::mem::size_of::<Stat>()];
        let stat_result = if follow_symlinks {
            syscalls::stat(&path_bytes, statbuf.as_mut_ptr())
        } else {
            syscalls::lstat(&path_bytes, statbuf.as_mut_ptr())
        };
        
        if stat_result < 0 {
            let msg = format!("stat: cannot stat '{}': ", path);
            syscalls::write(2, msg.as_bytes());
            syscalls::write(2, b"No such file or directory\n");
            continue;
        }
        
        let stat = unsafe { &*(statbuf.as_ptr() as *const Stat) };
        
        // Display file information
        let msg = format!("  File: {}\n", path);
        syscalls::write(1, msg.as_bytes());
        
        let msg = format!("  Size: {:<15} Blocks: {:<10} IO Block: {}\n",
                         stat.st_size, stat.st_blocks, stat.st_blksize);
        syscalls::write(1, msg.as_bytes());
        
        // File type
        let file_type = match stat.st_mode & S_IFMT {
            S_IFREG => "regular file",
            S_IFDIR => "directory",
            S_IFLNK => "symbolic link",
            S_IFCHR => "character device",
            S_IFBLK => "block device",
            S_IFIFO => "FIFO/pipe",
            S_IFSOCK => "socket",
            _ => "unknown",
        };
        
        let msg = format!("  Type: {}\n", file_type);
        syscalls::write(1, msg.as_bytes());
        
        // Device
        let major = (stat.st_dev >> 32) as u32;
        let minor = (stat.st_dev & 0xFFFFFFFF) as u32;
        let msg = format!("Device: {}:{}\tInode: {}\tLinks: {}\n",
                         major, minor, stat.st_ino, stat.st_nlink);
        syscalls::write(1, msg.as_bytes());
        
        // Permissions
        let msg = format!("Access: ({:04o}/", stat.st_mode & 0o7777);
        syscalls::write(1, msg.as_bytes());
        print_permissions(stat.st_mode);
        syscalls::write(1, b")\n");
        
        // Owner
        let msg = format!("Uid: ({}/user)\tGid: ({}/user)\n",
                         stat.st_uid, stat.st_gid);
        syscalls::write(1, msg.as_bytes());
        
        // Timestamps (simplified - just show raw values)
        let msg = format!("Access: {}\n", stat.st_atime);
        syscalls::write(1, msg.as_bytes());
        let msg = format!("Modify: {}\n", stat.st_mtime);
        syscalls::write(1, msg.as_bytes());
        let msg = format!("Change: {}\n", stat.st_ctime);
        syscalls::write(1, msg.as_bytes());
        
        if i < args.positional_count() - 1 {
            syscalls::write(1, b"\n");
        }
    }
    
    Ok(0)
}

fn print_permissions(mode: u32) {
    let mut buf = [0u8; 10];
    
    // File type
    buf[0] = match mode & S_IFMT {
        S_IFDIR => b'd',
        S_IFLNK => b'l',
        S_IFCHR => b'c',
        S_IFBLK => b'b',
        S_IFIFO => b'p',
        S_IFSOCK => b's',
        _ => b'-',
    };
    
    // Owner permissions
    buf[1] = if mode & 0o400 != 0 { b'r' } else { b'-' };
    buf[2] = if mode & 0o200 != 0 { b'w' } else { b'-' };
    buf[3] = if mode & 0o100 != 0 { b'x' } else { b'-' };
    
    // Group permissions
    buf[4] = if mode & 0o040 != 0 { b'r' } else { b'-' };
    buf[5] = if mode & 0o020 != 0 { b'w' } else { b'-' };
    buf[6] = if mode & 0o010 != 0 { b'x' } else { b'-' };
    
    // Other permissions
    buf[7] = if mode & 0o004 != 0 { b'r' } else { b'-' };
    buf[8] = if mode & 0o002 != 0 { b'w' } else { b'-' };
    buf[9] = if mode & 0o001 != 0 { b'x' } else { b'-' };
    
    syscalls::write(1, &buf);
}
