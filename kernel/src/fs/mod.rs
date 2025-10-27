//! Filesystem Support
//!
//! This module contains filesystem implementations and the Virtual File System (VFS) layer.

pub mod block_dev;
pub mod cache;
pub mod mfs;
pub mod proc;
pub mod syscalls;
pub mod test_simple;
pub mod vfs;

use crate::serial_println;

/// Initialize the VFS subsystem
pub fn init() {
    serial_println!("[VFS] Initializing Virtual File System...");

    // Initialize dentry cache
    crate::fs::vfs::dentry::clear();
    serial_println!("[VFS] Dentry cache initialized");

    // Mount mfs_ram as root filesystem
    mount_root_filesystem();
    serial_println!("[VFS] Root filesystem mounted");

    // Create initial directory structure
    create_initial_directories();
    serial_println!("[VFS] Initial directory structure created");

    serial_println!("[VFS] Virtual File System initialization complete");
}

/// Mount mfs_ram as the root filesystem
fn mount_root_filesystem() {
    use crate::fs::mfs::ram::MfsRamType;
    use crate::fs::vfs::mount;
    use crate::fs::vfs::superblock::{FsType, MountOpts};

    // Create mfs_ram filesystem type
    let fs_type = MfsRamType;

    // Mount as root with default options
    let opts = MountOpts::default();
    match fs_type.mount(opts) {
        Ok(superblock) => {
            // Register the mount
            match mount::register_mount("/", superblock, "mfs_ram") {
                Ok(_) => {
                    serial_println!("[VFS] Successfully mounted mfs_ram as root filesystem");
                }
                Err(e) => {
                    panic!("[VFS] Failed to register root mount: {:?}", e);
                }
            }
        }
        Err(e) => {
            panic!("[VFS] Failed to mount mfs_ram as root: {:?}", e);
        }
    }
}

/// Create initial directory structure
fn create_initial_directories() {
    use crate::fs::vfs::inode::FileMode;
    use crate::fs::vfs::path;

    // Get root inode
    let root = match path::resolve_path("/", None) {
        Ok(inode) => inode,
        Err(e) => {
            serial_println!("[VFS] Warning: Could not get root inode: {:?}", e);
            return;
        }
    };

    // Create /dev directory
    if let Err(e) = root.create("dev", FileMode::new(FileMode::S_IFDIR | 0o755), 0, 0) {
        serial_println!("[VFS] Warning: Could not create /dev: {:?}", e);
    } else {
        serial_println!("[VFS] Created /dev directory");
    }

    // Create /tmp directory
    if let Err(e) = root.create("tmp", FileMode::new(FileMode::S_IFDIR | 0o1777), 0, 0) {
        serial_println!("[VFS] Warning: Could not create /tmp: {:?}", e);
    } else {
        serial_println!("[VFS] Created /tmp directory");
    }

    // Create /proc directory (for future proc filesystem)
    if let Err(e) = root.create("proc", FileMode::new(FileMode::S_IFDIR | 0o555), 0, 0) {
        serial_println!("[VFS] Warning: Could not create /proc: {:?}", e);
    } else {
        serial_println!("[VFS] Created /proc directory");
    }

    // Create /home directory
    if let Err(e) = root.create("home", FileMode::new(FileMode::S_IFDIR | 0o755), 0, 0) {
        serial_println!("[VFS] Warning: Could not create /home: {:?}", e);
    } else {
        serial_println!("[VFS] Created /home directory");
    }
}
