//! MelloFS RAM Directory Operations

use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;
use crate::fs::vfs::inode::{Inode, FileMode, DirEnt, DirCookie};
use crate::fs::vfs::superblock::FsError;
use crate::fs::mfs::ram::inode::{RamInode, InodeKind};

impl RamInode {
    /// Look up a name in a directory
    pub fn dir_lookup(&self, name: &str) -> Result<Arc<dyn Inode>, FsError> {
        // Validate this is a directory
        let data = self.data.lock();
        let entries = match &data.data {
            InodeKind::Directory(dir) => &dir.entries,
            _ => return Err(FsError::NotADirectory),
        };
        
        // Look up the name in the BTreeMap (O(log N))
        entries.get(name)
            .map(|inode| inode.clone() as Arc<dyn Inode>)
            .ok_or(FsError::NotFound)
    }
    
    /// Create a new file in a directory
    pub fn dir_create(
        &self,
        name: &str,
        mode: FileMode,
        uid: u32,
        gid: u32,
        superblock: &crate::fs::mfs::ram::super_impl::MfsRamSuperBlock,
    ) -> Result<Arc<dyn Inode>, FsError> {
        // Validate name
        Self::validate_name(name)?;
        
        // Lock directory
        let mut data = self.data.lock();
        let entries = match &mut data.data {
            InodeKind::Directory(dir) => &mut dir.entries,
            _ => return Err(FsError::NotADirectory),
        };
        
        // Check if name already exists
        if entries.contains_key(name) {
            return Err(FsError::AlreadyExists);
        }
        
        // Allocate new inode number
        let ino = superblock.alloc_ino();
        
        // Create new inode based on file type
        let new_inode = if mode.is_dir() {
            // Create directory
            let dir_inode = RamInode::new_dir(ino, mode, uid, gid)?;
            
            // Increment parent nlink (for ".." entry)
            self.nlink.fetch_add(1, Ordering::SeqCst);
            
            dir_inode
        } else if mode.is_file() {
            // Create regular file
            RamInode::new_file(ino, mode, uid, gid)?
        } else {
            return Err(FsError::NotSupported);
        };
        
        // Insert into directory
        entries.insert(String::from(name), new_inode.clone());
        
        // Update directory mtime
        data.mtime = Self::current_time();
        
        Ok(new_inode as Arc<dyn Inode>)
    }
    
    /// Remove a file from a directory
    pub fn dir_unlink(&self, name: &str) -> Result<(), FsError> {
        // Validate name
        Self::validate_name(name)?;
        
        // Lock directory
        let mut data = self.data.lock();
        let entries = match &mut data.data {
            InodeKind::Directory(dir) => &mut dir.entries,
            _ => return Err(FsError::NotADirectory),
        };
        
        // Look up the entry and clone the Arc
        let inode = entries.get(name).ok_or(FsError::NotFound)?.clone();
        
        // Check if it's a directory
        let is_dir = inode.mode().is_dir();
        if is_dir {
            // For directories, check if empty
            let inode_data = inode.data.lock();
            if let InodeKind::Directory(dir) = &inode_data.data {
                if !dir.entries.is_empty() {
                    return Err(FsError::NotSupported); // Directory not empty
                }
            }
            
            // Decrement parent nlink (remove ".." reference)
            self.nlink.fetch_sub(1, Ordering::SeqCst);
        }
        
        // Remove from directory
        entries.remove(name);
        
        // Decrement inode nlink
        inode.nlink.fetch_sub(1, Ordering::SeqCst);
        
        // If nlink reaches 0, the inode will be freed when all references are dropped
        // (Rust's Arc handles this automatically)
        
        // Update directory mtime
        data.mtime = Self::current_time();
        
        Ok(())
    }
    
    /// Create a hard link (internal version with Arc<RamInode>)
    pub fn dir_link_internal(&self, name: &str, target: Arc<RamInode>) -> Result<(), FsError> {
        use crate::serial_println;
        
        serial_println!("[MFS_RAM] dir_link_internal() called: name='{}', target_ino={}", name, target.ino);
        
        // Validate name
        Self::validate_name(name)?;
        serial_println!("[MFS_RAM] name validated");
        
        // Lock directory first to check if entry exists
        let mut data = self.data.lock();
        let entries = match &mut data.data {
            InodeKind::Directory(dir) => &mut dir.entries,
            _ => {
                serial_println!("[MFS_RAM] not a directory");
                return Err(FsError::NotADirectory);
            }
        };
        
        // Check if name already exists
        if entries.contains_key(name) {
            serial_println!("[MFS_RAM] name already exists");
            return Err(FsError::AlreadyExists);
        }
        
        // For directories, we allow linking during create() but not for hardlinks
        // We can detect this by checking if the directory is empty (new)
        let is_new_dir = if target.mode().is_dir() {
            let target_data = target.data.lock();
            if let InodeKind::Directory(dir) = &target_data.data {
                dir.entries.is_empty()
            } else {
                false
            }
        } else {
            false
        };
        
        serial_println!("[MFS_RAM] target check: is_dir={}, is_new_dir={}", target.mode().is_dir(), is_new_dir);
        
        // Don't allow hardlinks to existing directories
        if target.mode().is_dir() && !is_new_dir {
            serial_println!("[MFS_RAM] rejecting hardlink to existing directory");
            return Err(FsError::NotSupported);
        }
        
        // Insert the same Arc (this is a true hardlink)
        entries.insert(String::from(name), target.clone());
        serial_println!("[MFS_RAM] inserted into directory");
        
        // Increment target nlink
        target.nlink.fetch_add(1, Ordering::SeqCst);
        serial_println!("[MFS_RAM] incremented nlink to {}", target.nlink.load(Ordering::SeqCst));
        
        // Update directory mtime
        data.mtime = Self::current_time();
        
        serial_println!("[MFS_RAM] dir_link_internal() success");
        Ok(())
    }
    
    /// Create a hard link (trait version - tries to downcast)
    pub fn dir_link(&self, name: &str, target: Arc<dyn Inode>) -> Result<(), FsError> {
        use crate::serial_println;
        
        serial_println!("[MFS_RAM] dir_link() called: name='{}', target_ino={}", name, target.ino());
        
        // For now, return NotSupported since we can't safely downcast Arc<dyn Inode> to Arc<RamInode>
        // This method is only used for hardlinks, not for create()
        serial_println!("[MFS_RAM] dir_link() returning NotSupported (hardlinks not supported yet)");
        Err(FsError::NotSupported)
    }
    
    /// Create a symbolic link
    pub fn dir_symlink(&self, name: &str, target: &str, uid: u32, gid: u32, superblock: &crate::fs::mfs::ram::super_impl::MfsRamSuperBlock) -> Result<Arc<dyn Inode>, FsError> {
        // Validate name
        Self::validate_name(name)?;
        
        // Lock directory
        let mut data = self.data.lock();
        let entries = match &mut data.data {
            InodeKind::Directory(dir) => &mut dir.entries,
            _ => return Err(FsError::NotADirectory),
        };
        
        // Check if name already exists
        if entries.contains_key(name) {
            return Err(FsError::AlreadyExists);
        }
        
        // Allocate new inode number
        let ino = superblock.alloc_ino();
        
        // Create symlink inode
        let symlink_inode = RamInode::new_symlink(ino, String::from(target), uid, gid)?;
        
        // Insert into directory
        entries.insert(String::from(name), symlink_inode.clone());
        
        // Update directory mtime
        data.mtime = Self::current_time();
        
        Ok(symlink_inode as Arc<dyn Inode>)
    }
    
    /// Read directory entries
    pub fn dir_readdir(&self, cookie: &mut DirCookie, entries_out: &mut Vec<DirEnt>) -> Result<(), FsError> {
        // Lock directory
        let data = self.data.lock();
        let entries = match &data.data {
            InodeKind::Directory(dir) => &dir.entries,
            _ => return Err(FsError::NotADirectory),
        };
        
        // Skip entries based on cookie offset
        let skip = cookie.offset as usize;
        
        // Iterate through BTreeMap entries (already sorted)
        for (idx, (name, inode)) in entries.iter().enumerate().skip(skip) {
            // Determine file type
            let file_type = if inode.mode().is_dir() {
                DirEnt::DT_DIR
            } else if inode.mode().is_file() {
                DirEnt::DT_REG
            } else if inode.mode().is_symlink() {
                DirEnt::DT_LNK
            } else {
                DirEnt::DT_UNKNOWN
            };
            
            // Add entry
            entries_out.push(DirEnt {
                ino: inode.ino,
                name: name.clone(),
                file_type,
            });
            
            // Update cookie
            cookie.offset = (idx + 1) as u64;
        }
        
        Ok(())
    }
    
    /// Validate filename
    fn validate_name(name: &str) -> Result<(), FsError> {
        // Check for empty name
        if name.is_empty() {
            return Err(FsError::InvalidArgument);
        }
        
        // Check for special names
        if name == "." || name == ".." {
            return Err(FsError::InvalidArgument);
        }
        
        // Check for '/' or null bytes
        if name.contains('/') || name.contains('\0') {
            return Err(FsError::InvalidArgument);
        }
        
        // Check length (max 255 bytes)
        if name.len() > 255 {
            return Err(FsError::NameTooLong);
        }
        
        Ok(())
    }
}
