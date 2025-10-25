//! MelloFS RAM Inode Implementation

use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use crate::fs::vfs::inode::{Inode, FileMode, Stat, SetAttr, DirEnt, DirCookie};
use crate::fs::vfs::superblock::FsError;
use crate::sync::SpinLock;

/// RAM inode data protected by RwLock
pub struct InodeData {
    /// File mode (type and permissions)
    pub mode: FileMode,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Access time (Unix timestamp in nanoseconds)
    pub atime: u64,
    /// Modification time
    pub mtime: u64,
    /// Change time
    pub ctime: u64,
    /// Inode-specific data
    pub data: InodeKind,
}

/// Inode kind-specific data
pub enum InodeKind {
    File(FileData),
    Directory(DirectoryData),
    Symlink(SymlinkData),
}

/// File data
pub struct FileData {
    /// File chunks (each chunk is 16-64 KiB)
    pub chunks: Vec<Arc<[u8]>>,
    /// Chunk size (power of 2)
    pub chunk_size: usize,
}

/// Directory data
pub struct DirectoryData {
    /// Directory entries (name -> inode)
    pub entries: BTreeMap<String, Arc<RamInode>>,
}

/// Symlink data
pub struct SymlinkData {
    /// Target path
    pub target: String,
}

/// Global inode number allocator for RAM filesystem
static NEXT_INO: AtomicU64 = AtomicU64::new(2); // Start from 2 (1 is root)

/// RAM inode
pub struct RamInode {
    /// Inode number
    pub(crate) ino: u64,
    /// Number of hard links
    pub(crate) nlink: AtomicU32,
    /// File size in bytes
    pub(crate) size: AtomicU64,
    /// Inode data (protected by lock)
    pub(crate) data: SpinLock<InodeData>,
    /// Extended attributes
    pub(crate) xattrs: SpinLock<BTreeMap<String, Vec<u8>>>,
}

impl RamInode {
    /// Allocate a new inode number
    fn alloc_ino() -> u64 {
        NEXT_INO.fetch_add(1, Ordering::SeqCst)
    }
}

impl RamInode {
    /// Default chunk size (32 KiB)
    const DEFAULT_CHUNK_SIZE: usize = 32 * 1024;
    
    /// Create a new directory inode
    pub fn new_dir(ino: u64, mode: FileMode, uid: u32, gid: u32) -> Result<Arc<Self>, FsError> {
        let now = Self::current_time();
        
        let data = InodeData {
            mode,
            uid,
            gid,
            atime: now,
            mtime: now,
            ctime: now,
            data: InodeKind::Directory(DirectoryData {
                entries: BTreeMap::new(),
            }),
        };
        
        Ok(Arc::new(Self {
            ino,
            nlink: AtomicU32::new(2), // . and parent
            size: AtomicU64::new(0),
            data: SpinLock::new(data),
            xattrs: SpinLock::new(BTreeMap::new()),
        }))
    }
    
    /// Create a new file inode
    pub fn new_file(ino: u64, mode: FileMode, uid: u32, gid: u32) -> Result<Arc<Self>, FsError> {
        let now = Self::current_time();
        
        let data = InodeData {
            mode,
            uid,
            gid,
            atime: now,
            mtime: now,
            ctime: now,
            data: InodeKind::File(FileData {
                chunks: Vec::new(),
                chunk_size: Self::DEFAULT_CHUNK_SIZE,
            }),
        };
        
        Ok(Arc::new(Self {
            ino,
            nlink: AtomicU32::new(1),
            size: AtomicU64::new(0),
            data: SpinLock::new(data),
            xattrs: SpinLock::new(BTreeMap::new()),
        }))
    }
    
    /// Create a new symlink inode
    pub fn new_symlink(ino: u64, target: String, uid: u32, gid: u32) -> Result<Arc<Self>, FsError> {
        let now = Self::current_time();
        let mode = FileMode::new(FileMode::S_IFLNK | 0o777);
        
        let data = InodeData {
            mode,
            uid,
            gid,
            atime: now,
            mtime: now,
            ctime: now,
            data: InodeKind::Symlink(SymlinkData { target }),
        };
        
        Ok(Arc::new(Self {
            ino,
            nlink: AtomicU32::new(1),
            size: AtomicU64::new(0),
            data: SpinLock::new(data),
            xattrs: SpinLock::new(BTreeMap::new()),
        }))
    }
    
    /// Get current time (stub - returns 0 for now)
    pub(crate) fn current_time() -> u64 {
        // TODO: Implement proper time tracking
        0
    }
}

impl Inode for RamInode {
    fn ino(&self) -> u64 {
        self.ino
    }
    
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
    
    fn mode(&self) -> FileMode {
        self.data.lock().mode
    }
    
    fn nlink(&self) -> u32 {
        self.nlink.load(Ordering::Relaxed)
    }
    
    fn uid_gid(&self) -> (u32, u32) {
        let data = self.data.lock();
        (data.uid, data.gid)
    }
    
    fn size(&self) -> u64 {
        self.size.load(Ordering::Relaxed)
    }
    
    fn stat(&self) -> Result<Stat, FsError> {
        let data = self.data.lock();
        let size = self.size.load(Ordering::Relaxed);
        let nlink = self.nlink.load(Ordering::Relaxed);
        
        Ok(Stat {
            st_dev: 0, // TODO: Device ID
            st_ino: self.ino,
            st_mode: data.mode.0 as u32,
            st_nlink: nlink,
            st_uid: data.uid,
            st_gid: data.gid,
            st_rdev: 0,
            st_size: size,
            st_blksize: 4096,
            st_blocks: (size + 511) / 512,
            st_atime_sec: (data.atime / 1_000_000_000) as i64,
            st_atime_nsec: (data.atime % 1_000_000_000) as i64,
            st_mtime_sec: (data.mtime / 1_000_000_000) as i64,
            st_mtime_nsec: (data.mtime % 1_000_000_000) as i64,
            st_ctime_sec: (data.ctime / 1_000_000_000) as i64,
            st_ctime_nsec: (data.ctime % 1_000_000_000) as i64,
        })
    }
    
    fn set_attr(&self, _attr: SetAttr) -> Result<(), FsError> {
        // TODO: Implement attribute setting
        Err(FsError::NotSupported)
    }
    
    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, FsError> {
        self.dir_lookup(name)
    }
    
    fn create(&self, name: &str, mode: FileMode, uid: u32, gid: u32) 
        -> Result<Arc<dyn Inode>, FsError> {
        // Check if this is a directory
        let data = self.data.lock();
        if !data.mode.is_dir() {
            return Err(FsError::NotADirectory);
        }
        drop(data);
        
        // Check if entry already exists
        if self.lookup(name).is_ok() {
            return Err(FsError::AlreadyExists);
        }
        
        // Allocate new inode number
        let ino = Self::alloc_ino();
        
        // Create new inode based on mode
        let new_inode: Arc<RamInode> = if mode.is_dir() {
            Self::new_dir(ino, mode, uid, gid)?
        } else if mode.is_symlink() {
            // For symlink, we need target - return error for now
            return Err(FsError::NotSupported);
        } else {
            // Regular file
            Self::new_file(ino, mode, uid, gid)?
        };
        
        // Link the new inode into this directory using internal method
        // (avoids the downcast issue with dir_link)
        self.dir_link_internal(name, new_inode.clone())?;
        
        Ok(new_inode)
    }
    
    fn unlink(&self, name: &str) -> Result<(), FsError> {
        self.dir_unlink(name)
    }
    
    fn link(&self, name: &str, target: Arc<dyn Inode>) -> Result<(), FsError> {
        self.dir_link(name, target)
    }
    
    fn symlink(&self, name: &str, target: &str) -> Result<Arc<dyn Inode>, FsError> {
        // Validate this is a directory
        let data = self.data.lock();
        if !data.mode.is_dir() {
            return Err(FsError::NotADirectory);
        }
        drop(data);
        
        // Validate name
        if name.is_empty() || name == "." || name == ".." {
            return Err(FsError::InvalidArgument);
        }
        if name.contains('/') || name.contains('\0') {
            return Err(FsError::InvalidArgument);
        }
        if name.len() > 255 {
            return Err(FsError::NameTooLong);
        }
        
        // Check if entry already exists
        if self.lookup(name).is_ok() {
            return Err(FsError::AlreadyExists);
        }
        
        // Allocate new inode number
        let ino = Self::alloc_ino();
        
        // Create symlink inode
        let symlink_inode = Self::new_symlink(ino, String::from(target), 0, 0)?;
        
        // Link into directory using internal method
        self.dir_link_internal(name, symlink_inode.clone())?;
        
        Ok(symlink_inode)
    }
    
    fn readdir(&self, cookie: &mut DirCookie, entries: &mut Vec<DirEnt>) 
        -> Result<(), FsError> {
        self.dir_readdir(cookie, entries)
    }
    
    fn read_at(&self, off: u64, dst: &mut [u8]) -> Result<usize, FsError> {
        self.file_read_at(off, dst)
    }
    
    fn write_at(&self, off: u64, src: &[u8]) -> Result<usize, FsError> {
        self.file_write_at(off, src)
    }
    
    fn truncate(&self, new_size: u64) -> Result<(), FsError> {
        self.file_truncate(new_size)
    }
    
    fn readlink(&self) -> Result<String, FsError> {
        let data = self.data.lock();
        match &data.data {
            InodeKind::Symlink(symlink) => Ok(symlink.target.clone()),
            _ => Err(FsError::InvalidArgument),
        }
    }
    
    fn set_xattr(&self, name: &str, value: &[u8]) -> Result<(), FsError> {
        self.xattr_set(name, value)
    }
    
    fn get_xattr(&self, name: &str) -> Result<Vec<u8>, FsError> {
        self.xattr_get(name)
    }
    
    fn list_xattr(&self) -> Result<Vec<String>, FsError> {
        self.xattr_list()
    }
}
