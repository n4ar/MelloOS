# MelloFS RAM Filesystem

Fast in-memory filesystem for boot and temporary storage.

## Features

- **O(log N) directory lookups** using BTreeMap
- **Chunked file storage** (32 KiB chunks by default) for efficient memory use
- **Hardlink support** via reference counting
- **Symlink support** with target path storage
- **Extended attributes** with namespace validation
- **SMP-safe** with fine-grained locking using SpinLock

## Architecture

### Superblock (`super_impl.rs`)

- `MfsRamType`: Filesystem type for registration
- `MfsRamSuperBlock`: Mounted filesystem instance
  - Tracks total memory usage
  - Allocates inode numbers
  - Provides root inode
  - Implements statfs for filesystem statistics

### Inode (`inode.rs`)

- `RamInode`: In-memory inode structure
  - Atomic reference counting for hardlinks (nlink)
  - Atomic file size tracking
  - SpinLock-protected metadata and data
  - Separate xattr storage

- `InodeKind`: Union of file types
  - `File`: Chunked data storage
  - `Directory`: BTreeMap of entries
  - `Symlink`: Target path string

### Directory Operations (`dir.rs`)

- `dir_lookup`: O(log N) name lookup in BTreeMap
- `dir_create`: Create files and directories
- `dir_unlink`: Remove entries with reference counting
- `dir_link`: Create hardlinks (shares Arc<RamInode>)
- `dir_symlink`: Create symbolic links
- `dir_readdir`: Iterate directory entries
- Name validation (length, characters, special names)

### File Operations (`file.rs`)

- `file_read_at`: Read from chunked storage
- `file_write_at`: Write with CoW semantics
- `file_truncate`: Resize file (grow or shrink)
- Automatic chunk allocation on write
- Efficient partial chunk updates

### Extended Attributes (`xattr.rs`)

- `xattr_set`: Set attribute with validation
- `xattr_get`: Retrieve attribute value
- `xattr_list`: List all attribute names
- `xattr_remove`: Remove attribute
- Namespace validation (user.*, system.*, security.*, trusted.*)
- Size limits: 255 bytes for names, 64 KiB for values

## Memory Management

- **Chunk-based storage**: Files are stored in 32 KiB chunks
- **Copy-on-Write**: Chunks are immutable (Arc<[u8]>)
- **Lazy allocation**: Chunks allocated on first write
- **Automatic cleanup**: Arc drop frees memory when nlink reaches 0

## Concurrency

- **Per-inode locking**: Each inode has its own SpinLock
- **Atomic operations**: nlink and size use AtomicU32/AtomicU64
- **No global locks**: Scales well on SMP systems

## Limitations

- **No persistence**: Data lost on unmount/reboot
- **Memory-bound**: Limited by available RAM
- **No quotas**: No per-user or per-group limits (yet)
- **No ACLs**: Only basic POSIX permissions

## Usage

```rust
use crate::fs::mfs::ram::MfsRamType;
use crate::fs::vfs::superblock::{FsType, MountOpts};

// Create filesystem type
let fs_type = MfsRamType;

// Mount filesystem
let sb = fs_type.mount(MountOpts::default())?;

// Get root inode
let root = sb.root();

// Create a file
let file = root.create("test.txt", FileMode::new(0o644), 0, 0)?;

// Write data
file.write_at(0, b"Hello, MelloOS!")?;

// Read data
let mut buf = [0u8; 15];
file.read_at(0, &mut buf)?;
```

## Testing

See `tests/mfs_ram_correctness.rs` and `tests/mfs_ram_perf.rs` for comprehensive tests.

## Future Enhancements

- [ ] Implement proper time tracking (atime, mtime, ctime)
- [ ] Add memory usage limits and enforcement
- [ ] Implement set_attr for chmod/chown
- [ ] Add support for special files (devices, FIFOs)
- [ ] Optimize for small files (inline data)
- [ ] Add compression for large files
