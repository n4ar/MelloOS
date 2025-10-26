# Filesystem & Storage Design Document

## Overview

This document describes the design of Phase 8: Filesystem & Storage for MelloOS. The implementation provides a robust Virtual File System (VFS) layer with two filesystem implementations: mfs_ram (in-memory) and mfs_disk (persistent Copy-on-Write). The design emphasizes Linux ABI compatibility, high performance comparable to modern filesystems like APFS, and data integrity through checksums and atomic transactions.

### Design Goals

1. **Unified VFS Interface**: Trait-based abstraction allowing multiple filesystem types to coexist
2. **Performance**: Achieve macOS/APFS-level responsiveness through intelligent caching and I/O optimization
3. **Data Integrity**: Checksums, atomic transactions, and crash recovery for reliable storage
4. **Linux Compatibility**: POSIX-compliant metadata, syscalls, and data structures for userspace compatibility
5. **Scalability**: Support for SMP-safe operations with per-CPU optimizations where appropriate

### Key Components

- **VFS Layer**: Path resolution, dentry/inode caching, mount management, file descriptor tables
- **Page Cache**: Adaptive read-ahead, write-back coalescing, mmap coherence
- **mfs_ram**: Fast in-memory filesystem for boot and temporary storage
- **mfs_disk**: Persistent CoW filesystem with B-tree indexing, checksums, and optional compression
- **Syscall Interface**: Comprehensive POSIX-compatible filesystem syscalls
- **Userspace Utilities**: Standard tools (ls, cat, mkdir, mount, etc.)

## Architecture

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Userspace Processes                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │   ls     │  │   cat    │  │  mello-sh│  │  mount   │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
└───────┼─────────────┼─────────────┼─────────────┼──────────┘
        │             │             │             │
        └─────────────┴─────────────┴─────────────┘
                      │ Syscall Interface
        ┌─────────────┴─────────────────────────────┐
        │         VFS Layer (kernel/src/fs/)        │
        │  ┌──────────────────────────────────────┐ │
        │  │  Path Resolver & Dentry Cache        │ │
        │  ├──────────────────────────────────────┤ │
        │  │  Inode Cache & File Descriptor Table │ │
        │  ├──────────────────────────────────────┤ │
        │  │  Mount Table & Superblock Registry   │ │
        │  └──────────────────────────────────────┘ │
        └───────────────┬───────────────────────────┘
                        │
        ┌───────────────┴───────────────┐
        │                               │
┌───────▼────────┐            ┌─────────▼────────┐
│  Page Cache    │            │  Buffer Cache    │
│  (file data)   │            │  (metadata)      │
└───────┬────────┘            └─────────┬────────┘
        │                               │
        └───────────────┬───────────────┘
                        │
        ┌───────────────┴───────────────┐
        │                               │
┌───────▼────────┐            ┌─────────▼────────┐
│   mfs_ram      │            │   mfs_disk       │
│  (in-memory)   │            │  (persistent)    │
│                │            │                  │
│  - Arc/RwLock  │            │  - B-tree        │
│  - Ordered Map │            │  - CoW           │
│  - Chunks      │            │  - Checksums     │
└────────────────┘            │  - TxG           │
                              │  - Compression   │
                              └─────────┬────────┘
                                        │
                              ┌─────────▼────────┐
                              │  Block I/O (BIO) │
                              │  - Queue Mgmt    │
                              │  - TRIM Support  │
                              └─────────┬────────┘
                                        │
                              ┌─────────▼────────┐
                              │  Block Devices   │
                              │  (virtio-blk,    │
                              │   NVMe, etc.)    │
                              └──────────────────┘
```

### Module Organization


```
kernel/src/fs/
├── mod.rs                    # Filesystem subsystem initialization
├── syscalls.rs               # Syscall implementations
├── vfs/                      # Virtual File System layer
│   ├── mod.rs
│   ├── inode.rs             # Inode trait and types
│   ├── file.rs              # File operations and descriptor management
│   ├── dentry.rs            # Directory entry cache
│   ├── path.rs              # Path resolution
│   ├── mount.rs             # Mount table and operations
│   └── superblock.rs        # Superblock trait and registry
├── cache/                    # Caching subsystem
│   ├── page_cache.rs        # File data page cache
│   └── buffer_cache.rs      # Metadata buffer cache
├── mfs/                      # MelloFS implementations
│   ├── ram/                 # In-memory filesystem
│   │   ├── mod.rs
│   │   ├── super.rs         # Superblock implementation
│   │   ├── inode.rs         # Inode implementation
│   │   ├── dir.rs           # Directory operations
│   │   ├── file.rs          # File operations
│   │   └── xattr.rs         # Extended attributes
│   └── disk/                # Persistent filesystem
│       ├── mod.rs
│       ├── super.rs         # Superblock and mount
│       ├── btree.rs         # B-tree implementation
│       ├── keys.rs          # Key type definitions
│       ├── extent.rs        # Extent management
│       ├── allocator.rs     # Space allocation
│       ├── txg.rs           # Transaction groups
│       ├── journal.rs       # Optional intent log
│       ├── replay.rs        # Crash recovery
│       ├── checksum.rs      # Checksum algorithms
│       ├── compress.rs      # Compression (LZ4/Zstd)
│       ├── dir.rs           # Directory operations
│       ├── file.rs          # File operations
│       └── xattr.rs         # Extended attributes
├── devfs.rs                  # Device filesystem (stub)
└── procfs.rs                 # Process filesystem (minimal)

kernel/src/mm/
└── mmap.rs                   # Memory-mapped file support

kernel/src/io/
└── bio.rs                    # Block I/O queue and TRIM
```


## Components and Interfaces

### VFS Layer

#### Core Traits

**FsType Trait** - Filesystem type registration and mounting

```rust
pub trait FsType: Send + Sync {
    fn name(&self) -> &'static str;
    fn mount(
        &self,
        dev: Option<Arc<dyn BlockDevice>>,
        opts: MountOpts
    ) -> Result<Arc<dyn SuperBlock>>;
}
```

**SuperBlock Trait** - Per-filesystem instance

```rust
pub trait SuperBlock: Send + Sync {
    fn root(&self) -> Arc<dyn Inode>;
    fn statfs(&self) -> StatFs;
    fn sync(&self) -> Result<()>;
    fn feature_flags(&self) -> FsFeatures;
}
```

**Inode Trait** - File system object operations

```rust
pub trait Inode: Send + Sync {
    // Metadata
    fn ino(&self) -> u64;
    fn mode(&self) -> FileMode;
    fn nlink(&self) -> u32;
    fn uid_gid(&self) -> (u32, u32);
    fn size(&self) -> u64;
    fn stat(&self) -> Result<Stat>;
    fn set_attr(&self, attr: SetAttr) -> Result<()>;
    
    // Directory operations
    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>>;
    fn create(&self, name: &str, mode: FileMode, uid: u32, gid: u32) 
        -> Result<Arc<dyn Inode>>;
    fn unlink(&self, name: &str) -> Result<()>;
    fn link(&self, name: &str, target: Arc<dyn Inode>) -> Result<()>;
    fn symlink(&self, name: &str, target: &str) -> Result<Arc<dyn Inode>>;
    fn readdir(&self, cookie: &mut DirCookie, sink: &mut dyn FnMut(DirEnt)) 
        -> Result<()>;
    
    // File operations
    fn read_at(&self, off: u64, dst: &mut [u8]) -> Result<usize>;
    fn write_at(&self, off: u64, src: &[u8]) -> Result<usize>;
    fn truncate(&self, new_size: u64) -> Result<()>;
    
    // Extended attributes
    fn set_xattr(&self, k: &str, v: &[u8]) -> Result<()>;
    fn get_xattr(&self, k: &str, out: &mut [u8]) -> Result<usize>;
}
```


#### Path Resolution

**Design**: Iterative path walker with symlink loop detection

**Algorithm**:
1. Split path by '/' separator
2. Start from root or current working directory
3. For each component:
   - Skip empty components and single dots
   - Handle ".." by moving to parent
   - Lookup component in current directory
   - If symlink, read target and recurse (with hop counter)
   - Check hop counter against limit (40)
4. Return final inode

**Optimizations**:
- Dentry cache lookup before inode lookup
- Negative dentry caching for non-existent paths
- RCU-like read-mostly locking for hot paths

#### Dentry Cache

**Structure**: Hash table keyed by (parent_ino, name_hash)

**Features**:
- LRU eviction when cache is full
- Negative entries for failed lookups
- Invalidation on directory modifications
- Per-entry reference counting

**Concurrency**: RwLock per hash bucket for fine-grained locking

#### Inode Cache

**Structure**: Hash table keyed by (sb_id, ino)

**Features**:
- Arc-based reference counting
- Automatic eviction when refcount reaches zero
- Dirty inode tracking for writeback
- Per-inode RwLock for metadata protection

#### File Descriptor Table

**Structure**: Per-process array of file descriptors

**File Descriptor Entry**:
```rust
struct FileDesc {
    inode: Arc<dyn Inode>,
    offset: AtomicU64,
    flags: FileFlags,  // O_RDONLY, O_WRONLY, O_RDWR, O_APPEND, O_CLOEXEC
    mode: FileMode,
}
```

**Operations**:
- Allocate lowest available FD number
- Clone on fork (with CLOEXEC handling)
- Close and release on process exit
- Thread-safe offset updates for concurrent access

#### Mount Table

**Structure**: Global table of mount points

**Mount Entry**:
```rust
struct MountPoint {
    path: String,
    sb: Arc<dyn SuperBlock>,
    flags: MountFlags,  // MS_RDONLY, MS_NOATIME, etc.
}
```

**Operations**:
- Mount: Create superblock, add to table, update dentry cache
- Umount: Sync filesystem, remove from table, invalidate dentries
- Lookup: Find mount point for given path during resolution


### Page Cache

**Purpose**: Cache file data pages in memory for fast access

**Structure**: Per-file radix tree (XArray) indexed by page number

**Key Features**:

1. **Adaptive Read-ahead**
   - Start with 2-page window
   - Grow to 32 pages on sequential access
   - Reset on random access pattern
   - Track per-file access history

2. **Write-back Coalescing**
   - Batch dirty pages (128-1024 KiB)
   - Background flusher thread
   - Deadline-based scheduling
   - Per-filesystem and global dirty limits

3. **Dirty Throttling**
   - Slow down writers when dirty ratio exceeds threshold
   - Per-filesystem limits prevent single FS from dominating
   - Global limit protects system memory

4. **mmap Coherence**
   - File write → invalidate or mark dirty mapped pages
   - msync → flush dirty pages to storage
   - mprotect → enforce file access permissions

**Concurrency**: Per-file RwLock for page tree, per-page atomic flags

### Buffer Cache

**Purpose**: Cache filesystem metadata blocks

**Structure**: Hash table keyed by (device, block_number)

**Features**:
- Separate from page cache for metadata isolation
- Checksum verification on read
- Write-through or write-back depending on filesystem
- Integration with transaction groups for atomic updates

**Concurrency**: Per-buffer RwLock, dirty list protected by spinlock


### MelloFS RAM (mfs_ram)

**Purpose**: Fast in-memory filesystem for boot, /tmp, and testing

**Design Philosophy**: Simplicity and speed over persistence

#### Data Structures

**Inode Storage**:
```rust
struct RamInode {
    ino: u64,
    mode: FileMode,
    uid: u32,
    gid: u32,
    nlink: AtomicU32,
    size: AtomicU64,
    atime: AtomicU64,
    mtime: AtomicU64,
    ctime: AtomicU64,
    data: RwLock<InodeData>,
}

enum InodeData {
    File(FileData),
    Dir(DirData),
    Symlink(String),
}
```

**File Data**: Chunked storage (16-64 KiB chunks)
```rust
struct FileData {
    chunks: Vec<Arc<[u8]>>,  // Immutable chunks for CoW
    xattrs: HashMap<String, Vec<u8>>,
}
```

**Directory Data**: Ordered map for fast lookup
```rust
struct DirData {
    entries: BTreeMap<String, u64>,  // name -> child_ino
    xattrs: HashMap<String, Vec<u8>>,
}
```

#### Operations

**File Read/Write**:
- Read: Copy from chunks to user buffer
- Write: Allocate new chunks, update size atomically
- CoW: Share chunks between hardlinks, copy on modification

**Directory Operations**:
- Lookup: O(log N) in BTreeMap
- Create: Insert entry, increment parent nlink
- Unlink: Remove entry, decrement target nlink, free if zero
- Readdir: Iterate BTreeMap, fill DirEnt structures

**Hardlinks**: Reference counting via nlink, shared inode

**Symlinks**: Store target path as string

**Extended Attributes**: HashMap per inode

#### Concurrency

- Per-inode RwLock for metadata and data
- Atomic operations for size, nlink, timestamps
- No global locks for scalability


### MelloFS Disk (mfs_disk)

**Purpose**: Persistent Copy-on-Write filesystem with data integrity

**Design Philosophy**: APFS-inspired with checksums, atomic transactions, and performance

#### On-Disk Layout

```
┌─────────────────────────────────────────────────────────┐
│ LBA 0-7: Boot Region (reserved, optional bootloader)   │
├─────────────────────────────────────────────────────────┤
│ LBA 8-15: Disk Header (MFS_HDR)                        │
├─────────────────────────────────────────────────────────┤
│ LBA 16-31: Primary Superblock                          │
├─────────────────────────────────────────────────────────┤
│ LBA 32-...: B-tree Nodes Area (metadata, CoW)          │
│   - Root B-tree (inodes, dirs, extents, xattrs)       │
│   - Allocator B-tree (free space tracking)            │
├─────────────────────────────────────────────────────────┤
│ LBA ...: Data Extents Area (file data, CoW)            │
├─────────────────────────────────────────────────────────┤
│ LBA ...: Journal/Intent Log (optional, circular)       │
├─────────────────────────────────────────────────────────┤
│ LBA -32..-1: Secondary Superblock (checkpoint)         │
└─────────────────────────────────────────────────────────┘
```

#### Superblock Structure

```rust
#[repr(C)]
struct MfsSuperblock {
    magic: u32,              // 0x4D465344 ("MFSD")
    version: u32,            // Format version (1)
    uuid: [u8; 16],          // Filesystem UUID
    txg_id: u64,             // Last committed transaction group
    root_btree_ptr: BtreePtr,    // Root of main B-tree
    alloc_btree_ptr: BtreePtr,   // Root of allocator B-tree
    features: u64,           // Feature flags
    block_size: u32,         // 4096, 8192, or 16384
    total_blocks: u64,       // Total filesystem blocks
    free_blocks: u64,        // Free blocks count
    checksum: u64,           // CRC32C of superblock
    // ... reserved space for future fields
}

struct BtreePtr {
    lba: u64,                // Physical block address
    length: u32,             // Block count
    checksum: u64,           // Expected checksum
    level: u8,               // Tree level
}
```

**Feature Flags**:
- COW: Copy-on-Write enabled
- CHECKSUM: Checksums enabled
- COMPRESSION: Compression support
- XATTR: Extended attributes
- INLINE_SMALL: Inline small files


#### B-tree Structure

**Node Layout**:
```rust
#[repr(C)]
struct BtreeNode {
    magic: u32,              // "MFN1"
    level: u16,              // 0 = leaf, >0 = internal
    nkeys: u16,              // Number of keys
    checksum: u64,           // CRC32C of node
    // Followed by:
    // - Key array
    // - Value array (leaf) or child pointer array (internal)
}
```

**Key Types**:

```rust
// Directory entry key
struct DirKey {
    parent_ino: u64,
    name_hash: u64,
    name_inline: [u8; 64],   // Optional inline name
}

// Inode metadata key
struct InodeKey {
    ino: u64,
}

// File extent key
struct ExtentKey {
    ino: u64,
    file_offset: u64,
}

// Extended attribute key
struct XattrKey {
    ino: u64,
    name_hash: u64,
}
```

**Value Types**:

```rust
// Inode value
struct InodeVal {
    mode: u16,
    uid: u32,
    gid: u32,
    nlink: u32,
    size: u64,
    atime_ns: u64,
    mtime_ns: u64,
    ctime_ns: u64,
    flags: u32,
    inline_len: u16,
    inline_data: [u8],       // Variable length, up to 4 KiB
}

// Directory entry value
struct DirVal {
    child_ino: u64,
    file_type: u8,           // DT_REG, DT_DIR, etc.
}

// Extent value
struct ExtentVal {
    phys_lba: u64,
    length: u32,
    flags: u16,              // Compression, checksum, etc.
    checksum: u64,           // Optional data checksum
}

// Extended attribute value
struct XattrVal {
    length: u32,
    data: [u8],              // Variable length
}
```

**Operations**:
- Search: Binary search in sorted key array
- Insert: Find position, split if full, propagate up
- Delete: Find key, merge if underfull, propagate up
- CoW: Allocate new node, copy and modify, update parent


#### Space Allocation

**Allocator B-tree**: Tracks free extents by (start_lba, length)

**Delayed Allocation**:
1. Write goes to page cache (dirty)
2. Allocation deferred until writeback
3. Coalesce adjacent dirty pages into large extents
4. Allocate contiguous space from free tree
5. Write extent, update file's extent tree

**Benefits**:
- Reduced fragmentation
- Better I/O coalescing
- Fewer metadata updates

**TRIM Support**:
- Track freed extents in TxG
- Issue TRIM commands after commit
- Batch TRIM operations for efficiency

**Inline Small Files**:
- Files ≤ 2-4 KiB stored in InodeVal
- Reduces I/O for small files
- Saves space (no extent overhead)

#### Transaction Groups (TxG)

**Purpose**: Atomic commit of multiple operations

**Lifecycle**:
1. **Open**: Accumulate dirty objects (inodes, B-tree nodes)
2. **Sync**: Flush page cache, prepare dirty list
3. **Commit**: 
   - Write all dirty B-tree nodes (CoW)
   - Write new root pointers
   - Write updated superblock with new txg_id
   - Barrier/flush to ensure ordering
4. **Complete**: Free old CoW blocks, issue TRIM

**Concurrency**:
- Multiple TxGs can be open simultaneously
- Only one TxG commits at a time
- Read operations see last committed TxG

**Tuning**:
- Time-based: Commit every 50-200 ms
- Size-based: Commit when dirty data exceeds threshold
- Sync-triggered: Explicit fsync/sync syscall


#### Checksums

**Algorithm**: CRC32C (hardware-accelerated on modern CPUs)

**Coverage**:
- **Metadata** (mandatory): All B-tree nodes, superblock
- **Data** (optional): File extents (per-extent flag)

**Verification**:
- On read: Compute checksum, compare with stored value
- On mismatch: Return EIO error, log corruption
- On write: Compute and store checksum

**Storage**:
- Superblock: checksum field
- B-tree nodes: checksum in header
- Extents: checksum in ExtentVal

#### Compression

**Algorithms**:
- **LZ4**: Fast compression/decompression, lower ratio
- **Zstd**: Higher compression ratio, more CPU

**Per-Extent Compression**:
- Compress extent before write
- Store compressed size in ExtentVal flags
- Decompress transparently on read

**Mount Options**:
- `compress=off`: No compression
- `compress=lz4`: Use LZ4 for all writes
- `compress=zstd`: Use Zstd for all writes

**Heuristics**:
- Skip compression for small extents (< 4 KiB)
- Skip if compressed size ≥ original size
- Prefer LZ4 for hot paths, Zstd for cold storage

#### Crash Recovery

**Replay Procedure**:
1. Read primary superblock
2. If checksum fails, try secondary superblock
3. Validate txg_id and root pointers
4. Walk B-tree from root, verify checksums
5. Rebuild free space map from extent tree
6. If journal exists, replay intent log
7. Mark filesystem clean

**Consistency Guarantees**:
- Superblock update is atomic (single sector write)
- All data/metadata written before superblock update
- CoW ensures old version remains valid until commit
- Checksum detects partial writes


#### Optional Journal

**Purpose**: Low-latency metadata operations (rename, unlink)

**Design**: Small circular log for intent records

**Intent Records**:
- RENAME: old_parent, old_name, new_parent, new_name
- UNLINK: parent, name, ino
- CREATE: parent, name, mode

**Workflow**:
1. Write intent to journal
2. Perform operation in B-tree (deferred to TxG)
3. On commit: Mark intent complete
4. On crash: Replay incomplete intents

**Benefits**:
- Faster metadata operations (no immediate TxG commit)
- Reduced latency for interactive workloads

**Trade-offs**:
- Additional complexity
- Small space overhead
- Optional feature (can disable)


## Data Models

### Linux-Compatible Structures

#### Stat Structure

```rust
#[repr(C)]
pub struct Stat {
    pub st_dev: u64,         // Device ID (filesystem ID)
    pub st_ino: u64,         // Inode number
    pub st_mode: u32,        // File type and mode
    pub st_nlink: u32,       // Number of hard links
    pub st_uid: u32,         // User ID
    pub st_gid: u32,         // Group ID
    pub st_rdev: u64,        // Device ID (if special file)
    pub st_size: u64,        // Total size in bytes
    pub st_blksize: u32,     // Block size for I/O
    pub st_blocks: u64,      // Number of 512B blocks
    pub st_atime_sec: i64,   // Access time seconds
    pub st_atime_nsec: i64,  // Access time nanoseconds
    pub st_mtime_sec: i64,   // Modification time seconds
    pub st_mtime_nsec: i64,  // Modification time nanoseconds
    pub st_ctime_sec: i64,   // Status change time seconds
    pub st_ctime_nsec: i64,  // Status change time nanoseconds
}
```

**Mode Bits** (POSIX):
- File type: S_IFREG, S_IFDIR, S_IFLNK, S_IFCHR, S_IFBLK, S_IFIFO, S_IFSOCK
- Permissions: S_IRUSR, S_IWUSR, S_IXUSR, S_IRGRP, S_IWGRP, S_IXGRP, S_IROTH, S_IWOTH, S_IXOTH
- Special: S_ISUID, S_ISGID, S_ISVTX

#### getdents64 Record

```rust
#[repr(C)]
pub struct Dirent64 {
    pub d_ino: u64,          // Inode number
    pub d_off: i64,          // Offset to next dirent
    pub d_reclen: u16,       // Length of this record
    pub d_type: u8,          // File type
    pub d_name: [u8],        // Null-terminated name (variable length)
}
```

**File Types**:
- DT_UNKNOWN = 0
- DT_FIFO = 1
- DT_CHR = 2
- DT_DIR = 4
- DT_BLK = 6
- DT_REG = 8
- DT_LNK = 10
- DT_SOCK = 12


### Internal Data Structures

#### FileMode

```rust
bitflags! {
    pub struct FileMode: u16 {
        // File types
        const S_IFREG  = 0o100000;  // Regular file
        const S_IFDIR  = 0o040000;  // Directory
        const S_IFLNK  = 0o120000;  // Symbolic link
        const S_IFCHR  = 0o020000;  // Character device
        const S_IFBLK  = 0o060000;  // Block device
        const S_IFIFO  = 0o010000;  // FIFO
        const S_IFSOCK = 0o140000;  // Socket
        
        // Permissions
        const S_IRUSR = 0o0400;
        const S_IWUSR = 0o0200;
        const S_IXUSR = 0o0100;
        const S_IRGRP = 0o0040;
        const S_IWGRP = 0o0020;
        const S_IXGRP = 0o0010;
        const S_IROTH = 0o0004;
        const S_IWOTH = 0o0002;
        const S_IXOTH = 0o0001;
        
        // Special bits
        const S_ISUID = 0o4000;
        const S_ISGID = 0o2000;
        const S_ISVTX = 0o1000;
    }
}
```

#### MountOpts

```rust
pub struct MountOpts {
    pub flags: MountFlags,
    pub block_size: u32,
    pub compress: Option<CompressionType>,
    pub noatime: bool,
    pub relatime: bool,
    pub nodiratime: bool,
    pub checksums: bool,
    pub cow: bool,
    pub trim: TrimMode,
}

bitflags! {
    pub struct MountFlags: u64 {
        const MS_RDONLY = 1 << 0;
        const MS_NOSUID = 1 << 1;
        const MS_NODEV = 1 << 2;
        const MS_NOEXEC = 1 << 3;
        const MS_SYNCHRONOUS = 1 << 4;
        const MS_REMOUNT = 1 << 5;
    }
}

pub enum CompressionType {
    None,
    Lz4,
    Zstd,
}

pub enum TrimMode {
    Off,
    Auto,
}
```


## Error Handling

### Error Codes

Standard Linux error codes returned by syscalls:

- **EINVAL**: Invalid argument (bad flags, invalid path)
- **ENOENT**: No such file or directory
- **EEXIST**: File exists (create with O_EXCL)
- **EACCES**: Permission denied
- **ENOTDIR**: Not a directory
- **EISDIR**: Is a directory (write to directory)
- **ENOSPC**: No space left on device
- **EIO**: I/O error (checksum mismatch, device error)
- **ENOMEM**: Out of memory
- **EFAULT**: Bad address (invalid userspace pointer)
- **ELOOP**: Too many symbolic links
- **ENAMETOOLONG**: File name too long (> 255 bytes)
- **EROFS**: Read-only filesystem
- **EMFILE**: Too many open files (per-process limit)
- **ENFILE**: Too many open files (system-wide limit)

### Error Handling Strategy

1. **Validation**: Check arguments early, return EINVAL for invalid input
2. **Logging**: Log errors with context (operation, path, error code)
3. **Recovery**: Attempt recovery where possible (retry I/O, use secondary superblock)
4. **Propagation**: Return errors to userspace with appropriate errno
5. **Consistency**: Never leave filesystem in inconsistent state on error

### Corruption Detection

1. **Checksum Mismatch**: Return EIO, log corruption, mark filesystem dirty
2. **Invalid Metadata**: Return EIO, refuse to mount if critical
3. **Unsupported Features**: Refuse to mount, return EINVAL
4. **Partial Writes**: Detect via checksums, use CoW to maintain consistency


## Testing Strategy

### Unit Tests

**VFS Layer**:
- Path resolution: empty paths, ".", "..", "//", symlink loops
- Dentry cache: hit/miss, negative entries, eviction
- Inode cache: reference counting, eviction
- FD table: allocation, CLOEXEC, fork behavior

**Page Cache**:
- Read-ahead: sequential detection, window growth
- Write-back: coalescing, throttling, deadline
- mmap coherence: write invalidation, msync flush

**mfs_ram**:
- File operations: read, write, truncate, append
- Directory operations: create, lookup, unlink, rename
- Hardlinks: reference counting, shared data
- Symlinks: target resolution, loop detection
- Extended attributes: set, get, list, size limits

**mfs_disk**:
- B-tree: insert, delete, split, merge, CoW
- Checksums: compute, verify, mismatch detection
- Allocator: allocation, freeing, coalescing, TRIM
- TxG: commit, atomicity, ordering
- Compression: compress, decompress, transparency
- Inline files: small file storage, threshold

### Integration Tests

**Syscall Interface**:
- open/close: flags, permissions, CLOEXEC
- read/write: offset, size, EOF, append mode
- stat/fstat: structure layout, field values
- getdents64: record layout, d_type, name handling
- mkdir/rmdir: creation, removal, non-empty directory
- link/symlink/unlink: hardlinks, symlinks, reference counting
- rename: same directory, cross-directory, atomicity
- chmod/chown: permission changes, ownership
- xattr: set, get, list, namespaces, size limits
- mmap/msync: mapping, coherence, protection

**Mount/Umount**:
- Mount: device, options, root access
- Umount: sync, busy filesystem, force
- Multiple mounts: different filesystems, mount points


### Crash Recovery Tests

**Scenarios**:
1. Power loss during write: Verify last committed TxG is consistent
2. Power loss during TxG commit: Verify rollback to previous TxG
3. Corrupted superblock: Verify secondary superblock recovery
4. Corrupted B-tree node: Verify checksum detection and error
5. Partial extent write: Verify checksum detection
6. Journal replay: Verify intent log replay after crash

**Methodology**:
- Fault injection: Simulate crashes at various points
- Automated testing: Script crash-loop scenarios
- Invariant checking: Verify filesystem consistency after recovery
- Leak detection: Check for lost blocks in free space map

### Performance Benchmarks

**Microbenchmarks**:
- Sequential read: 1-8 GiB files, measure throughput (target: ≥ 2.5 GB/s)
- Sequential write: 1-8 GiB files, measure throughput
- Random read: 4 KiB blocks, cache hit (target: ≥ 300k IOPS)
- Random write: 4 KiB blocks, measure IOPS
- Metadata operations: create, stat, unlink (measure latency)

**Macrobenchmarks**:
- Mount time: Cold and warm cache (target: < 150 ms cold, < 50 ms warm)
- ls directory: 100 entries, cached (target: < 5 ms)
- fork+exec: Small binary (target: P95 < 1.5 ms)
- Compile workload: Kernel build, measure time
- File copy: Large files, measure throughput

**Tuning Parameters**:
- Read-ahead window size
- Write-back batch size
- TxG commit interval
- Dirty page thresholds
- Compression thresholds
- TRIM cadence


### Fault Injection Tests

**Scenarios**:
- Out of space: Fill filesystem, verify ENOSPC handling
- Out of memory: Limit allocations, verify graceful degradation
- I/O errors: Simulate device errors, verify error propagation
- Checksum errors: Corrupt data, verify detection and EIO
- Invalid metadata: Corrupt structures, verify mount refusal

**Goals**:
- No data corruption on error
- Proper error codes returned
- Filesystem remains mountable after error
- Clear error messages in kernel log


## Performance Optimizations

### Hot Path Optimizations

1. **Dentry Cache Hit Path**
   - RCU-like read-mostly locking
   - Avoid allocations in lookup
   - Inline small name comparisons

2. **Page Cache Hit Path**
   - Zero-copy read when possible
   - Avoid lock contention with per-file locks
   - Batch page allocations

3. **Write Path**
   - Delayed allocation for coalescing
   - Batch dirty page writeback
   - Async I/O submission

4. **Metadata Path**
   - Buffer cache for B-tree nodes
   - Batch B-tree updates in TxG
   - Optional journal for low-latency ops

### Read-Ahead Strategy

**Sequential Detection**:
- Track last access offset per file
- If sequential: Grow window (2 → 4 → 8 → 16 → 32 pages)
- If random: Reset window to 2 pages

**Adaptive Window**:
- Start small to avoid waste
- Grow aggressively on sequential access
- Cap at 32 pages (128 KiB with 4K pages)

**Throttling**:
- Limit total read-ahead memory
- Prioritize active files
- Evict unused read-ahead pages first

### Write-Back Strategy

**Coalescing**:
- Batch adjacent dirty pages
- Target 128-1024 KiB per I/O
- Align to extent boundaries

**Scheduling**:
- Background flusher thread
- Deadline-based (30 seconds default)
- Sync-triggered immediate flush

**Throttling**:
- Per-filesystem dirty limit (e.g., 10% of cache)
- Global dirty limit (e.g., 20% of RAM)
- Slow down writers when limit exceeded


### SMP Scalability

**Per-CPU Structures**:
- Per-CPU page cache allocation pools
- Per-CPU dirty page lists
- Per-CPU statistics counters

**Lock Granularity**:
- Per-file locks (not global)
- Per-hash-bucket locks for caches
- Per-inode locks for metadata

**Lock-Free Paths**:
- Atomic reference counting
- RCU for read-heavy structures
- Seqlock for statistics

**Contention Avoidance**:
- Avoid cross-CPU synchronization
- Batch operations to reduce lock acquisitions
- Use try-lock for opportunistic operations

### I/O Optimization

**Queue Depth**:
- Submit multiple I/Os in parallel
- Target queue depth of 32-128 for NVMe
- Batch submissions for efficiency

**Alignment**:
- Align I/Os to device block size
- Prefer large, aligned I/Os
- Avoid read-modify-write cycles

**TRIM**:
- Batch TRIM commands
- Issue after TxG commit
- Avoid blocking foreground I/O

**Direct I/O** (future):
- Bypass page cache for large sequential I/O
- Reduce memory pressure
- Lower latency for database workloads


## Security Considerations

### Permission Checking

**File Access**:
- Check mode bits against process uid/gid
- Enforce read/write/execute permissions
- Handle SUID/SGID/sticky bits

**Directory Access**:
- Require execute permission for traversal
- Require write permission for modifications
- Enforce sticky bit for /tmp-like directories

**Special Files**:
- Restrict device node creation to privileged processes
- Validate major/minor numbers
- Enforce access controls on device files

### Input Validation

**Path Validation**:
- Reject paths with null bytes
- Limit path length (4096 bytes)
- Limit component length (255 bytes)
- Detect and prevent symlink loops

**Name Validation**:
- Reject empty names
- Reject names with '/' or null bytes
- Enforce length limits

**Pointer Validation**:
- Verify userspace pointers before access
- Use copy_from_user/copy_to_user
- Return EFAULT for invalid pointers

### Resource Limits

**Per-Process Limits**:
- Maximum open files (RLIMIT_NOFILE)
- Maximum file size (RLIMIT_FSIZE)

**System-Wide Limits**:
- Maximum open files (system-wide)
- Maximum inodes
- Maximum dentries

**Filesystem Limits**:
- Maximum file size (2^63 bytes)
- Maximum filename length (255 bytes)
- Maximum path length (4096 bytes)
- Maximum xattr size (64 KiB per attribute)


## Implementation Phases

### Phase 8.1: VFS Core (Milestone M1)

**Components**:
- Trait definitions (FsType, SuperBlock, Inode)
- Dentry cache with LRU and negative entries
- Inode cache with reference counting
- Path resolver with symlink loop detection
- Mount table and mount/umount operations
- File descriptor table with CLOEXEC support

**Syscalls**:
- open, openat, close
- read, write, pread, pwrite, lseek
- fstat, stat, lstat
- getdents64
- mkdir, rmdir
- link, symlink, unlink
- renameat2

**Tests**: fs_vfs_correctness.rs, fs_fd_ops.rs, fs_dir_ops.rs

**Gate**: All VFS tests pass

### Phase 8.2: Page and Buffer Cache (Milestone M2)

**Components**:
- Page cache with radix tree per file
- Adaptive read-ahead (2-32 pages)
- Write-back coalescing (128-1024 KiB)
- Dirty page throttling
- Buffer cache for metadata
- mmap hooks for coherence

**Syscalls**:
- mmap, msync, mprotect (basic)

**Tests**: fs_cache_perf.rs, fs_cache_behavior.rs, fs_mmap_coherence.rs

**Gate**: Cache tests pass, read-ahead working

### Phase 8.3: mfs_ram (Milestone M3)

**Components**:
- In-memory inode storage with Arc/RwLock
- BTreeMap for directory entries
- Chunked file data storage
- Hardlink and symlink support
- Extended attributes
- statfs implementation

**Tests**: mfs_ram_correctness.rs, mfs_ram_perf.rs

**Gate**: Bootable root on RAM, all mfs_ram tests pass


### Phase 8.4: mfs_disk MVP (Milestone M4)

**Components**:
- On-disk format specification (docs/mfs_disk_format.md)
- Superblock read/write with checksums
- B-tree implementation (insert, delete, search, CoW)
- Key and value type definitions
- Extent manager
- Space allocator with free tree
- Transaction group commit and replay
- Basic file and directory operations

**Tests**: mfs_disk_meta.rs, mfs_disk_replay.rs

**Gate**: Mount/umount working, basic create/read/write/rename, crash recovery

### Phase 8.5: Integrity and Compression (Milestone M5)

**Components**:
- CRC32C checksum implementation
- Metadata checksum verification
- Optional data checksums
- LZ4 compression
- Zstd compression
- Per-extent compression flags
- Transparent decompression

**Tests**: mfs_disk_checksum.rs, mfs_disk_compress.rs

**Gate**: Checksums detect corruption, compression works transparently

### Phase 8.6: Complete Syscalls and Userland (Milestone M6)

**Components**:
- Complete mmap/msync/mprotect implementation
- Extended attribute syscalls (setxattr, getxattr, listxattr)
- Special file support (mknod, FIFO, socket nodes)
- chmod, chown, utimensat syscalls
- Userspace utilities: ls, cat, touch, mkdir, rm, mv, ln, df, stat, mount, umount

**Tests**: fs_stat_compat.rs, fs_xattr.rs, fs_syscalls_api.rs, userland_smoke.rs

**Gate**: All syscalls working, utilities functional, Linux ABI compatibility verified


### Phase 8.7: Performance and Robustness (Milestone M7)

**Components**:
- Performance tuning (read-ahead, write-back, TxG)
- TRIM optimization
- Compression threshold tuning
- Fault injection tests
- Performance benchmarks
- Stress testing

**Tests**: fs_seq_rand.rs, fork_exec_p95.rs, fs_faults.rs

**Gate**: Performance targets met, all tests pass, CI green

## Design Decisions and Rationale

### Why Copy-on-Write?

**Benefits**:
- Atomic updates (no partial writes)
- Crash consistency without journaling
- Efficient snapshots (future feature)
- Reduced write amplification

**Trade-offs**:
- Fragmentation over time (mitigated by delayed allocation)
- Metadata overhead (mitigated by B-tree efficiency)

### Why B-tree over Hash Table?

**Benefits**:
- Ordered iteration (efficient readdir)
- Range queries (future feature)
- Better space efficiency
- Predictable performance

**Trade-offs**:
- Slightly slower lookup than hash (O(log N) vs O(1))
- More complex implementation

### Why Delayed Allocation?

**Benefits**:
- Better extent coalescing
- Reduced fragmentation
- Fewer metadata updates
- Higher throughput

**Trade-offs**:
- Complexity in error handling
- Potential for ENOSPC on writeback


### Why CRC32C over SHA256?

**Benefits**:
- Hardware acceleration on modern CPUs
- Much faster (10-100x)
- Sufficient for detecting corruption

**Trade-offs**:
- Not cryptographically secure (not a goal)
- Weaker collision resistance (acceptable for filesystem use)

### Why Optional Compression?

**Benefits**:
- Space savings for compressible data
- Flexibility (can disable for performance)
- Per-extent granularity

**Trade-offs**:
- CPU overhead
- Complexity
- Variable extent sizes

**Rationale**: Modern CPUs have spare cycles, and storage is often the bottleneck. Optional compression provides flexibility.

### Why Transaction Groups over Traditional Journal?

**Benefits**:
- Simpler implementation
- Better performance (batch updates)
- Natural fit with CoW
- No separate journal space needed

**Trade-offs**:
- Higher latency for single operations (mitigated by optional intent log)
- More memory for dirty tracking

**Rationale**: TxG aligns well with CoW and provides good performance for most workloads. Optional journal addresses latency-sensitive cases.

## Future Enhancements

### Phase 9+ Features

1. **Snapshots**: Leverage CoW for instant snapshots
2. **Clones**: Share extents between files
3. **Deduplication**: Content-based deduplication
4. **Encryption**: Per-file or per-filesystem encryption
5. **Quotas**: Per-user and per-group quotas
6. **ACLs**: POSIX ACLs for fine-grained permissions
7. **Direct I/O**: Bypass page cache for large sequential I/O
8. **Async I/O**: io_uring integration
9. **AHCI/NVMe**: Additional block device drivers
10. **ext2/FAT32**: Read-only support for compatibility

## Summary

This design provides a robust, high-performance filesystem subsystem for MelloOS with:
- Clean VFS abstraction for multiple filesystem types
- Fast in-memory filesystem for boot and temporary storage
- Persistent CoW filesystem with data integrity and performance
- Linux ABI compatibility for userspace applications
- Comprehensive testing and validation strategy

The phased implementation approach ensures incremental progress with clear milestones and gates.
