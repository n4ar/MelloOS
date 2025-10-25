# MelloFS Disk Filesystem Usage Guide

## Overview

MelloFS Disk (mfs_disk) is a persistent, Copy-on-Write filesystem for MelloOS. This guide explains how to format, mount, and use mfs_disk filesystems.

**Status:** Implemented in Phase 8, Task 9.1-9.3  
**Date:** 2025-01-XX

---

## Prerequisites

- MelloOS kernel with VFS and block device support
- Block device (e.g., VirtIO block device)
- mkfs.mfs utility for formatting

---

## Quick Start

### 1. Check Available Block Devices

```bash
lsdev
```

Expected output:
```
Block Devices:
  vda: VirtIO Block Device (sectors: 131072, size: 64MB)
```

### 2. Format the Device

```bash
mkfs.mfs /dev/vda
```

Options:
- `-b, --block-size SIZE` - Block size (4096, 8192, or 16384 bytes)
- `-l, --label LABEL` - Filesystem label

Example with options:
```bash
mkfs.mfs -b 4096 -l "MyData" /dev/vda
```

Expected output:
```
Creating MelloFS filesystem:
  Device: /dev/vda
  Block size: 4096 bytes
  Label: MyData
Formatting device...
  Total blocks: 16384
Filesystem created successfully!
```

### 3. Create Mount Point

```bash
mkdir /data
```

### 4. Mount the Filesystem

```bash
mount -t mfs_disk /dev/vda /data
```

Or using the mount command with device option:
```bash
mount -t mfs_disk -o device=vda /data
```

Expected kernel output:
```
[MFS_DISK] Mounted filesystem on device 'vda'
[MFS_DISK] Loaded superblock:
[MFS_DISK]   Block size: 4096 bytes
[MFS_DISK]   Total blocks: 16384
[MFS_DISK]   Free blocks: 16320
```

### 5. Use the Filesystem

```bash
# Change to mounted filesystem
cd /data

# Create a file
echo "Hello, MelloFS!" > test.txt

# Read the file
cat test.txt

# Create a directory
mkdir mydir

# List contents
ls -l

# Check filesystem statistics
df /data
```

### 6. Unmount the Filesystem

```bash
umount /data
```

---

## Filesystem Features

### Current Features

- **Persistent Storage:** Data survives reboots
- **Block-based I/O:** Efficient sector-aligned operations
- **Superblock Redundancy:** Primary and secondary superblocks
- **Checksum Verification:** Data integrity checking
- **Flexible Block Sizes:** 4K, 8K, or 16K blocks

### Planned Features (Future)

- **Copy-on-Write (CoW):** Efficient snapshots and cloning
- **B-tree Indexing:** Fast metadata operations
- **Compression:** Transparent data compression (LZ4, Zstd)
- **Transaction Groups (TxG):** ACID transactions
- **Extended Attributes:** User and system metadata
- **Inline Small Files:** Store small files in inode

---

## Filesystem Structure

### On-Disk Layout

```
┌─────────────────────────────────────────────────────────┐
│ Sector 0-7: Primary Superblock (4KB)                   │
├─────────────────────────────────────────────────────────┤
│ Sector 8-63: Reserved for metadata                     │
├─────────────────────────────────────────────────────────┤
│ Sector 64+: Data blocks                                │
│   - Root B-tree                                         │
│   - Allocator B-tree                                    │
│   - File data                                           │
│   - Directory entries                                   │
│   - Extended attributes                                 │
├─────────────────────────────────────────────────────────┤
│ Last 8 sectors: Secondary Superblock (4KB)             │
└─────────────────────────────────────────────────────────┘
```

### Superblock Structure

```
Offset  Size  Field
------  ----  -----
0x00    4     Magic number (0x4D465344 "MFSD")
0x04    4     Version (1)
0x08    16    UUID
0x18    8     Transaction Group ID
0x20    32    Root B-tree pointer
0x40    32    Allocator B-tree pointer
0x60    8     Feature flags
0x68    4     Block size
0x70    8     Total blocks
0x78    8     Free blocks
0x80    8     Created timestamp
0x88    8     Modified timestamp
0x90    8     Mounted timestamp
0x98    4     Mount count
0x9C    4     Filesystem state
0xA0    64    Filesystem label
0xF8    8     Checksum
```

---

## Testing

### Basic Functionality Test

```bash
# 1. Format device
mkfs.mfs /dev/vda

# 2. Mount filesystem
mkdir /data
mount -t mfs_disk /dev/vda /data

# 3. Create test file
echo "test data" > /data/test.txt

# 4. Verify content
cat /data/test.txt

# 5. Sync to disk
sync

# 6. Unmount
umount /data

# 7. Remount and verify persistence
mount -t mfs_disk /dev/vda /data
cat /data/test.txt  # Should show "test data"
```

### Persistence Test

```bash
# 1. Create data
mount -t mfs_disk /dev/vda /data
echo "persistent data" > /data/persist.txt
sync
umount /data

# 2. Reboot system
reboot

# 3. After reboot, remount and verify
mount -t mfs_disk /dev/vda /data
cat /data/persist.txt  # Should show "persistent data"
```

### Stress Test

```bash
# Create many files
for i in {1..100}; do
    echo "File $i" > /data/file$i.txt
done

# Verify all files
for i in {1..100}; do
    cat /data/file$i.txt
done

# Check filesystem statistics
df /data
```

---

## Troubleshooting

### Mount Fails: "Device not found"

**Problem:** Block device not registered

**Solution:**
```bash
# Check available devices
lsdev

# Ensure VirtIO driver is loaded
dmesg | grep virtio
```

### Mount Fails: "Invalid superblock"

**Problem:** Device not formatted or corrupted

**Solution:**
```bash
# Reformat the device
mkfs.mfs /dev/vda

# Try mounting again
mount -t mfs_disk /dev/vda /data
```

### Mount Fails: "Checksum mismatch"

**Problem:** Superblock corruption

**Solution:**
```bash
# Try secondary superblock (automatic fallback)
# If that fails, reformat
mkfs.mfs /dev/vda
```

### Write Fails: "No space left on device"

**Problem:** Filesystem full

**Solution:**
```bash
# Check space usage
df /data

# Remove unnecessary files
rm /data/old_file.txt

# Or use a larger device
```

---

## Performance Considerations

### Block Size Selection

- **4KB (4096 bytes):** Best for small files, lower memory usage
- **8KB (8192 bytes):** Balanced performance
- **16KB (16384 bytes):** Best for large files, higher throughput

### Best Practices

1. **Use appropriate block size** for your workload
2. **Sync regularly** to ensure data persistence
3. **Monitor free space** with `df` command
4. **Unmount cleanly** before shutdown

---

## Current Limitations

### Known Issues

1. **No root inode loading:** Root directory not yet implemented
2. **Stub VirtIO driver:** Block I/O returns zeros (placeholder)
3. **No B-tree operations:** Metadata operations not implemented
4. **No file operations:** Cannot create/read/write files yet
5. **No directory operations:** Cannot create directories yet

### Workarounds

- Use mfs_ram for root filesystem
- Mount mfs_disk as secondary filesystem
- Test with simple operations only

---

## Future Enhancements

### Short Term

- Implement root inode loading
- Implement basic file operations
- Implement directory operations
- Complete VirtIO block driver

### Long Term

- Full B-tree implementation
- Copy-on-Write support
- Compression support
- Transaction groups
- Extended attributes
- Snapshots and cloning

---

## References

- [VFS Integration Documentation](VFS_BLOCK_INTEGRATION.md)
- [MFS Disk Format Specification](mfs_disk_format.md)
- [Block Device Architecture](architecture/device-drivers.md)

---

**Document Version:** 1.0  
**Last Updated:** 2025-01-XX  
**Status:** In Progress
