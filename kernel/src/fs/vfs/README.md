# VFS Implementation Note

## Current Status

The VFS trait definitions and infrastructure have been created, but the implementation currently uses `alloc` crate types (Arc, Vec, String, BTreeMap) which are not available in the MelloOS kernel.

## Issue

MelloOS kernel is a `no_std` environment without the `alloc` crate. The current VFS implementation needs to be redesigned to work without dynamic allocation.

## Required Changes

To make the VFS work in the kernel, we need to:

1. **Replace Arc with raw pointers or indices**: Use a global inode table with indices instead of Arc<dyn Inode>
2. **Replace Vec with fixed-size arrays**: Use static arrays with capacity limits
3. **Replace String with fixed-size buffers**: Use `[u8; MAX_NAME_LEN]` for names
4. **Replace BTreeMap with custom hash tables**: Implement hash tables using static arrays

## Alternative Approach

Consider enabling the `alloc` crate in the kernel by:
1. Implementing a global allocator using the kernel's memory management
2. Adding `extern crate alloc;` to lib.rs or main.rs
3. This would allow using Arc, Vec, String, etc.

## Next Steps

1. Decide whether to enable `alloc` in the kernel or redesign without it
2. If enabling alloc: implement GlobalAlloc trait for kernel allocator
3. If not: redesign VFS to use static allocation only

## Files Affected

- `kernel/src/fs/vfs/inode.rs` - Uses Arc, Vec, String, BTreeMap
- `kernel/src/fs/vfs/dentry.rs` - Uses Arc, Vec, String, BTreeMap
- `kernel/src/fs/vfs/path.rs` - Uses Arc, Vec, String
- `kernel/src/fs/vfs/mount.rs` - Uses Arc, Vec, String, BTreeMap
- `kernel/src/fs/vfs/file.rs` - Uses Arc, Vec
- `kernel/src/fs/vfs/superblock.rs` - Uses Arc, String
- `kernel/src/fs/syscalls.rs` - Uses Arc, Vec, String
