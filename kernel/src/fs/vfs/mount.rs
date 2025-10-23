//! Mount Table
//!
//! This module implements the global mount point table and mount/umount operations.
//! It tracks all mounted filesystems and provides mount point lookup during path resolution.

extern crate alloc;

use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::{RwLock, Once};
use super::inode::{Inode, FsError, FsResult};
use super::superblock::{SuperBlock, FsType, MountOpts, MountFlags};
use super::dentry::dentry_cache;

/// Mount point entry
#[derive(Clone)]
pub struct MountPoint {
    /// Mount path (e.g., "/", "/mnt/disk")
    pub path: String,
    /// Superblock of the mounted filesystem
    pub sb: Arc<dyn SuperBlock>,
    /// Mount flags
    pub flags: MountFlags,
    /// Mount ID (unique identifier)
    pub mount_id: u64,
}

/// Global mount table
pub struct MountTable {
    /// Map from mount path to mount point
    mounts: RwLock<BTreeMap<String, MountPoint>>,
    /// Next mount ID
    next_mount_id: RwLock<u64>,
    /// Registered filesystem types
    fs_types: RwLock<BTreeMap<String, Arc<dyn FsType>>>,
}

impl MountTable {
    /// Create a new mount table
    pub fn new() -> Self {
        Self {
            mounts: RwLock::new(BTreeMap::new()),
            next_mount_id: RwLock::new(1),
            fs_types: RwLock::new(BTreeMap::new()),
        }
    }
    
    /// Register a filesystem type
    pub fn register_fs_type(&self, fs_type: Arc<dyn FsType>) {
        let name = fs_type.name().to_string();
        self.fs_types.write().insert(name, fs_type);
    }
    
    /// Get a registered filesystem type
    pub fn get_fs_type(&self, name: &str) -> Option<Arc<dyn FsType>> {
        self.fs_types.read().get(name).cloned()
    }
    
    /// Mount a filesystem
    ///
    /// # Arguments
    /// * `fs_type_name` - Name of the filesystem type (e.g., "mfs_ram", "mfs_disk")
    /// * `dev` - Optional block device for persistent filesystems
    /// * `path` - Mount point path
    /// * `opts` - Mount options
    pub fn mount(
        &self,
        fs_type_name: &str,
        dev: Option<Arc<dyn crate::drivers::block::BlockDevice>>,
        path: String,
        opts: MountOpts,
    ) -> FsResult<u64> {
        // Get filesystem type
        let fs_type = self
            .get_fs_type(fs_type_name)
            .ok_or(FsError::InvalidArgument)?;
        
        // Check if already mounted
        {
            let mounts = self.mounts.read();
            if mounts.contains_key(&path) {
                return Err(FsError::AlreadyExists);
            }
        }
        
        // Create superblock
        let sb = fs_type.mount(dev, opts.clone())?;
        
        // Allocate mount ID
        let mount_id = {
            let mut next_id = self.next_mount_id.write();
            let id = *next_id;
            *next_id += 1;
            id
        };
        
        // Create mount point
        let mount_point = MountPoint {
            path: path.clone(),
            sb,
            flags: opts.flags,
            mount_id,
        };
        
        // Add to mount table
        self.mounts.write().insert(path.clone(), mount_point);
        
        // Invalidate dentry cache for the mount point
        // (so subsequent lookups will see the new filesystem)
        dentry_cache().clear();
        
        Ok(mount_id)
    }
    
    /// Unmount a filesystem
    ///
    /// # Arguments
    /// * `path` - Mount point path
    pub fn umount(&self, path: &str) -> FsResult<()> {
        // Remove from mount table
        let mount_point = {
            let mut mounts = self.mounts.write();
            mounts.remove(path).ok_or(FsError::NotFound)?
        };
        
        // Sync the filesystem
        mount_point.sb.sync()?;
        
        // Invalidate dentry cache
        dentry_cache().clear();
        
        Ok(())
    }
    
    /// Get a mount point by path
    pub fn get_mount(&self, path: &str) -> Option<MountPoint> {
        self.mounts.read().get(path).cloned()
    }
    
    /// Find the mount point for a given path
    ///
    /// Returns the mount point with the longest matching prefix.
    pub fn find_mount(&self, path: &str) -> Option<MountPoint> {
        let mounts = self.mounts.read();
        
        // Find the longest matching prefix
        let mut best_match: Option<MountPoint> = None;
        let mut best_len = 0;
        
        for (mount_path, mount_point) in mounts.iter() {
            if path.starts_with(mount_path) && mount_path.len() > best_len {
                best_match = Some(mount_point.clone());
                best_len = mount_path.len();
            }
        }
        
        best_match
    }
    
    /// Get the root mount point
    pub fn root_mount(&self) -> Option<MountPoint> {
        self.get_mount("/")
    }
    
    /// List all mount points
    pub fn list_mounts(&self) -> Vec<MountPoint> {
        self.mounts.read().values().cloned().collect()
    }
    
    /// Check if a path is a mount point
    pub fn is_mount_point(&self, path: &str) -> bool {
        self.mounts.read().contains_key(path)
    }
    
    /// Sync all filesystems
    pub fn sync_all(&self) -> FsResult<()> {
        let mounts = self.mounts.read();
        for mount_point in mounts.values() {
            mount_point.sb.sync()?;
        }
        Ok(())
    }
}

/// Global mount table instance
static MOUNT_TABLE: Once<MountTable> = Once::new();

/// Get the global mount table
pub fn mount_table() -> &'static MountTable {
    MOUNT_TABLE.call_once(|| MountTable::new())
}

/// Initialize the mount table with a root filesystem
///
/// This should be called during kernel initialization to mount the root filesystem.
pub fn init_root_mount(
    fs_type_name: &str,
    dev: Option<Arc<dyn crate::drivers::block::BlockDevice>>,
    opts: MountOpts,
) -> FsResult<()> {
    let table = mount_table();
    table.mount(fs_type_name, dev, "/".to_string(), opts)?;
    Ok(())
}

/// Get the root inode
///
/// Returns the root inode of the root filesystem.
pub fn root_inode() -> FsResult<Arc<dyn Inode>> {
    let table = mount_table();
    let root_mount = table.root_mount().ok_or(FsError::NotFound)?;
    Ok(root_mount.sb.root())
}
