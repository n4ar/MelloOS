# Requirements Document

## Introduction

This document specifies the requirements for Phase 8 of MelloOS: Filesystem & Storage. The system SHALL provide a robust Virtual File System (VFS) layer with multiple filesystem implementations, including an in-memory filesystem (mfs_ram) and a persistent Copy-on-Write filesystem (mfs_disk). The implementation SHALL support Linux ABI compatibility for seamless integration with standard userspace tools and SHALL deliver performance characteristics comparable to modern filesystems like APFS.

## Glossary

- **VFS_Layer**: The Virtual File System abstraction layer that provides a unified interface for all filesystem operations
- **MelloFS**: The native filesystem family for MelloOS, including both RAM and disk variants
- **mfs_ram**: In-memory temporary filesystem similar to tmpfs, used for boot and testing
- **mfs_disk**: Persistent on-disk filesystem with Copy-on-Write, B-tree indexing, and checksums
- **Inode**: Index node representing a file system object (file, directory, device, etc.)
- **Dentry**: Directory entry cache for fast path resolution
- **Page_Cache**: In-memory cache of file data pages for performance optimization
- **CoW**: Copy-on-Write, a technique where modifications create new copies rather than modifying in place
- **TxG**: Transaction Group, atomic collection of filesystem modifications
- **Block_Device**: Hardware abstraction for block-oriented storage devices (virtio-blk, NVMe, etc.)
- **Extent**: Contiguous range of disk blocks allocated to a file
- **B-tree**: Balanced tree data structure for efficient key-value storage and retrieval
- **Superblock**: Filesystem metadata structure containing global filesystem information
- **xattr**: Extended attributes for storing additional metadata on filesystem objects

## Requirements

### Requirement 1: VFS Core Infrastructure

**User Story:** As a kernel developer, I want a unified VFS layer, so that multiple filesystem types can coexist and be accessed through a common interface.

#### Acceptance Criteria

1. THE VFS_Layer SHALL provide trait-based abstractions for FsType, SuperBlock, and Inode operations
2. THE VFS_Layer SHALL implement a dentry cache with LRU eviction and negative dentry support
3. THE VFS_Layer SHALL implement an inode cache keyed by superblock ID and inode number
4. THE VFS_Layer SHALL resolve absolute and relative paths with symlink loop detection limited to 40 hops maximum
5. THE VFS_Layer SHALL maintain a mount table supporting mount and unmount operations

### Requirement 2: File Descriptor Management

**User Story:** As a userspace process, I want to open and manipulate files through file descriptors, so that I can perform standard I/O operations.

#### Acceptance Criteria

1. THE VFS_Layer SHALL implement per-process file descriptor tables with reference counting
2. THE VFS_Layer SHALL support POSIX open flags including O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, O_TRUNC, O_APPEND, and O_CLOEXEC
3. THE VFS_Layer SHALL provide read, write, pread, pwrite, lseek, and ftruncate operations on file descriptors
4. THE VFS_Layer SHALL implement fstat and fcntl operations for file descriptor metadata and control
5. THE VFS_Layer SHALL enforce file access permissions based on inode mode bits and process credentials

### Requirement 3: Directory Operations

**User Story:** As a userspace process, I want to create, list, and navigate directories, so that I can organize files hierarchically.

#### Acceptance Criteria


1. THE VFS_Layer SHALL implement mkdir and rmdir syscalls for directory creation and removal
2. THE VFS_Layer SHALL implement getdents64 syscall with Linux-compatible record layout
3. THE VFS_Layer SHALL support directory traversal with readdir operations using position cookies
4. THE VFS_Layer SHALL validate directory names rejecting empty names, names containing null bytes, and names exceeding 255 bytes
5. THE VFS_Layer SHALL support hard links and symbolic links through link and symlink operations

### Requirement 4: Page Cache and Buffer Management

**User Story:** As the kernel, I want to cache file data in memory, so that repeated file accesses achieve high performance without redundant disk I/O.

#### Acceptance Criteria

1. THE Page_Cache SHALL maintain per-file radix trees indexed by page offset
2. THE Page_Cache SHALL implement adaptive read-ahead with window sizes growing from 2 to 32 pages based on sequential access patterns
3. THE Page_Cache SHALL implement write-back coalescing with batching of 128 to 1024 KiB per flush operation
4. THE Page_Cache SHALL enforce dirty page throttling with per-filesystem and global limits
5. THE Page_Cache SHALL provide coherence between file operations and memory-mapped regions

### Requirement 5: Memory-Mapped File Support

**User Story:** As a userspace process, I want to memory-map files, so that I can access file contents through direct memory operations.

#### Acceptance Criteria

1. THE VFS_Layer SHALL implement mmap syscall for file-backed memory mappings
2. THE VFS_Layer SHALL implement msync syscall for synchronizing mapped regions to storage
3. THE VFS_Layer SHALL implement mprotect syscall for changing protection on mapped regions
4. THE VFS_Layer SHALL invalidate or mark dirty mapped pages when file content changes through write operations
5. THE VFS_Layer SHALL enforce permission checks on mapped regions based on file access modes

### Requirement 6: MelloFS RAM Filesystem

**User Story:** As the kernel, I want an in-memory filesystem, so that I can provide temporary storage and a root filesystem during boot.

#### Acceptance Criteria

1. THE mfs_ram SHALL store all data structures in kernel memory using Arc and RwLock for concurrency
2. THE mfs_ram SHALL implement directory storage using ordered maps with O(log N) lookup complexity
3. THE mfs_ram SHALL implement file storage using chunk lists with 16 to 64 KiB chunk sizes
4. THE mfs_ram SHALL support hard links, symbolic links, and extended attributes
5. THE mfs_ram SHALL provide statfs information including total and available space

### Requirement 7: MelloFS Disk Filesystem - Core Structure

**User Story:** As a system administrator, I want a persistent filesystem, so that data survives across reboots and power cycles.

#### Acceptance Criteria

1. THE mfs_disk SHALL implement Copy-on-Write semantics for all metadata and data modifications
2. THE mfs_disk SHALL use B-tree data structures for metadata, directory entries, and extent management
3. THE mfs_disk SHALL maintain a superblock containing magic number 0x4D465344, filesystem UUID, transaction group ID, and root B-tree pointer
4. THE mfs_disk SHALL support configurable block sizes of 4096, 8192, or 16384 bytes
5. THE mfs_disk SHALL implement atomic transaction groups committing multiple operations as a single unit
6. THE mfs_disk SHALL store B-tree nodes with header containing magic MFN1, level, key count, and checksum

### Requirement 7.1: MelloFS Disk Filesystem - Key and Value Types

**User Story:** As the filesystem implementation, I want well-defined key and value structures, so that B-tree operations are consistent and efficient.

#### Acceptance Criteria

1. THE mfs_disk SHALL define DIR_KEY containing parent inode number, name hash, and optional inline name up to 64 bytes
2. THE mfs_disk SHALL define INODE_KEY containing inode number as the primary key
3. THE mfs_disk SHALL define EXTENT_KEY containing inode number and file offset for extent lookups
4. THE mfs_disk SHALL define XATTR_KEY containing inode number and attribute name hash
5. THE mfs_disk SHALL define INODE_VAL containing mode, uid, gid, nlink, size, timestamps, flags, and optional inline data
6. THE mfs_disk SHALL define DIR_VAL containing child inode number and file type
7. THE mfs_disk SHALL define EXTENT_VAL containing physical LBA, length, and flags for compression and checksums
8. THE mfs_disk SHALL define XATTR_VAL containing attribute value length and data

### Requirement 8: MelloFS Disk Filesystem - Data Integrity

**User Story:** As a user, I want my data protected from corruption, so that I can trust the filesystem to detect and prevent data loss.

#### Acceptance Criteria

1. THE mfs_disk SHALL compute and verify checksums for all metadata blocks using CRC32C algorithm
2. THE mfs_disk SHALL optionally compute and verify checksums for data extents when enabled
3. THE mfs_disk SHALL detect checksum mismatches during read operations and return I/O errors
4. THE mfs_disk SHALL maintain a secondary superblock as a backup checkpoint
5. THE mfs_disk SHALL implement crash recovery through transaction group replay ensuring filesystem consistency

### Requirement 9: MelloFS Disk Filesystem - Space Management

**User Story:** As the filesystem, I want efficient space allocation, so that disk space is utilized optimally and fragmentation is minimized.

#### Acceptance Criteria

1. THE mfs_disk SHALL maintain a free space B-tree tracking available extents by start LBA and length
2. THE mfs_disk SHALL implement delayed allocation deferring block assignment until writeback
3. THE mfs_disk SHALL coalesce adjacent writes into larger extents during allocation
4. THE mfs_disk SHALL support TRIM operations for SSD optimization when the underlying Block_Device supports it
5. THE mfs_disk SHALL inline small files of 2 to 4 KiB within inode structures to reduce I/O overhead

### Requirement 10: MelloFS Disk Filesystem - Compression

**User Story:** As a system administrator, I want optional data compression, so that I can reduce storage requirements for compressible data.

#### Acceptance Criteria

1. THE mfs_disk SHALL support LZ4 compression algorithm for fast compression and decompression
2. THE mfs_disk SHALL support Zstd compression algorithm for higher compression ratios
3. THE mfs_disk SHALL allow per-extent compression with flags indicating compression codec
4. THE mfs_disk SHALL allow mount-time configuration of compression settings
5. THE mfs_disk SHALL transparently decompress data during read operations

### Requirement 11: Linux ABI Compatibility - File Metadata

**User Story:** As a Linux userspace application, I want standard POSIX file metadata, so that I can run without modification on MelloOS.

#### Acceptance Criteria

1. THE VFS_Layer SHALL implement inode mode bits including file type and permission flags per POSIX specification
2. THE VFS_Layer SHALL maintain uid and gid fields for file ownership
3. THE VFS_Layer SHALL maintain atime, mtime, and ctime timestamps with nanosecond precision
4. THE VFS_Layer SHALL implement stat, fstat, and lstat syscalls returning Linux-compatible stat structures
5. THE VFS_Layer SHALL implement chmod, chown, and utimensat syscalls for metadata modification

### Requirement 12: Linux ABI Compatibility - Extended Attributes

**User Story:** As a userspace application, I want extended attributes, so that I can store additional metadata beyond standard POSIX attributes.

#### Acceptance Criteria

1. THE VFS_Layer SHALL implement setxattr, getxattr, and listxattr syscalls
2. THE VFS_Layer SHALL support xattr namespaces including user and system prefixes
3. THE VFS_Layer SHALL enforce maximum xattr name length of 255 bytes
4. THE VFS_Layer SHALL enforce maximum xattr value size of 64 KiB per attribute
5. THE VFS_Layer SHALL store xattr data persistently in filesystem implementations

### Requirement 13: Linux ABI Compatibility - Special Files

**User Story:** As the system, I want to support device nodes and special files, so that the /dev filesystem and IPC mechanisms work correctly.

#### Acceptance Criteria

1. THE VFS_Layer SHALL support character device inodes with major and minor device numbers
2. THE VFS_Layer SHALL support block device inodes with major and minor device numbers
3. THE VFS_Layer SHALL support FIFO (named pipe) inodes for inter-process communication
4. THE VFS_Layer SHALL support Unix domain socket inodes for local socket communication
5. THE VFS_Layer SHALL encode device numbers in inode rdev field as major shifted left 32 bits OR minor

### Requirement 14: Filesystem Syscalls

**User Story:** As a userspace process, I want comprehensive filesystem syscalls, so that I can perform all necessary file operations.

#### Acceptance Criteria

1. THE VFS_Layer SHALL implement open, openat, and close syscalls with proper flag handling
2. THE VFS_Layer SHALL implement unlink, link, symlink, and renameat2 syscalls with atomic rename semantics
3. THE VFS_Layer SHALL implement sync, fsync, and fdatasync syscalls for data persistence
4. THE VFS_Layer SHALL implement mount and umount syscalls for filesystem mounting
5. THE VFS_Layer SHALL implement chmod, chown, and utimensat syscalls for metadata modification
6. THE VFS_Layer SHALL implement mknod syscall for creating device nodes and FIFO files
7. THE VFS_Layer SHALL validate all userspace pointers and return appropriate error codes for invalid arguments

### Requirement 15: Performance Targets

**User Story:** As a user, I want fast filesystem operations, so that the system feels responsive and efficient.

#### Acceptance Criteria

1. WHEN mounting a root filesystem on NVMe with warm cache, THE VFS_Layer SHALL complete within 50 milliseconds
2. WHEN mounting a root filesystem on NVMe with cold cache, THE VFS_Layer SHALL complete within 150 milliseconds
3. WHEN listing a cached directory with 100 entries using ls command, THE VFS_Layer SHALL complete within 5 milliseconds
4. WHEN performing sequential reads of 1 to 8 GiB files, THE Page_Cache SHALL achieve throughput of at least 2.5 GB per second
5. WHEN performing random 4 KiB reads with cache hits, THE Page_Cache SHALL achieve at least 300,000 IOPS
6. WHEN executing fork and exec for a small binary, THE VFS_Layer SHALL complete at P95 latency below 1.5 milliseconds

### Requirement 16: Error Handling and Robustness

**User Story:** As a developer, I want clear error reporting, so that I can diagnose and fix filesystem issues.

#### Acceptance Criteria

1. THE VFS_Layer SHALL return standard Linux error codes including EINVAL, ENOENT, EACCES, ENOSPC, and EIO
2. THE VFS_Layer SHALL log detailed error messages to kernel log for debugging
3. THE mfs_disk SHALL detect and report corruption through checksum verification
4. THE mfs_disk SHALL refuse to mount filesystems with unsupported feature flags
5. THE VFS_Layer SHALL handle out-of-memory conditions gracefully without data corruption

### Requirement 17: Userspace Utilities

**User Story:** As a system administrator, I want standard filesystem utilities, so that I can manage files from the command line.

#### Acceptance Criteria

1. THE MelloOS SHALL provide ls utility for listing directory contents
2. THE MelloOS SHALL provide cat utility for displaying file contents
3. THE MelloOS SHALL provide touch, mkdir, and rm utilities for file manipulation
4. THE MelloOS SHALL provide mv and ln utilities for moving and linking files
5. THE MelloOS SHALL provide df, mount, umount, and stat utilities for filesystem management

### Requirement 18: On-Disk Format Documentation

**User Story:** As a filesystem developer, I want detailed on-disk format documentation, so that I can implement and maintain the filesystem correctly.

#### Acceptance Criteria

1. THE mfs_disk documentation SHALL specify all superblock fields with byte offsets, sizes, and alignment requirements
2. THE mfs_disk documentation SHALL specify B-tree node layout including header format and key-value packing
3. THE mfs_disk documentation SHALL specify all key and value type layouts with field definitions
4. THE mfs_disk documentation SHALL specify transaction group lifecycle and commit ordering requirements
5. THE mfs_disk documentation SHALL specify checksum algorithms, coverage, and verification procedures
6. THE mfs_disk documentation SHALL specify feature flags and compatibility rules for forward and backward compatibility
7. THE mfs_disk documentation SHALL specify endianness requirements and sector size assumptions

### Requirement 19: Testing and Validation

**User Story:** As a developer, I want comprehensive tests, so that I can verify filesystem correctness and performance.

#### Acceptance Criteria

1. THE filesystem implementation SHALL include unit tests for path resolution edge cases including empty paths, dot segments, double slashes, and symlink loops
2. THE filesystem implementation SHALL include correctness tests for all directory and file operations including rename atomicity and hardlink count accuracy
3. THE filesystem implementation SHALL include crash recovery tests simulating power loss during writes with transaction group replay verification
4. THE filesystem implementation SHALL include performance benchmarks measuring sequential and random I/O throughput and latency
5. THE filesystem implementation SHALL include integration tests verifying Linux ABI compatibility for stat structure layout and getdents64 record format
6. THE filesystem implementation SHALL include fault injection tests for out-of-space, I/O errors, and out-of-memory conditions
