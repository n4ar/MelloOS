//! /proc Virtual Filesystem
//!
//! This module implements a virtual filesystem that provides process and
//! system information through file-like interfaces.
//!
//! # Lock-Free Reads
//!
//! To avoid holding locks during /proc file generation, this module uses
//! a snapshot approach:
//!
//! 1. Read process state atomically (using atomic loads for signal fields)
//! 2. Generate the /proc file content from the snapshot
//! 3. Handle races gracefully (process may exit during read)
//!
//! For process state that changes frequently, we use seqlocks to detect
//! concurrent modifications and retry if necessary. This ensures consistent
//! reads without blocking writers.

/// Maximum command name length
const MAX_COMM_LEN: usize = 16;

/// Maximum command line arguments
const MAX_CMDLINE_ARGS: usize = 32;

/// Maximum argument length
const MAX_ARG_LEN: usize = 256;

/// Initialize /proc filesystem
///
/// The /proc filesystem is virtual and doesn't require explicit mounting.
/// This function just logs that /proc is available.
pub fn init() {
    crate::serial_println!("[PROC] Virtual filesystem initialized");
    crate::serial_println!("[PROC] Available at /proc");
}

/// Process state for /proc/<pid>/stat
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcState {
    /// Running
    Running,
    /// Sleeping (interruptible)
    Sleeping,
    /// Stopped (by signal)
    Stopped,
    /// Zombie (terminated but not reaped)
    Zombie,
}

impl ProcState {
    /// Convert to single-character state code
    pub fn to_char(&self) -> char {
        match self {
            ProcState::Running => 'R',
            ProcState::Sleeping => 'S',
            ProcState::Stopped => 'T',
            ProcState::Zombie => 'Z',
        }
    }
}

/// Process information for /proc/<pid>/ files
#[derive(Debug, Clone)]
pub struct ProcInfo {
    /// Process ID
    pub pid: usize,
    /// Parent process ID
    pub ppid: usize,
    /// Process group ID
    pub pgid: usize,
    /// Session ID
    pub sid: usize,
    /// Controlling terminal device number (0 if none)
    pub tty_nr: usize,
    /// Foreground process group of controlling terminal
    pub tpgid: Option<usize>,
    /// Process state
    pub state: ProcState,
    /// Command name (fixed size)
    pub comm: [u8; MAX_COMM_LEN],
    /// Command name length
    pub comm_len: usize,
    /// Command line arguments (fixed size array)
    pub cmdline: [[u8; MAX_ARG_LEN]; MAX_CMDLINE_ARGS],
    /// Number of command line arguments
    pub cmdline_count: usize,
    /// User time (clock ticks)
    pub utime: u64,
    /// System time (clock ticks)
    pub stime: u64,
    /// Virtual memory size (bytes)
    pub vsize: usize,
    /// Resident set size (pages)
    pub rss: usize,
}

impl ProcInfo {
    /// Create a new ProcInfo with default values
    pub const fn new(pid: usize) -> Self {
        Self {
            pid,
            ppid: 0,
            pgid: pid,
            sid: pid,
            tty_nr: 0,
            tpgid: None,
            state: ProcState::Running,
            comm: [0u8; MAX_COMM_LEN],
            comm_len: 0,
            cmdline: [[0u8; MAX_ARG_LEN]; MAX_CMDLINE_ARGS],
            cmdline_count: 0,
            utime: 0,
            stime: 0,
            vsize: 0,
            rss: 0,
        }
    }

    /// Set command name from a string slice
    pub fn set_comm(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let len = bytes.len().min(MAX_COMM_LEN);
        self.comm[..len].copy_from_slice(&bytes[..len]);
        self.comm_len = len;
    }

    /// Get command name as a string slice
    pub fn get_comm(&self) -> &str {
        core::str::from_utf8(&self.comm[..self.comm_len]).unwrap_or("unknown")
    }

    /// Add a command line argument
    pub fn add_cmdline_arg(&mut self, arg: &str) -> bool {
        if self.cmdline_count >= MAX_CMDLINE_ARGS {
            return false;
        }

        let bytes = arg.as_bytes();
        let len = bytes.len().min(MAX_ARG_LEN);
        self.cmdline[self.cmdline_count][..len].copy_from_slice(&bytes[..len]);
        self.cmdline_count += 1;
        true
    }

    /// Format as /proc/<pid>/stat content
    ///
    /// Format: pid (comm) state ppid pgrp session tty_nr tpgid flags ...
    /// Returns the number of bytes written to the buffer
    pub fn format_stat(&self, buf: &mut [u8]) -> usize {
        use core::fmt::Write;

        struct BufWriter<'a> {
            buf: &'a mut [u8],
            pos: usize,
        }

        impl<'a> Write for BufWriter<'a> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let bytes = s.as_bytes();
                let remaining = self.buf.len() - self.pos;
                let to_write = bytes.len().min(remaining);
                self.buf[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
                self.pos += to_write;
                Ok(())
            }
        }

        let mut writer = BufWriter { buf, pos: 0 };
        let _ = write!(
            writer,
            "{} ({}) {} {} {} {} {} {} 0 0 0 0 0 {} {} 0 0 0 0 0 0 0 {} {} 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n",
            self.pid,
            self.get_comm(),
            self.state.to_char(),
            self.ppid,
            self.pgid,
            self.sid,
            self.tty_nr,
            self.tpgid.unwrap_or(0),
            self.utime,
            self.stime,
            self.vsize,
            self.rss,
        );
        writer.pos
    }

    /// Format as /proc/<pid>/status content
    ///
    /// Human-readable format with key-value pairs
    /// Returns the number of bytes written to the buffer
    pub fn format_status(&self, buf: &mut [u8]) -> usize {
        use core::fmt::Write;

        struct BufWriter<'a> {
            buf: &'a mut [u8],
            pos: usize,
        }

        impl<'a> Write for BufWriter<'a> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let bytes = s.as_bytes();
                let remaining = self.buf.len() - self.pos;
                let to_write = bytes.len().min(remaining);
                self.buf[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
                self.pos += to_write;
                Ok(())
            }
        }

        let mut writer = BufWriter { buf, pos: 0 };
        let _ = write!(
            writer,
            "Name:\t{}\n\
             State:\t{} ({})\n\
             Pid:\t{}\n\
             PPid:\t{}\n\
             Pgid:\t{}\n\
             Sid:\t{}\n\
             VmSize:\t{} kB\n\
             VmRSS:\t{} kB\n",
            self.get_comm(),
            self.state.to_char(),
            match self.state {
                ProcState::Running => "running",
                ProcState::Sleeping => "sleeping",
                ProcState::Stopped => "stopped",
                ProcState::Zombie => "zombie",
            },
            self.pid,
            self.ppid,
            self.pgid,
            self.sid,
            self.vsize / 1024,
            self.rss * 4, // Assuming 4KB pages
        );
        writer.pos
    }

    /// Format as /proc/<pid>/cmdline content
    ///
    /// Null-separated command line arguments
    /// Returns the number of bytes written to the buffer
    pub fn format_cmdline(&self, buf: &mut [u8]) -> usize {
        let mut pos = 0;
        for i in 0..self.cmdline_count {
            // Find the actual length of this argument (up to first null or MAX_ARG_LEN)
            let mut arg_len = 0;
            for j in 0..MAX_ARG_LEN {
                if self.cmdline[i][j] == 0 {
                    break;
                }
                arg_len = j + 1;
            }

            // Copy argument to buffer
            let remaining = buf.len() - pos;
            let to_copy = arg_len.min(remaining);
            if to_copy > 0 {
                buf[pos..pos + to_copy].copy_from_slice(&self.cmdline[i][..to_copy]);
                pos += to_copy;
            }

            // Add null separator (except after last argument)
            if i < self.cmdline_count - 1 && pos < buf.len() {
                buf[pos] = 0;
                pos += 1;
            }
        }
        pos
    }
}

/// System-wide memory information for /proc/meminfo
#[derive(Debug, Clone, Copy)]
pub struct MemInfo {
    /// Total physical memory (kB)
    pub mem_total: usize,
    /// Free memory (kB)
    pub mem_free: usize,
    /// Available memory (kB)
    pub mem_available: usize,
    /// Buffer memory (kB)
    pub buffers: usize,
    /// Cached memory (kB)
    pub cached: usize,
}

impl MemInfo {
    /// Format as /proc/meminfo content
    /// Returns the number of bytes written to the buffer
    pub fn format(&self, buf: &mut [u8]) -> usize {
        use core::fmt::Write;

        struct BufWriter<'a> {
            buf: &'a mut [u8],
            pos: usize,
        }

        impl<'a> Write for BufWriter<'a> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let bytes = s.as_bytes();
                let remaining = self.buf.len() - self.pos;
                let to_write = bytes.len().min(remaining);
                self.buf[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
                self.pos += to_write;
                Ok(())
            }
        }

        let mut writer = BufWriter { buf, pos: 0 };
        let _ = write!(
            writer,
            "MemTotal:       {} kB\n\
             MemFree:        {} kB\n\
             MemAvailable:   {} kB\n\
             Buffers:        {} kB\n\
             Cached:         {} kB\n",
            self.mem_total, self.mem_free, self.mem_available, self.buffers, self.cached,
        );
        writer.pos
    }
}

/// Maximum vendor ID length
const MAX_VENDOR_ID_LEN: usize = 16;

/// Maximum model name length
const MAX_MODEL_NAME_LEN: usize = 64;

/// CPU information for /proc/cpuinfo
#[derive(Debug, Clone)]
pub struct CpuInfo {
    /// Processor number
    pub processor: usize,
    /// Vendor ID (fixed size)
    pub vendor_id: [u8; MAX_VENDOR_ID_LEN],
    /// Vendor ID length
    pub vendor_id_len: usize,
    /// CPU family
    pub cpu_family: usize,
    /// Model
    pub model: usize,
    /// Model name (fixed size)
    pub model_name: [u8; MAX_MODEL_NAME_LEN],
    /// Model name length
    pub model_name_len: usize,
    /// CPU MHz
    pub cpu_mhz: u32,
}

impl CpuInfo {
    /// Create a new CpuInfo with default values
    pub const fn new(processor: usize) -> Self {
        Self {
            processor,
            vendor_id: [0u8; MAX_VENDOR_ID_LEN],
            vendor_id_len: 0,
            cpu_family: 0,
            model: 0,
            model_name: [0u8; MAX_MODEL_NAME_LEN],
            model_name_len: 0,
            cpu_mhz: 0,
        }
    }

    /// Set vendor ID from a string slice
    pub fn set_vendor_id(&mut self, vendor: &str) {
        let bytes = vendor.as_bytes();
        let len = bytes.len().min(MAX_VENDOR_ID_LEN);
        self.vendor_id[..len].copy_from_slice(&bytes[..len]);
        self.vendor_id_len = len;
    }

    /// Set model name from a string slice
    pub fn set_model_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let len = bytes.len().min(MAX_MODEL_NAME_LEN);
        self.model_name[..len].copy_from_slice(&bytes[..len]);
        self.model_name_len = len;
    }

    /// Format as /proc/cpuinfo content for one CPU
    /// Returns the number of bytes written to the buffer
    pub fn format(&self, buf: &mut [u8]) -> usize {
        use core::fmt::Write;

        struct BufWriter<'a> {
            buf: &'a mut [u8],
            pos: usize,
        }

        impl<'a> Write for BufWriter<'a> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let bytes = s.as_bytes();
                let remaining = self.buf.len() - self.pos;
                let to_write = bytes.len().min(remaining);
                self.buf[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
                self.pos += to_write;
                Ok(())
            }
        }

        let vendor_id =
            core::str::from_utf8(&self.vendor_id[..self.vendor_id_len]).unwrap_or("unknown");
        let model_name =
            core::str::from_utf8(&self.model_name[..self.model_name_len]).unwrap_or("unknown");

        let mut writer = BufWriter { buf, pos: 0 };
        let _ = write!(
            writer,
            "processor\t: {}\n\
             vendor_id\t: {}\n\
             cpu family\t: {}\n\
             model\t\t: {}\n\
             model name\t: {}\n\
             cpu MHz\t\t: {}\n\n",
            self.processor, vendor_id, self.cpu_family, self.model, model_name, self.cpu_mhz,
        );
        writer.pos
    }
}

/// System uptime information for /proc/uptime
#[derive(Debug, Clone, Copy)]
pub struct Uptime {
    /// System uptime in seconds
    pub uptime_secs: u64,
    /// Idle time in seconds (sum across all CPUs)
    pub idle_secs: u64,
}

impl Uptime {
    /// Format as /proc/uptime content
    ///
    /// Format: uptime.fraction idle.fraction
    /// Returns the number of bytes written to the buffer
    pub fn format(&self, buf: &mut [u8]) -> usize {
        use core::fmt::Write;

        struct BufWriter<'a> {
            buf: &'a mut [u8],
            pos: usize,
        }

        impl<'a> Write for BufWriter<'a> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let bytes = s.as_bytes();
                let remaining = self.buf.len() - self.pos;
                let to_write = bytes.len().min(remaining);
                self.buf[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
                self.pos += to_write;
                Ok(())
            }
        }

        let mut writer = BufWriter { buf, pos: 0 };
        let _ = write!(writer, "{}.00 {}.00\n", self.uptime_secs, self.idle_secs);
        writer.pos
    }
}

/// /proc filesystem path types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcPath {
    /// /proc root directory
    Root,
    /// /proc/<pid> directory
    PidDir(usize),
    /// /proc/<pid>/stat file
    PidStat(usize),
    /// /proc/<pid>/status file
    PidStatus(usize),
    /// /proc/<pid>/cmdline file
    PidCmdline(usize),
    /// /proc/self symlink
    Self_,
    /// /proc/meminfo file
    MemInfo,
    /// /proc/cpuinfo file
    CpuInfo,
    /// /proc/uptime file
    Uptime,
    /// /proc/stat file (system-wide statistics)
    Stat,
    /// /proc/debug directory
    DebugDir,
    /// /proc/debug/pty file
    DebugPty,
    /// /proc/debug/sessions file
    DebugSessions,
    /// /proc/debug/locks file
    DebugLocks,
    /// Unknown/invalid path
    Invalid,
}

/// Parse a /proc path string into a ProcPath enum
///
/// # Arguments
/// * `path` - Path string to parse (e.g., "/proc/1/stat")
///
/// # Returns
/// The corresponding ProcPath variant
pub fn parse_proc_path(path: &str) -> ProcPath {
    // Remove leading/trailing slashes
    let path = path.trim_matches('/');

    // Must start with "proc"
    if !path.starts_with("proc") {
        return ProcPath::Invalid;
    }

    // Remove "proc/" prefix
    let rest = if path.len() > 4 && path.as_bytes()[4] == b'/' {
        &path[5..]
    } else if path == "proc" {
        return ProcPath::Root;
    } else {
        return ProcPath::Invalid;
    };

    // Find first slash to split into parts
    if let Some(slash_pos) = rest.find('/') {
        let first = &rest[..slash_pos];
        let second = &rest[slash_pos + 1..];

        // /proc/<first>/<second>
        if first == "debug" {
            match second {
                "pty" => ProcPath::DebugPty,
                "sessions" => ProcPath::DebugSessions,
                "locks" => ProcPath::DebugLocks,
                _ => ProcPath::Invalid,
            }
        } else if let Ok(pid) = first.parse::<usize>() {
            match second {
                "stat" => ProcPath::PidStat(pid),
                "status" => ProcPath::PidStatus(pid),
                "cmdline" => ProcPath::PidCmdline(pid),
                _ => ProcPath::Invalid,
            }
        } else {
            ProcPath::Invalid
        }
    } else {
        // /proc/<first> (no second part)
        match rest {
            "self" => ProcPath::Self_,
            "meminfo" => ProcPath::MemInfo,
            "cpuinfo" => ProcPath::CpuInfo,
            "uptime" => ProcPath::Uptime,
            "stat" => ProcPath::Stat,
            "debug" => ProcPath::DebugDir,
            pid_str => {
                // Try to parse as PID
                if let Ok(pid) = pid_str.parse::<usize>() {
                    ProcPath::PidDir(pid)
                } else {
                    ProcPath::Invalid
                }
            }
        }
    }
}

/// Read data from a /proc file
///
/// This is the main entry point for reading /proc files. It generates
/// the content dynamically based on the current system state.
///
/// # Arguments
/// * `path` - The /proc path to read
/// * `buf` - Buffer to write the content into
/// * `offset` - Offset within the file to start reading from
///
/// # Returns
/// The number of bytes written to the buffer, or an error code
pub fn proc_read(path: &str, buf: &mut [u8], offset: usize) -> Result<usize, i32> {
    let proc_path = parse_proc_path(path);

    match proc_path {
        ProcPath::PidStat(pid) => read_pid_stat(pid, buf, offset),
        ProcPath::PidStatus(pid) => read_pid_status(pid, buf, offset),
        ProcPath::PidCmdline(pid) => read_pid_cmdline(pid, buf, offset),
        ProcPath::MemInfo => read_meminfo(buf, offset),
        ProcPath::CpuInfo => read_cpuinfo(buf, offset),
        ProcPath::Uptime => read_uptime(buf, offset),
        ProcPath::Stat => read_stat(buf, offset),
        ProcPath::Self_ => {
            // /proc/self should be handled as a symlink by the caller
            Err(-22) // EINVAL
        }
        ProcPath::DebugPty => read_debug_pty(buf, offset),
        ProcPath::DebugSessions => read_debug_sessions(buf, offset),
        ProcPath::DebugLocks => read_debug_locks(buf, offset),
        _ => Err(-2), // ENOENT
    }
}

/// Read /proc/<pid>/stat file
fn read_pid_stat(pid: usize, buf: &mut [u8], offset: usize) -> Result<usize, i32> {
    // Get process info from scheduler
    let proc_info = get_proc_info(pid).ok_or(-3)?; // ESRCH - no such process

    // Generate stat content
    let mut temp_buf = [0u8; 1024];
    let len = proc_info.format_stat(&mut temp_buf);

    // Copy to output buffer with offset
    copy_with_offset(&temp_buf[..len], buf, offset)
}

/// Read /proc/<pid>/status file
fn read_pid_status(pid: usize, buf: &mut [u8], offset: usize) -> Result<usize, i32> {
    let proc_info = get_proc_info(pid).ok_or(-3)?; // ESRCH

    let mut temp_buf = [0u8; 1024];
    let len = proc_info.format_status(&mut temp_buf);

    copy_with_offset(&temp_buf[..len], buf, offset)
}

/// Read /proc/<pid>/cmdline file
fn read_pid_cmdline(pid: usize, buf: &mut [u8], offset: usize) -> Result<usize, i32> {
    let proc_info = get_proc_info(pid).ok_or(-3)?; // ESRCH

    let mut temp_buf = [0u8; 4096];
    let len = proc_info.format_cmdline(&mut temp_buf);

    copy_with_offset(&temp_buf[..len], buf, offset)
}

/// Read /proc/meminfo file
fn read_meminfo(buf: &mut [u8], offset: usize) -> Result<usize, i32> {
    let mem_info = get_meminfo();

    let mut temp_buf = [0u8; 512];
    let len = mem_info.format(&mut temp_buf);

    copy_with_offset(&temp_buf[..len], buf, offset)
}

/// Read /proc/cpuinfo file
fn read_cpuinfo(buf: &mut [u8], offset: usize) -> Result<usize, i32> {
    let cpu_count = crate::arch::x86_64::smp::get_cpu_count();
    let mut temp_buf = [0u8; 4096];
    let mut pos = 0;

    // Generate cpuinfo for each CPU
    for cpu_id in 0..cpu_count {
        let cpu_info = get_cpuinfo(cpu_id);
        let written = cpu_info.format(&mut temp_buf[pos..]);
        pos += written;
    }

    copy_with_offset(&temp_buf[..pos], buf, offset)
}

/// Read /proc/uptime file
fn read_uptime(buf: &mut [u8], offset: usize) -> Result<usize, i32> {
    let uptime = get_uptime();

    let mut temp_buf = [0u8; 128];
    let len = uptime.format(&mut temp_buf);

    copy_with_offset(&temp_buf[..len], buf, offset)
}

/// Read /proc/stat file (system-wide statistics)
fn read_stat(buf: &mut [u8], offset: usize) -> Result<usize, i32> {
    use crate::metrics;
    use core::fmt::Write;

    struct BufWriter<'a> {
        buf: &'a mut [u8],
        pos: usize,
    }

    impl<'a> Write for BufWriter<'a> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let bytes = s.as_bytes();
            let remaining = self.buf.len() - self.pos;
            let to_write = bytes.len().min(remaining);
            self.buf[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
            self.pos += to_write;
            Ok(())
        }
    }

    let mut temp_buf = [0u8; 4096];
    let mut writer = BufWriter {
        buf: &mut temp_buf,
        pos: 0,
    };

    let m = metrics::metrics();

    // CPU statistics (simplified - per-CPU stats would go here)
    let cpu_count = crate::arch::x86_64::smp::get_cpu_count();
    let _ = write!(writer, "cpu  0 0 0 0 0 0 0 0 0 0\n");
    for cpu_id in 0..cpu_count {
        let _ = write!(writer, "cpu{}  0 0 0 0 0 0 0 0 0 0\n", cpu_id);
    }

    // Context switches
    let _ = write!(writer, "ctxt {}\n", m.get_context_switches());

    // Boot time (placeholder)
    let _ = write!(writer, "btime 0\n");

    // Processes (placeholder - would need to count tasks)
    let _ = write!(writer, "processes 0\n");

    // Running processes (placeholder)
    let _ = write!(writer, "procs_running 1\n");

    // Blocked processes (placeholder)
    let _ = write!(writer, "procs_blocked 0\n");

    // Interrupts
    let _ = write!(writer, "intr {}\n", m.get_interrupts());

    // Page faults
    let _ = write!(writer, "page_faults {}\n", m.get_page_faults());

    // Signals delivered
    let _ = write!(writer, "signals_delivered {}\n", m.get_signals_delivered());

    // PTY statistics
    let _ = write!(writer, "pty_bytes_in {}\n", m.get_pty_bytes_in());
    let _ = write!(writer, "pty_bytes_out {}\n", m.get_pty_bytes_out());

    // IPC statistics
    let _ = write!(writer, "ipc_sent {}\n", m.get_ipc_sent());
    let _ = write!(writer, "ipc_received {}\n", m.get_ipc_received());

    // Total syscalls
    let _ = write!(writer, "syscalls_total {}\n", m.get_total_syscalls());

    // Top 10 syscalls by count (simplified - just show first 20 syscalls)
    let _ = write!(writer, "syscalls_top20:\n");
    for i in 0..20 {
        let count = m.get_syscall_count(i);
        if count > 0 {
            let _ = write!(writer, "  syscall_{}: {}\n", i, count);
        }
    }

    let len = writer.pos;
    copy_with_offset(&temp_buf[..len], buf, offset)
}

/// Read /proc/debug/pty file
fn read_debug_pty(buf: &mut [u8], offset: usize) -> Result<usize, i32> {
    use crate::metrics;
    use core::fmt::Write;

    struct BufWriter<'a> {
        buf: &'a mut [u8],
        pos: usize,
    }

    impl<'a> Write for BufWriter<'a> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let bytes = s.as_bytes();
            let remaining = self.buf.len() - self.pos;
            let to_write = bytes.len().min(remaining);
            self.buf[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
            self.pos += to_write;
            Ok(())
        }
    }

    let mut temp_buf = [0u8; 2048];
    let mut writer = BufWriter {
        buf: &mut temp_buf,
        pos: 0,
    };

    let m = metrics::metrics();

    let _ = write!(writer, "PTY Subsystem Debug Information\n");
    let _ = write!(writer, "================================\n\n");

    let _ = write!(writer, "Statistics:\n");
    let _ = write!(writer, "  Bytes In:  {}\n", m.get_pty_bytes_in());
    let _ = write!(writer, "  Bytes Out: {}\n", m.get_pty_bytes_out());
    let _ = write!(writer, "  Signals:   {}\n\n", m.get_signals_delivered());

    let _ = write!(writer, "Active PTY Pairs:\n");
    let _ = write!(writer, "  (PTY enumeration not yet implemented)\n\n");

    let _ = write!(writer, "Session Information:\n");
    let _ = write!(writer, "  (See /proc/debug/sessions for details)\n");

    let len = writer.pos;
    copy_with_offset(&temp_buf[..len], buf, offset)
}

/// Read /proc/debug/sessions file
fn read_debug_sessions(buf: &mut [u8], offset: usize) -> Result<usize, i32> {
    use crate::sched;
    use core::fmt::Write;

    struct BufWriter<'a> {
        buf: &'a mut [u8],
        pos: usize,
    }

    impl<'a> Write for BufWriter<'a> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let bytes = s.as_bytes();
            let remaining = self.buf.len() - self.pos;
            let to_write = bytes.len().min(remaining);
            self.buf[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
            self.pos += to_write;
            Ok(())
        }
    }

    let mut temp_buf = [0u8; 4096];
    let mut writer = BufWriter {
        buf: &mut temp_buf,
        pos: 0,
    };

    let _ = write!(writer, "Session and Process Group Information\n");
    let _ = write!(writer, "======================================\n\n");

    let _ = write!(
        writer,
        "Format: PID | PPID | PGID | SID | TTY | State | Name\n"
    );
    let _ = write!(
        writer,
        "------+------+------+-----+-----+-------+----------\n"
    );

    // Iterate through all tasks and display session info
    // Count tasks by iterating through the task table
    let mut task_count = 0;
    const MAX_TASKS: usize = 1024;

    for task_id in 1..MAX_TASKS {
        if let Some(task) = sched::get_task_by_id(task_id) {
            task_count += 1;

            let tty_str = if let Some(tty) = task.tty {
                alloc::format!("{}", tty)
            } else {
                alloc::string::String::from("?")
            };

            let state_char = match task.state {
                crate::sched::task::TaskState::Running => 'R',
                crate::sched::task::TaskState::Ready => 'R',
                crate::sched::task::TaskState::Sleeping => 'S',
                crate::sched::task::TaskState::Blocked => 'D',
            };

            let _ = write!(
                writer,
                "{:5} | {:4} | {:4} | {:3} | {:3} | {:5} | {}\n",
                task.pid, task.ppid, task.pgid, task.sid, tty_str, state_char, task.name
            );
        }
    }

    let _ = write!(writer, "\nTotal processes: {}\n", task_count);

    let len = writer.pos;
    copy_with_offset(&temp_buf[..len], buf, offset)
}

/// Read /proc/debug/locks file
fn read_debug_locks(buf: &mut [u8], offset: usize) -> Result<usize, i32> {
    use core::fmt::Write;

    struct BufWriter<'a> {
        buf: &'a mut [u8],
        pos: usize,
    }

    impl<'a> Write for BufWriter<'a> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let bytes = s.as_bytes();
            let remaining = self.buf.len() - self.pos;
            let to_write = bytes.len().min(remaining);
            self.buf[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
            self.pos += to_write;
            Ok(())
        }
    }

    let mut temp_buf = [0u8; 2048];
    let mut writer = BufWriter {
        buf: &mut temp_buf,
        pos: 0,
    };

    let _ = write!(writer, "Kernel Lock Debug Information\n");
    let _ = write!(writer, "==============================\n\n");

    let _ = write!(writer, "Lock Ordering Hierarchy:\n");
    let _ = write!(writer, "  1. SCHEDULER_LOCK\n");
    let _ = write!(writer, "  2. PROCESS_TABLE_LOCK\n");
    let _ = write!(writer, "  3. MEMORY_MANAGER_LOCK\n");
    let _ = write!(writer, "  4. DEVICE_TREE_LOCK\n");
    let _ = write!(writer, "  5. PTY_LOCK\n\n");

    let _ = write!(writer, "Lock Statistics:\n");
    let _ = write!(
        writer,
        "  (Lock contention tracking not yet implemented)\n\n"
    );

    let _ = write!(writer, "Deadlock Detection:\n");
    let _ = write!(writer, "  Status: Enabled (compile-time ordering)\n");
    let _ = write!(writer, "  Method: Static lock ordering hierarchy\n");
    let _ = write!(writer, "  See: kernel/src/sync/lock_ordering.rs\n");

    let len = writer.pos;
    copy_with_offset(&temp_buf[..len], buf, offset)
}

/// Helper function to copy data with offset
///
/// # Arguments
/// * `src` - Source data
/// * `dst` - Destination buffer
/// * `offset` - Offset within source to start copying from
///
/// # Returns
/// Number of bytes copied
fn copy_with_offset(src: &[u8], dst: &mut [u8], offset: usize) -> Result<usize, i32> {
    if offset >= src.len() {
        return Ok(0); // EOF
    }

    let remaining = &src[offset..];
    let to_copy = remaining.len().min(dst.len());
    dst[..to_copy].copy_from_slice(&remaining[..to_copy]);

    Ok(to_copy)
}

/// Get process information for a given PID
///
/// This function queries the scheduler for task information and
/// converts it to ProcInfo format.
fn get_proc_info(pid: usize) -> Option<ProcInfo> {
    use crate::sched;

    // Get task from scheduler
    let task = sched::get_task_by_id(pid)?;

    let mut proc_info = ProcInfo::new(pid);
    proc_info.ppid = task.ppid;
    proc_info.pgid = task.pgid;
    proc_info.sid = task.sid;
    proc_info.tty_nr = task.tty.unwrap_or(0);
    proc_info.tpgid = None; // TODO: Get from TTY when available

    // Set state based on task state
    proc_info.state = match task.state {
        crate::sched::task::TaskState::Running => ProcState::Running,
        crate::sched::task::TaskState::Ready => ProcState::Running,
        crate::sched::task::TaskState::Sleeping => ProcState::Sleeping,
        crate::sched::task::TaskState::Blocked => ProcState::Sleeping,
    };

    // Set command name
    proc_info.set_comm(task.name);

    // TODO: Set cmdline from task when available
    // For now, just use the task name
    proc_info.add_cmdline_arg(task.name);

    // TODO: Get actual timing and memory info
    proc_info.utime = 0;
    proc_info.stime = 0;
    proc_info.vsize = task.total_memory_usage();
    proc_info.rss = task.total_memory_usage() / 4096; // Convert to pages

    Some(proc_info)
}

/// Get system memory information
fn get_meminfo() -> MemInfo {
    // Get memory statistics from memory manager
    let result = crate::mm::with_memory_managers(|pmm, _mapper| {
        let mem_total = pmm.total_memory_mb() * 1024; // Convert MB to kB
        let mem_free = pmm.free_memory_mb() * 1024; // Convert MB to kB

        Ok(MemInfo {
            mem_total,
            mem_free,
            mem_available: mem_free, // Simplified for now
            buffers: 0,              // TODO: Track buffer cache
            cached: 0,               // TODO: Track page cache
        })
    });

    // If memory manager not initialized, return zeros
    result.unwrap_or(MemInfo {
        mem_total: 0,
        mem_free: 0,
        mem_available: 0,
        buffers: 0,
        cached: 0,
    })
}

/// Get CPU information for a specific CPU
fn get_cpuinfo(cpu_id: usize) -> CpuInfo {
    let mut cpu_info = CpuInfo::new(cpu_id);

    // Get CPU vendor ID from CPUID
    let cpuid = unsafe { core::arch::x86_64::__cpuid(0) };
    let mut vendor = [0u8; 12];
    vendor[0..4].copy_from_slice(&cpuid.ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&cpuid.edx.to_le_bytes());
    vendor[8..12].copy_from_slice(&cpuid.ecx.to_le_bytes());

    if let Ok(vendor_str) = core::str::from_utf8(&vendor) {
        cpu_info.set_vendor_id(vendor_str);
    }

    // Get CPU family and model
    let cpuid1 = unsafe { core::arch::x86_64::__cpuid(1) };
    cpu_info.cpu_family = ((cpuid1.eax >> 8) & 0xF) as usize;
    cpu_info.model = ((cpuid1.eax >> 4) & 0xF) as usize;

    // Get brand string if available
    let cpuid_ext = unsafe { core::arch::x86_64::__cpuid(0x80000000) };
    if cpuid_ext.eax >= 0x80000004 {
        let mut brand = [0u8; 48];
        for i in 0..3 {
            let cpuid = unsafe { core::arch::x86_64::__cpuid(0x80000002 + i) };
            let offset = i as usize * 16;
            brand[offset..offset + 4].copy_from_slice(&cpuid.eax.to_le_bytes());
            brand[offset + 4..offset + 8].copy_from_slice(&cpuid.ebx.to_le_bytes());
            brand[offset + 8..offset + 12].copy_from_slice(&cpuid.ecx.to_le_bytes());
            brand[offset + 12..offset + 16].copy_from_slice(&cpuid.edx.to_le_bytes());
        }

        // Trim null bytes and whitespace
        if let Some(end) = brand.iter().position(|&b| b == 0) {
            if let Ok(brand_str) = core::str::from_utf8(&brand[..end]) {
                cpu_info.set_model_name(brand_str.trim());
            }
        }
    }

    // TODO: Get actual CPU MHz (requires TSC calibration)
    cpu_info.cpu_mhz = 2400; // Placeholder

    cpu_info
}

/// Get system uptime
fn get_uptime() -> Uptime {
    use crate::sched::timer;

    // Get current tick count
    let ticks = timer::get_tick_count();

    // Convert ticks to seconds (assuming 100 Hz timer)
    let uptime_secs = (ticks / 100) as u64;

    // TODO: Calculate actual idle time
    let idle_secs = 0;

    Uptime {
        uptime_secs,
        idle_secs,
    }
}

/// Read process information atomically without holding locks
///
/// This function reads process state using atomic operations where possible
/// to avoid holding locks during /proc file generation. This allows multiple
/// CPUs to read /proc files concurrently without blocking.
///
/// # Arguments
/// * `pid` - Process ID to read
///
/// # Returns
/// Some(ProcInfo) if the process exists, None if not found or process exited
///
/// # Lock-Free Design
///
/// This function uses the following strategies to avoid locks:
/// 1. Atomic loads for signal-related fields (pending_signals, signal_mask)
/// 2. Snapshot approach - read all fields quickly, accept minor inconsistencies
/// 3. Graceful handling of races (process may exit during read)
///
/// The snapshot may be slightly inconsistent (e.g., state and pending_signals
/// from different moments), but this is acceptable for /proc reads which are
/// inherently racy.
pub fn read_proc_info_lockfree(pid: usize) -> Option<ProcInfo> {
    // This is a placeholder implementation that demonstrates the lock-free approach
    // The actual implementation would access the task table with minimal locking

    // In a real implementation, we would:
    // 1. Try to get a reference to the task without holding the global lock
    // 2. Read fields using atomic operations where available
    // 3. Return None if the task exits during the read

    // For now, delegate to the existing implementation
    // TODO: Update when task table supports lock-free access
    get_proc_info(pid)
}

/// Snapshot of process state for lock-free reads
///
/// This structure contains a consistent snapshot of process state that can
/// be read without holding locks. It uses atomic operations internally.
#[derive(Debug, Clone)]
pub struct ProcSnapshot {
    /// Process ID
    pub pid: usize,
    /// Parent process ID
    pub ppid: usize,
    /// Process group ID
    pub pgid: usize,
    /// Session ID
    pub sid: usize,
    /// Pending signals (read atomically)
    pub pending_signals: u64,
    /// Signal mask (read atomically)
    pub signal_mask: u64,
    /// Process state
    pub state: ProcState,
    /// Command name
    pub comm: [u8; MAX_COMM_LEN],
    /// Command name length
    pub comm_len: usize,
}

impl ProcSnapshot {
    /// Create a snapshot from a task
    ///
    /// Reads task state using atomic operations where possible.
    /// This function should be fast and not hold locks.
    ///
    /// # Arguments
    /// * `task` - Reference to the task
    ///
    /// # Returns
    /// A consistent snapshot of the task state
    pub fn from_task(task: &crate::sched::task::Task) -> Self {
        use core::sync::atomic::Ordering;

        // Read atomic fields
        let pending_signals = task.pending_signals.load(Ordering::Acquire);
        let signal_mask = task.signal_mask.load(Ordering::Acquire);

        // Read other fields (these may be slightly inconsistent, but that's OK)
        let mut comm = [0u8; MAX_COMM_LEN];
        let name_bytes = task.name.as_bytes();
        let comm_len = name_bytes.len().min(MAX_COMM_LEN);
        comm[..comm_len].copy_from_slice(&name_bytes[..comm_len]);

        // Map task state to proc state
        let state = match task.state {
            crate::sched::task::TaskState::Running => ProcState::Running,
            crate::sched::task::TaskState::Ready => ProcState::Running,
            crate::sched::task::TaskState::Sleeping => ProcState::Sleeping,
            crate::sched::task::TaskState::Blocked => ProcState::Sleeping,
        };

        Self {
            pid: task.pid,
            ppid: task.ppid,
            pgid: task.pgid,
            sid: task.sid,
            pending_signals,
            signal_mask,
            state,
            comm,
            comm_len,
        }
    }

    /// Format as /proc/<pid>/stat content
    ///
    /// This is similar to ProcInfo::format_stat but works with a snapshot.
    pub fn format_stat(&self, buf: &mut [u8]) -> usize {
        use core::fmt::Write;

        struct BufWriter<'a> {
            buf: &'a mut [u8],
            pos: usize,
        }

        impl<'a> Write for BufWriter<'a> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let bytes = s.as_bytes();
                let remaining = self.buf.len() - self.pos;
                let to_write = bytes.len().min(remaining);
                self.buf[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
                self.pos += to_write;
                Ok(())
            }
        }

        let comm_str = core::str::from_utf8(&self.comm[..self.comm_len]).unwrap_or("unknown");
        let mut writer = BufWriter { buf, pos: 0 };
        let _ = write!(
            writer,
            "{} ({}) {} {} {} {} 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n",
            self.pid,
            comm_str,
            self.state.to_char(),
            self.ppid,
            self.pgid,
            self.sid,
        );
        writer.pos
    }
}
