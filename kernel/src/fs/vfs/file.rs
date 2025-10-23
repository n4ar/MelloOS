//! File Descriptor Table
//!
//! This module implements per-process file descriptor tables with reference counting.
//! It handles FD allocation, flags (O_RDONLY, O_WRONLY, O_RDWR, O_APPEND, O_CLOEXEC),
//! and thread-safe offset tracking.

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::RwLock;
use super::inode::{Inode, FileMode, FsError, FsResult};

/// File descriptor flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileFlags(u32);

impl FileFlags {
    // Access modes (mutually exclusive)
    pub const O_RDONLY: u32 = 0x0000;
    pub const O_WRONLY: u32 = 0x0001;
    pub const O_RDWR: u32 = 0x0002;
    pub const O_ACCMODE: u32 = 0x0003;
    
    // File creation and status flags
    pub const O_CREAT: u32 = 0x0040;
    pub const O_EXCL: u32 = 0x0080;
    pub const O_NOCTTY: u32 = 0x0100;
    pub const O_TRUNC: u32 = 0x0200;
    pub const O_APPEND: u32 = 0x0400;
    pub const O_NONBLOCK: u32 = 0x0800;
    pub const O_DSYNC: u32 = 0x1000;
    pub const O_DIRECT: u32 = 0x4000;
    pub const O_LARGEFILE: u32 = 0x8000;
    pub const O_DIRECTORY: u32 = 0x10000;
    pub const O_NOFOLLOW: u32 = 0x20000;
    pub const O_NOATIME: u32 = 0x40000;
    pub const O_CLOEXEC: u32 = 0x80000;
    pub const O_SYNC: u32 = 0x101000;
    pub const O_PATH: u32 = 0x200000;
    pub const O_TMPFILE: u32 = 0x410000;
    
    pub const fn new(flags: u32) -> Self {
        Self(flags)
    }
    
    pub const fn empty() -> Self {
        Self(0)
    }
    
    pub const fn bits(&self) -> u32 {
        self.0
    }
    
    pub const fn contains(&self, flag: u32) -> bool {
        (self.0 & flag) != 0
    }
    
    pub fn insert(&mut self, flag: u32) {
        self.0 |= flag;
    }
    
    pub fn remove(&mut self, flag: u32) {
        self.0 &= !flag;
    }
    
    pub const fn access_mode(&self) -> u32 {
        self.0 & Self::O_ACCMODE
    }
    
    pub const fn is_readable(&self) -> bool {
        let mode = self.access_mode();
        mode == Self::O_RDONLY || mode == Self::O_RDWR
    }
    
    pub const fn is_writable(&self) -> bool {
        let mode = self.access_mode();
        mode == Self::O_WRONLY || mode == Self::O_RDWR
    }
    
    pub const fn is_append(&self) -> bool {
        self.contains(Self::O_APPEND)
    }
    
    pub const fn is_cloexec(&self) -> bool {
        self.contains(Self::O_CLOEXEC)
    }
}

/// File descriptor entry
pub struct FileDesc {
    /// The inode this FD refers to
    inode: Arc<dyn Inode>,
    /// Current file offset (for read/write)
    offset: AtomicU64,
    /// File flags
    flags: RwLock<FileFlags>,
}

impl FileDesc {
    /// Create a new file descriptor
    pub fn new(inode: Arc<dyn Inode>, flags: FileFlags) -> Self {
        Self {
            inode,
            offset: AtomicU64::new(0),
            flags: RwLock::new(flags),
        }
    }
    
    /// Get the inode
    pub fn inode(&self) -> &Arc<dyn Inode> {
        &self.inode
    }
    
    /// Get the current offset
    pub fn offset(&self) -> u64 {
        self.offset.load(Ordering::SeqCst)
    }
    
    /// Set the offset
    pub fn set_offset(&self, offset: u64) {
        self.offset.store(offset, Ordering::SeqCst);
    }
    
    /// Advance the offset by a delta
    pub fn advance_offset(&self, delta: u64) -> u64 {
        self.offset.fetch_add(delta, Ordering::SeqCst) + delta
    }
    
    /// Get the flags
    pub fn flags(&self) -> FileFlags {
        *self.flags.read()
    }
    
    /// Set the flags
    pub fn set_flags(&self, flags: FileFlags) {
        *self.flags.write() = flags;
    }
    
    /// Check if readable
    pub fn is_readable(&self) -> bool {
        self.flags.read().is_readable()
    }
    
    /// Check if writable
    pub fn is_writable(&self) -> bool {
        self.flags.read().is_writable()
    }
    
    /// Check if append mode
    pub fn is_append(&self) -> bool {
        self.flags.read().is_append()
    }
    
    /// Check if close-on-exec
    pub fn is_cloexec(&self) -> bool {
        self.flags.read().is_cloexec()
    }
    
    /// Read from the file
    pub fn read(&self, buf: &mut [u8]) -> FsResult<usize> {
        if !self.is_readable() {
            return Err(FsError::PermissionDenied);
        }
        
        let offset = self.offset();
        let n = self.inode.read_at(offset, buf)?;
        self.advance_offset(n as u64);
        Ok(n)
    }
    
    /// Write to the file
    pub fn write(&self, buf: &[u8]) -> FsResult<usize> {
        if !self.is_writable() {
            return Err(FsError::PermissionDenied);
        }
        
        let offset = if self.is_append() {
            // Append mode: always write at end
            self.inode.size()
        } else {
            self.offset()
        };
        
        let n = self.inode.write_at(offset, buf)?;
        
        if !self.is_append() {
            self.advance_offset(n as u64);
        }
        
        Ok(n)
    }
    
    /// Read at a specific offset (pread)
    pub fn pread(&self, offset: u64, buf: &mut [u8]) -> FsResult<usize> {
        if !self.is_readable() {
            return Err(FsError::PermissionDenied);
        }
        
        self.inode.read_at(offset, buf)
    }
    
    /// Write at a specific offset (pwrite)
    pub fn pwrite(&self, offset: u64, buf: &[u8]) -> FsResult<usize> {
        if !self.is_writable() {
            return Err(FsError::PermissionDenied);
        }
        
        self.inode.write_at(offset, buf)
    }
    
    /// Seek to a new offset
    pub fn seek(&self, offset: i64, whence: SeekWhence) -> FsResult<u64> {
        let new_offset = match whence {
            SeekWhence::Set => {
                if offset < 0 {
                    return Err(FsError::InvalidArgument);
                }
                offset as u64
            }
            SeekWhence::Cur => {
                let current = self.offset() as i64;
                let result = current.checked_add(offset).ok_or(FsError::InvalidSeek)?;
                if result < 0 {
                    return Err(FsError::InvalidArgument);
                }
                result as u64
            }
            SeekWhence::End => {
                let size = self.inode.size() as i64;
                let result = size.checked_add(offset).ok_or(FsError::InvalidSeek)?;
                if result < 0 {
                    return Err(FsError::InvalidArgument);
                }
                result as u64
            }
        };
        
        self.set_offset(new_offset);
        Ok(new_offset)
    }
}

/// Seek whence parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekWhence {
    /// Seek from beginning of file
    Set,
    /// Seek from current position
    Cur,
    /// Seek from end of file
    End,
}

/// File descriptor table
pub struct FileDescTable {
    /// Array of file descriptors (None = closed)
    fds: RwLock<Vec<Option<Arc<FileDesc>>>>,
    /// Maximum number of file descriptors
    max_fds: usize,
}

impl FileDescTable {
    /// Create a new file descriptor table
    pub fn new(max_fds: usize) -> Self {
        Self {
            fds: RwLock::new(Vec::new()),
            max_fds,
        }
    }
    
    /// Allocate a new file descriptor
    ///
    /// Returns the lowest available FD number.
    pub fn alloc(&self, file: Arc<FileDesc>) -> FsResult<usize> {
        let mut fds = self.fds.write();
        
        // Find the lowest available FD
        for (i, slot) in fds.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(file);
                return Ok(i);
            }
        }
        
        // No free slot found, append if under limit
        if fds.len() < self.max_fds {
            let fd = fds.len();
            fds.push(Some(file));
            Ok(fd)
        } else {
            Err(FsError::TooManyOpenFiles)
        }
    }
    
    /// Get a file descriptor
    pub fn get(&self, fd: usize) -> FsResult<Arc<FileDesc>> {
        let fds = self.fds.read();
        fds.get(fd)
            .and_then(|slot| slot.as_ref())
            .map(Arc::clone)
            .ok_or(FsError::InvalidArgument)
    }
    
    /// Close a file descriptor
    pub fn close(&self, fd: usize) -> FsResult<()> {
        let mut fds = self.fds.write();
        if fd < fds.len() && fds[fd].is_some() {
            fds[fd] = None;
            Ok(())
        } else {
            Err(FsError::InvalidArgument)
        }
    }
    
    /// Duplicate a file descriptor
    pub fn dup(&self, old_fd: usize) -> FsResult<usize> {
        let file = self.get(old_fd)?;
        self.alloc(file)
    }
    
    /// Duplicate a file descriptor to a specific FD number
    pub fn dup2(&self, old_fd: usize, new_fd: usize) -> FsResult<usize> {
        if old_fd == new_fd {
            // Check that old_fd is valid
            let _ = self.get(old_fd)?;
            return Ok(new_fd);
        }
        
        let file = self.get(old_fd)?;
        
        let mut fds = self.fds.write();
        
        // Ensure the vector is large enough
        if new_fd >= fds.len() {
            if new_fd >= self.max_fds {
                return Err(FsError::TooManyOpenFiles);
            }
            fds.resize(new_fd + 1, None);
        }
        
        // Close the old FD at new_fd if it exists
        fds[new_fd] = Some(file);
        
        Ok(new_fd)
    }
    
    /// Clone the file descriptor table for fork
    ///
    /// Filters out CLOEXEC file descriptors.
    pub fn clone_for_fork(&self) -> Self {
        let fds = self.fds.read();
        let new_fds: Vec<Option<Arc<FileDesc>>> = fds
            .iter()
            .map(|slot| {
                slot.as_ref().and_then(|file| {
                    if file.is_cloexec() {
                        None
                    } else {
                        Some(Arc::clone(file))
                    }
                })
            })
            .collect();
        
        Self {
            fds: RwLock::new(new_fds),
            max_fds: self.max_fds,
        }
    }
    
    /// Close all file descriptors marked as CLOEXEC
    pub fn close_cloexec(&self) {
        let mut fds = self.fds.write();
        for slot in fds.iter_mut() {
            if let Some(file) = slot {
                if file.is_cloexec() {
                    *slot = None;
                }
            }
        }
    }
    
    /// Get the number of open file descriptors
    pub fn count(&self) -> usize {
        self.fds.read().iter().filter(|slot| slot.is_some()).count()
    }
    
    /// Close all file descriptors
    pub fn close_all(&self) {
        let mut fds = self.fds.write();
        fds.clear();
    }
}
