//! Path Resolution
//!
//! This module implements path resolution with symlink loop detection.
//! It handles absolute and relative paths, "." and ".." components,
//! and integrates with the dentry cache for fast lookups.

extern crate alloc;

use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use super::inode::{Inode, FileMode, FsError, FsResult};
use super::dentry::dentry_cache;

/// Maximum number of symlink hops allowed
const MAX_SYMLINK_HOPS: usize = 40;

/// Maximum path length
const MAX_PATH_LEN: usize = 4096;

/// Maximum component length
const MAX_COMPONENT_LEN: usize = 255;

/// Path resolution context
pub struct PathResolver {
    /// Current working directory (for relative paths)
    cwd: Arc<dyn Inode>,
    /// Root directory (for absolute paths)
    root: Arc<dyn Inode>,
    /// Symlink hop counter
    symlink_hops: usize,
}

impl PathResolver {
    /// Create a new path resolver
    pub fn new(cwd: Arc<dyn Inode>, root: Arc<dyn Inode>) -> Self {
        Self {
            cwd,
            root,
            symlink_hops: 0,
        }
    }
    
    /// Resolve a path to an inode
    ///
    /// # Arguments
    /// * `path` - The path to resolve (absolute or relative)
    /// * `follow_final_symlink` - Whether to follow the final component if it's a symlink
    ///
    /// # Returns
    /// The resolved inode, or an error
    pub fn resolve(&mut self, path: &str, follow_final_symlink: bool) -> FsResult<Arc<dyn Inode>> {
        // Validate path length
        if path.len() > MAX_PATH_LEN {
            return Err(FsError::NameTooLong);
        }
        
        // Check for null bytes
        if path.contains('\0') {
            return Err(FsError::InvalidArgument);
        }
        
        // Start from root or cwd
        let mut current = if path.starts_with('/') {
            Arc::clone(&self.root)
        } else {
            Arc::clone(&self.cwd)
        };
        
        // Split path into components
        let components: Vec<&str> = path
            .split('/')
            .filter(|c| !c.is_empty() && *c != ".")
            .collect();
        
        // Handle empty path
        if components.is_empty() {
            return Ok(current);
        }
        
        // Resolve each component
        for (i, component) in components.iter().enumerate() {
            let is_last = i == components.len() - 1;
            
            // Validate component length
            if component.len() > MAX_COMPONENT_LEN {
                return Err(FsError::NameTooLong);
            }
            
            // Handle ".."
            if *component == ".." {
                current = self.resolve_parent(&current)?;
                continue;
            }
            
            // Ensure current is a directory
            if !current.mode().is_directory() {
                return Err(FsError::NotDirectory);
            }
            
            // Try dentry cache first
            let sb_id = 0; // TODO: Get from superblock
            let parent_ino = current.ino();
            
            let next = if let Some(cached) = dentry_cache().lookup(parent_ino, component) {
                // Cache hit
                match cached {
                    Some(inode) => inode,
                    None => return Err(FsError::NotFound), // Negative entry
                }
            } else {
                // Cache miss - lookup in directory
                match current.lookup(component) {
                    Ok(inode) => {
                        // Cache positive entry
                        dentry_cache().insert_positive(
                            parent_ino,
                            component.to_string(),
                            Arc::clone(&inode),
                        );
                        inode
                    }
                    Err(FsError::NotFound) => {
                        // Cache negative entry
                        dentry_cache().insert_negative(parent_ino, component.to_string());
                        return Err(FsError::NotFound);
                    }
                    Err(e) => return Err(e),
                }
            };
            
            // Handle symlinks
            if next.mode().is_symlink() {
                // Don't follow if it's the last component and follow_final_symlink is false
                if is_last && !follow_final_symlink {
                    return Ok(next);
                }
                
                // Check symlink hop limit
                self.symlink_hops += 1;
                if self.symlink_hops > MAX_SYMLINK_HOPS {
                    return Err(FsError::TooManySymlinks);
                }
                
                // Read symlink target
                let target = next.readlink()?;
                
                // Recursively resolve the symlink target
                let resolved = self.resolve(&target, true)?;
                current = resolved;
            } else {
                current = next;
            }
        }
        
        Ok(current)
    }
    
    /// Resolve the parent of a directory
    fn resolve_parent(&self, inode: &Arc<dyn Inode>) -> FsResult<Arc<dyn Inode>> {
        // If we're at root, parent is root
        if inode.ino() == self.root.ino() {
            return Ok(Arc::clone(&self.root));
        }
        
        // Look up ".." in the directory
        inode.lookup("..")
    }
}

/// Resolve a path from a starting directory
///
/// This is a convenience function that creates a PathResolver and resolves the path.
///
/// # Arguments
/// * `start` - Starting directory (cwd for relative paths, root for absolute paths)
/// * `root` - Root directory
/// * `path` - Path to resolve
/// * `follow_final_symlink` - Whether to follow the final component if it's a symlink
pub fn resolve_path(
    start: Arc<dyn Inode>,
    root: Arc<dyn Inode>,
    path: &str,
    follow_final_symlink: bool,
) -> FsResult<Arc<dyn Inode>> {
    let mut resolver = PathResolver::new(start, root);
    resolver.resolve(path, follow_final_symlink)
}

/// Split a path into parent directory and filename
///
/// # Returns
/// (parent_path, filename)
pub fn split_path(path: &str) -> (&str, &str) {
    if let Some(pos) = path.rfind('/') {
        let parent = if pos == 0 { "/" } else { &path[..pos] };
        let name = &path[pos + 1..];
        (parent, name)
    } else {
        (".", path)
    }
}

/// Validate a filename
///
/// Returns an error if the name is invalid:
/// - Empty
/// - Contains '/' or null bytes
/// - Too long (> 255 bytes)
pub fn validate_filename(name: &str) -> FsResult<()> {
    if name.is_empty() {
        return Err(FsError::InvalidArgument);
    }
    
    if name.len() > MAX_COMPONENT_LEN {
        return Err(FsError::NameTooLong);
    }
    
    if name.contains('/') || name.contains('\0') {
        return Err(FsError::InvalidArgument);
    }
    
    // Reject "." and ".." as filenames
    if name == "." || name == ".." {
        return Err(FsError::InvalidArgument);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_split_path() {
        assert_eq!(split_path("/foo/bar"), ("/foo", "bar"));
        assert_eq!(split_path("/foo"), ("/", "foo"));
        assert_eq!(split_path("foo"), (".", "foo"));
        assert_eq!(split_path("foo/bar"), ("foo", "bar"));
    }
    
    #[test]
    fn test_validate_filename() {
        assert!(validate_filename("foo").is_ok());
        assert!(validate_filename("foo.txt").is_ok());
        assert!(validate_filename("foo-bar_123").is_ok());
        
        assert!(validate_filename("").is_err());
        assert!(validate_filename(".").is_err());
        assert!(validate_filename("..").is_err());
        assert!(validate_filename("foo/bar").is_err());
        assert!(validate_filename("foo\0bar").is_err());
        assert!(validate_filename(&"a".repeat(256)).is_err());
    }
}
