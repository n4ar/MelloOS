//! Inode Trait and Types
//!
//! This module defines the core Inode trait that all filesystem implementations must provide,
//! along with associated types for file metadata, directory entries, and operations.

use crate::fs::vfs::superblock::FsError;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

/// Inode trait representing a filesystem object
pub trait Inode: Send + Sync {
    // Metadata operations

    /// Get inode number
    fn ino(&self) -> u64;

    /// Downcast to Any for type checking
    fn as_any(&self) -> &dyn core::any::Any;

    /// Get file mode (type and permissions)
    fn mode(&self) -> FileMode;

    /// Get number of hard links
    fn nlink(&self) -> u32;

    /// Get user and group IDs
    fn uid_gid(&self) -> (u32, u32);

    /// Get file size in bytes
    fn size(&self) -> u64;

    /// Get full stat information
    fn stat(&self) -> Result<Stat, FsError>;

    /// Set inode attributes
    fn set_attr(&self, attr: SetAttr) -> Result<(), FsError>;

    // Directory operations

    /// Look up a name in a directory
    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, FsError>;

    /// Create a new file in a directory
    fn create(
        &self,
        name: &str,
        mode: FileMode,
        uid: u32,
        gid: u32,
    ) -> Result<Arc<dyn Inode>, FsError>;

    /// Remove a file from a directory
    fn unlink(&self, name: &str) -> Result<(), FsError>;

    /// Create a hard link
    fn link(&self, name: &str, target: Arc<dyn Inode>) -> Result<(), FsError>;

    /// Create a symbolic link
    fn symlink(&self, name: &str, target: &str) -> Result<Arc<dyn Inode>, FsError>;

    /// Read directory entries
    fn readdir(&self, cookie: &mut DirCookie, entries: &mut Vec<DirEnt>) -> Result<(), FsError>;

    // File operations

    /// Read data from file at offset
    fn read_at(&self, off: u64, dst: &mut [u8]) -> Result<usize, FsError>;

    /// Write data to file at offset
    fn write_at(&self, off: u64, src: &[u8]) -> Result<usize, FsError>;

    /// Truncate file to new size
    fn truncate(&self, new_size: u64) -> Result<(), FsError>;

    /// Read symlink target
    fn readlink(&self) -> Result<String, FsError>;

    // Extended attributes

    /// Set extended attribute
    fn set_xattr(&self, name: &str, value: &[u8]) -> Result<(), FsError>;

    /// Get extended attribute
    fn get_xattr(&self, name: &str) -> Result<Vec<u8>, FsError>;

    /// List extended attribute names
    fn list_xattr(&self) -> Result<Vec<String>, FsError>;
}

/// File mode (type and permissions)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMode(pub u16);

impl FileMode {
    // File types
    pub const S_IFREG: u16 = 0o100000; // Regular file
    pub const S_IFDIR: u16 = 0o040000; // Directory
    pub const S_IFLNK: u16 = 0o120000; // Symbolic link
    pub const S_IFCHR: u16 = 0o020000; // Character device
    pub const S_IFBLK: u16 = 0o060000; // Block device
    pub const S_IFIFO: u16 = 0o010000; // FIFO
    pub const S_IFSOCK: u16 = 0o140000; // Socket
    pub const S_IFMT: u16 = 0o170000; // File type mask

    // Permissions
    pub const S_IRUSR: u16 = 0o0400;
    pub const S_IWUSR: u16 = 0o0200;
    pub const S_IXUSR: u16 = 0o0100;
    pub const S_IRGRP: u16 = 0o0040;
    pub const S_IWGRP: u16 = 0o0020;
    pub const S_IXGRP: u16 = 0o0010;
    pub const S_IROTH: u16 = 0o0004;
    pub const S_IWOTH: u16 = 0o0002;
    pub const S_IXOTH: u16 = 0o0001;

    // Special bits
    pub const S_ISUID: u16 = 0o4000;
    pub const S_ISGID: u16 = 0o2000;
    pub const S_ISVTX: u16 = 0o1000;

    pub fn new(mode: u16) -> Self {
        Self(mode)
    }

    pub fn file_type(&self) -> u16 {
        self.0 & Self::S_IFMT
    }

    pub fn permissions(&self) -> u16 {
        self.0 & 0o7777
    }

    pub fn is_dir(&self) -> bool {
        self.file_type() == Self::S_IFDIR
    }

    pub fn is_file(&self) -> bool {
        self.file_type() == Self::S_IFREG
    }

    pub fn is_symlink(&self) -> bool {
        self.file_type() == Self::S_IFLNK
    }
}

/// Stat structure (Linux-compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub st_size: u64,
    pub st_blksize: u32,
    pub st_blocks: u64,
    pub st_atime_sec: i64,
    pub st_atime_nsec: i64,
    pub st_mtime_sec: i64,
    pub st_mtime_nsec: i64,
    pub st_ctime_sec: i64,
    pub st_ctime_nsec: i64,
}

/// Attributes to set on an inode
#[derive(Debug, Clone, Copy, Default)]
pub struct SetAttr {
    pub mode: Option<FileMode>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub size: Option<u64>,
    pub atime: Option<u64>,
    pub mtime: Option<u64>,
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEnt {
    pub ino: u64,
    pub name: String,
    pub file_type: u8,
}

impl DirEnt {
    // File types for d_type field
    pub const DT_UNKNOWN: u8 = 0;
    pub const DT_FIFO: u8 = 1;
    pub const DT_CHR: u8 = 2;
    pub const DT_DIR: u8 = 4;
    pub const DT_BLK: u8 = 6;
    pub const DT_REG: u8 = 8;
    pub const DT_LNK: u8 = 10;
    pub const DT_SOCK: u8 = 12;
}

/// Directory iteration cookie
#[derive(Debug, Clone, Copy, Default)]
pub struct DirCookie {
    pub offset: u64,
}
