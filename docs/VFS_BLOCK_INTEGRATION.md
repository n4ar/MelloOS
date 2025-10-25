# VFS and Block Layer Integration

## Overview

This document describes the integration between the Virtual File System (VFS) layer and the Block Device layer in MelloOS, enabling persistent storage support for filesystems.

**Status:** Implemented in Phase 8, Task 8.9-8.10  
**Date:** 2025-01-XX

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    User Applications                        │
│              (mello-sh, mellobox, etc.)                   │
└─────────────────────────────────────────────────────────────┘
                              ↕ syscalls
┌─────────────────────────────────────────────────────────────┐
│                    VFS Layer                                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │   Dentry    │  │    Inode    │  │ SuperBlock  │        │
│  │    Cache    │  │   Cache     │  │   Manager   │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
└─────────────────────────────────────────────────────────────┘
                              ↕
┌─────────────────────────────────────────────────────────────┐
│                 Filesystem Implementations                  │
│  ┌─────────────┐              ┌─────────────┐              │
│  │  MFS RAM    │              │  MFS Disk   │              │
│  │ (In-Memory) │              │(Persistent) │              │
│  └─────────────┘              └─────────────┘              │
└─────────────────────────────────────────────────────────────┘
                              ↕ (MFS Disk only)
┌─────────────────────────────────────────────────────────────┐
│                Block Device Layer                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │   Block     │  │    BIO      │  │   Device    │        │
│  │  Manager    │  │   Layer     │  │  Drivers    │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
└─────────────────────────────────────────────────────────────┘
                              ↕
┌─────────────────────────────────────────────────────────────┐
│                Hardware Devices                             │
│           (VirtIO Block, SATA, NVMe, etc.)                │
└─────────────────────────────────────────────────────────────┘
```

---

## Components

### 1. Block Device Interface

**File:** `kernel/src/fs/block_dev.rs`

#### BlockDevice Trait

```rust
pub trait BlockDevice: Send + Sync {
    fn sector_size(&self) -> u32;
    fn sector_count(&self) -> u64;
    fn read_sectors(&self, sector: u64, count: u32, buffer: &mut [u8]) -> Result<(), BlockError>;
    fn write_sectors(&self, sector: u64, count: u32, buffer: &[u8]) -> Result<(), BlockError>;
    fn flush(&self) -> Result<(), BlockError>;
    fn name(&self) -> &str;
}
```

**Purpose:** Provides a uniform interface for all block devices, abstracting away hardware-specific details.

#### BlockDeviceManager

```rust
pub struct BlockDeviceManager {
    devices: SpinLock<Vec<Arc<dyn BlockDevice>>>,
    next_id: AtomicU64,
}
```

**Purpose:** Manages all registered block devices in the system.

**Key Methods:**
- `register_device()` - Register a new block device
- `get_device_by_name()` - Find device by name (e.g., "vda")
- `list_devices()` - List all available devices

### 2. VirtIO Block Device Wrapper

**File:** `kernel/src/fs/block_dev.rs`

```rust
pub struct VirtioBlockDevice {
    device_id: u64,
    name: &'static str,
    sector_size: u32,
    sector_count: u64,
}
```

**Purpose:** Wraps the low-level VirtIO block driver to implement the `BlockDevice` trait.

### 3. MFS Disk Integration

**File:** `kernel/src/fs/mfs/disk/super_impl.rs`

```rust
pub struct MfsDiskFs {
    superblock: SpinLock<MfsSuperblock>,
    btree_ops: BtreeOps,
    extent_mgr: SpinLock<ExtentManager>,
    allocator: SpinLock<SpaceAllocator>,
    txg_mgr: TxgManager,
    block_device: Option<Arc<dyn BlockDevice>>,
}
```

**Purpose:** Implements persistent filesystem operations using block devices.

---

## Integration Flow

### 1. System Initialization

```rust
// In main.rs
fn main() {
    // 1. Initialize block device subsystem
    crate::fs::block_dev::init_block_devices();
    
    // 2. Initialize VFS
    crate::fs::init();
    
    // 3. Mount root filesystem (mfs_ram for now)
    // Future: mount mfs_disk from block device
}
```

### 2. Device Registration

```rust
// In VirtIO driver initialization
fn init_virtio_block() {
    let device_id = 0;
    let sector_count = 128 * 1024; // 64MB
    
    // Register with block device manager
    crate::fs::block_dev::register_virtio_block_device(
        device_id,
        "vda",
        sector_count
    );
}
```

### 3. Filesystem Mounting (Future)

```rust
// Mount persistent filesystem
let device = crate::fs::block_dev::block_device_manager()
    .get_device_by_name("vda")
    .expect("Device not found");

let fs = MfsDiskType::mount_from_device(device)?;
```

**Flow:**
1. VFS looks up device by name in `BlockDeviceManager`
2. MFS Disk reads superblock from device sector 0
3. Parses and validates superblock structure
4. Creates filesystem instance with block device reference
5. Returns mounted filesystem

### 4. File I/O Operations

```rust
// Read file data
fn read_file_data(fs: &MfsDiskFs, block_num: u64, buffer: &mut [u8]) {
    // 1. Calculate which disk sectors contain the data
    let sectors_per_block = fs.superblock.lock().block_size / 512;
    let start_sector = block_num * sectors_per_block as u64;
    
    // 2. Read from block device
    fs.read_block(block_num, buffer)?;
    
    // 3. Return data to caller
}
```

---

## Error Handling

### Block Device Errors

```rust
pub enum BlockError {
    IoError,           // Hardware I/O failure
    InvalidSector,     // Sector number out of range
    BufferTooSmall,    // Buffer size insufficient
    NotReady,          // Device not ready
    DeviceNotReady,    // Device not ready (alias)
    NotSupported,      // Operation not supported
}
```

### Error Propagation

Block errors are propagated up through the filesystem layer to VFS, where they are converted to appropriate errno values for syscalls.

---

## Performance Considerations

### 1. Sector Alignment

- All I/O operations are aligned to device sector boundaries (typically 512 bytes)
- Partial sector reads require read-modify-write operations
- Buffer cache helps reduce redundant I/O

### 2. Asynchronous I/O (Future)

**Current Implementation:** Synchronous I/O (blocking)

**Future Enhancement:** Asynchronous I/O with completion callbacks

```rust
// Future async interface
trait AsyncBlockDevice {
    async fn read_sectors_async(&self, sector: u64, buffer: &mut [u8]) -> Result<(), BlockError>;
    async fn write_sectors_async(&self, sector: u64, buffer: &[u8]) -> Result<(), BlockError>;
}
```

### 3. Caching Strategy

- **Page Cache:** Caches file data pages in memory
- **Buffer Cache:** Caches raw disk blocks
- **Metadata Cache:** Caches filesystem metadata (inodes, directory entries)

---

## Security Considerations

### 1. Device Access Control

- Only kernel code can access block devices directly
- User processes must go through VFS layer
- Mount operations require appropriate privileges

### 2. Data Integrity

- Checksums for critical metadata (superblock, inodes)
- Transaction logging for atomic operations
- Flush operations ensure data persistence

---

## Testing

### 1. Unit Tests

```rust
#[test]
fn test_block_device_registration() {
    let manager = BlockDeviceManager::new();
    let device = Arc::new(VirtioBlockDevice::new(0, "test", 1024));
    
    let id = manager.register_device(device.clone());
    assert_eq!(manager.get_device(0).unwrap().name(), "test");
}
```

### 2. Integration Tests

```rust
#[test]
fn test_mfs_disk_mount() {
    // Create mock block device
    let device = Arc::new(VirtioBlockDevice::new(0, "test_vda", 128 * 1024));
    register_virtio_block_device(0, "test_vda", 128 * 1024);
    
    // Mount filesystem
    let device = block_device_manager().get_device_by_name("test_vda").unwrap();
    let fs = MfsDiskType::mount_from_device(device).unwrap();
    
    // Verify mount
    assert!(fs.block_device().is_some());
}
```

### 3. Performance Tests

- Sequential read/write throughput
- Random I/O performance
- Concurrent access patterns
- Cache hit/miss ratios

---

## Current Limitations

### 1. Synchronous I/O Only

**Issue:** All block device operations are synchronous, blocking the calling thread.

**Impact:** Reduced system responsiveness during I/O operations.

**Solution:** Implement asynchronous I/O with completion queues.

### 2. No Hot-Plug Support

**Issue:** Block devices must be registered at boot time.

**Impact:** Cannot add/remove storage devices at runtime.

**Solution:** Implement device hotplug notifications.

### 3. Limited Error Recovery

**Issue:** Basic error handling without retry mechanisms.

**Impact:** Transient I/O errors may cause unnecessary failures.

**Solution:** Implement retry logic and error recovery strategies.

### 4. Stub VirtIO Implementation

**Issue:** Current VirtIO block device is a stub that returns zeros on read.

**Impact:** Cannot actually persist data to disk yet.

**Solution:** Implement full VirtIO block protocol with virtqueue management.

---

## Future Enhancements

### 1. Advanced Block Layer Features

- **I/O Scheduling:** Elevator algorithms for optimized disk access
- **Request Merging:** Combine adjacent I/O requests
- **Bandwidth Throttling:** QoS controls for different processes
- **Encryption:** Transparent block-level encryption

### 2. Additional Filesystem Support

- **FAT32:** For compatibility with other systems
- **EXT4:** Linux filesystem compatibility
- **NTFS:** Windows filesystem support (read-only)

### 3. Storage Management

- **LVM:** Logical Volume Management
- **RAID:** Software RAID support
- **Snapshots:** Copy-on-write snapshots
- **Compression:** Transparent compression

---

## Debugging

### 1. Debug Logging

```rust
// Enable block device debugging
crate::serial_println!("[BLOCK] Reading {} sectors from device '{}' at sector {}", 
                      count, device.name(), sector);
```

### 2. Statistics

```rust
pub struct BlockDeviceStats {
    pub read_operations: u64,
    pub write_operations: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub errors: u64,
}
```

### 3. Tracing

- I/O request tracing
- Performance profiling
- Error tracking

---

## Conclusion

The VFS and Block Layer integration provides a solid foundation for persistent storage in MelloOS. The modular design allows for:

- **Flexibility:** Easy addition of new block devices and filesystems
- **Performance:** Efficient I/O operations with caching
- **Reliability:** Error handling and data integrity features
- **Extensibility:** Clear interfaces for future enhancements

The current implementation supports basic persistent storage operations and can be extended with advanced features as needed.

---

## References

- Linux Block Layer Documentation
- VirtIO Specification
- MelloFS Design Document
- VFS Architecture Guide

---

**Document Version:** 1.0  
**Last Updated:** 2025-01-XX  
**Status:** Complete
