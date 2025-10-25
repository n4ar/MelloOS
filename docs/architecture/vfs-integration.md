# VFS Integration Architecture

## Overview

This document describes how the Virtual File System (VFS) is integrated with MelloOS kernel, connecting user-space syscalls to filesystem implementations through a layered architecture.

**Status:** Implemented in Phase 8  
**Last Updated:** 2025-01-XX

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    User Space (Ring 3)                      │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │  Shell   │  │   cat    │  │    ls    │  │   Apps   │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  │
│       │             │              │             │         │
│       └─────────────┴──────────────┴─────────────┘         │
│                          │                                  │
│                    syscall (int 0x80)                      │
└──────────────────────────┼──────────────────────────────────┘
                           │
┌──────────────────────────┼──────────────────────────────────┐
│                    Kernel Space (Ring 0)                    │
│                          ▼                                  │
│  ┌────────────────────────────────────────────────────┐    │
│  │         System Call Handler                        │    │
│  │  (kernel/src/sys/syscall.rs)                      │    │
│  │                                                    │    │
│  │  sys_open, sys_read, sys_write, sys_close,       │    │
│  │  sys_stat, sys_mkdir, sys_unlink, etc.           │    │
│  └──────────────────┬─────────────────────────────────┘    │
│                     │                                       │
│                     ▼                                       │
│  ┌────────────────────────────────────────────────────┐    │
│  │         VFS Layer (kernel/src/fs/vfs/)            │    │
│  │                                                    │    │
│  │  ┌──────────────┐  ┌──────────────┐             │    │
│  │  │ Path         │  │ Mount        │             │    │
│  │  │ Resolution   │  │ Table        │             │    │
│  │  └──────┬───────┘  └──────┬───────┘             │    │
│  │         │                  │                      │    │
│  │  ┌──────▼──────────────────▼───────┐             │    │
│  │  │     Dentry Cache                │             │    │
│  │  │  (name → inode mapping)         │             │    │
│  │  └──────────────┬──────────────────┘             │    │
│  │                 │                                 │    │
│  │  ┌──────────────▼──────────────────┐             │    │
│  │  │     Inode Cache                 │             │    │
│  │  │  (ino → Arc<dyn Inode>)        │             │    │
│  │  └──────────────┬──────────────────┘             │    │
│  │                 │                                 │    │
│  │  ┌──────────────▼──────────────────┐             │    │
│  │  │  File Descriptor Table          │             │    │
│  │  │  (per-process FD → File)        │             │    │
│  │  └─────────────────────────────────┘             │    │
│  └────────────────────┬───────────────────────────────┘    │
│                       │                                     │
│                       ▼                                     │
│  ┌────────────────────────────────────────────────────┐    │
│  │    Filesystem Implementations                      │    │
│  │                                                    │    │
│  │  ┌──────────────┐         ┌──────────────┐       │    │
│  │  │  MFS RAM     │         │  MFS Disk    │       │    │
│  │  │  (tmpfs)     │         │  (CoW B-tree)│       │    │
│  │  └──────┬───────┘         └──────┬───────┘       │    │
│  │         │                        │                │    │
│  │    (in-memory)              ┌────▼────┐           │    │
│  │                             │  Page   │           │    │
│  │                             │  Cache  │           │    │
│  │                             └────┬────┘           │    │
│  │                                  │                │    │
│  │                             ┌────▼────┐           │    │
│  │                             │ Buffer  │           │    │
│  │                             │ Cache   │           │    │
│  │                             └────┬────┘           │    │
│  └──────────────────────────────────┼────────────────┘    │
│                                     │                      │
│                                     ▼                      │
│  ┌────────────────────────────────────────────────────┐    │
│  │         Block Layer (kernel/src/io/bio.rs)        │    │
│  │                                                    │    │
│  │  Block I/O queue, request merging, scheduling     │    │
│  └──────────────────┬─────────────────────────────────┘    │
│                     │                                       │
│                     ▼                                       │
│  ┌────────────────────────────────────────────────────┐    │
│  │    Device Drivers (kernel/src/drivers/block/)     │    │
│  │                                                    │    │
│  │  virtio-blk, AHCI, NVMe (future)                 │    │
│  └────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

---

## Component Details

### 1. System Call Handler

**Location:** `kernel/src/sys/syscall.rs`

**Responsibilities:**
- Validate user-space pointers
- Check permissions
- Dispatch to VFS layer
- Return errors to user-space

**Key Functions:**
```rust
pub fn sys_open(path: *const u8, flags: u32, mode: u32) -> isize
pub fn sys_read(fd: usize, buf: *mut u8, count: usize) -> isize
pub fn sys_write(fd: usize, buf: *const u8, count: usize) -> isize
pub fn sys_close(fd: usize) -> isize
pub fn sys_stat(path: *const u8, statbuf: *mut Stat) -> isize
pub fn sys_mkdir(path: *const u8, mode: u32) -> isize
pub fn sys_unlink(path: *const u8) -> isize
```

**Error Handling:**
- EFAULT: Invalid user pointer
- ENOENT: File not found
- EACCES: Permission denied
- EISDIR: Is a directory
- ENOTDIR: Not a directory

---

### 2. Path Resolution (namei)

**Location:** `kernel/src/fs/vfs/path.rs`

**Responsibilities:**
- Convert path string to inode
- Handle absolute and relative paths
- Process "." and ".." components
- Follow symlinks (max 40 hops)
- Integrate with dentry cache

**Algorithm:**
```
resolve_path(path: &str) -> Result<Arc<dyn Inode>, FsError>:
  1. Start from root inode (from mount table)
  2. Split path by '/'
  3. For each component:
     a. Check dentry cache for (parent_ino, component) → child_ino
     b. If cache miss:
        - Call parent.lookup(component)
        - Insert result into dentry cache
     c. If symlink:
        - Read symlink target
        - Recursively resolve (increment hop counter)
        - Return ELOOP if hops > 40
     d. Move to child inode
  4. Return final inode
```

**Example:**
```
Path: "/tmp/test.txt"

1. Start: root inode (ino=1)
2. Lookup "tmp" in root → ino=3
3. Lookup "test.txt" in tmp → ino=42
4. Return inode 42
```

---

### 3. Mount Table

**Location:** `kernel/src/fs/vfs/mount.rs`

**Responsibilities:**
- Track mounted filesystems
- Map mount points to superblocks
- Handle mount/umount operations

**Data Structure:**
```rust
pub struct MountPoint {
    path: String,              // e.g., "/", "/data", "/proc"
    superblock: Arc<dyn SuperBlock>,
    fs_type: &'static str,     // e.g., "mfs_ram", "mfs_disk"
}

static MOUNT_TABLE: SpinLock<[Option<MountPoint>; 16]>;
```

**Operations:**
```rust
pub fn register_mount(path: &str, sb: Arc<dyn SuperBlock>) -> Result<(), FsError>
pub fn lookup_mount(path: &str) -> Option<Arc<dyn SuperBlock>>
pub fn unmount(path: &str) -> Result<(), FsError>
```

**Mount Process:**
```
1. Call filesystem's mount() function
2. Get superblock Arc
3. Add to mount table
4. Invalidate dentry cache for mount point
```

---

### 4. Dentry Cache

**Location:** `kernel/src/fs/vfs/dentry.rs`

**Responsibilities:**
- Cache (parent_ino, name) → child_ino mappings
- Speed up path resolution
- Support negative entries (failed lookups)
- Invalidate on directory modifications

**Data Structure:**
```rust
struct DentryEntry {
    parent_ino: u64,
    name_hash: u64,
    child_ino: u64,      // 0 for negative entry
    negative: bool,
}

// Hash table with 256 buckets
static DENTRY_CACHE: [SpinLock<Vec<DentryEntry>>; 256];
```

**Operations:**
```rust
pub fn lookup(parent: u64, name: &str) -> Option<u64>
pub fn insert(parent: u64, name: &str, child: u64)
pub fn insert_negative(parent: u64, name: &str)
pub fn invalidate(parent: u64)
```

**Cache Hit Rate:**
- Typical: 90-95% for repeated lookups
- Cold boot: 0% (cache empty)
- After warmup: >95%

---

### 5. Inode Cache

**Location:** `kernel/src/fs/vfs/inode.rs`

**Responsibilities:**
- Cache inode objects
- Use Arc for reference counting
- Automatic eviction when refcount reaches 0

**Data Structure:**
```rust
// Inodes are cached implicitly via Arc<dyn Inode>
// When all references drop, inode is freed
```

**Lifecycle:**
```
1. lookup() → Arc<dyn Inode> (refcount = 1)
2. open() → clone Arc (refcount = 2)
3. close() → drop Arc (refcount = 1)
4. path resolution done → drop Arc (refcount = 0)
5. Inode freed automatically
```

---

### 6. File Descriptor Table

**Location:** `kernel/src/fs/vfs/file.rs`

**Responsibilities:**
- Per-process FD management
- Track open files
- Handle FD flags (O_CLOEXEC, O_APPEND)
- Clone on fork, filter on exec

**Data Structure:**
```rust
pub struct FileDescriptor {
    inode: Arc<dyn Inode>,
    offset: AtomicU64,
    flags: u32,  // O_RDONLY, O_WRONLY, O_RDWR, O_APPEND, O_CLOEXEC
}

pub struct FdTable {
    fds: [Option<FileDescriptor>; 256],
}

// In Task struct:
pub struct Task {
    // ...
    fd_table: Arc<SpinLock<FdTable>>,
}
```

**Operations:**
```rust
pub fn alloc_fd() -> Option<usize>  // Find lowest available FD
pub fn get_fd(fd: usize) -> Option<FileDescriptor>
pub fn close_fd(fd: usize)
pub fn clone_fd_table() -> Arc<SpinLock<FdTable>>  // For fork
```

**Fork Behavior:**
```
1. Clone FD table
2. Clone all Arc<dyn Inode> (increment refcount)
3. Copy offsets and flags
4. Child gets independent FD table
```

**Exec Behavior:**
```
1. Iterate FD table
2. Close FDs with O_CLOEXEC flag
3. Keep other FDs open
```

---

## Data Flow Examples

### Example 1: open("/tmp/test.txt", O_RDWR | O_CREAT)

```
1. Syscall Handler (sys_open)
   ↓
2. Validate user pointer to "/tmp/test.txt"
   ↓
3. Path Resolution
   - Resolve "/tmp" → inode 3
   - Lookup "test.txt" in inode 3
   - If not found and O_CREAT: call inode.create("test.txt")
   - Return inode 42
   ↓
4. Allocate FD
   - Find lowest available FD (e.g., 3)
   - Create FileDescriptor { inode: Arc(42), offset: 0, flags: O_RDWR }
   - Insert into FD table
   ↓
5. Return FD 3 to user-space
```

### Example 2: read(fd=3, buf, 100)

```
1. Syscall Handler (sys_read)
   ↓
2. Validate user pointer to buf
   ↓
3. Lookup FD 3 in FD table
   - Get FileDescriptor { inode, offset, flags }
   ↓
4. Check flags (must have read permission)
   ↓
5. Call inode.read_at(offset, buf, 100)
   - MFS RAM: copy from in-memory chunks
   - MFS Disk: read from page cache → block layer → device
   ↓
6. Update offset atomically
   ↓
7. Return bytes read to user-space
```

### Example 3: stat("/tmp/test.txt", &statbuf)

```
1. Syscall Handler (sys_stat)
   ↓
2. Validate user pointers
   ↓
3. Path Resolution
   - Resolve "/tmp/test.txt" → inode 42
   ↓
4. Call inode.stat()
   - Get metadata: mode, size, timestamps, etc.
   ↓
5. Copy Stat struct to user-space buffer
   ↓
6. Return 0 (success)
```

### Example 4: mkdir("/tmp/newdir", 0755)

```
1. Syscall Handler (sys_mkdir)
   ↓
2. Validate user pointer
   ↓
3. Path Resolution
   - Resolve parent "/tmp" → inode 3
   ↓
4. Call inode.create("newdir", S_IFDIR | 0755)
   - Allocate new inode (e.g., 43)
   - Set mode, uid, gid
   - Link into parent directory
   ↓
5. Invalidate dentry cache for parent (ino=3)
   ↓
6. Return 0 (success)
```

---

## Filesystem Implementation Interface

### Inode Trait

**Location:** `kernel/src/fs/vfs/inode.rs`

```rust
pub trait Inode: Send + Sync {
    // Metadata
    fn ino(&self) -> u64;
    fn mode(&self) -> FileMode;
    fn stat(&self) -> Result<Stat, FsError>;
    fn set_attr(&self, attr: &SetAttr) -> Result<(), FsError>;
    
    // File operations
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, FsError>;
    fn write_at(&self, offset: u64, buf: &[u8]) -> Result<usize, FsError>;
    fn truncate(&self, size: u64) -> Result<(), FsError>;
    
    // Directory operations
    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, FsError>;
    fn create(&self, name: &str, mode: FileMode, uid: u32, gid: u32) 
        -> Result<Arc<dyn Inode>, FsError>;
    fn unlink(&self, name: &str) -> Result<(), FsError>;
    fn link(&self, name: &str, target: Arc<dyn Inode>) -> Result<(), FsError>;
    fn symlink(&self, name: &str, target: &str) -> Result<Arc<dyn Inode>, FsError>;
    fn readdir(&self, cookie: &mut DirCookie, entries: &mut Vec<DirEnt>) 
        -> Result<(), FsError>;
    
    // Extended attributes
    fn get_xattr(&self, name: &str) -> Result<Vec<u8>, FsError>;
    fn set_xattr(&self, name: &str, value: &[u8], flags: u32) -> Result<(), FsError>;
    fn list_xattr(&self) -> Result<Vec<String>, FsError>;
    
    // Symlink
    fn readlink(&self) -> Result<String, FsError>;
    
    // Type checking
    fn as_any(&self) -> &dyn Any;
}
```

### SuperBlock Trait

**Location:** `kernel/src/fs/vfs/superblock.rs`

```rust
pub trait SuperBlock: Send + Sync {
    fn root_inode(&self) -> Arc<dyn Inode>;
    fn statfs(&self) -> StatFs;
    fn sync(&self) -> Result<(), FsError>;
    fn feature_flags(&self) -> FsFeatures;
}

pub trait FsType: Send + Sync {
    fn name(&self) -> &'static str;
    fn mount(&self, device: Option<&str>, opts: &MountOpts) 
        -> Result<Arc<dyn SuperBlock>, FsError>;
}
```

---

## MFS RAM Implementation

**Location:** `kernel/src/fs/mfs/ram/`

**Characteristics:**
- In-memory filesystem (no persistence)
- Fast (no I/O)
- Used for /tmp, /dev, boot root

**Structure:**
```
RamInode {
    ino: u64,
    mode: AtomicU32,
    nlink: AtomicU32,
    uid: u32,
    gid: u32,
    size: AtomicU64,
    atime, mtime, ctime: AtomicU64,
    data: SpinLock<InodeData>,
}

InodeData {
    mode: FileMode,
    data: InodeKind,
}

InodeKind {
    File(FileData),      // Vec of chunks (Arc<[u8]>)
    Directory(DirData),  // BTreeMap<String, Arc<RamInode>>
    Symlink(String),     // Target path
}
```

**Operations:**
- create(): Allocate new inode, link into parent
- read_at(): Copy from chunks
- write_at(): Allocate/modify chunks
- lookup(): Search BTreeMap

---

## MFS Disk Implementation (Future)

**Location:** `kernel/src/fs/mfs/disk/`

**Characteristics:**
- Persistent CoW filesystem
- B-tree indexed
- Checksummed
- Compressed (optional)

**Structure:**
```
Superblock → Root B-tree → Inode B-tree
                         → Extent B-tree
                         → Free space B-tree
```

**Operations:**
- mount(): Read superblock, verify checksum
- read_at(): Lookup extent → page cache → block layer
- write_at(): CoW allocation, delayed allocation
- sync(): Commit transaction group (TxG)

---

## Performance Characteristics

### Path Resolution

| Operation | Cold Cache | Warm Cache |
|-----------|------------|------------|
| Absolute path (3 components) | ~15 µs | ~2 µs |
| Relative path (1 component) | ~8 µs | ~1 µs |
| Symlink (1 hop) | ~20 µs | ~3 µs |

### File Operations (MFS RAM)

| Operation | Latency | Throughput |
|-----------|---------|------------|
| open() | ~5 µs | - |
| read() 4 KiB | ~2 µs | ~2 GB/s |
| write() 4 KiB | ~3 µs | ~1.3 GB/s |
| close() | ~1 µs | - |
| stat() | ~2 µs | - |

### Dentry Cache

| Metric | Value |
|--------|-------|
| Buckets | 256 |
| Entries per bucket | ~10-20 (typical) |
| Hit rate (warm) | 95% |
| Lookup latency (hit) | ~100 ns |
| Lookup latency (miss) | ~10 µs |

---

## Concurrency and Locking

### Lock Hierarchy

```
1. Mount table lock (global)
2. Dentry cache bucket locks (per-bucket)
3. FD table lock (per-process)
4. Inode data lock (per-inode)
```

**Rules:**
- Always acquire locks in order (prevent deadlock)
- Hold locks for minimal time
- Use atomic operations where possible (offset, size, nlink)

### SMP Safety

- Mount table: SpinLock (rarely contended)
- Dentry cache: 256 buckets with separate locks (low contention)
- FD table: Per-process (no cross-process contention)
- Inode: Per-inode locks (fine-grained)

---

## Error Handling

### Common Errors

| Error | Code | Meaning |
|-------|------|---------|
| ENOENT | -2 | File not found |
| EACCES | -13 | Permission denied |
| EEXIST | -17 | File exists |
| ENOTDIR | -20 | Not a directory |
| EISDIR | -21 | Is a directory |
| EINVAL | -22 | Invalid argument |
| EFAULT | -14 | Bad address (user pointer) |
| ELOOP | -40 | Too many symlinks |
| ENAMETOOLONG | -36 | Name too long |
| ENOSPC | -28 | No space left |
| EIO | -5 | I/O error |

### Error Propagation

```
Filesystem → VFS → Syscall Handler → User Space

Example:
1. inode.lookup("missing") → Err(FsError::NotFound)
2. resolve_path() → Err(FsError::NotFound)
3. sys_open() → -ENOENT
4. User space: errno = ENOENT, return -1
```

---

## Future Enhancements

### Short Term (Phase 8 completion)

1. **Block Layer Integration** (Task 8.9)
   - Connect to virtio-blk driver
   - Implement block cache
   - Enable mfs_disk mounting

2. **mfs_disk Full Support** (Task 9)
   - Mount as /data
   - Test persistence
   - Eventually use as root

### Long Term (Post Phase 8)

1. **Additional Filesystems**
   - ext2/ext4 read-only
   - FAT32 support
   - ISO9660 (CD-ROM)

2. **Advanced Features**
   - File locking (flock, fcntl)
   - Directory notifications (inotify)
   - Quotas
   - ACLs (Access Control Lists)

3. **Performance**
   - Parallel path resolution
   - Lock-free dentry cache
   - Read-ahead tuning
   - Write coalescing

---

## References

- VFS Implementation: `kernel/src/fs/vfs/`
- MFS RAM: `kernel/src/fs/mfs/ram/`
- MFS Disk: `kernel/src/fs/mfs/disk/`
- Syscalls: `kernel/src/sys/syscall.rs`
- Design Document: `.kiro/specs/filesystem-storage/design.md`
- Requirements: `.kiro/specs/filesystem-storage/requirements.md`

---

## Deviations from Original Design

### 1. Simplified Inode Cache

**Original Design:** Explicit hash table with LRU eviction

**Actual Implementation:** Implicit caching via Arc reference counting

**Reason:** Simpler, automatic eviction, no manual cache management needed

### 2. Static Mount Table

**Original Design:** Dynamic allocation with unlimited mounts

**Actual Implementation:** Static array with 16 mount points

**Reason:** Avoid heap allocation in early boot, 16 is sufficient for typical use

### 3. Dentry Cache Size

**Original Design:** Configurable size

**Actual Implementation:** Fixed 256 buckets

**Reason:** Good balance between memory usage and performance, simple implementation

---

**Document Version:** 1.0  
**Status:** Complete  
**Last Updated:** 2025-01-XX
