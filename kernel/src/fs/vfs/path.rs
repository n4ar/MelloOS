//! Path Resolution
//!
//! This module implements path resolution with symlink loop detection.
//! It handles absolute and relative paths, "." and ".." components,
//! and integrates with the dentry cache for fast lookups.

use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use crate::fs::vfs::inode::Inode;
use crate::fs::vfs::superblock::FsError;
use crate::fs::vfs::mount;

/// Maximum number of symlink hops allowed (prevents infinite loops)
const MAX_SYMLINK_HOPS: usize = 40;

/// Maximum path component length (Linux NAME_MAX)
const MAX_NAME_LEN: usize = 255;

/// Maximum path length
const MAX_PATH_LEN: usize = 4096;

/// Resolve a path to an inode
///
/// # Arguments
/// * `path` - Path to resolve (absolute or relative)
/// * `current_dir` - Current working directory (for relative paths), None means use root
///
/// # Returns
/// Arc<dyn Inode> on success, FsError on failure
///
/// # Errors
/// * ENOENT - Path component not found
/// * ENOTDIR - Path component is not a directory
/// * ELOOP - Too many symlink hops
/// * ENAMETOOLONG - Path or component too long
/// * EINVAL - Invalid path (empty, contains null bytes)
pub fn resolve_path(
    path: &str,
    current_dir: Option<Arc<dyn Inode>>,
) -> Result<Arc<dyn Inode>, FsError> {
    resolve_path_internal(path, current_dir, 0)
}

/// Internal path resolution with symlink hop counter
fn resolve_path_internal(
    path: &str,
    current_dir: Option<Arc<dyn Inode>>,
    symlink_hops: usize,
) -> Result<Arc<dyn Inode>, FsError> {
    // Validate path
    if path.is_empty() {
        return Err(FsError::InvalidArgument);
    }
    
    if path.len() > MAX_PATH_LEN {
        return Err(FsError::NameTooLong);
    }
    
    if path.contains('\0') {
        return Err(FsError::InvalidArgument);
    }
    
    // Check symlink hop limit
    if symlink_hops > MAX_SYMLINK_HOPS {
        return Err(FsError::TooManySymlinks);
    }
    
    // Determine starting point
    let mut current: Arc<dyn Inode> = if path.starts_with('/') {
        // Absolute path - start from root
        get_root_inode()?
    } else {
        // Relative path - start from current directory
        current_dir.ok_or(FsError::InvalidArgument)?
    };
    
    // Split path into components
    let components: Vec<&str> = path
        .split('/')
        .filter(|c| !c.is_empty() && *c != ".")
        .collect();
    
    // If path is just "/" or ".", return root/current
    if components.is_empty() {
        return Ok(current);
    }
    
    // Walk through path components
    for (idx, component) in components.iter().enumerate() {
        // Validate component length
        if component.len() > MAX_NAME_LEN {
            return Err(FsError::NameTooLong);
        }
        
        // Handle ".." - move to parent
        if *component == ".." {
            // Try to get parent (for now, just stay at current if we can't go up)
            // TODO: Implement proper parent tracking
            // For root, ".." stays at root
            continue;
        }
        
        // Ensure current is a directory
        if !current.mode().is_dir() {
            return Err(FsError::NotADirectory);
        }
        
        // Lookup component in current directory
        let next = current.lookup(component)
            .map_err(|_| FsError::NotFound)?;
        
        // Check if it's a symlink
        if next.mode().is_symlink() {
            // Read symlink target
            let target = next.readlink()?;
            
            // Recursively resolve symlink target
            // If there are more components after this, we need to continue from the symlink target
            let remaining_path = if idx + 1 < components.len() {
                let remaining: Vec<&str> = components[idx + 1..].to_vec();
                if target.starts_with('/') {
                    // Absolute symlink
                    format!("{}/{}", target, remaining.join("/"))
                } else {
                    // Relative symlink
                    format!("{}/{}", target, remaining.join("/"))
                }
            } else {
                target.clone()
            };
            
            // Resolve the symlink (increment hop counter)
            return resolve_path_internal(
                &remaining_path,
                Some(current.clone()),
                symlink_hops + 1,
            );
        }
        
        // Move to next component
        current = next;
    }
    
    Ok(current)
}

/// Get the root inode from the root mount point
fn get_root_inode() -> Result<Arc<dyn Inode>, FsError> {
    let mount_point = mount::lookup_mount("/")
        .ok_or(FsError::NotFound)?;
    
    Ok(mount_point.superblock.root())
}

/// Resolve a path and return the parent directory and final component name
///
/// This is useful for operations like create, unlink, etc. that need to
/// operate on the parent directory.
///
/// # Arguments
/// * `path` - Path to resolve
/// * `current_dir` - Current working directory (for relative paths)
///
/// # Returns
/// (parent_inode, component_name) on success
///
/// # Errors
/// * ENOENT - Parent directory not found
/// * ENOTDIR - Path component is not a directory
/// * EINVAL - Invalid path (empty, no parent)
pub fn resolve_parent(
    path: &str,
    current_dir: Option<Arc<dyn Inode>>,
) -> Result<(Arc<dyn Inode>, String), FsError> {
    // Validate path
    if path.is_empty() || path == "/" {
        return Err(FsError::InvalidArgument);
    }
    
    // Find last '/' to split parent and name
    if let Some(last_slash) = path.rfind('/') {
        let parent_path = if last_slash == 0 {
            // Path is "/name" - parent is root
            "/"
        } else {
            &path[..last_slash]
        };
        
        let name = &path[last_slash + 1..];
        
        if name.is_empty() {
            return Err(FsError::InvalidArgument);
        }
        
        if name.len() > MAX_NAME_LEN {
            return Err(FsError::NameTooLong);
        }
        
        let parent = resolve_path(parent_path, current_dir)?;
        
        if !parent.mode().is_dir() {
            return Err(FsError::NotADirectory);
        }
        
        Ok((parent, name.into()))
    } else {
        // No slash - name is in current directory
        let parent = current_dir.ok_or(FsError::InvalidArgument)?;
        
        if !parent.mode().is_dir() {
            return Err(FsError::NotADirectory);
        }
        
        if path.len() > MAX_NAME_LEN {
            return Err(FsError::NameTooLong);
        }
        
        Ok((parent, path.into()))
    }
}

#[cfg(test)]
mod tests {
    // Tests will be added once filesystem is operational
    // Test cases:
    // - Empty path
    // - Root path "/"
    // - Simple path "/foo/bar"
    // - Path with "." components
    // - Path with ".." components
    // - Path with multiple slashes "//foo///bar"
    // - Symlink resolution
    // - Symlink loop detection (> 40 hops)
    // - Name too long
    // - Path too long
    // - ENOTDIR when component is not a directory
}
