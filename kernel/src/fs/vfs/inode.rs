//! Inode Trait and Types
//!
//! This module defines the core Inode trait that all filesystem implementations must provide,
//! along with associated types for file metadata, directory entries, and operations.

use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// File mode bits following POSIX specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct FileMode(pub u16);

impl FileMode {
    // File types
    pub const S_IFMT: u16 = 0o170000;   // File type mask
    pub const S_IFREG: u16 = 0o100000;  // Regular file
    pub const S_IFDIR: u16 = 0o040000;  // Directory
    pub const S_IFLNK: u16 = 0o120000;  // Symbolic link
    pub const S_IFCHR: u16 = 0o020000;  // Character device
    pub const S_IFBLK: u16 = 0o060000;  // Block device
    pub const S_IFIFO: u16 = 0o010000;  // FIFO
    pub const S_IFSOCK: u16 = 0o140000; // Socket
    
    // Permissions
    pub const S_IRUSR: u16 = 0o0400;    // User read
    pub const S_IWUSR: u16 = 0o0200;    // User write
    pub const S_IXUSR: u16 = 0o0100;    // User execute
    pub const S_IRGRP: u16 = 0o0040;    // Group read
    pub const S_IWGRP: u16 = 0o0020;    // Group write
    pub const S_IXGRP: u16 = 0o0010;    // Group execute
    pub const S_IROTH: u16 = 0o0004;    // Other read
    pub const S_IWOTH: u16 = 0o0002;    // Other write
    pub const S_IXOTH: u16 = 0o0001;    // Other execute
    
    // Special bits
    pub const S_ISUID: u16 = 0o4000;    // Set UID
    pub const S_ISGID: u16 = 0o2000;    // Set GID
    pub const S_ISVTX: u16 = 0o1000;    // Sticky bit
    
    /// Create a new FileMode
    pub const fn new(mode: u16) -> Self {
        Self(mode)
    }
    
    /// Get the file type
    pub const fn file_type(&self) -> u16 {
        self.0 & Self::S_IFMT
    }
    
    /// Get the permission bits
    pub const fn permissions(&self) -> u16 {
        self.0 & 0o7777
    }
    
    /// Check if this is a regular file
    pub const fn is_regular(&self) -> bool {
        self.file_type() == Self::S_IFREG
    }
    
    /// Check if this is a directory
    pub const fn is_directory(&self) -> bool {
        self.file_type() == Self::S_IFDIR
    }
    
    /// Check if this is a symbolic link
    pub const fn is_symlink(&self) -> bool {
        self.file_type() == Self::S_IFLNK
    }
    
    /// Check if this is a character device
    pub const fn is_char_device(&self) -> bool {
        self.file_type() == Self::S_IFCHR
    }
    
    /// Check if this is a block device
    pub const fn is_block_device(&self) -> bool {
        self.file_type() == Self::S_IFBLK
    }
    
    /// Check if this is a FIFO
    pub const fn is_fifo(&self) -> bool {
        self.file_type() == Self::S_IFIFO
    }
    
    /// Check if this is a socket
    pub const fn is_socket(&self) -> bool {
        self.file_type() == Self::S_IFSOCK
    }
}

/// Linux-compatible stat structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Stat {
    pub st_dev: u64,         // Device ID (filesystem ID)
    pub st_ino: u64,         // Inode number
    pub st_mode: u32,        // File type and mode
    pub st_nlink: u32,       // Number of hard links
    pub st_uid: u32,         // User ID
    pub st_gid: u32,         // Group ID
    pub st_rdev: u64,        // Device ID (if special file)
    pub st_size: u64,        // Total size in bytes
    pub st_blksize: u32,     // Block size for I/O
    pub st_blocks: u64,      // Number of 512B blocks
    pub st_atime_sec: i64,   // Access time seconds
    pub st_atime_nsec: i64,  // Access time nanoseconds
    pub st_mtime_sec: i64,   // Modification time seconds
    pub st_mtime_nsec: i64,  // Modification time nanoseconds
    pub st_ctime_sec: i64,   // Status change time seconds
    pub st_ctime_nsec: i64,  // Status change time nanoseconds
}

/// Directory entry for getdents64
#[derive(Debug, Clone)]
pub struct DirEnt {
    pub ino: u64,            // Inode number
    pub off: i64,            // Offset to next entry
    pub name: String,        // Entry name
    pub file_type: u8,       // File type (DT_*)
}

impl DirEnt {
    // File type constants for d_type field
    pub const DT_UNKNOWN: u8 = 0;
    pub const DT_FIFO: u8 = 1;
    pub const DT_CHR: u8 = 2;
    pub const DT_DIR: u8 = 4;
    pub const DT_BLK: u8 = 6;
    pub const DT_REG: u8 = 8;
    pub const DT_LNK: u8 = 10;
    pub const DT_SOCK: u8 = 12;
    
    /// Convert FileMode to d_type
    pub fn file_type_from_mode(mode: FileMode) -> u8 {
        match mode.file_type() {
            FileMode::S_IFREG => Self::DT_REG,
            FileMode::S_IFDIR => Self::DT_DIR,
            FileMode::S_IFLNK => Self::DT_LNK,
            FileMode::S_IFCHR => Self::DT_CHR,
            FileMode::S_IFBLK => Self::DT_BLK,
            FileMode::S_IFIFO => Self::DT_FIFO,
            FileMode::S_IFSOCK => Self::DT_SOCK,
            _ => Self::DT_UNKNOWN,
        }
    }
}

/// Directory iteration cookie
#[derive(Debug, Clone, Copy, Default)]
pub struct DirCookie {
    pub offset: u64,
}

/// Attributes to set on an inode
#[derive(Debug, Clone, Default)]
pub struct SetAttr {
    pub mode: Option<u16>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub size: Option<u64>,
    pub atime_sec: Option<i64>,
    pub atime_nsec: Option<i64>,
    pub mtime_sec: Option<i64>,
    pub mtime_nsec: Option<i64>,
}

/// Result type for filesystem operations
pub type FsResult<T> = Result<T, FsError>;

/// Filesystem error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    /// Invalid argument
    InvalidArgument,
    /// No such file or directory
    NotFound,
    /// File exists
    AlreadyExists,
    /// Permission denied
    PermissionDenied,
    /// Not a directory
    NotDirectory,
    /// Is a directory
    IsDirectory,
    /// No space left on device
    NoSpace,
    /// I/O error
    IoError,
    /// Out of memory
    OutOfMemory,
    /// Bad address (invalid userspace pointer)
    BadAddress,
    /// Too many symbolic links
    TooManySymlinks,
    /// Name too long
    NameTooLong,
    /// Read-only filesystem
    ReadOnly,
    /// Too many open files (per-process)
    TooManyOpenFiles,
    /// Too many open files (system-wide)
    TooManyOpenFilesSystem,
    /// Directory not empty
    DirectoryNotEmpty,
    /// Cross-device link
    CrossDevice,
    /// Invalid seek
    InvalidSeek,
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "Invalid argument"),
            Self::NotFound => write!(f, "No such file or directory"),
            Self::AlreadyExists => write!(f, "File exists"),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::NotDirectory => write!(f, "Not a directory"),
            Self::IsDirectory => write!(f, "Is a directory"),
            Self::NoSpace => write!(f, "No space left on device"),
            Self::IoError => write!(f, "I/O error"),
            Self::OutOfMemory => write!(f, "Out of memory"),
            Self::BadAddress => write!(f, "Bad address"),
            Self::TooManySymlinks => write!(f, "Too many symbolic links"),
            Self::NameTooLong => write!(f, "Name too long"),
            Self::ReadOnly => write!(f, "Read-only filesystem"),
            Self::TooManyOpenFiles => write!(f, "Too many open files"),
            Self::TooManyOpenFilesSystem => write!(f, "Too many open files (system)"),
            Self::DirectoryNotEmpty => write!(f, "Directory not empty"),
            Self::CrossDevice => write!(f, "Cross-device link"),
            Self::InvalidSeek => write!(f, "Invalid seek"),
        }
    }
}

/// Inode trait - represents a filesystem object
///
/// All filesystem implementations must provide this trait for their inodes.
/// The trait is Send + Sync to ensure SMP safety.
pub trait Inode: Send + Sync {
    // Metadata operations
    
    /// Get the inode number
    fn ino(&self) -> u64;
    
    /// Get the file mode (type and permissions)
    fn mode(&self) -> FileMode;
    
    /// Get the number of hard links
    fn nlink(&self) -> u32;
    
    /// Get the user and group IDs
    fn uid_gid(&self) -> (u32, u32);
    
    /// Get the file size in bytes
    fn size(&self) -> u64;
    
    /// Get full stat information
    fn stat(&self) -> FsResult<Stat>;
    
    /// Set attributes on the inode
    fn set_attr(&self, attr: &SetAttr) -> FsResult<()>;
    
    // Directory operations
    
    /// Look up a name in a directory
    /// Returns the child inode if found
    fn lookup(&self, name: &str) -> FsResult<Arc<dyn Inode>>;
    
    /// Create a new file or directory entry
    fn create(&self, name: &str, mode: FileMode, uid: u32, gid: u32) -> FsResult<Arc<dyn Inode>>;
    
    /// Remove a directory entry
    fn unlink(&self, name: &str) -> FsResult<()>;
    
    /// Create a hard link
    fn link(&self, name: &str, target: Arc<dyn Inode>) -> FsResult<()>;
    
    /// Create a symbolic link
    fn symlink(&self, name: &str, target: &str) -> FsResult<Arc<dyn Inode>>;
    
    /// Read directory entries
    /// The sink function is called for each entry
    fn readdir(&self, cookie: &mut DirCookie, sink: &mut dyn FnMut(DirEnt)) -> FsResult<()>;
    
    // File operations
    
    /// Read data from the file at the given offset
    fn read_at(&self, off: u64, dst: &mut [u8]) -> FsResult<usize>;
    
    /// Write data to the file at the given offset
    fn write_at(&self, off: u64, src: &[u8]) -> FsResult<usize>;
    
    /// Truncate the file to the given size
    fn truncate(&self, new_size: u64) -> FsResult<()>;
    
    /// Read the target of a symbolic link
    fn readlink(&self) -> FsResult<String>;
    
    // Extended attributes
    
    /// Set an extended attribute
    fn set_xattr(&self, name: &str, value: &[u8]) -> FsResult<()>;
    
    /// Get an extended attribute
    fn get_xattr(&self, name: &str, out: &mut [u8]) -> FsResult<usize>;
    
    /// List extended attribute names
    fn list_xattr(&self) -> FsResult<Vec<String>>;
}


// Inode Cache Implementation

extern crate alloc;

use alloc::collections::BTreeMap;
use spin::{RwLock, Once};

/// Maximum number of inodes in the cache
const INODE_CACHE_SIZE: usize = 8192;

/// Number of hash buckets for fine-grained locking
const INODE_HASH_BUCKETS: usize = 256;

/// Inode cache key
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct InodeCacheKey {
    sb_id: u64,
    ino: u64,
}

impl InodeCacheKey {
    fn new(sb_id: u64, ino: u64) -> Self {
        Self { sb_id, ino }
    }
    
    fn bucket_index(&self) -> usize {
        // Mix sb_id and ino for better distribution
        let hash = self.sb_id.wrapping_mul(31).wrapping_add(self.ino);
        (hash as usize) % INODE_HASH_BUCKETS
    }
}

/// Inode cache entry
struct InodeCacheEntry {
    inode: Arc<dyn Inode>,
    dirty: bool,
}

/// Hash bucket for inode cache
struct InodeBucket {
    entries: BTreeMap<InodeCacheKey, InodeCacheEntry>,
}

impl InodeBucket {
    fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
}

/// Global inode cache
pub struct InodeCache {
    buckets: Vec<RwLock<InodeBucket>>,
    size: RwLock<usize>,
}

impl InodeCache {
    /// Create a new inode cache
    pub fn new() -> Self {
        let mut buckets = Vec::with_capacity(INODE_HASH_BUCKETS);
        for _ in 0..INODE_HASH_BUCKETS {
            buckets.push(RwLock::new(InodeBucket::new()));
        }
        
        Self {
            buckets,
            size: RwLock::new(0),
        }
    }
    
    /// Look up an inode in the cache
    pub fn lookup(&self, sb_id: u64, ino: u64) -> Option<Arc<dyn Inode>> {
        let key = InodeCacheKey::new(sb_id, ino);
        let bucket_index = key.bucket_index();
        let bucket = self.buckets[bucket_index].read();
        
        bucket.entries.get(&key).map(|entry| Arc::clone(&entry.inode))
    }
    
    /// Insert an inode into the cache
    ///
    /// If the inode already exists, returns the existing inode.
    /// Otherwise, inserts the new inode and returns it.
    pub fn insert(&self, sb_id: u64, ino: u64, inode: Arc<dyn Inode>) -> Arc<dyn Inode> {
        let key = InodeCacheKey::new(sb_id, ino);
        let bucket_index = key.bucket_index();
        let mut bucket = self.buckets[bucket_index].write();
        
        // Check if already exists
        if let Some(entry) = bucket.entries.get(&key) {
            return Arc::clone(&entry.inode);
        }
        
        // Check if we need to evict
        let size = *self.size.read();
        if size >= INODE_CACHE_SIZE {
            drop(bucket);
            self.evict_one();
            bucket = self.buckets[bucket_index].write();
        }
        
        // Insert new entry
        bucket.entries.insert(
            key,
            InodeCacheEntry {
                inode: Arc::clone(&inode),
                dirty: false,
            },
        );
        
        *self.size.write() += 1;
        inode
    }
    
    /// Mark an inode as dirty (needs writeback)
    pub fn mark_dirty(&self, sb_id: u64, ino: u64) {
        let key = InodeCacheKey::new(sb_id, ino);
        let bucket_index = key.bucket_index();
        let mut bucket = self.buckets[bucket_index].write();
        
        if let Some(entry) = bucket.entries.get_mut(&key) {
            entry.dirty = true;
        }
    }
    
    /// Mark an inode as clean (writeback complete)
    pub fn mark_clean(&self, sb_id: u64, ino: u64) {
        let key = InodeCacheKey::new(sb_id, ino);
        let bucket_index = key.bucket_index();
        let mut bucket = self.buckets[bucket_index].write();
        
        if let Some(entry) = bucket.entries.get_mut(&key) {
            entry.dirty = false;
        }
    }
    
    /// Get all dirty inodes for a filesystem
    pub fn get_dirty_inodes(&self, sb_id: u64) -> Vec<Arc<dyn Inode>> {
        let mut dirty_inodes = Vec::new();
        
        for bucket_lock in &self.buckets {
            let bucket = bucket_lock.read();
            for (key, entry) in &bucket.entries {
                if key.sb_id == sb_id && entry.dirty {
                    dirty_inodes.push(Arc::clone(&entry.inode));
                }
            }
        }
        
        dirty_inodes
    }
    
    /// Remove an inode from the cache
    pub fn remove(&self, sb_id: u64, ino: u64) {
        let key = InodeCacheKey::new(sb_id, ino);
        let bucket_index = key.bucket_index();
        let mut bucket = self.buckets[bucket_index].write();
        
        if bucket.entries.remove(&key).is_some() {
            *self.size.write() -= 1;
        }
    }
    
    /// Clear all inodes for a filesystem
    pub fn clear_filesystem(&self, sb_id: u64) {
        for bucket_lock in &self.buckets {
            let mut bucket = bucket_lock.write();
            let keys_to_remove: Vec<_> = bucket
                .entries
                .iter()
                .filter(|(k, _)| k.sb_id == sb_id)
                .map(|(k, _)| *k)
                .collect();
            
            for key in keys_to_remove {
                bucket.entries.remove(&key);
                *self.size.write() -= 1;
            }
        }
    }
    
    /// Clear the entire cache
    pub fn clear(&self) {
        for bucket_lock in &self.buckets {
            let mut bucket = bucket_lock.write();
            bucket.entries.clear();
        }
        *self.size.write() = 0;
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> InodeCacheStats {
        let size = *self.size.read();
        let mut dirty_count = 0;
        
        for bucket_lock in &self.buckets {
            let bucket = bucket_lock.read();
            dirty_count += bucket.entries.values().filter(|e| e.dirty).count();
        }
        
        InodeCacheStats {
            size,
            capacity: INODE_CACHE_SIZE,
            dirty_count,
        }
    }
    
    // Private helper methods
    
    fn evict_one(&self) {
        // Simple eviction: find first non-dirty inode with refcount == 1
        // (only held by cache)
        for bucket_lock in &self.buckets {
            let mut bucket = bucket_lock.write();
            
            // Find a candidate for eviction
            let key_to_remove = bucket
                .entries
                .iter()
                .find(|(_, entry)| {
                    !entry.dirty && Arc::strong_count(&entry.inode) == 1
                })
                .map(|(k, _)| *k);
            
            if let Some(key) = key_to_remove {
                bucket.entries.remove(&key);
                *self.size.write() -= 1;
                return;
            }
        }
        
        // If no clean inode found, we're at capacity but can't evict
        // This is acceptable - the cache will remain at capacity
    }
}

/// Inode cache statistics
#[derive(Debug, Clone, Copy)]
pub struct InodeCacheStats {
    pub size: usize,
    pub capacity: usize,
    pub dirty_count: usize,
}

/// Global inode cache instance
static INODE_CACHE: Once<InodeCache> = Once::new();

/// Get the global inode cache
pub fn inode_cache() -> &'static InodeCache {
    INODE_CACHE.call_once(|| InodeCache::new())
}
