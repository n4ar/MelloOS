//! ls - list directory contents

use crate::args::Args;
use crate::error::{Error, Result};
use crate::syscalls;
use alloc::vec::Vec;
use alloc::string::String;

// Directory entry structure (must match kernel's getdents)
#[repr(C)]
struct DirEnt {
    d_ino: u64,
    d_off: i64,
    d_reclen: u16,
    d_type: u8,
    // d_name follows
}

// File stat structure (simplified)
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

// File type constants
#[allow(dead_code)]
const DT_UNKNOWN: u8 = 0;
#[allow(dead_code)]
const DT_REG: u8 = 8;
const DT_DIR: u8 = 4;
const DT_LNK: u8 = 10;

// Mode constants
const S_IFMT: u32 = 0o170000;
const S_IFDIR: u32 = 0o040000;
#[allow(dead_code)]
const S_IFREG: u32 = 0o100000;
const S_IFLNK: u32 = 0o120000;

pub fn main(argv: &'static [&'static str]) -> Result<i32> {
    let args = Args::parse(argv, "lah")?;
    
    let long_format = args.has_option('l');
    let show_hidden = args.has_option('a');
    let human_readable = args.has_option('h');
    
    // Get directory to list (default to current directory)
    let path = if args.positional_count() > 0 {
        args.get_positional(0).unwrap()
    } else {
        "."
    };
    
    // Open directory
    let mut path_bytes = Vec::new();
    path_bytes.extend_from_slice(path.as_bytes());
    path_bytes.push(0); // null terminator
    
    let fd = syscalls::openat(
        syscalls::AT_FDCWD,
        &path_bytes,
        syscalls::O_RDONLY | syscalls::O_DIRECTORY,
        0,
    );
    
    if fd < 0 {
        return Err(Error::from_errno(fd));
    }
    
    // Read directory entries
    let mut entries = Vec::new();
    let mut buf = [0u8; 4096];
    
    loop {
        let nread = syscalls::getdents(fd as i32, &mut buf);
        if nread < 0 {
            syscalls::close(fd as i32);
            return Err(Error::from_errno(nread));
        }
        if nread == 0 {
            break;
        }
        
        // Parse directory entries
        let mut pos = 0;
        while pos < nread as usize {
            let dirent = unsafe { &*(buf.as_ptr().add(pos) as *const DirEnt) };
            let reclen = dirent.d_reclen as usize;
            
            // Get name (starts after DirEnt struct)
            let name_start = pos + core::mem::size_of::<DirEnt>();
            let name_end = pos + reclen - 1; // -1 for null terminator
            let name_bytes = &buf[name_start..name_end];
            
            // Find actual end of name (null-terminated)
            let name_len = name_bytes.iter().position(|&b| b == 0).unwrap_or(name_bytes.len());
            let name = core::str::from_utf8(&name_bytes[..name_len]).unwrap_or("<invalid>");
            
            // Skip hidden files unless -a is specified
            if !show_hidden && name.starts_with('.') && name != "." && name != ".." {
                pos += reclen;
                continue;
            }
            
            entries.push((String::from(name), dirent.d_type));
            pos += reclen;
        }
    }
    
    syscalls::close(fd as i32);
    
    // Sort entries
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    
    // Display entries
    if long_format {
        for (name, dtype) in entries {
            // Get file stats
            let mut full_path = Vec::new();
            full_path.extend_from_slice(path.as_bytes());
            if !path.ends_with('/') {
                full_path.push(b'/');
            }
            full_path.extend_from_slice(name.as_bytes());
            full_path.push(0);
            
            let mut statbuf = [0u8; core::mem::size_of::<Stat>()];
            let stat_result = syscalls::lstat(&full_path, statbuf.as_mut_ptr());
            
            if stat_result < 0 {
                // If stat fails, show basic info
                print_long_entry_basic(&name, dtype);
            } else {
                let stat = unsafe { &*(statbuf.as_ptr() as *const Stat) };
                print_long_entry(stat, &name, human_readable);
            }
        }
    } else {
        // Simple format - just names
        for (name, _) in entries {
            syscalls::write(1, name.as_bytes());
            syscalls::write(1, b"\n");
        }
    }
    
    Ok(0)
}

fn print_long_entry_basic(name: &str, dtype: u8) {
    // Type character
    let type_char = match dtype {
        DT_DIR => b'd',
        DT_LNK => b'l',
        _ => b'-',
    };
    
    syscalls::write(1, &[type_char]);
    syscalls::write(1, b"rwxr-xr-x  1 user user        0 Jan  1 00:00 ");
    syscalls::write(1, name.as_bytes());
    syscalls::write(1, b"\n");
}

fn print_long_entry(stat: &Stat, name: &str, human_readable: bool) {
    let mut buf = [0u8; 256];
    let mut pos = 0;
    
    // File type
    let type_char = match stat.st_mode & S_IFMT {
        S_IFDIR => b'd',
        S_IFLNK => b'l',
        _ => b'-',
    };
    buf[pos] = type_char;
    pos += 1;
    
    // Permissions
    let mode = stat.st_mode;
    buf[pos] = if mode & 0o400 != 0 { b'r' } else { b'-' };
    pos += 1;
    buf[pos] = if mode & 0o200 != 0 { b'w' } else { b'-' };
    pos += 1;
    buf[pos] = if mode & 0o100 != 0 { b'x' } else { b'-' };
    pos += 1;
    buf[pos] = if mode & 0o040 != 0 { b'r' } else { b'-' };
    pos += 1;
    buf[pos] = if mode & 0o020 != 0 { b'w' } else { b'-' };
    pos += 1;
    buf[pos] = if mode & 0o010 != 0 { b'x' } else { b'-' };
    pos += 1;
    buf[pos] = if mode & 0o004 != 0 { b'r' } else { b'-' };
    pos += 1;
    buf[pos] = if mode & 0o002 != 0 { b'w' } else { b'-' };
    pos += 1;
    buf[pos] = if mode & 0o001 != 0 { b'x' } else { b'-' };
    pos += 1;
    
    // Padding
    buf[pos] = b' ';
    pos += 1;
    buf[pos] = b' ';
    pos += 1;
    
    // Number of links (simplified)
    let nlink_str = format_number(stat.st_nlink as i64);
    for &b in nlink_str.as_bytes() {
        buf[pos] = b;
        pos += 1;
    }
    buf[pos] = b' ';
    pos += 1;
    
    // Owner (simplified - just show "user")
    for &b in b"user " {
        buf[pos] = b;
        pos += 1;
    }
    
    // Group (simplified - just show "user")
    for &b in b"user " {
        buf[pos] = b;
        pos += 1;
    }
    
    // Size
    let size_str = if human_readable {
        format_size_human(stat.st_size)
    } else {
        format_number(stat.st_size)
    };
    
    // Right-align size (8 chars)
    let padding = if size_str.len() < 8 { 8 - size_str.len() } else { 0 };
    for _ in 0..padding {
        buf[pos] = b' ';
        pos += 1;
    }
    for &b in size_str.as_bytes() {
        buf[pos] = b;
        pos += 1;
    }
    buf[pos] = b' ';
    pos += 1;
    
    // Date (simplified - just show "Jan  1 00:00")
    for &b in b"Jan  1 00:00 " {
        buf[pos] = b;
        pos += 1;
    }
    
    // Name
    for &b in name.as_bytes() {
        if pos >= buf.len() - 1 {
            break;
        }
        buf[pos] = b;
        pos += 1;
    }
    
    buf[pos] = b'\n';
    pos += 1;
    
    syscalls::write(1, &buf[..pos]);
}

fn format_number(n: i64) -> String {
    if n == 0 {
        return String::from("0");
    }
    
    let mut result = String::new();
    let mut num = n.abs();
    
    while num > 0 {
        let digit = (num % 10) as u8;
        result.insert(0, (b'0' + digit) as char);
        num /= 10;
    }
    
    if n < 0 {
        result.insert(0, '-');
    }
    
    result
}

fn format_size_human(size: i64) -> String {
    if size < 1024 {
        return format_number(size);
    }
    
    let units = ['K', 'M', 'G', 'T'];
    let mut size_f = size as f64;
    let mut unit_idx = 0;
    
    while size_f >= 1024.0 && unit_idx < units.len() - 1 {
        size_f /= 1024.0;
        unit_idx += 1;
    }
    
    // Format with 1 decimal place
    let whole = size_f as i64;
    let frac = ((size_f - whole as f64) * 10.0) as i64;
    
    let mut result = format_number(whole);
    result.push('.');
    result.push((b'0' + frac as u8) as char);
    result.push(units[unit_idx]);
    
    result
}
