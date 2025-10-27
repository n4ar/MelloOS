//! ps - report process status

use crate::args::Args;
use crate::error::{Error, Result};
use crate::syscalls;
use alloc::string::String;
use alloc::vec::Vec;

// Directory entry structure (must match kernel's getdents)
#[repr(C)]
struct DirEnt {
    d_ino: u64,
    d_off: i64,
    d_reclen: u16,
    d_type: u8,
    // d_name follows
}

pub fn main(argv: &'static [&'static str]) -> Result<i32> {
    let args = Args::parse(argv, "aux")?;

    let _all_processes = args.has_option('a') || args.has_option('x');
    let user_format = args.has_option('u');

    // Print header
    if user_format {
        syscalls::write(
            1,
            b"USER       PID  PPID  PGID   SID TTY      STAT   TIME COMMAND\n",
        );
    } else {
        syscalls::write(1, b"  PID TTY          TIME CMD\n");
    }

    // Open /proc directory
    let proc_path = b"/proc\0";
    let proc_fd = syscalls::openat(
        syscalls::AT_FDCWD,
        proc_path,
        syscalls::O_RDONLY | syscalls::O_DIRECTORY,
        0,
    );

    if proc_fd < 0 {
        return Err(Error::from_errno(proc_fd));
    }

    // Read directory entries
    let mut buf = [0u8; 4096];
    let mut processes = Vec::new();

    loop {
        let nread = syscalls::getdents(proc_fd as i32, &mut buf);
        if nread < 0 {
            syscalls::close(proc_fd as i32);
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

            // Get name
            let name_start = pos + core::mem::size_of::<DirEnt>();
            let name_end = pos + reclen - 1;
            let name_bytes = &buf[name_start..name_end];
            let name_len = name_bytes
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(name_bytes.len());
            let name = core::str::from_utf8(&name_bytes[..name_len]).unwrap_or("");

            // Check if this is a PID directory (numeric name)
            if name.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(pid) = name.parse::<usize>() {
                    processes.push(pid);
                }
            }

            pos += reclen;
        }
    }

    syscalls::close(proc_fd as i32);

    // Sort processes by PID
    processes.sort();

    // Display each process
    for pid in processes {
        if let Ok(info) = read_proc_stat(pid) {
            if user_format {
                print_process_user_format(&info);
            } else {
                print_process_simple(&info);
            }
        }
    }

    Ok(0)
}

struct ProcessInfo {
    pid: usize,
    ppid: usize,
    pgid: usize,
    sid: usize,
    tty_nr: usize,
    state: char,
    #[allow(dead_code)]
    utime: usize,
    #[allow(dead_code)]
    stime: usize,
    comm: String,
}

fn read_proc_stat(pid: usize) -> Result<ProcessInfo> {
    // Build path: /proc/<pid>/stat
    let mut path = String::from("/proc/");
    path.push_str(&format_number(pid));
    path.push_str("/stat");

    let mut path_bytes = Vec::new();
    path_bytes.extend_from_slice(path.as_bytes());
    path_bytes.push(0);

    // Open stat file
    let fd = syscalls::openat(syscalls::AT_FDCWD, &path_bytes, syscalls::O_RDONLY, 0);

    if fd < 0 {
        return Err(Error::from_errno(fd));
    }

    // Read stat file
    let mut buf = [0u8; 1024];
    let nread = syscalls::read(fd as i32, &mut buf);
    syscalls::close(fd as i32);

    if nread < 0 {
        return Err(Error::from_errno(nread));
    }

    // Parse stat file
    let stat_str =
        core::str::from_utf8(&buf[..nread as usize]).map_err(|_| Error::InvalidArgument)?;
    parse_stat(stat_str)
}

fn parse_stat(stat: &str) -> Result<ProcessInfo> {
    // Format: pid (comm) state ppid pgrp session tty_nr tpgid flags ...
    let parts: Vec<&str> = stat.split_whitespace().collect();

    if parts.len() < 14 {
        return Err(Error::InvalidArgument);
    }

    // Extract comm (between parentheses)
    let comm_start = stat.find('(').ok_or(Error::InvalidArgument)?;
    let comm_end = stat.rfind(')').ok_or(Error::InvalidArgument)?;
    let comm = &stat[comm_start + 1..comm_end];

    // Parse numeric fields
    let pid = parts[0].parse().map_err(|_| Error::InvalidArgument)?;
    let state = parts[2].chars().next().unwrap_or('?');
    let ppid = parts[3].parse().map_err(|_| Error::InvalidArgument)?;
    let pgid = parts[4].parse().map_err(|_| Error::InvalidArgument)?;
    let sid = parts[5].parse().map_err(|_| Error::InvalidArgument)?;
    let tty_nr = parts[6].parse().map_err(|_| Error::InvalidArgument)?;
    let utime = parts[13].parse().unwrap_or(0);
    let stime = parts[14].parse().unwrap_or(0);

    Ok(ProcessInfo {
        pid,
        ppid,
        pgid,
        sid,
        tty_nr,
        state,
        utime,
        stime,
        comm: String::from(comm),
    })
}

fn print_process_simple(info: &ProcessInfo) {
    let mut buf = [0u8; 256];
    let mut pos = 0;

    // PID (5 chars, right-aligned)
    let pid_str = format_number(info.pid);
    let padding = if pid_str.len() < 5 {
        5 - pid_str.len()
    } else {
        0
    };
    for _ in 0..padding {
        buf[pos] = b' ';
        pos += 1;
    }
    for &b in pid_str.as_bytes() {
        buf[pos] = b;
        pos += 1;
    }
    buf[pos] = b' ';
    pos += 1;

    // TTY
    if info.tty_nr == 0 {
        for &b in b"?            " {
            buf[pos] = b;
            pos += 1;
        }
    } else {
        for &b in b"pts/0        " {
            buf[pos] = b;
            pos += 1;
        }
    }

    // TIME (simplified - just show 00:00:00)
    for &b in b"00:00:00 " {
        buf[pos] = b;
        pos += 1;
    }

    // Command
    for &b in info.comm.as_bytes() {
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

fn print_process_user_format(info: &ProcessInfo) {
    let mut buf = [0u8; 256];
    let mut pos = 0;

    // USER (simplified - just show "user")
    for &b in b"user      " {
        buf[pos] = b;
        pos += 1;
    }

    // PID (5 chars)
    let pid_str = format_number(info.pid);
    let padding = if pid_str.len() < 5 {
        5 - pid_str.len()
    } else {
        0
    };
    for _ in 0..padding {
        buf[pos] = b' ';
        pos += 1;
    }
    for &b in pid_str.as_bytes() {
        buf[pos] = b;
        pos += 1;
    }
    buf[pos] = b' ';
    pos += 1;

    // PPID (5 chars)
    let ppid_str = format_number(info.ppid);
    let padding = if ppid_str.len() < 5 {
        5 - ppid_str.len()
    } else {
        0
    };
    for _ in 0..padding {
        buf[pos] = b' ';
        pos += 1;
    }
    for &b in ppid_str.as_bytes() {
        buf[pos] = b;
        pos += 1;
    }
    buf[pos] = b' ';
    pos += 1;

    // PGID (5 chars)
    let pgid_str = format_number(info.pgid);
    let padding = if pgid_str.len() < 5 {
        5 - pgid_str.len()
    } else {
        0
    };
    for _ in 0..padding {
        buf[pos] = b' ';
        pos += 1;
    }
    for &b in pgid_str.as_bytes() {
        buf[pos] = b;
        pos += 1;
    }
    buf[pos] = b' ';
    pos += 1;

    // SID (5 chars)
    let sid_str = format_number(info.sid);
    let padding = if sid_str.len() < 5 {
        5 - sid_str.len()
    } else {
        0
    };
    for _ in 0..padding {
        buf[pos] = b' ';
        pos += 1;
    }
    for &b in sid_str.as_bytes() {
        buf[pos] = b;
        pos += 1;
    }
    buf[pos] = b' ';
    pos += 1;

    // TTY
    if info.tty_nr == 0 {
        for &b in b"?        " {
            buf[pos] = b;
            pos += 1;
        }
    } else {
        for &b in b"pts/0    " {
            buf[pos] = b;
            pos += 1;
        }
    }

    // STAT
    buf[pos] = info.state as u8;
    pos += 1;
    for &b in b"      " {
        buf[pos] = b;
        pos += 1;
    }

    // TIME
    for &b in b"0:00 " {
        buf[pos] = b;
        pos += 1;
    }

    // COMMAND
    for &b in info.comm.as_bytes() {
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

fn format_number(n: usize) -> String {
    if n == 0 {
        return String::from("0");
    }

    let mut result = String::new();
    let mut num = n;

    while num > 0 {
        let digit = (num % 10) as u8;
        result.insert(0, (b'0' + digit) as char);
        num /= 10;
    }

    result
}
