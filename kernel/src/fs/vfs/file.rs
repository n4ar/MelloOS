//! File Descriptor Table
//!
//! This module implements per-process file descriptor tables with reference counting.
//! It handles FD allocation, flags (O_RDONLY, O_WRONLY, O_RDWR, O_APPEND, O_CLOEXEC),
//! and thread-safe offset tracking.

#![allow(dead_code)] // Methods will be used when syscalls are wired up in Task 8.8

use crate::fs::vfs::inode::Inode;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};

/// Maximum number of file descriptors per process
pub const MAX_FDS: usize = 256;

/// File descriptor flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FdFlags {
    bits: u32,
}

impl FdFlags {
    pub const O_RDONLY: u32 = 0x0000;
    pub const O_WRONLY: u32 = 0x0001;
    pub const O_RDWR: u32 = 0x0002;
    pub const O_APPEND: u32 = 0x0008;
    pub const O_CLOEXEC: u32 = 0x0080;

    pub const fn new(bits: u32) -> Self {
        Self { bits }
    }

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn bits(&self) -> u32 {
        self.bits
    }

    pub fn is_readable(&self) -> bool {
        (self.bits & 0x3) == Self::O_RDONLY || (self.bits & 0x3) == Self::O_RDWR
    }

    pub fn is_writable(&self) -> bool {
        (self.bits & 0x3) == Self::O_WRONLY || (self.bits & 0x3) == Self::O_RDWR
    }

    pub fn is_append(&self) -> bool {
        (self.bits & Self::O_APPEND) != 0
    }

    pub fn is_cloexec(&self) -> bool {
        (self.bits & Self::O_CLOEXEC) != 0
    }
}

/// File descriptor entry
#[derive(Clone)]
pub struct FileDescriptor {
    /// Inode reference
    pub inode: Arc<dyn Inode>,
    /// Current file offset (atomic for thread-safety)
    pub offset: Arc<AtomicU64>,
    /// File descriptor flags
    pub flags: FdFlags,
}

impl core::fmt::Debug for FileDescriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FileDescriptor")
            .field("ino", &self.inode.ino())
            .field("offset", &self.offset.load(Ordering::SeqCst))
            .field("flags", &self.flags)
            .finish()
    }
}

impl FileDescriptor {
    /// Create a new file descriptor
    pub fn new(inode: Arc<dyn Inode>, flags: FdFlags) -> Self {
        Self {
            inode,
            offset: Arc::new(AtomicU64::new(0)),
            flags,
        }
    }

    /// Get current offset
    pub fn get_offset(&self) -> u64 {
        self.offset.load(Ordering::SeqCst)
    }

    /// Set offset
    pub fn set_offset(&self, offset: u64) {
        self.offset.store(offset, Ordering::SeqCst);
    }

    /// Advance offset by delta and return old offset
    pub fn advance_offset(&self, delta: u64) -> u64 {
        self.offset.fetch_add(delta, Ordering::SeqCst)
    }
}

/// File descriptor table for a process
#[derive(Debug)]
pub struct FdTable {
    /// Array of file descriptors
    fds: [Option<FileDescriptor>; MAX_FDS],
}

impl FdTable {
    /// Create a new empty FD table
    pub fn new() -> Self {
        Self {
            fds: [const { None }; MAX_FDS],
        }
    }

    /// Allocate a new file descriptor (find lowest available)
    ///
    /// # Arguments
    /// * `fd` - File descriptor to allocate
    ///
    /// # Returns
    /// The allocated FD number, or None if table is full
    pub fn alloc_fd(&mut self, fd: FileDescriptor) -> Option<usize> {
        for (i, slot) in self.fds.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(fd);
                return Some(i);
            }
        }
        None
    }

    /// Allocate a specific FD number
    ///
    /// # Arguments
    /// * `fd_num` - FD number to allocate
    /// * `fd` - File descriptor to allocate
    ///
    /// # Returns
    /// Ok(()) if successful, Err if FD already in use
    pub fn alloc_fd_at(&mut self, fd_num: usize, fd: FileDescriptor) -> Result<(), &'static str> {
        if fd_num >= MAX_FDS {
            return Err("FD number out of range");
        }

        if self.fds[fd_num].is_some() {
            return Err("FD already in use");
        }

        self.fds[fd_num] = Some(fd);
        Ok(())
    }

    /// Get a file descriptor by number
    ///
    /// # Arguments
    /// * `fd_num` - FD number to get
    ///
    /// # Returns
    /// Clone of the file descriptor, or None if not found
    pub fn get_fd(&self, fd_num: usize) -> Option<FileDescriptor> {
        if fd_num >= MAX_FDS {
            return None;
        }
        self.fds[fd_num].clone()
    }

    /// Close a file descriptor
    ///
    /// # Arguments
    /// * `fd_num` - FD number to close
    ///
    /// # Returns
    /// Ok(()) if successful, Err if FD not found
    pub fn close_fd(&mut self, fd_num: usize) -> Result<(), &'static str> {
        if fd_num >= MAX_FDS {
            return Err("FD number out of range");
        }

        if self.fds[fd_num].is_none() {
            return Err("FD not open");
        }

        self.fds[fd_num] = None;
        Ok(())
    }

    /// Clone FD table for fork(), filtering out CLOEXEC FDs
    ///
    /// # Returns
    /// A new FD table with all non-CLOEXEC FDs cloned
    pub fn clone_for_fork(&self) -> Self {
        let mut new_table = Self::new();

        for (i, fd) in self.fds.iter().enumerate() {
            if let Some(fd) = fd {
                // Don't clone CLOEXEC FDs
                if !fd.flags.is_cloexec() {
                    new_table.fds[i] = Some(fd.clone());
                }
            }
        }

        new_table
    }

    /// Close all CLOEXEC FDs for exec()
    pub fn close_cloexec_fds(&mut self) {
        for fd in self.fds.iter_mut() {
            if let Some(descriptor) = fd {
                if descriptor.flags.is_cloexec() {
                    *fd = None;
                }
            }
        }
    }

    /// Get count of open FDs
    pub fn count(&self) -> usize {
        self.fds.iter().filter(|fd| fd.is_some()).count()
    }
}
