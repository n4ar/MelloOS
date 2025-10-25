//! Filesystem Type Registry
//!
//! This module manages registration and lookup of filesystem types.

use super::superblock::FsType;
use crate::sync::SpinLock;
use alloc::sync::Arc;
use alloc::vec::Vec;

/// Global filesystem type registry
static FS_REGISTRY: SpinLock<Vec<Arc<dyn FsType>>> = SpinLock::new(Vec::new());

/// Register a filesystem type
pub fn register_filesystem(fs_type: Arc<dyn FsType>) {
    let mut registry = FS_REGISTRY.lock();
    
    // Check if already registered
    for existing in registry.iter() {
        if existing.name() == fs_type.name() {
            crate::serial_println!("[VFS] Filesystem type '{}' already registered", fs_type.name());
            return;
        }
    }
    
    crate::serial_println!("[VFS] Registered filesystem type '{}'", fs_type.name());
    registry.push(fs_type);
}

/// Lookup a filesystem type by name
pub fn lookup_filesystem(name: &str) -> Option<Arc<dyn FsType>> {
    let registry = FS_REGISTRY.lock();
    
    for fs_type in registry.iter() {
        if fs_type.name() == name {
            return Some(fs_type.clone());
        }
    }
    
    None
}

/// List all registered filesystem types
pub fn list_filesystems() -> Vec<Arc<dyn FsType>> {
    let registry = FS_REGISTRY.lock();
    registry.clone()
}
