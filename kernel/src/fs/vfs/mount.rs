//! Mount Table
//!
//! This module implements the global mount point table and mount/umount operations.
//! It tracks all mounted filesystems and provides mount point lookup during path resolution.
//!
//! Implementation uses static arrays to avoid heap allocation during early boot.

use crate::fs::vfs::superblock::SuperBlock;
use alloc::string::String;
use alloc::sync::Arc;
use spin::Mutex as SpinLock;

/// Maximum number of mount points supported
const MAX_MOUNTS: usize = 16;

/// A single mount point entry
#[derive(Clone)]
pub struct MountPoint {
    /// Mount path (e.g., "/", "/dev", "/tmp")
    pub path: String,
    /// Superblock of the mounted filesystem
    pub superblock: Arc<dyn SuperBlock>,
    /// Filesystem type name (e.g., "mfs_ram", "mfs_disk")
    pub fs_type: String,
    /// Is this mount point active?
    pub active: bool,
}

/// Global mount table
pub struct MountTable {
    /// Array of mount points
    mounts: [Option<MountPoint>; MAX_MOUNTS],
    /// Number of active mounts
    count: usize,
}

impl MountTable {
    /// Create a new empty mount table
    const fn new() -> Self {
        Self {
            mounts: [const { None }; MAX_MOUNTS],
            count: 0,
        }
    }

    /// Register a new mount point
    ///
    /// # Arguments
    /// * `path` - Mount path (must start with '/')
    /// * `sb` - Superblock of the filesystem
    /// * `fs_type` - Filesystem type name
    ///
    /// # Returns
    /// Ok(()) on success, Err if mount table is full or path already mounted
    pub fn register_mount(
        &mut self,
        path: String,
        sb: Arc<dyn SuperBlock>,
        fs_type: String,
    ) -> Result<(), &'static str> {
        // Validate path
        if !path.starts_with('/') {
            return Err("Mount path must start with '/'");
        }

        // Check if path already mounted
        for mount in self.mounts.iter().flatten() {
            if mount.active && mount.path == path {
                return Err("Path already mounted");
            }
        }

        // Find empty slot
        for slot in &mut self.mounts {
            if slot.is_none() {
                *slot = Some(MountPoint {
                    path,
                    superblock: sb,
                    fs_type,
                    active: true,
                });
                self.count += 1;
                return Ok(());
            }
        }

        Err("Mount table full")
    }

    /// Lookup a mount point by path
    ///
    /// Returns the mount point with the longest matching prefix.
    /// For example, if "/dev" and "/" are mounted, looking up "/dev/null"
    /// will return the "/dev" mount.
    ///
    /// # Arguments
    /// * `path` - Path to lookup
    ///
    /// # Returns
    /// Some(MountPoint) if found, None otherwise
    pub fn lookup_mount(&self, path: &str) -> Option<MountPoint> {
        let mut best_match: Option<&MountPoint> = None;
        let mut best_len = 0;

        for mount in self.mounts.iter().flatten() {
            if !mount.active {
                continue;
            }

            // Check if path starts with mount path
            if path.starts_with(&mount.path) {
                let mount_len = mount.path.len();
                // Ensure we match on path boundaries
                // "/" matches everything
                // "/dev" matches "/dev" and "/dev/..." but not "/device"
                if mount_len > best_len {
                    if mount.path == "/"
                        || path.len() == mount_len
                        || path.as_bytes()[mount_len] == b'/'
                    {
                        best_match = Some(mount);
                        best_len = mount_len;
                    }
                }
            }
        }

        best_match.cloned()
    }

    /// Unmount a filesystem
    ///
    /// # Arguments
    /// * `path` - Mount path to unmount
    ///
    /// # Returns
    /// Ok(()) on success, Err if path not found or is root
    pub fn unmount(&mut self, path: &str) -> Result<(), &'static str> {
        // Don't allow unmounting root
        if path == "/" {
            return Err("Cannot unmount root filesystem");
        }

        // Find and deactivate mount
        for mount in self.mounts.iter_mut().flatten() {
            if mount.active && mount.path == path {
                mount.active = false;
                self.count -= 1;
                return Ok(());
            }
        }

        Err("Mount point not found")
    }

    /// Get number of active mounts
    pub fn count(&self) -> usize {
        self.count
    }

    /// List all active mounts
    pub fn list_mounts(&self) -> alloc::vec::Vec<MountPoint> {
        self.mounts
            .iter()
            .flatten()
            .filter(|m| m.active)
            .cloned()
            .collect()
    }
}

/// Global mount table instance
static MOUNT_TABLE: SpinLock<MountTable> = SpinLock::new(MountTable::new());

/// Register a new mount point (public API)
pub fn register_mount(
    path: &str,
    sb: Arc<dyn SuperBlock>,
    fs_type: &str,
) -> Result<(), &'static str> {
    let mut table = MOUNT_TABLE.lock();
    table.register_mount(path.into(), sb, fs_type.into())
}

/// Lookup a mount point by path (public API)
pub fn lookup_mount(path: &str) -> Option<MountPoint> {
    let table = MOUNT_TABLE.lock();
    table.lookup_mount(path)
}

/// Unmount a filesystem (public API)
pub fn unmount(path: &str) -> Result<(), &'static str> {
    let mut table = MOUNT_TABLE.lock();
    table.unmount(path)
}

/// Get number of active mounts (public API)
pub fn mount_count() -> usize {
    let table = MOUNT_TABLE.lock();
    table.count()
}

/// List all active mounts (public API)
pub fn list_mounts() -> alloc::vec::Vec<MountPoint> {
    let table = MOUNT_TABLE.lock();
    table.list_mounts()
}

// Tests will be added in integration test suite once filesystem is operational

/// Sync all mounted filesystems
///
/// This function calls sync() on all active mount points,
/// ensuring all dirty data is written to disk.
pub fn sync_all_filesystems() -> Result<(), &'static str> {
    let table = MOUNT_TABLE.lock();

    for mount_opt in &table.mounts {
        if let Some(mount) = mount_opt {
            if mount.active {
                // Call sync on the superblock
                if let Err(e) = mount.superblock.sync() {
                    crate::serial_println!(
                        "[VFS] Failed to sync filesystem at {}: {:?}",
                        mount.path,
                        e
                    );
                    return Err("Failed to sync filesystem");
                }
            }
        }
    }

    Ok(())
}
