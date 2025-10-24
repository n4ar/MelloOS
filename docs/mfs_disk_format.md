# MelloFS Disk Format Specification

## Version 1.0

**Document Status:** Draft  
**Last Updated:** 2025-10-24  
**Authors:** MelloOS Development Team

---

## Table of Contents

1. [Introduction](#introduction)
2. [Design Philosophy](#design-philosophy)
3. [Disk Layout Overview](#disk-layout-overview)
4. [Data Types and Conventions](#data-types-and-conventions)
5. [Superblock Structure](#superblock-structure)
6. [B-tree Node Format](#b-tree-node-format)
7. [Key Types](#key-types)
8. [Value Types](#value-types)
9. [Transaction Groups](#transaction-groups)
10. [Feature Flags](#feature-flags)
11. [Checksums](#checksums)
12. [Compatibility Rules](#compatibility-rules)

---

## Introduction

MelloFS is a modern Copy-on-Write (CoW) filesystem designed for MelloOS. It provides:

- **Data Integrity**: CRC32C checksums for all metadata and optional data checksums
- **Atomic Transactions**: Transaction groups ensure consistency across crashes
- **Efficient Storage**: B-tree indexing, delayed allocation, and optional compression
- **Scalability**: Support for large files and directories with minimal fragmentation

This document specifies the on-disk format for MelloFS version 1.0, enabling correct implementation and ensuring compatibility across versions.

---

## Design Philosophy

### Copy-on-Write (CoW)

All modifications create new copies rather than modifying data in place. This ensures:
- Atomic updates (old version remains valid until commit)
- Crash consistency (no partial writes)
- Snapshot support (future feature)

### B-tree Indexing

All metadata (inodes, directory entries, extents, xattrs) is stored in B-trees:
- O(log N) lookup, insert, delete operations
- Efficient range queries for directory listings
- Balanced tree structure for predictable performance

### Transaction Groups (TxG)

Multiple operations are batched into atomic transaction groups:
- Reduces metadata overhead
- Improves write performance through coalescing
- Ensures filesystem consistency

---

## Disk Layout Overview

```
┌─────────────────────────────────────────────────────────────┐
│ LBA 0-7: Boot Region (8 blocks, 32 KiB @ 4K blocks)        │
│   Reserved for bootloader or partition table                │
├─────────────────────────────────────────────────────────────┤
│ LBA 8-15: Disk Header (8 blocks, 32 KiB)                   │
│   Magic: "MFSDISK\0"                                        │
│   Version, UUID, creation timestamp                         │
├─────────────────────────────────────────────────────────────┤
│ LBA 16-31: Primary Superblock (16 blocks, 64 KiB)          │
│   Current filesystem state, root pointers                   │
├─────────────────────────────────────────────────────────────┤
│ LBA 32+: B-tree Nodes Area (metadata, CoW)                 │
│   - Root B-tree (inodes, dirs, extents, xattrs)            │
│   - Allocator B-tree (free space tracking)                 │
│   - Nodes allocated dynamically, CoW on modification        │
├─────────────────────────────────────────────────────────────┤
│ LBA ...: Data Extents Area (file data, CoW)                │
│   - File data blocks                                        │
│   - Optionally compressed                                   │
│   - Optionally checksummed                                  │
├─────────────────────────────────────────────────────────────┤
│ LBA ...: Journal/Intent Log (optional, circular)           │
│   - Low-latency metadata operations                         │
│   - Intent records for crash recovery                       │
├─────────────────────────────────────────────────────────────┤
│ LBA -32..-1: Secondary Superblock (16 blocks, 64 KiB)      │
│   Checkpoint copy for recovery                              │
└─────────────────────────────────────────────────────────────┘
```

### Block Addressing

- **LBA (Logical Block Address)**: 64-bit block number
- **Block Size**: Configurable at format time (4096, 8192, or 16384 bytes)
- **Alignment**: All structures aligned to block boundaries

---

## Data Types and Conventions

### Endianness

**All multi-byte integers are stored in little-endian format.**

### Primitive Types

| Type      | Size    | Description                          |
|-----------|---------|--------------------------------------|
| `u8`      | 1 byte  | Unsigned 8-bit integer               |
| `u16`     | 2 bytes | Unsigned 16-bit integer (LE)         |
| `u32`     | 4 bytes | Unsigned 32-bit integer (LE)         |
| `u64`     | 8 bytes | Unsigned 64-bit integer (LE)         |
| `i64`     | 8 bytes | Signed 64-bit integer (LE)           |
| `uuid`    | 16 bytes| UUID (RFC 4122 format)               |

### Alignment Requirements

- **Superblock**: Must start at block-aligned LBA
- **B-tree Nodes**: Must be block-aligned
- **Extents**: Data extents should be block-aligned for efficiency
- **Structures**: Internal structure fields follow natural alignment

### Sector Size Assumptions

- **Minimum Sector Size**: 512 bytes
- **Recommended**: 4096 bytes (modern drives)
- **Block Size**: Must be multiple of sector size

---

## Superblock Structure

### Location

- **Primary**: LBA 16-31 (16 blocks = 64 KiB)
- **Secondary**: Last 16 blocks of device (checkpoint)

### Format

```
Offset  Size  Field                Description
------  ----  -------------------  ----------------------------------
0x0000  4     magic                Magic number: 0x4D465344 ("MFSD")
0x0004  4     version              Format version (1)
0x0008  16    uuid                 Filesystem UUID
0x0018  8     txg_id               Last committed transaction group ID
0x0020  8     root_btree_lba       Root B-tree physical LBA
0x0028  4     root_btree_len       Root B-tree length in blocks
0x002C  8     root_btree_checksum  Expected CRC32C of root B-tree
0x0034  1     root_btree_level     Root B-tree level
0x0035  3     _reserved1           Reserved (padding)
0x0038  8     alloc_btree_lba      Allocator B-tree physical LBA
0x0040  4     alloc_btree_len      Allocator B-tree length in blocks
0x0044  8     alloc_btree_checksum Expected CRC32C of allocator B-tree
0x004C  1     alloc_btree_level    Allocator B-tree level
0x004D  3     _reserved2           Reserved (padding)
0x0050  8     features             Feature flags (bitfield)
0x0058  4     block_size           Block size (4096, 8192, or 16384)
0x005C  4     _reserved3           Reserved (padding)
0x0060  8     total_blocks         Total filesystem blocks
0x0068  8     free_blocks          Free blocks count
0x0070  8     created_time         Creation timestamp (Unix epoch ns)
0x0078  8     modified_time        Last modification timestamp
0x0080  8     mounted_time         Last mount timestamp
0x0088  4     mount_count          Number of times mounted
0x008C  4     state                Filesystem state (clean/dirty)
0x0090  64    label                Filesystem label (UTF-8, null-term)
0x00D0  48    _reserved4           Reserved for future use
0x0100  8     checksum             CRC32C of superblock (0x0000-0x00FF)
0x0108  ...   _padding             Pad to block size
```

### Field Descriptions

#### magic (0x4D465344)

Magic number identifying MelloFS disk format. ASCII: "MFSD"

#### version

Format version number. Current version is 1.

#### uuid

128-bit UUID uniquely identifying this filesystem instance.

#### txg_id

Transaction group ID of the last committed transaction. Increments on each commit.

#### root_btree_lba, root_btree_len, root_btree_checksum, root_btree_level

Pointer to the root B-tree containing all filesystem metadata:
- **lba**: Physical block address
- **len**: Length in blocks (typically 1 for root node)
- **checksum**: Expected CRC32C checksum
- **level**: Tree level (0 = leaf, >0 = internal)

#### alloc_btree_lba, alloc_btree_len, alloc_btree_checksum, alloc_btree_level

Pointer to the allocator B-tree tracking free space.

#### features

64-bit bitfield of feature flags (see [Feature Flags](#feature-flags)).

#### block_size

Block size in bytes. Must be one of: 4096, 8192, 16384.

#### total_blocks, free_blocks

Total and free block counts for space management.

#### created_time, modified_time, mounted_time

Timestamps in nanoseconds since Unix epoch (1970-01-01 00:00:00 UTC).

#### mount_count

Number of times the filesystem has been mounted.

#### state

Filesystem state:
- `0x00`: Clean (properly unmounted)
- `0x01`: Dirty (needs recovery)
- `0x02`: Error (corruption detected)

#### label

Filesystem label (up to 63 UTF-8 characters + null terminator).

#### checksum

CRC32C checksum of bytes 0x0000-0x00FF (all fields except checksum itself).

---

## B-tree Node Format

### Node Header

```
Offset  Size  Field                Description
------  ----  -------------------  ----------------------------------
0x0000  4     magic                Node magic: 0x4D464E31 ("MFN1")
0x0004  2     level                Node level (0 = leaf, >0 = internal)
0x0006  2     nkeys                Number of keys in node
0x0008  8     txg_id               Transaction group ID when created
0x0010  8     node_id              Unique node identifier
0x0018  8     checksum             CRC32C of entire node
0x0020  8     parent_node_id       Parent node ID (0 for root)
0x0028  8     _reserved            Reserved for future use
0x0030  ...   keys                 Key array (variable size)
...     ...   values/children      Value array (leaf) or child pointers (internal)
...     ...   _padding             Pad to block size
```

### Node Types

#### Leaf Node (level = 0)

Contains key-value pairs:

```
[Header]
[Key 0] [Key 1] ... [Key N-1]
[Value 0] [Value 1] ... [Value N-1]
[Padding to block size]
```

#### Internal Node (level > 0)

Contains keys and child pointers:

```
[Header]
[Key 0] [Key 1] ... [Key N-1]
[Child Ptr 0] [Child Ptr 1] ... [Child Ptr N]
[Padding to block size]
```

**Note**: Internal nodes have N+1 child pointers for N keys.

### Child Pointer Format

```
Offset  Size  Field                Description
------  ----  -------------------  ----------------------------------
0x00    8     lba                  Physical block address
0x08    4     length               Length in blocks
0x0C    8     checksum             Expected CRC32C checksum
0x14    1     level                Child node level
0x15    3     _reserved            Reserved (padding)
```

Total size: 24 bytes

### Key Ordering

Keys are stored in sorted order within each node. Key comparison follows these rules:

1. Compare key type (DIR_KEY < INODE_KEY < EXTENT_KEY < XATTR_KEY)
2. Within same type, compare type-specific fields (see [Key Types](#key-types))

### Node Capacity

Maximum keys per node depends on block size and key/value sizes:

```
max_keys = (block_size - header_size - padding) / (key_size + value_or_ptr_size)
```

Typical values:
- **4K blocks**: ~100-200 keys (depending on key/value types)
- **8K blocks**: ~200-400 keys
- **16K blocks**: ~400-800 keys

---

## Key Types

All keys begin with a 1-byte type discriminator:

```
0x01 = DIR_KEY
0x02 = INODE_KEY
0x03 = EXTENT_KEY
0x04 = XATTR_KEY
```

### DIR_KEY (Directory Entry Key)

```
Offset  Size  Field                Description
------  ----  -------------------  ----------------------------------
0x00    1     key_type             0x01 (DIR_KEY)
0x01    7     _reserved            Reserved (padding)
0x08    8     parent_ino           Parent directory inode number
0x10    8     name_hash            Hash of entry name (FNV-1a 64-bit)
0x18    1     name_len             Length of inline name (0-64)
0x19    1     _reserved2           Reserved (padding)
0x1A    64    name_inline          Inline name (UTF-8, up to 64 bytes)
```

Total size: 90 bytes

**Comparison Order**:
1. parent_ino (ascending)
2. name_hash (ascending)
3. name_inline (lexicographic, if present)

**Notes**:
- If name fits in 64 bytes, it's stored inline for faster lookups
- If name > 64 bytes, name_len = 0 and full name stored in value
- Hash collisions resolved by comparing full names

### INODE_KEY (Inode Metadata Key)

```
Offset  Size  Field                Description
------  ----  -------------------  ----------------------------------
0x00    1     key_type             0x02 (INODE_KEY)
0x01    7     _reserved            Reserved (padding)
0x08    8     ino                  Inode number
```

Total size: 16 bytes

**Comparison Order**:
1. ino (ascending)

**Notes**:
- Inode numbers are unique within filesystem
- Inode 1 is reserved for root directory
- Inode 0 is invalid

### EXTENT_KEY (File Extent Key)

```
Offset  Size  Field                Description
------  ----  -------------------  ----------------------------------
0x00    1     key_type             0x03 (EXTENT_KEY)
0x01    7     _reserved            Reserved (padding)
0x08    8     ino                  Inode number
0x10    8     file_offset          Offset within file (bytes)
```

Total size: 24 bytes

**Comparison Order**:
1. ino (ascending)
2. file_offset (ascending)

**Notes**:
- file_offset must be block-aligned
- Extents are non-overlapping for a given inode
- Sparse files have gaps in extent coverage

### XATTR_KEY (Extended Attribute Key)

```
Offset  Size  Field                Description
------  ----  -------------------  ----------------------------------
0x00    1     key_type             0x04 (XATTR_KEY)
0x01    7     _reserved            Reserved (padding)
0x08    8     ino                  Inode number
0x10    8     name_hash            Hash of attribute name (FNV-1a)
0x18    1     name_len             Length of attribute name
0x19    1     _reserved2           Reserved (padding)
0x1A    254   name                 Attribute name (UTF-8, up to 254 bytes)
```

Total size: 272 bytes

**Comparison Order**:
1. ino (ascending)
2. name_hash (ascending)
3. name (lexicographic)

**Notes**:
- Attribute names limited to 254 bytes (255 with null terminator)
- Namespaces encoded in name (e.g., "user.myattr", "system.posix_acl")

---

## Value Types

### DIR_VAL (Directory Entry Value)

```
Offset  Size  Field                Description
------  ----  -------------------  ----------------------------------
0x00    8     child_ino            Child inode number
0x08    1     file_type            File type (DT_REG, DT_DIR, etc.)
0x09    1     name_len             Full name length (if > 64 bytes)
0x0A    2     _reserved            Reserved (padding)
0x0C    ...   name_overflow        Full name (if > 64 bytes, variable)
```

Minimum size: 12 bytes  
Maximum size: 12 + 255 = 267 bytes

**file_type values**:
- `0x01`: DT_FIFO (FIFO/pipe)
- `0x02`: DT_CHR (character device)
- `0x04`: DT_DIR (directory)
- `0x06`: DT_BLK (block device)
- `0x08`: DT_REG (regular file)
- `0x0A`: DT_LNK (symbolic link)
- `0x0C`: DT_SOCK (socket)

### INODE_VAL (Inode Metadata Value)

```
Offset  Size  Field                Description
------  ----  -------------------  ----------------------------------
0x00    2     mode                 File mode and type (POSIX)
0x02    2     _reserved1           Reserved (padding)
0x04    4     uid                  User ID
0x08    4     gid                  Group ID
0x0C    4     nlink                Hard link count
0x10    8     size                 File size in bytes
0x18    8     atime_ns             Access time (ns since epoch)
0x20    8     mtime_ns             Modification time (ns since epoch)
0x28    8     ctime_ns             Status change time (ns since epoch)
0x30    8     crtime_ns            Creation time (ns since epoch)
0x38    4     flags                Inode flags (see below)
0x3C    2     inline_len           Inline data length (0-4096)
0x3E    2     _reserved2           Reserved (padding)
0x40    8     rdev                 Device ID (for special files)
0x48    8     _reserved3           Reserved for future use
0x50    ...   inline_data          Inline data (up to 4096 bytes)
```

Minimum size: 80 bytes  
Maximum size: 80 + 4096 = 4176 bytes

**mode field** (POSIX file mode):
- Bits 0-8: Permissions (rwxrwxrwx)
- Bits 9-11: Special bits (sticky, setgid, setuid)
- Bits 12-15: File type (S_IFREG, S_IFDIR, S_IFLNK, etc.)

**flags field**:
- Bit 0: IMMUTABLE (file cannot be modified)
- Bit 1: APPEND_ONLY (only append writes allowed)
- Bit 2: NODUMP (exclude from backups)
- Bit 3: COMPRESSED (file data is compressed)
- Bit 4-31: Reserved

**inline_data**:
- For small files (≤ 2-4 KiB), data stored directly in inode
- For symlinks, target path stored here
- Reduces I/O for small files

**rdev field**:
- For character/block devices: `(major << 32) | minor`
- For other file types: 0

### EXTENT_VAL (File Extent Value)

```
Offset  Size  Field                Description
------  ----  -------------------  ----------------------------------
0x00    8     phys_lba             Physical block address
0x08    4     length               Length in blocks
0x0C    2     flags                Extent flags (see below)
0x0E    2     _reserved            Reserved (padding)
0x10    8     checksum             Data checksum (if enabled)
```

Total size: 24 bytes

**flags field**:
- Bit 0: COMPRESSED (extent data is compressed)
- Bit 1: CHECKSUMMED (checksum field is valid)
- Bits 2-3: COMPRESSION_TYPE (0=none, 1=LZ4, 2=Zstd, 3=reserved)
- Bits 4-15: Reserved

**Notes**:
- Extents are contiguous on disk
- Multiple extents may be needed for large or fragmented files
- Sparse regions have no corresponding extent entries

### XATTR_VAL (Extended Attribute Value)

```
Offset  Size  Field                Description
------  ----  -------------------  ----------------------------------
0x00    4     length               Value length in bytes
0x04    4     _reserved            Reserved (padding)
0x08    ...   data                 Attribute value (up to 64 KiB)
```

Minimum size: 8 bytes  
Maximum size: 8 + 65536 = 65544 bytes

**Notes**:
- Maximum attribute value size: 64 KiB
- Values are opaque byte arrays
- Interpretation depends on attribute namespace

---

## Transaction Groups

### Concept

A Transaction Group (TxG) is an atomic collection of filesystem modifications. All changes within a TxG are committed together or not at all.

### TxG Lifecycle

1. **Open**: TxG is open for new modifications
2. **Sync**: Flush dirty pages, prepare commit
3. **Commit**: Write all dirty metadata (CoW), update superblock
4. **Complete**: Free old CoW blocks, issue TRIM

### TxG Commit Procedure

1. Write all dirty B-tree nodes (CoW) to new locations
2. Update parent pointers up to root
3. Write new root B-tree node
4. Update superblock with new root pointer and txg_id
5. Issue write barrier / flush command
6. Write secondary superblock (checkpoint)
7. Mark old blocks as free in allocator B-tree

### Commit Triggers

- **Time-based**: Every 50-200 ms (configurable)
- **Size-based**: When dirty data exceeds threshold
- **Explicit**: fsync/sync syscall

### Atomicity Guarantee

The superblock update (step 4) is atomic because:
- Superblock fits in single sector (512 bytes)
- Modern drives guarantee atomic sector writes
- Old superblock remains valid until new one is written

---

## Feature Flags

Feature flags are stored in the superblock `features` field (64-bit bitfield).

### Read-Only Compatible Features (Bits 0-15)

Can be safely ignored by implementations that don't support them:

- **Bit 0**: XATTR (extended attributes)
- **Bit 1**: INLINE_SMALL (inline small files in inodes)
- **Bit 2**: COMPRESSION (data compression support)
- **Bit 3**: DATA_CHECKSUM (optional data checksums)
- **Bits 4-15**: Reserved

### Read-Write Incompatible Features (Bits 16-31)

Must be supported to mount read-write:

- **Bit 16**: COW (copy-on-write, always set)
- **Bit 17**: BTREE (B-tree indexing, always set)
- **Bit 18**: METADATA_CHECKSUM (metadata checksums, always set)
- **Bits 19-31**: Reserved

### Incompatible Features (Bits 32-63)

Must be supported to mount at all:

- **Bits 32-63**: Reserved for future incompatible features

### Feature Detection

```
if (features & UNKNOWN_INCOMPATIBLE_MASK) {
    refuse_mount("Unsupported incompatible features");
}
if (mount_flags & MS_RDWR) {
    if (features & UNKNOWN_RW_INCOMPATIBLE_MASK) {
        refuse_mount("Unsupported read-write features");
    }
}
// Read-only compatible features can be ignored
```

---

## Checksums

### Algorithm

**CRC32C** (Castagnoli polynomial) with hardware acceleration where available.

- Polynomial: 0x1EDC6F41
- Initial value: 0xFFFFFFFF
- Final XOR: 0xFFFFFFFF
- Hardware: SSE4.2 on x86_64 (crc32 instruction)

### Coverage

#### Metadata (Mandatory)

- Superblock: Bytes 0x0000-0x00FF
- B-tree nodes: Entire node (header + keys + values + padding)
- Allocator B-tree: Same as regular B-tree nodes

#### Data (Optional)

- File extents: Per-extent checksums (if CHECKSUMMED flag set)
- Stored in EXTENT_VAL.checksum field

### Verification

On read:
1. Compute checksum of data
2. Compare with stored checksum
3. If mismatch: Return EIO error, log corruption

On write:
1. Compute checksum of data
2. Store in structure's checksum field

### Checksum Failures

- **Metadata**: Refuse to mount or return EIO
- **Data**: Return EIO to application, log error
- **Superblock**: Try secondary superblock

---

## Compatibility Rules

### Forward Compatibility

Newer implementations can read filesystems created by older implementations if:
- No incompatible features are set
- All read-write incompatible features are supported (for RW mount)

### Backward Compatibility

Older implementations can read filesystems created by newer implementations if:
- No new incompatible features are set
- Read-only compatible features can be safely ignored

### Version Bumps

- **Minor version**: Add read-only compatible features
- **Major version**: Add incompatible features or change on-disk format

### Upgrade Path

To add new features:
1. Implement feature in code
2. Add feature flag to specification
3. Set feature flag in superblock on first use
4. Older implementations will refuse mount if incompatible

### Downgrade Path

To remove features:
1. Clear feature flag in superblock
2. Remove all data structures using that feature
3. Older implementations can now mount filesystem

---

## Appendix A: Example Calculations

### Superblock Checksum

```
superblock_bytes = read(lba=16, len=256)  // First 256 bytes
checksum = crc32c(superblock_bytes[0:256])
superblock.checksum = checksum
```

### B-tree Node Capacity (4K blocks)

```
block_size = 4096
header_size = 48
key_size = 90  // DIR_KEY (largest)
value_size = 267  // DIR_VAL (largest)
entry_size = key_size + value_size = 357

max_keys = (4096 - 48) / 357 = 11 keys per node
```

### File Extent Lookup

To find extent for file offset 1,048,576 (1 MiB):

```
1. Search B-tree for EXTENT_KEY(ino=123, file_offset=1048576)
2. If exact match: Use that extent
3. If no match: Find largest key < target (predecessor)
4. Check if offset falls within extent range
5. If yes: Calculate block offset within extent
6. If no: Sparse region (return zeros)
```

---

## Appendix B: Magic Numbers Summary

| Structure      | Magic          | Hex Value    |
|----------------|----------------|--------------|
| Disk Header    | "MFSDISK\0"    | N/A          |
| Superblock     | "MFSD"         | 0x4D465344   |
| B-tree Node    | "MFN1"         | 0x4D464E31   |

---

## Appendix C: References

- **APFS**: Apple File System Reference (inspiration for CoW design)
- **ZFS**: OpenZFS On-Disk Specification (inspiration for checksums)
- **Btrfs**: Btrfs On-Disk Format (inspiration for B-trees)
- **CRC32C**: RFC 3720 (iSCSI) Appendix B
- **UUID**: RFC 4122 (UUID specification)

---

## Document History

| Version | Date       | Changes                          |
|---------|------------|----------------------------------|
| 1.0     | 2025-10-24 | Initial specification            |

---

**End of MelloFS Disk Format Specification v1.0**
