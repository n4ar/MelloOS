// virtio-blk block device driver

use crate::drivers::{Device, Driver, DriverError};
use crate::sync::SpinLock;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec;

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
}

/// virtio-blk device structure
struct VirtioBlkDevice {
    base_addr: usize,
    capacity: u64,
    block_size: usize,
    storage: SpinLock<BTreeMap<u64, Box<[u8]>>>,
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
            storage: SpinLock::new(BTreeMap::new()),
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
        self.set_status(1);

        // 3. Set DRIVER status bit
        self.set_status(2);

        // 4. Read feature bits and negotiate
        self.negotiate_features();

        // 5. Set FEATURES_OK status bit
        self.set_status(8);

        // 6. Set DRIVER_OK status bit
        self.set_status(4);

        crate::serial_println!(
            "[VIRTIO-BLK] ✓ virtio-blk initialized: {} blocks of {} bytes",
            self.capacity,
            self.block_size
        );

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

        // The current implementation uses an in-memory backing store to
        // provide deterministic behavior until virtqueue support lands.
        let storage = self.storage.lock();
        if let Some(block) = storage.get(&lba) {
            dest.copy_from_slice(block);
        } else {
            dest.fill(0);
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

        let mut storage = self.storage.lock();
        if let Some(existing) = storage.get_mut(&lba) {
            existing.copy_from_slice(src);
        } else {
            let mut new_block = vec![0u8; block_size];
            new_block.copy_from_slice(src);
            storage.insert(lba, new_block.into_boxed_slice());
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
