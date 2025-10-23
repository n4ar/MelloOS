# Filesystem Cache Implementation

This directory contains the implementation of the filesystem caching subsystem for MelloOS, including page cache, buffer cache, writeback coalescing, and dirty page throttling.

## Components

### Page Cache (`page_cache.rs`)

The page cache maintains per-file cached pages with adaptive read-ahead:

- **Per-file caching**: Each file has its own cache with up to 256 pages
- **Adaptive read-ahead**: Starts at 2 pages, grows to 32 pages on sequential access
- **Sequential detection**: Tracks access patterns and adjusts read-ahead window
- **Dirty page tracking**: Tracks which pages have been modified
- **LRU eviction**: Evicts least recently used pages when cache is full

**Key Features:**
- Static array-based implementation (no dynamic allocation)
- Lock-free atomic operations for counters
- Per-file RwLock for page access
- Global page cache managing up to 64 files

### Buffer Cache (`buffer_cache.rs`)

The buffer cache maintains cached filesystem metadata blocks:

- **Hash table**: Keyed by (device_id, block_number)
- **Metadata blocks**: Up to 4 KiB per buffer
- **Dirty tracking**: Tracks modified buffers
- **LRU eviction**: Evicts least recently used buffers

**Key Features:**
- Static array-based implementation (512 buffers max)
- Per-buffer locking with RwLock
- Device-level invalidation support

### Writeback Coalescing (`writeback.rs`)

Implements efficient writeback of dirty pages:

- **Batching**: Groups adjacent dirty pages (128 KiB - 1 MiB per batch)
- **Deadline scheduling**: Default 30-second writeback deadline
- **Sync support**: Immediate flush on sync syscall
- **Coalescing algorithm**: Merges adjacent pages for efficient I/O

**Key Features:**
- Configurable batch sizes and deadlines
- Background flusher thread support (when scheduler ready)
- Force flush mechanism for sync operations

### Dirty Page Throttling (`throttle.rs`)

Prevents excessive dirty pages from exhausting memory:

- **Per-filesystem limits**: Default 10% of cache per filesystem
- **Global limits**: Default 20% of total RAM
- **Writer slowdown**: Throttles writers when limits exceeded
- **Tunable thresholds**: Configurable via mount options

**Key Features:**
- Per-filesystem and global tracking
- Atomic counters for dirty pages
- Throttle statistics (count of throttle events)
- Support for up to 16 filesystems

## Memory-Mapped Files (`mm/mmap.rs`)

Implements mmap support for file-backed memory mappings:

- **mmap syscall**: Maps files into process address space
- **msync syscall**: Synchronizes mapped regions with files
- **mprotect syscall**: Changes protection of mapped regions
- **Coherence**: Integrates with page cache for consistency

**Key Features:**
- POSIX-compatible flags (PROT_*, MAP_*)
- Per-process mapping tables (256 mappings max)
- Support for shared and private mappings
- Anonymous mapping support

## Block I/O Queue (`io/bio.rs`)

Manages block I/O operations with queue depth management:

- **Queue depth**: Target 32-128 for NVMe, 32 for SATA
- **Batch submissions**: Submit multiple I/Os efficiently
- **TRIM support**: SSD optimization with batch TRIM
- **Priority support**: Prioritize critical I/O operations

**Key Features:**
- Per-device queues (up to 16 devices)
- Request tracking with unique IDs
- Completion tracking
- Support for Read, Write, Flush, and Trim operations

## Integration Points

### With VFS
- Page cache will be integrated with file read/write operations
- Buffer cache will be used for metadata (inodes, directory entries)
- mmap will use VFS file descriptors

### With Block Devices
- BIO queue will interface with block device drivers
- Writeback will use BIO queue for efficient I/O
- TRIM operations will be sent to supporting devices

### With Memory Management
- mmap integrates with page tables
- Page cache uses physical memory frames
- Coherence maintained between mmap and file I/O

## Testing

Test files are located in `tests/`:
- `fs_cache_behavior.rs`: Cache hit/miss, eviction tests
- `fs_cache_perf.rs`: Read-ahead, writeback performance tests
- `fs_mmap_coherence.rs`: mmap coherence and msync tests

## Implementation Notes

### No Dynamic Allocation
All components use static arrays instead of dynamic allocation (Vec, BTreeMap) because the kernel doesn't have the `alloc` crate set up yet. This means:
- Fixed maximum limits (files, pages, buffers, mappings)
- Slightly less efficient than dynamic structures
- But simpler and more predictable memory usage

### Atomic Operations
Extensive use of atomic operations for:
- Reference counting
- Dirty page tracking
- Statistics counters
- Lock-free fast paths

### SMP Safety
All components are designed for multi-core safety:
- Per-file/per-device locking
- Atomic operations for shared state
- No global locks on fast paths

## Future Enhancements

1. **Dynamic Allocation**: Once kernel has alloc crate, replace static arrays
2. **Background Flusher**: Implement when task scheduler supports kernel threads
3. **Direct I/O**: Bypass cache for large sequential I/O
4. **Page Cache Pressure**: Integrate with memory pressure system
5. **NUMA Awareness**: Per-NUMA-node caches for scalability
6. **Compression**: Transparent page compression for space savings

## Requirements Satisfied

This implementation satisfies requirements from the filesystem storage spec:
- R4.1: Page cache with LRU eviction
- R4.2: Adaptive read-ahead (2-32 pages)
- R4.3: Write-back coalescing (128-1024 KiB)
- R4.4: Dirty page throttling
- R4.5: mmap coherence with file I/O
- R5.1-R5.5: mmap, msync, mprotect syscalls
- R9.4: Block I/O queue with TRIM support
