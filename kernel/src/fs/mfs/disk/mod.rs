//! MelloFS Disk Filesystem
//!
//! Persistent Copy-on-Write filesystem with B-tree indexing.

pub mod allocator;
pub mod btree;
pub mod checksum;
pub mod compress;
pub mod extent;
pub mod keys;
pub mod replay;
#[path = "super.rs"]
pub mod super_;
pub mod super_impl;
pub mod superblock_impl;
pub mod txg;

use crate::fs::block_dev::block_device_manager;
use crate::fs::vfs::superblock::{FsError, FsType, MountOpts, SuperBlock};
use alloc::sync::Arc;

/// MelloFS Disk filesystem type
pub struct MfsDiskType;

impl FsType for MfsDiskType {
    fn name(&self) -> &'static str {
        "mfs_disk"
    }

    fn mount(&self, opts: MountOpts) -> Result<Arc<dyn SuperBlock>, FsError> {
        // Extract device name from mount options
        let device_name = opts.data.as_ref().ok_or(FsError::InvalidArgument)?;

        // Find the block device
        let block_dev = block_device_manager()
            .get_device_by_name(device_name)
            .ok_or(FsError::DeviceNotFound)?;

        // Create disk superblock
        let superblock = superblock_impl::MfsDiskSuperBlock::new(block_dev)?;

        crate::serial_println!("[MFS_DISK] Mounted filesystem on device '{}'", device_name);
        Ok(Arc::new(superblock))
    }
}

/// Initialize MFS disk filesystem
pub fn init() {
    crate::serial_println!("[MFS_DISK] Initializing MelloFS disk filesystem...");

    // Register filesystem type
    crate::fs::vfs::register_filesystem(Arc::new(MfsDiskType));

    crate::serial_println!("[MFS_DISK] MelloFS disk filesystem initialized");
}
