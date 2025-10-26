# Implementation Plan

## Overview

This implementation plan breaks down Phase 8: Filesystem & Storage into discrete, actionable tasks. Each task builds incrementally on previous work, with clear testing gates at each milestone. The plan follows a bottom-up approach: VFS foundation → caching → in-memory filesystem → persistent filesystem → complete syscalls → performance tuning.

## Task List

- [x] 1. VFS Core Infrastructure (Milestone M1)
  - Implement trait-based VFS abstractions and core caching mechanisms
  - Wire basic filesystem syscalls
  - _Requirements: R1.1, R1.2, R1.3, R1.4, R1.5, R2.1, R2.2, R2.3, R2.4, R2.5, R3.1, R3.2, R3.3, R3.4, R3.5_

- [x] 1.1 Define VFS trait interfaces
  - Create `kernel/src/fs/vfs/inode.rs` with `Inode` trait including metadata, directory, file, and xattr operations
  - Create `kernel/src/fs/vfs/superblock.rs` with `FsType` and `SuperBlock` traits
  - Define `FileMode`, `Stat`, `DirEnt`, `DirCookie`, `SetAttr`, `StatFs`, `FsFeatures`, and `MountOpts` types
  - Ensure all types are Send + Sync for SMP safety
  - _Requirements: R1.1_

- [x] 1.2 Implement dentry cache
  - Create `kernel/src/fs/vfs/dentry.rs` with hash table keyed by (parent_ino, name_hash)
  - Implement LRU eviction policy when cache reaches capacity
  - Support negative dentry entries for failed lookups
  - Provide invalidation API for directory modifications
  - Use RwLock per hash bucket for fine-grained concurrency
  - _Requirements: R1.2_

- [x] 1.3 Implement inode cache
  - Create inode cache in `kernel/src/fs/vfs/inode.rs` with hash table keyed by (sb_id, ino)
  - Use Arc-based reference counting for automatic eviction
  - Track dirty inodes for writeback
  - Provide per-inode RwLock for metadata protection
  - _Requirements: R1.3_

- [x] 1.4 Implement path resolution
  - Create `kernel/src/fs/vfs/path.rs` with iterative path walker
  - Handle absolute and relative paths, empty components, "." and ".." segments
  - Implement symlink following with hop counter (limit: 40)
  - Return ELOOP when symlink limit exceeded
  - Integrate with dentry cache for fast lookups
  - _Requirements: R1.4, R3.3_

- [x] 1.5 Implement mount table
  - Create `kernel/src/fs/vfs/mount.rs` with global mount point table
  - Implement mount operation: create superblock, add to table, update dentry cache
  - Implement umount operation: sync filesystem, remove from table, invalidate dentries
  - Provide mount point lookup during path resolution
  - _Requirements: R1.5_


- [x] 1.6 Implement file descriptor table
  - Create `kernel/src/fs/vfs/file.rs` with per-process FD table
  - Implement FD allocation (lowest available number)
  - Support file descriptor flags: O_RDONLY, O_WRONLY, O_RDWR, O_APPEND, O_CLOEXEC
  - Implement thread-safe offset tracking with AtomicU64
  - Handle FD cloning on fork with CLOEXEC filtering
  - _Requirements: R2.1, R2.2_

- [x] 1.7 Wire basic filesystem syscalls
  - Implement in `kernel/src/fs/syscalls.rs`: open, openat, close
  - Implement: read, write, pread, pwrite, lseek, ftruncate
  - Implement: fstat, stat, lstat
  - Implement: getdents64 with Linux-compatible record layout
  - Implement: mkdir, rmdir, link, symlink, unlink, renameat2
  - Validate userspace pointers and return EFAULT for invalid addresses
  - Enforce file access permissions based on inode mode and process credentials
  - _Requirements: R2.3, R2.4, R2.5, R3.1, R3.2, R3.4, R3.5, R14.1, R14.2_

- [x] 1.8 Write VFS correctness tests
  - Create `tests/fs_vfs_correctness.rs` testing path resolution edge cases
  - Test empty paths, "//", "/./", "/../", symlink loops (> 40 hops)
  - Test cross-directory rename atomicity
  - Create `tests/fs_fd_ops.rs` testing FD operations
  - Test open flags, CLOEXEC behavior, fcntl operations
  - Create `tests/fs_dir_ops.rs` testing directory operations
  - Test getdents64 binary layout (ino:u64, off:i64, reclen:u16, d_type:u8, name)
  - Test name validation: empty names, null bytes, names > 255 bytes
  - _Requirements: R19.1_

- [x] 2. Page Cache and Buffer Management (Milestone M2)
  - Implement intelligent caching for file data and metadata
  - Support memory-mapped files with coherence
  - _Requirements: R4.1, R4.2, R4.3, R4.4, R4.5, R5.1, R5.2, R5.3, R5.4, R5.5_

- [x] 2.1 Implement page cache
  - Create `kernel/src/fs/cache/page_cache.rs` with per-file radix tree indexed by page number
  - Implement adaptive read-ahead: start with 2 pages, grow to 32 on sequential access
  - Track per-file access history for sequential detection
  - Reset read-ahead window on random access pattern
  - _Requirements: R4.1, R4.2_

- [x] 2.2 Implement write-back coalescing
  - Implement dirty page tracking in page cache
  - Create background flusher thread for writeback
  - Batch adjacent dirty pages into 128-1024 KiB I/O operations
  - Implement deadline-based scheduling (default: 30 seconds)
  - Support sync-triggered immediate flush
  - _Requirements: R4.3_

- [x] 2.3 Implement dirty page throttling
  - Implement per-filesystem dirty page limits (e.g., 10% of cache)
  - Implement global dirty page limit (e.g., 20% of RAM)
  - Slow down writers when limits exceeded
  - Provide tunable thresholds via mount options
  - _Requirements: R4.4_

- [x] 2.4 Implement buffer cache
  - Create `kernel/src/fs/cache/buffer_cache.rs` for metadata blocks
  - Use hash table keyed by (device, block_number)
  - Integrate with checksum verification on read
  - Support write-through or write-back depending on filesystem
  - Protect with per-buffer RwLock and dirty list spinlock
  - _Requirements: R4.1_

- [x] 2.5 Implement block I/O queue
  - Create `kernel/src/io/bio.rs` for block I/O management
  - Implement queue depth management (target: 32-128 for NVMe)
  - Support batch I/O submissions for efficiency
  - Add TRIM/DISCARD support hook for SSD optimization
  - _Requirements: R9.4_

- [x] 2.6 Implement mmap support
  - Create `kernel/src/mm/mmap.rs` for file-backed memory mappings
  - Implement mmap syscall with file descriptor and offset
  - Implement msync syscall for synchronizing mapped regions
  - Implement mprotect syscall for changing protection
  - Integrate with page cache for coherence
  - _Requirements: R5.1, R5.2, R5.3_

- [x] 2.7 Implement mmap coherence
  - Add hooks in page cache to invalidate or mark dirty mapped pages on file write
  - Ensure msync flushes dirty pages to storage
  - Enforce file access permissions on mapped regions
  - Handle concurrent file write and mmap access correctly
  - _Requirements: R4.5, R5.4, R5.5_

- [x] 2.8 Write cache and mmap tests
  - Create `tests/fs_cache_behavior.rs` testing cache hit/miss, eviction
  - Create `tests/fs_cache_perf.rs` testing read-ahead growth and writeback coalescing
  - Create `tests/fs_mmap_coherence.rs` testing write invalidation and msync flush
  - Verify dirty throttling prevents memory exhaustion
  - _Requirements: R19.2_


- [x] 3. MelloFS RAM Filesystem (Milestone M3)
  - Implement fast in-memory filesystem for boot and temporary storage
  - _Requirements: R6.1, R6.2, R6.3, R6.4, R6.5_

- [x] 3.1 Implement mfs_ram superblock
  - Create `kernel/src/fs/mfs/ram/super.rs` implementing SuperBlock trait
  - Register FsType with name "mfs_ram"
  - Implement mount operation creating in-memory root inode
  - Implement statfs returning total and available space
  - Implement sync operation (no-op for RAM filesystem)
  - _Requirements: R6.5_

- [x] 3.2 Implement mfs_ram inode structure
  - Create `kernel/src/fs/mfs/ram/inode.rs` with RamInode struct
  - Use Arc<RwLock<InodeData>> for concurrency
  - Store metadata: ino, mode, uid, gid, nlink (AtomicU32), size (AtomicU64), timestamps (AtomicU64)
  - Support file, directory, and symlink types
  - _Requirements: R6.1_

- [x] 3.3 Implement mfs_ram directory operations
  - Create `kernel/src/fs/mfs/ram/dir.rs` with BTreeMap for directory entries
  - Implement lookup with O(log N) complexity
  - Implement create, unlink, link operations
  - Implement readdir iterating BTreeMap entries
  - Update parent nlink on directory creation/removal
  - _Requirements: R6.2, R3.5_

- [x] 3.4 Implement mfs_ram file operations
  - Create `kernel/src/fs/mfs/ram/file.rs` with chunked storage (16-64 KiB chunks)
  - Implement read_at copying from chunks to user buffer
  - Implement write_at allocating new chunks and updating size atomically
  - Implement truncate adjusting chunk list and size
  - Use Arc<[u8]> for immutable chunks enabling CoW for hardlinks
  - _Requirements: R6.3_

- [x] 3.5 Implement mfs_ram extended attributes
  - Create `kernel/src/fs/mfs/ram/xattr.rs` with HashMap per inode
  - Implement set_xattr and get_xattr operations
  - Enforce maximum xattr name length (255 bytes)
  - Enforce maximum xattr value size (64 KiB)
  - Support xattr namespaces (user.*, system.*)
  - _Requirements: R6.4, R12.2, R12.3, R12.4_

- [x] 3.6 Implement mfs_ram hardlinks and symlinks
  - Support hardlinks via reference counting (nlink field)
  - Share inode between multiple directory entries
  - Implement symlink storing target path as string
  - Free inode when nlink reaches zero
  - _Requirements: R6.4_

- [x] 3.7 Write mfs_ram tests
  - Create `tests/mfs_ram_correctness.rs` testing all file and directory operations
  - Test hardlink reference counting and shared data
  - Test symlink target resolution
  - Test extended attribute operations and limits
  - Create `tests/mfs_ram_perf.rs` benchmarking basic operations
  - Verify bootable root filesystem on RAM
  - _Requirements: R19.2_

- [x] 4. MelloFS Disk Filesystem - Core Structure (Milestone M4)
  - Implement persistent CoW filesystem with B-tree indexing
  - _Requirements: R7.1, R7.2, R7.3, R7.4, R7.5, R7.6, R9.1, R9.2, R9.3_

- [x] 4.1 Write on-disk format specification
  - Create `docs/mfs_disk_format.md` with complete format specification
  - Document superblock layout with byte offsets and field descriptions
  - Document B-tree node format including header, keys, values, and footer
  - Document all key types (DIR_KEY, INODE_KEY, EXTENT_KEY, XATTR_KEY) with field layouts
  - Document all value types (INODE_VAL, DIR_VAL, EXTENT_VAL, XATTR_VAL) with field layouts
  - Specify endianness (little-endian), alignment requirements, and sector size assumptions
  - Document feature flags and compatibility rules
  - _Requirements: R18.1, R18.2, R18.3, R18.6, R18.7_

- [x] 4.2 Implement superblock structure
  - Create `kernel/src/fs/mfs/disk/super.rs` with MfsSuperblock struct
  - Define magic number 0x4D465344 ("MFSD") and version 1
  - Include fields: uuid, txg_id, root_btree_ptr, alloc_btree_ptr, features, block_size, total_blocks, free_blocks, checksum
  - Support block sizes: 4096, 8192, 16384 bytes
  - Implement superblock read with checksum verification
  - Implement superblock write with checksum computation
  - _Requirements: R7.3, R7.4_

- [x] 4.3 Implement B-tree node structure
  - Create `kernel/src/fs/mfs/disk/btree.rs` with BtreeNode struct
  - Define node header with magic "MFN1", level, nkeys, txg_id, node_id, checksums
  - Support leaf nodes (level 0) and internal nodes (level > 0)
  - Implement node serialization and deserialization
  - Align nodes to block size with padding
  - _Requirements: R7.2, R7.6_


- [x] 4.4 Implement key and value types
  - Create `kernel/src/fs/mfs/disk/keys.rs` with key type definitions
  - Implement DIR_KEY with parent_ino, name_hash, and optional inline name (up to 64 bytes)
  - Implement INODE_KEY with ino field
  - Implement EXTENT_KEY with ino and file_offset fields
  - Implement XATTR_KEY with ino and name_hash fields
  - Implement value types: INODE_VAL, DIR_VAL, EXTENT_VAL, XATTR_VAL
  - Support inline small files (≤ 2-4 KiB) in INODE_VAL
  - _Requirements: R7.1.1, R7.1.2, R7.1.3, R7.1.4, R7.1.5, R7.1.6, R7.1.7, R7.1.8, R9.5_

- [x] 4.5 Implement B-tree operations
  - Implement search operation with binary search in sorted key array
  - Implement insert operation with node splitting when full
  - Implement delete operation with node merging when underfull
  - Implement Copy-on-Write: allocate new node, copy and modify, update parent
  - Propagate changes up the tree to root
  - Maintain B-tree invariants: sorted keys, balanced tree
  - _Requirements: R7.1, R7.2_

- [x] 4.6 Implement extent manager
  - Create `kernel/src/fs/mfs/disk/extent.rs` for extent allocation and management
  - Track file extents in B-tree with EXTENT_KEY/EXTENT_VAL
  - Support extent allocation, extension, and freeing
  - Implement extent lookup by file offset
  - Handle extent fragmentation and coalescing
  - _Requirements: R9.1, R9.2, R9.3_

- [x] 4.7 Implement space allocator
  - Create `kernel/src/fs/mfs/disk/allocator.rs` with free space B-tree
  - Track free extents by (start_lba, length)
  - Implement delayed allocation: defer block assignment until writeback
  - Coalesce adjacent dirty pages into large extents during allocation
  - Implement best-fit or first-fit allocation strategy
  - Update free space tree on allocation and freeing
  - _Requirements: R9.1, R9.2, R9.3_

- [x] 4.8 Implement transaction groups
  - Create `kernel/src/fs/mfs/disk/txg.rs` for transaction group management
  - Accumulate dirty objects (inodes, B-tree nodes) in open TxG
  - Implement commit procedure: write dirty nodes (CoW), update root pointers, write superblock
  - Ensure atomic commit with proper write ordering and barriers
  - Support time-based (50-200 ms) and size-based commit triggers
  - Free old CoW blocks after successful commit
  - _Requirements: R7.5, R18.4_

- [x] 4.9 Implement mfs_disk mount and basic operations
  - Implement FsType trait for "mfs_disk" in `kernel/src/fs/mfs/disk/super.rs`
  - Implement mount: read superblock, verify magic/version/checksum, validate features
  - Try secondary superblock if primary fails
  - Refuse mount if unsupported features present
  - Implement basic file and directory operations using B-tree
  - Implement sync operation triggering TxG commit
  - _Requirements: R7.3, R16.4_

- [x] 4.10 Write mfs_disk core tests
  - Create `tests/mfs_disk_meta.rs` testing B-tree operations
  - Test insert, delete, split, merge, and CoW correctness
  - Verify B-tree invariants: sorted keys, balanced tree, no overlaps
  - Test INODE_VAL inline data (≤ 4 KiB)
  - Create `tests/mfs_disk_alloc.rs` testing space allocation
  - Test delayed allocation, extent coalescing, and free space tracking
  - Verify mount/umount, basic create/read/write/rename operations
  - _Requirements: R19.2_

- [x] 5. Data Integrity and Compression (Milestone M5)
  - Implement checksums for corruption detection
  - Implement crash recovery with transaction replay
  - Add optional compression support
  - _Requirements: R8.1, R8.2, R8.3, R8.4, R8.5, R10.1, R10.2, R10.3, R10.4, R10.5_

- [x] 5.1 Implement checksum algorithms
  - Create `kernel/src/fs/mfs/disk/checksum.rs` with CRC32C implementation
  - Use hardware acceleration where available (SSE4.2 on x86_64)
  - Provide checksum computation and verification functions
  - Support checksums for metadata (mandatory) and data (optional)
  - _Requirements: R8.1, R18.5_

- [x] 5.2 Integrate checksums into B-tree
  - Compute and store checksums in B-tree node headers
  - Verify checksums on node read
  - Return EIO error on checksum mismatch
  - Log corruption details to kernel log
  - Compute and store superblock checksum
  - _Requirements: R8.1, R8.2, R8.3_

- [x] 5.3 Implement secondary superblock
  - Write secondary superblock at end of device as checkpoint
  - Update secondary superblock periodically (e.g., every 10 TxGs)
  - Try secondary superblock if primary checksum fails during mount
  - _Requirements: R8.4_

- [x] 5.4 Implement crash recovery
  - Create `kernel/src/fs/mfs/disk/replay.rs` for crash recovery
  - Read superblock (primary or secondary) and validate checksum
  - Walk B-tree from root, verify all node checksums
  - Rebuild free space map from extent tree
  - Mark filesystem clean after successful recovery
  - _Requirements: R8.5, R18.4_


- [x] 5.5 Implement compression support
  - Create `kernel/src/fs/mfs/disk/compress.rs` with compression algorithms
  - Implement LZ4 compression for fast compression/decompression
  - Implement Zstd compression for higher compression ratios
  - Add per-extent compression flags in EXTENT_VAL
  - Compress extents before write based on mount options
  - Decompress transparently on read
  - Skip compression for small extents (< 4 KiB) or if compressed size ≥ original
  - _Requirements: R10.1, R10.2, R10.3, R10.4, R10.5_

- [x] 5.6 Write integrity and compression tests
  - Create `tests/mfs_disk_checksum.rs` testing checksum verification
  - Test checksum mismatch detection and EIO error
  - Test secondary superblock recovery
  - Create `tests/mfs_disk_replay.rs` testing crash recovery
  - Simulate power loss during writes (fault injection)
  - Verify filesystem consistency after replay
  - Create `tests/mfs_disk_compress.rs` testing compression
  - Test LZ4 and Zstd compression and decompression
  - Verify transparent operation and compression ratios
  - _Requirements: R19.3_

- [x] 6. Complete Syscalls and Userspace Utilities (Milestone M6)
  - Implement remaining syscalls for full POSIX compatibility
  - Create userspace utilities for filesystem management
  - _Requirements: R11.1, R11.2, R11.3, R11.4, R11.5, R12.1, R12.2, R12.3, R12.4, R12.5, R13.1, R13.2, R13.3, R13.4, R13.5, R14.3, R14.4, R14.5, R14.6, R14.7, R17.1, R17.2, R17.3, R17.4, R17.5_

- [x] 6.1 Implement metadata syscalls
  - Implement chmod syscall for changing file permissions
  - Implement chown syscall for changing file ownership
  - Implement utimensat syscall for updating timestamps with nanosecond precision
  - Ensure stat, fstat, lstat return Linux-compatible Stat structure
  - Verify struct packing matches Linux layout (test with binary comparison)
  - _Requirements: R11.1, R11.2, R11.3, R11.4, R11.5, R14.5_

- [x] 6.2 Implement extended attribute syscalls
  - Implement setxattr syscall for setting extended attributes
  - Implement getxattr syscall for reading extended attributes
  - Implement listxattr syscall for listing attribute names
  - Support xattr namespaces: user.* and system.*
  - Enforce maximum name length (255 bytes) and value size (64 KiB)
  - _Requirements: R12.1, R12.2, R12.3, R12.4, R12.5_

- [x] 6.3 Implement special file support
  - Implement mknod syscall for creating device nodes and FIFOs
  - Support character device inodes with major/minor numbers
  - Support block device inodes with major/minor numbers
  - Support FIFO (named pipe) inodes
  - Support Unix domain socket inodes
  - Encode device numbers as (major << 32) | minor in rdev field
  - _Requirements: R13.1, R13.2, R13.3, R13.4, R13.5, R14.6_

- [x] 6.4 Implement sync syscalls
  - Implement sync syscall for syncing all filesystems
  - Implement fsync syscall for syncing specific file
  - Implement fdatasync syscall for syncing file data only
  - Trigger TxG commit and flush page cache
  - _Requirements: R14.3_

- [x] 6.5 Implement mount and umount syscalls
  - Implement mount syscall with device path, mount point, filesystem type, and options
  - Parse mount options: noatime, relatime, compress, checksums, cow, trim
  - Implement umount syscall with sync and busy filesystem handling
  - Support force umount option
  - _Requirements: R14.4_

- [x] 6.6 Create userspace utility: ls
  - Create `userland/coreutils/ls/` with ls implementation
  - Support listing directory contents with file names
  - Support -l flag for long format with permissions, owner, size, mtime
  - Support -a flag for showing hidden files
  - Use getdents64 and stat syscalls
  - _Requirements: R17.1_

- [x] 6.7 Create userspace utility: cat
  - Create `userland/coreutils/cat/` with cat implementation
  - Read file contents and write to stdout
  - Support multiple file arguments
  - Handle errors gracefully (ENOENT, EACCES, etc.)
  - _Requirements: R17.2_

- [x] 6.8 Create userspace utilities: file manipulation
  - Create `userland/coreutils/touch/` for creating empty files or updating timestamps
  - Create `userland/coreutils/mkdir/` for creating directories
  - Create `userland/coreutils/rm/` for removing files and directories (-r flag)
  - Create `userland/coreutils/mv/` for moving/renaming files
  - Create `userland/coreutils/ln/` for creating hard links and symbolic links (-s flag)
  - _Requirements: R17.3, R17.4_

- [x] 6.9 Create userspace utilities: filesystem management
  - Create `userland/coreutils/df/` for displaying filesystem disk space usage
  - Create `userland/coreutils/stat/` for displaying detailed file information
  - Create `userland/coreutils/mount/` for mounting filesystems
  - Create `userland/coreutils/umount/` for unmounting filesystems
  - _Requirements: R17.5_

- [x] 6.10 Write syscall and utility tests
  - Create `tests/fs_stat_compat.rs` verifying Linux stat structure binary layout
  - Create `tests/fs_xattr.rs` testing xattr operations and limits
  - Create `tests/fs_special_nodes.rs` testing device nodes, FIFOs, sockets
  - Create `tests/fs_syscalls_api.rs` testing syscall error handling
  - Test pointer validation (EFAULT), flag combinations, permission checks
  - Create `tests/userland_smoke.rs` testing all userspace utilities
  - Verify utilities work correctly with both mfs_ram and mfs_disk
  - _Requirements: R19.2, R19.5_


- [ ] 7. Performance Tuning and Robustness (Milestone M7)
  - Optimize performance to meet targets
  - Implement comprehensive fault injection testing
  - _Requirements: R15.1, R15.2, R15.3, R15.4, R15.5, R15.6, R16.1, R16.2, R16.3, R16.4, R16.5, R19.4, R19.6_

- [x] 7.1 Create performance benchmarks
  - Create `benches/fs_seq_rand.rs` for I/O benchmarks
  - Benchmark sequential read of 1-8 GiB files (target: ≥ 2.5 GB/s)
  - Benchmark sequential write of 1-8 GiB files
  - Benchmark random 4 KiB reads with cache hits (target: ≥ 300k IOPS)
  - Benchmark random 4 KiB writes
  - Create `benches/fork_exec_p95.rs` for process creation benchmark
  - Measure fork+exec of small binary at P95 latency (target: < 1.5 ms)
  - _Requirements: R15.4, R15.5, R15.6, R19.4_

- [x] 7.2 Tune read-ahead parameters
  - Experiment with read-ahead window sizes (2, 4, 8, 16, 32 pages)
  - Tune sequential detection heuristics
  - Optimize window growth rate for different workloads
  - Measure impact on sequential read throughput
  - _Requirements: R15.4_

- [x] 7.3 Tune write-back parameters
  - Experiment with write-back batch sizes (128 KiB, 256 KiB, 512 KiB, 1024 KiB)
  - Tune dirty page thresholds (per-filesystem and global)
  - Optimize flusher thread scheduling
  - Measure impact on write throughput and latency
  - _Requirements: R15.4_

- [x] 7.4 Tune transaction group parameters
  - Experiment with TxG commit intervals (50 ms, 100 ms, 200 ms)
  - Tune size-based commit thresholds
  - Optimize CoW batching for metadata updates
  - Measure impact on metadata operation latency
  - _Requirements: R15.1, R15.2, R15.3_

- [x] 7.5 Optimize TRIM operations
  - Batch TRIM commands to reduce overhead
  - Issue TRIM after TxG commit, not during
  - Tune TRIM cadence to avoid blocking foreground I/O
  - Measure impact on write latency and throughput
  - _Requirements: R9.4_

- [x] 7.6 Tune compression thresholds
  - Experiment with compression thresholds (4 KiB, 8 KiB, 16 KiB)
  - Measure CPU overhead vs space savings for LZ4 and Zstd
  - Optimize compression level for Zstd
  - Provide mount options for compression tuning
  - _Requirements: R10.4_

- [x] 7.7 Implement fault injection tests
  - Create `tests/fs_faults.rs` for fault injection testing
  - Test out-of-space handling: fill filesystem, verify ENOSPC, check no partial metadata
  - Test I/O error handling: simulate device errors, verify EIO propagation
  - Test out-of-memory handling: limit allocations, verify graceful degradation
  - Test checksum error handling: corrupt data, verify detection and EIO
  - Test invalid metadata handling: corrupt structures, verify mount refusal
  - Verify filesystem remains consistent after all error scenarios
  - _Requirements: R16.1, R16.2, R16.3, R16.4, R16.5, R19.6_

- [x] 7.8 Verify performance targets
  - Run all benchmarks and verify targets are met:
  - Mount time: cold < 150 ms, warm < 50 ms
  - ls cached directory (100 entries): < 5 ms
  - Sequential read: ≥ 2.5 GB/s
  - Random 4 KiB read (cache hit): ≥ 300k IOPS
  - fork+exec P95: < 1.5 ms
  - Document any targets not met with analysis and mitigation plan
  - _Requirements: R15.1, R15.2, R15.3, R15.4, R15.5, R15.6_

- [x] 7.9 Run comprehensive test suite
  - Run all unit tests, integration tests, crash recovery tests
  - Run fault injection tests
  - Run performance benchmarks
  - Verify all tests pass in CI environment
  - Generate test coverage report
  - _Requirements: R19.1, R19.2, R19.3, R19.4, R19.5, R19.6_


## Testing Gates

Each milestone has specific testing requirements that must pass before proceeding:

**M1 (VFS Core)**: fs_vfs_correctness.rs, fs_fd_ops.rs, fs_dir_ops.rs all pass

**M2 (Caching)**: fs_cache_behavior.rs, fs_cache_perf.rs, fs_mmap_coherence.rs all pass

**M3 (mfs_ram)**: mfs_ram_correctness.rs, mfs_ram_perf.rs pass; bootable root on RAM

**M4 (mfs_disk Core)**: mfs_disk_meta.rs, mfs_disk_alloc.rs pass; mount/umount working

**M5 (Integrity)**: mfs_disk_checksum.rs, mfs_disk_replay.rs, mfs_disk_compress.rs pass

**M6 (Complete)**: fs_stat_compat.rs, fs_xattr.rs, fs_special_nodes.rs, fs_syscalls_api.rs, userland_smoke.rs pass

**M7 (Performance)**: All benchmarks meet targets, all tests pass, CI green

## Implementation Notes

- Always run `cargo check` immediately after modifying Rust code
- Use `getDiagnostics` tool to verify code correctness
- Follow SMP-safe patterns: per-CPU structures, fine-grained locking, atomic operations
- Test incrementally: write tests for each component as it's implemented
- Document design decisions and trade-offs in code comments
- Follow MelloOS coding standards and best practices

## Traceability Matrix

This implementation plan addresses all requirements from the requirements document:

- **R1 (VFS Core)**: Tasks 1.1-1.5
- **R2 (FD Management)**: Tasks 1.6-1.7
- **R3 (Directory Ops)**: Tasks 1.4, 1.7, 3.3
- **R4 (Page Cache)**: Tasks 2.1-2.4
- **R5 (mmap)**: Tasks 2.6-2.7
- **R6 (mfs_ram)**: Tasks 3.1-3.6
- **R7 (mfs_disk Core)**: Tasks 4.1-4.9
- **R7.1 (Key/Value Types)**: Task 4.4
- **R8 (Integrity)**: Tasks 5.1-5.4
- **R9 (Space Management)**: Tasks 4.6-4.7, 7.5
- **R10 (Compression)**: Tasks 5.5, 7.6
- **R11 (Metadata)**: Task 6.1
- **R12 (xattr)**: Tasks 3.5, 6.2
- **R13 (Special Files)**: Task 6.3
- **R14 (Syscalls)**: Tasks 1.7, 6.1-6.5
- **R15 (Performance)**: Tasks 7.1-7.6, 7.8
- **R16 (Robustness)**: Task 7.7
- **R17 (Utilities)**: Tasks 6.6-6.9
- **R18 (Documentation)**: Task 4.1
- **R19 (Testing)**: Tasks 1.8, 2.8, 3.7, 4.10, 5.6, 6.10, 7.7, 7.9


---

## Task 8: VFS Integration and Kernel Bring-up (NEW - Critical)

**Status**: In Progress  
**Priority**: CRITICAL - Required for filesystem to work

**Problem**: Tasks 1-7 created filesystem code but it's NOT integrated with the kernel. Syscalls are stubs, no mount table, no path resolution working. Filesystem code exists but doesn't run!

**Goal**: Integrate VFS with kernel syscalls, mount mfs_ram as root, make filesystem actually work.

- [x] 8.1 Implement Global Mount Table
  - Implement `kernel/src/fs/vfs/mount.rs` with static array-based mount table (no alloc needed)
  - Define `MountPoint` struct with: mount path, superblock Arc, filesystem type
  - Implement `register_mount(path: &str, sb: Arc<dyn SuperBlock>)` 
  - Implement `lookup_mount(path: &str) -> Option<Arc<dyn SuperBlock>>`
  - Implement `unmount(path: &str) -> Result<(), FsError>`
  - Use SpinLock for thread-safety
  - Support up to 16 mount points initially
  - _Requirements: R1.5_

- [x] 8.2 Implement Path Resolution (namei)
  - Implement `kernel/src/fs/vfs/path.rs` with `resolve_path(path: &str) -> Result<Arc<dyn Inode>, FsError>`
  - Handle absolute paths starting from root mount
  - Handle relative paths (future: from current working directory)
  - Implement ".." and "." handling
  - Implement symlink following with loop detection (max 40 hops)
  - Integrate with dentry cache for performance
  - Return proper errors: ENOENT, ENOTDIR, ELOOP, ENAMETOOLONG
  - _Requirements: R1.4, R3.3_

- [x] 8.3 Implement Dentry Cache
  - Implement `kernel/src/fs/vfs/dentry.rs` with hash table
  - Use static array of buckets (e.g., 256 buckets)
  - Each bucket has SpinLock and linked list of entries
  - Dentry entry: parent_ino, name_hash, child_ino, negative flag
  - Implement `lookup(parent: u64, name: &str) -> Option<u64>`
  - Implement `insert(parent: u64, name: &str, child: u64)`
  - Implement `invalidate(parent: u64)` for directory modifications
  - Support negative entries for failed lookups
  - _Requirements: R1.2_

- [x] 8.4 Wire VFS to Syscalls
  - Modify `kernel/src/sys/syscall.rs`:
    - `sys_open()`: call VFS path resolution → get inode → allocate FD
    - `sys_read()`: lookup FD → call inode.read()
    - `sys_write()`: lookup FD → call inode.write()
    - `sys_close()`: deallocate FD → drop inode reference
    - `sys_stat()`: resolve path → call inode.stat()
    - `sys_mkdir()`: resolve parent → call inode.mkdir()
    - `sys_unlink()`: resolve parent → call inode.unlink()
  - Keep existing PTY device handling as special case
  - Add proper error handling and user pointer validation
  - _Requirements: R14.1, R14.2_

- [x] 8.5 Integrate mfs_ram Implementation
  - Review existing `kernel/src/fs/mfs/ram/*.rs` code
  - Fix any compilation errors (check for alloc usage)
  - Implement `MfsRamFsType` that implements `FsType` trait
  - Implement `mount()` function that creates root inode
  - Register mfs_ram filesystem type with VFS
  - Test basic operations: create file, write, read, delete
  - _Requirements: R6.1-R6.6_

- [x] 8.6 Mount mfs_ram as Root Filesystem
  - Add VFS initialization in `kernel/src/main.rs`:
    - Initialize mount table
    - Initialize dentry cache
    - Register mfs_ram filesystem type
    - Mount mfs_ram as "/" (root)
  - Create initial directory structure: /dev, /tmp, /proc
  - Mount existing /dev/ptmx and PTY devices into VFS
  - Update init process to use real filesystem
  - Test: boot → shell → create file → read file → delete file
  - _Requirements: R1.5, R6.1_

- [x] 8.7 Implement Per-Process FD Table
  - Add `fd_table: Arc<SpinLock<FdTable>>` to `Task` struct in `kernel/src/sched/task.rs`
  - Implement FD table in `kernel/src/fs/vfs/file.rs`:
    - Array of `Option<FileDescriptor>` (max 256 FDs per process)
    - `FileDescriptor` contains: inode Arc, offset, flags (O_RDONLY, O_WRONLY, O_RDWR, O_APPEND, O_CLOEXEC)
  - Implement `alloc_fd() -> Option<usize>` (find lowest available)
  - Implement `get_fd(fd: usize) -> Option<FileDescriptor>`
  - Implement `close_fd(fd: usize)`
  - Handle fork(): clone FD table, filter CLOEXEC
  - Handle exec(): close CLOEXEC FDs
  - _Requirements: R2.1, R2.2, R2.3_

- [x] 8.8 Test Basic Filesystem Operations
  - Create test userspace program `kernel/userspace/fs_test/`:
    - Test open/read/write/close
    - Test mkdir/rmdir
    - Test stat
    - Test symlinks
    - Test hard links
  - Run in QEMU and verify all operations work
  - Check kernel logs for errors
  - Verify no memory leaks (check allocator stats)
  - _Requirements: R19.1, R19.2_

### 8.9 Integrate with Block Layer (Preparation for mfs_disk)

**Status**: Not Started

Connect filesystem to block devices:

- [x] 8.9 Integrate with Block Layer (Preparation for mfs_disk)
  - Review `kernel/src/io/bio.rs` (Block I/O interface from Phase 7)
  - Create `kernel/src/fs/block_dev.rs`:
    - BlockDevice trait for uniform interface
    - BlockDeviceManager for device registration
    - VirtioBlockDevice wrapper implementation
  - Integrate with mfs_disk superblock
  - Initialize block device subsystem in main.rs
  - _Requirements: R7.2, R9.1_

- [x] 8.10 Document Integration Architecture
  - Create `docs/VFS_BLOCK_INTEGRATION.md`:
    - Diagram showing: syscall → VFS → mount table → filesystem impl → block layer
    - Explain integration flow and components
    - Document error handling and performance considerations
    - List current limitations and future enhancements
  - _Requirements: R18.1_

---

## Task 9: mfs_disk Integration and Testing (Future)

**Status**: Not Started  
**Priority**: HIGH - After Task 8 complete

**Goal**: Mount mfs_disk as /data, test persistence, eventually move to root.

### 9.1 Bring up mfs_disk

- Review existing `kernel/src/fs/mfs/disk/*.rs` code
- Fix compilation errors
- Implement `MfsDiskFsType` that implements `FsType` trait
- Implement mount() with block device parameter
- Test basic operations on RAM disk first

### 9.2 Implement mkfs for mfs_disk

- Create userspace utility `kernel/userspace/mkfs.mfs/`
- Format block device with mfs_disk superblock
- Initialize root inode, free space tree
- Write initial metadata

### 9.3 Mount mfs_disk as /data

- Mount mfs_disk on /data mount point
- Test file creation, persistence across reboots
- Verify checksums, CoW, TxG commits work

### 9.4 Move root to mfs_disk

- Create initramfs with mfs_ram
- Boot from mfs_ram, mount mfs_disk as /
- Pivot root to mfs_disk
- Test full system on persistent storage

---

## Updated Traceability Matrix

This implementation plan addresses all requirements:

- **R1 (VFS Core)**: Tasks 1.1-1.5, **8.1-8.4**
- **R2 (FD Management)**: Tasks 1.6-1.7, **8.4, 8.7**
- **R3 (Directory Ops)**: Tasks 1.4, 1.7, 3.3, **8.2**
- **R4 (Page Cache)**: Tasks 2.1-2.4
- **R5 (mmap)**: Tasks 2.6-2.7
- **R6 (mfs_ram)**: Tasks 3.1-3.6, **8.5-8.6**
- **R7 (mfs_disk Core)**: Tasks 4.1-4.9, **9.1-9.4**
- **R7.1 (Key/Value Types)**: Task 4.4
- **R8 (Integrity)**: Tasks 5.1-5.4
- **R9 (Space Management)**: Tasks 4.6-4.7, 7.5, **8.9**
- **R10 (Compression)**: Tasks 5.5, 7.6
- **R11 (Metadata)**: Task 6.1
- **R12 (xattr)**: Tasks 3.5, 6.2
- **R13 (Special Files)**: Task 6.3
- **R14 (Syscalls)**: Tasks 1.7, 6.1-6.5, **8.4**
- **R15 (Performance)**: Tasks 7.1-7.6, 7.8
- **R16 (Robustness)**: Task 7.7
- **R17 (Utilities)**: Tasks 6.6-6.9
- **R18 (Documentation)**: Task 4.1, **8.10**
- **R19 (Testing)**: Tasks 1.8, 2.8, 3.7, 4.10, 5.6, 6.10, 7.7, 7.9, **8.8**

## Critical Path Forward

**Current Status**: Task 7 (M7 - Performance Infrastructure) complete, but filesystem NOT integrated!

**Next Steps**:
1. **Task 8.1-8.3**: Implement mount table, path resolution, dentry cache
2. **Task 8.4**: Wire VFS to syscalls (replace stubs)
3. **Task 8.5-8.6**: Integrate mfs_ram and mount as root
4. **Task 8.7**: Move FD table to per-process
5. **Task 8.8**: Test everything works!
6. **Task 8.9-8.10**: Prepare for mfs_disk integration
7. **Task 9**: Bring up mfs_disk (future)

**Success Criteria for Task 8**:
- ✅ Boot MelloOS with mfs_ram mounted as /
- ✅ Shell can create/read/write/delete files
- ✅ Path resolution works (/dev/ptmx, /tmp/test.txt, etc.)
- ✅ Multiple processes can open files independently
- ✅ No kernel panics, no memory leaks

