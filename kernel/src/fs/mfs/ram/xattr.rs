//! MelloFS RAM Extended Attributes

use crate::fs::mfs::ram::inode::RamInode;
use crate::fs::vfs::superblock::FsError;
use alloc::string::String;
use alloc::vec::Vec;

impl RamInode {
    /// Maximum xattr name length (255 bytes)
    const MAX_XATTR_NAME_LEN: usize = 255;

    /// Maximum xattr value size (64 KiB)
    const MAX_XATTR_VALUE_SIZE: usize = 64 * 1024;

    /// Set extended attribute
    pub fn xattr_set(&self, name: &str, value: &[u8]) -> Result<(), FsError> {
        // Validate name
        Self::validate_xattr_name(name)?;

        // Validate value size
        if value.len() > Self::MAX_XATTR_VALUE_SIZE {
            return Err(FsError::InvalidArgument);
        }

        // Lock xattrs
        let mut xattrs = self.xattrs.lock();

        // Insert or update xattr
        xattrs.insert(String::from(name), value.to_vec());

        Ok(())
    }

    /// Get extended attribute
    pub fn xattr_get(&self, name: &str) -> Result<Vec<u8>, FsError> {
        // Validate name
        Self::validate_xattr_name(name)?;

        // Lock xattrs
        let xattrs = self.xattrs.lock();

        // Look up xattr
        xattrs.get(name).map(|v| v.clone()).ok_or(FsError::NotFound)
    }

    /// List extended attribute names
    pub fn xattr_list(&self) -> Result<Vec<String>, FsError> {
        // Lock xattrs
        let xattrs = self.xattrs.lock();

        // Collect all names
        Ok(xattrs.keys().cloned().collect())
    }

    /// Remove extended attribute
    pub fn xattr_remove(&self, name: &str) -> Result<(), FsError> {
        // Validate name
        Self::validate_xattr_name(name)?;

        // Lock xattrs
        let mut xattrs = self.xattrs.lock();

        // Remove xattr
        xattrs.remove(name).map(|_| ()).ok_or(FsError::NotFound)
    }

    /// Validate xattr name
    fn validate_xattr_name(name: &str) -> Result<(), FsError> {
        // Check for empty name
        if name.is_empty() {
            return Err(FsError::InvalidArgument);
        }

        // Check length
        if name.len() > Self::MAX_XATTR_NAME_LEN {
            return Err(FsError::NameTooLong);
        }

        // Check for null bytes
        if name.contains('\0') {
            return Err(FsError::InvalidArgument);
        }

        // Check namespace (must have a dot)
        if !name.contains('.') {
            return Err(FsError::InvalidArgument);
        }

        // Validate namespace prefix
        let namespace = name.split('.').next().unwrap();
        match namespace {
            "user" | "system" | "security" | "trusted" => Ok(()),
            _ => Err(FsError::NotSupported),
        }
    }
}
