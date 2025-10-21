//! /proc Virtual Filesystem
//!
//! This module implements a virtual filesystem that provides process and
//! system information through file-like interfaces.

/// Maximum command name length
const MAX_COMM_LEN: usize = 16;

/// Maximum command line arguments
const MAX_CMDLINE_ARGS: usize = 32;

/// Maximum argument length
const MAX_ARG_LEN: usize = 256;

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

        let vendor_id = core::str::from_utf8(&self.vendor_id[..self.vendor_id_len]).unwrap_or("unknown");
        let model_name = core::str::from_utf8(&self.model_name[..self.model_name_len]).unwrap_or("unknown");

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
