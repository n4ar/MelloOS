//! Path Resolution
//!
//! This module implements path resolution with symlink loop detection.
//! It handles absolute and relative paths, "." and ".." components,
//! and integrates with the dentry cache for fast lookups.
//!
//! Parent tracking is now properly implemented using the Dentry structure.

use crate::fs::vfs::dentry::Dentry;
use crate::fs::vfs::inode::Inode;
use crate::fs::vfs::mount;
use crate::fs::vfs::superblock::FsError;
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

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
            // Proper parent tracking implementation
            // We need to track dentries instead of just inodes for this to work
            // For now, we'll implement a basic version that looks up ".." in the directory
            // A full implementation would maintain a dentry tree structure

            // Try to lookup ".." in the current directory
            match current.lookup("..") {
                Ok(parent_inode) => {
                    current = parent_inode;
                    continue;
                }
                Err(_) => {
                    // If ".." lookup fails, stay at current (likely root)
                    continue;
                }
            }
        }

        // Ensure current is a directory
        if !current.mode().is_dir() {
            return Err(FsError::NotADirectory);
        }

        // Lookup component in current directory
        let next = current.lookup(component).map_err(|_| FsError::NotFound)?;

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
            return resolve_path_internal(&remaining_path, Some(current.clone()), symlink_hops + 1);
        }

        // Move to next component
        current = next;
    }

    Ok(current)
}

/// Get the root inode from the root mount point
fn get_root_inode() -> Result<Arc<dyn Inode>, FsError> {
    let mount_point = mount::lookup_mount("/").ok_or(FsError::NotFound)?;

    Ok(mount_point.superblock.root())
}

/// Get the root dentry from the root mount point
fn get_root_dentry() -> Result<Arc<Dentry>, FsError> {
    let root_inode = get_root_inode()?;
    Ok(Dentry::new_root(root_inode))
}

/// Resolve a path to a dentry (with proper parent tracking)
///
/// This version maintains the dentry tree structure, enabling proper ".." handling.
///
/// # Arguments
/// * `path` - Path to resolve (absolute or relative)
/// * `current_dentry` - Current working directory dentry (for relative paths), None means use root
///
/// # Returns
/// Arc<Dentry> on success, FsError on failure
///
/// # Errors
/// * ENOENT - Path component not found
/// * ENOTDIR - Path component is not a directory
/// * ELOOP - Too many symlink hops
/// * ENAMETOOLONG - Path or component too long
/// * EINVAL - Invalid path (empty, contains null bytes)
pub fn resolve_path_dentry(
    path: &str,
    current_dentry: Option<Arc<Dentry>>,
) -> Result<Arc<Dentry>, FsError> {
    resolve_path_dentry_internal(path, current_dentry, 0)
}

/// Internal dentry-based path resolution with symlink hop counter
fn resolve_path_dentry_internal(
    path: &str,
    current_dentry: Option<Arc<Dentry>>,
    symlink_hops: usize,
) -> Result<Arc<Dentry>, FsError> {
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
    let mut current: Arc<Dentry> = if path.starts_with('/') {
        // Absolute path - start from root
        get_root_dentry()?
    } else {
        // Relative path - start from current directory
        current_dentry.ok_or(FsError::InvalidArgument)?
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

        // Handle ".." - move to parent using dentry parent tracking
        if *component == ".." {
            // Use parent tracking from dentry structure
            if let Some(parent_dentry) = current.parent() {
                current = parent_dentry;
            }
            // If no parent (root), stay at current
            continue;
        }

        // Ensure current is a directory
        if !current.inode().mode().is_dir() {
            return Err(FsError::NotADirectory);
        }

        // Lookup component in current directory
        let next_inode = current
            .inode()
            .lookup(component)
            .map_err(|_| FsError::NotFound)?;

        // Create dentry for the next component with proper parent tracking
        let next_dentry = Dentry::new_child(
            next_inode.clone(),
            current.clone(),
            String::from(*component),
        );

        // Check if it's a symlink
        if next_inode.mode().is_symlink() {
            // Read symlink target
            let target = next_inode.readlink()?;

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
            return resolve_path_dentry_internal(
                &remaining_path,
                Some(current.clone()),
                symlink_hops + 1,
            );
        }

        // Move to next component
        current = next_dentry;
    }

    Ok(current)
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
#[allow(dead_code)]
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

/// Resolve a path and return the parent dentry and final component name
///
/// This version uses dentry-based resolution with proper parent tracking.
///
/// # Arguments
/// * `path` - Path to resolve
/// * `current_dentry` - Current working directory dentry (for relative paths)
///
/// # Returns
/// (parent_dentry, component_name) on success
///
/// # Errors
/// * ENOENT - Parent directory not found
/// * ENOTDIR - Path component is not a directory
/// * EINVAL - Invalid path (empty, no parent)
#[allow(dead_code)]
pub fn resolve_parent_dentry(
    path: &str,
    current_dentry: Option<Arc<Dentry>>,
) -> Result<(Arc<Dentry>, String), FsError> {
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

        let parent = resolve_path_dentry(parent_path, current_dentry)?;

        if !parent.inode().mode().is_dir() {
            return Err(FsError::NotADirectory);
        }

        Ok((parent, name.into()))
    } else {
        // No slash - name is in current directory
        let parent = current_dentry.ok_or(FsError::InvalidArgument)?;

        if !parent.inode().mode().is_dir() {
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
