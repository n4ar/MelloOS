// virtio-blk block device driver

use crate::drivers::{Device, Driver, DriverError};
use crate::sync::SpinLock;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;
use core::mem;

/// Block device trait for filesystem integration
pub trait BlockDevice: Send + Sync {
    fn read_block(&self, lba: u64, buf: &mut [u8]) -> Result<(), BlockError>;
    fn write_block(&self, lba: u64, buf: &[u8]) -> Result<(), BlockError>;
    fn block_count(&self) -> u64;
    fn block_size(&self) -> usize;
}

/// Block I/O error types
#[derive(Debug, Clone, Copy)]
pub enum BlockError {
    IoError,
    InvalidLba,
    BufferTooSmall,
    DeviceNotReady,
    VirtqueueFull,
    InvalidDescriptor,
}

// ============================================================================
// VirtIO Constants and Structures
// ============================================================================

/// VirtIO device status bits
const VIRTIO_STATUS_ACKNOWLEDGE: u8 = 1;
const VIRTIO_STATUS_DRIVER: u8 = 2;
const VIRTIO_STATUS_DRIVER_OK: u8 = 4;
const VIRTIO_STATUS_FEATURES_OK: u8 = 8;
const VIRTIO_STATUS_FAILED: u8 = 128;

/// VirtIO block request types
const VIRTIO_BLK_T_IN: u32 = 0; // Read
const VIRTIO_BLK_T_OUT: u32 = 1; // Write

/// VirtIO block request status codes
const VIRTIO_BLK_S_OK: u8 = 0;
const VIRTIO_BLK_S_IOERR: u8 = 1;
const VIRTIO_BLK_S_UNSUPP: u8 = 2;

/// Virtqueue descriptor flags
const VIRTQ_DESC_F_NEXT: u16 = 1; // Descriptor continues via next field
const VIRTQ_DESC_F_WRITE: u16 = 2; // Buffer is write-only (device writes)
const VIRTQ_DESC_F_INDIRECT: u16 = 4; // Buffer contains list of descriptors

/// Virtqueue available ring flags
const VIRTQ_AVAIL_F_NO_INTERRUPT: u16 = 1;

/// Virtqueue used ring flags
const VIRTQ_USED_F_NO_NOTIFY: u16 = 1;

/// Maximum queue size
const QUEUE_SIZE: u16 = 128;

/// VirtIO block request header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct VirtioBlkReqHeader {
    req_type: u32, // VIRTIO_BLK_T_IN or VIRTIO_BLK_T_OUT
    reserved: u32,
    sector: u64, // Sector number (512-byte sectors)
}

/// VirtIO block request status (footer)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct VirtioBlkReqStatus {
    status: u8, // VIRTIO_BLK_S_OK, VIRTIO_BLK_S_IOERR, or VIRTIO_BLK_S_UNSUPP
}

/// Virtqueue descriptor
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
struct VirtqDesc {
    addr: u64,  // Physical address
    len: u32,   // Length
    flags: u16, // Flags (VIRTQ_DESC_F_*)
    next: u16,  // Next descriptor index (if VIRTQ_DESC_F_NEXT)
}

/// Virtqueue available ring
#[repr(C, align(2))]
struct VirtqAvail {
    flags: u16,
    idx: u16,
    ring: [u16; QUEUE_SIZE as usize],
    used_event: u16, // Only if VIRTIO_F_EVENT_IDX
}

/// Virtqueue used element
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct VirtqUsedElem {
    id: u32,  // Descriptor chain head index
    len: u32, // Total bytes written
}

/// Virtqueue used ring
#[repr(C, align(4))]
struct VirtqUsed {
    flags: u16,
    idx: u16,
    ring: [VirtqUsedElem; QUEUE_SIZE as usize],
    avail_event: u16, // Only if VIRTIO_F_EVENT_IDX
}

/// Complete virtqueue structure
struct Virtqueue {
    /// Descriptor table
    desc: Box<[VirtqDesc; QUEUE_SIZE as usize]>,
    /// Available ring
    avail: Box<VirtqAvail>,
    /// Used ring
    used: Box<VirtqUsed>,
    /// Free descriptor list
    free_desc: Vec<u16>,
    /// Last seen used index
    last_used_idx: u16,
    /// Queue size
    queue_size: u16,
}

impl Virtqueue {
    /// Create a new virtqueue
    fn new() -> Self {
        let desc = Box::new(
            [VirtqDesc {
                addr: 0,
                len: 0,
                flags: 0,
                next: 0,
            }; QUEUE_SIZE as usize],
        );

        // Initialize descriptor free list
        let mut free_desc = Vec::with_capacity(QUEUE_SIZE as usize);
        for i in 0..QUEUE_SIZE {
            free_desc.push(i);
        }

        let avail = Box::new(VirtqAvail {
            flags: 0,
            idx: 0,
            ring: [0; QUEUE_SIZE as usize],
            used_event: 0,
        });

        let used = Box::new(VirtqUsed {
            flags: 0,
            idx: 0,
            ring: [VirtqUsedElem { id: 0, len: 0 }; QUEUE_SIZE as usize],
            avail_event: 0,
        });

        Virtqueue {
            desc,
            avail,
            used,
            free_desc,
            last_used_idx: 0,
            queue_size: QUEUE_SIZE,
        }
    }

    /// Allocate a descriptor from the free list
    fn alloc_desc(&mut self) -> Option<u16> {
        self.free_desc.pop()
    }

    /// Free a descriptor back to the free list
    fn free_desc(&mut self, idx: u16) {
        if (idx as usize) < self.queue_size as usize {
            self.free_desc.push(idx);
        }
    }

    /// Get physical address of descriptor table
    fn desc_paddr(&self) -> u64 {
        self.desc.as_ptr() as u64
    }

    /// Get physical address of available ring
    fn avail_paddr(&self) -> u64 {
        self.avail.as_ref() as *const VirtqAvail as u64
    }

    /// Get physical address of used ring
    fn used_paddr(&self) -> u64 {
        self.used.as_ref() as *const VirtqUsed as u64
    }

    /// Add a descriptor chain to the available ring
    fn add_to_avail(&mut self, desc_idx: u16) {
        let avail_idx = self.avail.idx;
        self.avail.ring[avail_idx as usize % QUEUE_SIZE as usize] = desc_idx;

        // Memory barrier to ensure ring update is visible before index update
        core::sync::atomic::fence(core::sync::atomic::Ordering::Release);

        self.avail.idx = avail_idx.wrapping_add(1);
    }

    /// Check if there are completed requests in the used ring
    fn has_used(&self) -> bool {
        self.last_used_idx != self.used.idx
    }

    /// Get the next completed request from the used ring
    fn pop_used(&mut self) -> Option<(u16, u32)> {
        if !self.has_used() {
            return None;
        }

        let idx = self.last_used_idx;
        let elem = self.used.ring[idx as usize % QUEUE_SIZE as usize];
        self.last_used_idx = self.last_used_idx.wrapping_add(1);

        Some((elem.id as u16, elem.len))
    }
}

/// virtio-blk device structure
struct VirtioBlkDevice {
    base_addr: usize,
    capacity: u64,
    block_size: usize,
    /// Virtqueue for I/O requests
    virtqueue: Option<Virtqueue>,
    /// Fallback in-memory storage for when virtqueue is not available
    storage: SpinLock<BTreeMap<u64, Box<[u8]>>>,
    /// Pending request tracking
    pending_requests: SpinLock<BTreeMap<u16, PendingRequest>>,
}

/// Pending request information
struct PendingRequest {
    header: Box<VirtioBlkReqHeader>,
    data_buffer: Vec<u8>,
    status: Box<VirtioBlkReqStatus>,
    is_write: bool,
}

impl VirtioBlkDevice {
    /// Create a new virtio-blk device
    fn new(base_addr: usize) -> Self {
        // Read capacity from virtio config space
        let capacity = if base_addr != 0 {
            unsafe { crate::io::mmio::mmio_read32(base_addr + 0x14) as u64 }
        } else {
            // Default capacity for testing
            1024 * 1024 // 1M blocks = 512 MB
        };

        VirtioBlkDevice {
            base_addr,
            capacity,
            block_size: 512, // Standard sector size
            virtqueue: None,
            storage: SpinLock::new(BTreeMap::new()),
            pending_requests: SpinLock::new(BTreeMap::new()),
        }
    }

    /// Initialize the virtio device
    fn init(&mut self) -> Result<(), DriverError> {
        crate::serial_println!("[VIRTIO-BLK] Initializing virtio-blk device");

        if self.base_addr == 0 {
            crate::serial_println!("[VIRTIO-BLK] ⚠ No MMIO base address, using stub mode");
            return Ok(());
        }

        // 1. Reset device
        self.reset();

        // 2. Set ACKNOWLEDGE status bit
        self.set_status(VIRTIO_STATUS_ACKNOWLEDGE);

        // 3. Set DRIVER status bit
        self.set_status(VIRTIO_STATUS_DRIVER);

        // 4. Read feature bits and negotiate
        self.negotiate_features();

        // 5. Set FEATURES_OK status bit
        self.set_status(VIRTIO_STATUS_FEATURES_OK);

        // 6. Initialize virtqueue
        self.init_virtqueue()?;

        // 7. Set DRIVER_OK status bit
        self.set_status(VIRTIO_STATUS_DRIVER_OK);

        crate::serial_println!(
            "[VIRTIO-BLK] ✓ virtio-blk initialized: {} blocks of {} bytes",
            self.capacity,
            self.block_size
        );

        Ok(())
    }

    /// Initialize the virtqueue
    fn init_virtqueue(&mut self) -> Result<(), DriverError> {
        let vq = Virtqueue::new();

        // Configure virtqueue in device
        // Queue select (queue 0 for virtio-blk)
        unsafe {
            crate::io::mmio::mmio_write32(self.base_addr + 0x30, 0);
        }

        // Set queue size
        unsafe {
            crate::io::mmio::mmio_write32(self.base_addr + 0x38, QUEUE_SIZE as u32);
        }

        // Set queue descriptor table address
        let desc_paddr = vq.desc_paddr();
        unsafe {
            crate::io::mmio::mmio_write32(self.base_addr + 0x80, (desc_paddr & 0xFFFFFFFF) as u32);
            crate::io::mmio::mmio_write32(self.base_addr + 0x84, (desc_paddr >> 32) as u32);
        }

        // Set queue available ring address
        let avail_paddr = vq.avail_paddr();
        unsafe {
            crate::io::mmio::mmio_write32(self.base_addr + 0x90, (avail_paddr & 0xFFFFFFFF) as u32);
            crate::io::mmio::mmio_write32(self.base_addr + 0x94, (avail_paddr >> 32) as u32);
        }

        // Set queue used ring address
        let used_paddr = vq.used_paddr();
        unsafe {
            crate::io::mmio::mmio_write32(self.base_addr + 0xA0, (used_paddr & 0xFFFFFFFF) as u32);
            crate::io::mmio::mmio_write32(self.base_addr + 0xA4, (used_paddr >> 32) as u32);
        }

        // Enable the queue
        unsafe {
            crate::io::mmio::mmio_write32(self.base_addr + 0x44, 1);
        }

        self.virtqueue = Some(vq);

        crate::serial_println!("[VIRTIO-BLK] ✓ Virtqueue initialized");

        Ok(())
    }

    /// Reset the virtio device
    fn reset(&self) {
        if self.base_addr != 0 {
            unsafe {
                crate::io::mmio::mmio_write32(self.base_addr + 0x70, 0);
            }
        }
    }

    /// Set device status bits
    fn set_status(&self, status: u8) {
        if self.base_addr != 0 {
            unsafe {
                let current = crate::io::mmio::mmio_read32(self.base_addr + 0x70) as u8;
                crate::io::mmio::mmio_write32(self.base_addr + 0x70, (current | status) as u32);
            }
        }
    }

    /// Negotiate device features
    fn negotiate_features(&self) {
        // For now, accept default features
        // In full implementation, would read and write feature bits
        if self.base_addr != 0 {
            // Read device features
            let _device_features = unsafe { crate::io::mmio::mmio_read32(self.base_addr + 0x10) };

            // For basic operation, we don't need to negotiate specific features
            // Just accept the defaults
        }
    }

    /// Submit a block I/O request via virtqueue
    fn submit_virtqueue_request(
        &mut self,
        lba: u64,
        buffer: &mut [u8],
        is_write: bool,
    ) -> Result<(), BlockError> {
        let vq = self.virtqueue.as_mut().ok_or(BlockError::DeviceNotReady)?;

        // Allocate descriptors for the request (need 3: header, data, status)
        let desc_head = vq.alloc_desc().ok_or(BlockError::VirtqueueFull)?;
        let desc_data = vq.alloc_desc().ok_or(BlockError::VirtqueueFull)?;
        let desc_status = vq.alloc_desc().ok_or(BlockError::VirtqueueFull)?;

        // Create request header
        let header = Box::new(VirtioBlkReqHeader {
            req_type: if is_write {
                VIRTIO_BLK_T_OUT
            } else {
                VIRTIO_BLK_T_IN
            },
            reserved: 0,
            sector: lba,
        });

        // Create status buffer
        let status = Box::new(VirtioBlkReqStatus {
            status: 0xFF, // Will be filled by device
        });

        // Copy data for write operations
        let data_buffer = if is_write {
            buffer.to_vec()
        } else {
            vec![0u8; buffer.len()]
        };

        // Setup descriptor chain
        // Descriptor 0: Request header (device reads)
        vq.desc[desc_head as usize] = VirtqDesc {
            addr: header.as_ref() as *const VirtioBlkReqHeader as u64,
            len: mem::size_of::<VirtioBlkReqHeader>() as u32,
            flags: VIRTQ_DESC_F_NEXT,
            next: desc_data,
        };

        // Descriptor 1: Data buffer (device reads for write, writes for read)
        vq.desc[desc_data as usize] = VirtqDesc {
            addr: data_buffer.as_ptr() as u64,
            len: buffer.len() as u32,
            flags: VIRTQ_DESC_F_NEXT | if is_write { 0 } else { VIRTQ_DESC_F_WRITE },
            next: desc_status,
        };

        // Descriptor 2: Status byte (device writes)
        vq.desc[desc_status as usize] = VirtqDesc {
            addr: status.as_ref() as *const VirtioBlkReqStatus as u64,
            len: mem::size_of::<VirtioBlkReqStatus>() as u32,
            flags: VIRTQ_DESC_F_WRITE,
            next: 0,
        };

        // Store pending request info
        let pending = PendingRequest {
            header,
            data_buffer,
            status,
            is_write,
        };
        self.pending_requests.lock().insert(desc_head, pending);

        // Add to available ring
        vq.add_to_avail(desc_head);

        // Ring doorbell (notify device)
        self.notify_device();

        // Wait for completion
        self.wait_for_completion(desc_head, buffer)?;

        Ok(())
    }

    /// Notify the device that new requests are available
    fn notify_device(&self) {
        if self.base_addr != 0 {
            // Write to queue notify register (queue 0)
            unsafe {
                crate::io::mmio::mmio_write32(self.base_addr + 0x50, 0);
            }
        }
    }

    /// Wait for a request to complete
    fn wait_for_completion(&mut self, desc_head: u16, buffer: &mut [u8]) -> Result<(), BlockError> {
        let vq = self.virtqueue.as_mut().ok_or(BlockError::DeviceNotReady)?;

        // Poll for completion (in a real implementation, this would use interrupts)
        let mut timeout = 1000000;
        while timeout > 0 {
            if vq.has_used() {
                if let Some((used_id, _len)) = vq.pop_used() {
                    if used_id == desc_head {
                        // Request completed, check status and copy data
                        return self.handle_completion(desc_head, buffer);
                    }
                }
            }
            timeout -= 1;
        }

        Err(BlockError::IoError)
    }

    /// Handle a completed request
    fn handle_completion(&mut self, desc_head: u16, buffer: &mut [u8]) -> Result<(), BlockError> {
        let vq = self.virtqueue.as_mut().ok_or(BlockError::DeviceNotReady)?;

        // Retrieve pending request
        let pending = self
            .pending_requests
            .lock()
            .remove(&desc_head)
            .ok_or(BlockError::InvalidDescriptor)?;

        // Check status
        if pending.status.status != VIRTIO_BLK_S_OK {
            // Free descriptors
            vq.free_desc(desc_head);
            let desc_data = vq.desc[desc_head as usize].next;
            vq.free_desc(desc_data);
            let desc_status = vq.desc[desc_data as usize].next;
            vq.free_desc(desc_status);

            return Err(BlockError::IoError);
        }

        // For read operations, copy data back to user buffer
        if !pending.is_write {
            buffer.copy_from_slice(&pending.data_buffer);
        }

        // Free descriptors
        vq.free_desc(desc_head);
        let desc_data = vq.desc[desc_head as usize].next;
        vq.free_desc(desc_data);
        let desc_status = vq.desc[desc_data as usize].next;
        vq.free_desc(desc_status);

        Ok(())
    }
}

impl BlockDevice for VirtioBlkDevice {
    fn read_block(&self, lba: u64, buf: &mut [u8]) -> Result<(), BlockError> {
        if lba >= self.capacity {
            return Err(BlockError::InvalidLba);
        }

        if buf.len() < self.block_size {
            return Err(BlockError::BufferTooSmall);
        }

        let block_size = self.block_size;
        let dest = &mut buf[..block_size];

        // Use virtqueue if available, otherwise fall back to in-memory storage
        if self.virtqueue.is_some() && self.base_addr != 0 {
            // Need mutable access for virtqueue operations
            // This is safe because we're the only thread accessing this device
            let self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
            self_mut.submit_virtqueue_request(lba, dest, false)?;
        } else {
            // Fallback: use in-memory backing store
            let storage = self.storage.lock();
            if let Some(block) = storage.get(&lba) {
                dest.copy_from_slice(block);
            } else {
                dest.fill(0);
            }
        }

        Ok(())
    }

    fn write_block(&self, lba: u64, buf: &[u8]) -> Result<(), BlockError> {
        if lba >= self.capacity {
            return Err(BlockError::InvalidLba);
        }

        if buf.len() < self.block_size {
            return Err(BlockError::BufferTooSmall);
        }

        let block_size = self.block_size;
        let src = &buf[..block_size];

        // Use virtqueue if available, otherwise fall back to in-memory storage
        if self.virtqueue.is_some() && self.base_addr != 0 {
            // Need mutable access for virtqueue operations
            // This is safe because we're the only thread accessing this device
            let self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
            let temp_buf = src.to_vec();
            self_mut.submit_virtqueue_request(lba, &mut temp_buf.clone(), true)?;
        } else {
            // Fallback: use in-memory backing store
            let mut storage = self.storage.lock();
            if let Some(existing) = storage.get_mut(&lba) {
                existing.copy_from_slice(src);
            } else {
                let mut new_block = vec![0u8; block_size];
                new_block.copy_from_slice(src);
                storage.insert(lba, new_block.into_boxed_slice());
            }
        }

        Ok(())
    }

    fn block_count(&self) -> u64 {
        self.capacity
    }

    fn block_size(&self) -> usize {
        self.block_size
    }
}

/// Global virtio-blk device instance
static VIRTIO_BLK: SpinLock<Option<VirtioBlkDevice>> = SpinLock::new(None);

/// Probe function for virtio-blk driver
pub fn virtio_blk_probe(device: &Device) -> bool {
    device.name == "virtio-blk"
}

/// Initialize virtio-blk driver
pub fn virtio_blk_init(device: &Device) -> Result<(), DriverError> {
    crate::serial_println!("[VIRTIO-BLK] Initializing virtio-blk driver");

    let mut blk_device = VirtioBlkDevice::new(device.io_base as usize);
    blk_device.init()?;

    let mut global = VIRTIO_BLK.lock();
    *global = Some(blk_device);

    Ok(())
}

/// Shutdown virtio-blk driver
pub fn virtio_blk_shutdown(_device: &Device) -> Result<(), DriverError> {
    crate::serial_println!("[VIRTIO-BLK] Shutting down virtio-blk");
    let mut global = VIRTIO_BLK.lock();
    *global = None;
    Ok(())
}

/// Public API: Read a block from disk
pub fn block_read(lba: u64, buf: &mut [u8]) -> Result<(), BlockError> {
    let device = VIRTIO_BLK.lock();
    device
        .as_ref()
        .ok_or(BlockError::DeviceNotReady)?
        .read_block(lba, buf)
}

/// Public API: Write a block to disk
pub fn block_write(lba: u64, buf: &[u8]) -> Result<(), BlockError> {
    let device = VIRTIO_BLK.lock();
    device
        .as_ref()
        .ok_or(BlockError::DeviceNotReady)?
        .write_block(lba, buf)
}

/// Driver constant for registration
pub const VIRTIO_BLK_DRIVER: Driver = Driver {
    name: "virtio-blk",
    probe: virtio_blk_probe,
    init: virtio_blk_init,
    shutdown: virtio_blk_shutdown,
};
