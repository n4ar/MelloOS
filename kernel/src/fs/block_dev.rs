//! Block Device Management for Filesystem Integration
//!
//! This module provides the interface between the VFS layer and block devices,
//! enabling persistent storage support for filesystems like mfs_disk.

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::sync::SpinLock;
use crate::io::bio::{BioRequest, BioOp};

/// Block device interface for filesystems
pub trait BlockDevice: Send + Sync {
    /// Get device sector size (usually 512 bytes)
    fn sector_size(&self) -> u32;
    
    /// Get total device size in sectors
    fn sector_count(&self) -> u64;
    
    /// Read sectors from device
    fn read_sectors(&self, sector: u64, count: u32, buffer: &mut [u8]) -> Result<(), BlockError>;
    
    /// Write sectors to device
    fn write_sectors(&self, sector: u64, count: u32, buffer: &[u8]) -> Result<(), BlockError>;
    
    /// Flush any pending writes
    fn flush(&self) -> Result<(), BlockError>;
    
    /// Get device name/identifier
    fn name(&self) -> &str;
}

/// Block device errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockError {
    /// I/O error
    IoError,
    /// Invalid sector number
    InvalidSector,
    /// Buffer too small
    BufferTooSmall,
    /// Device not ready
    NotReady,
    /// Device not ready (alias)
    DeviceNotReady,
    /// Operation not supported
    NotSupported,
}

/// Block device manager
pub struct BlockDeviceManager {
    devices: SpinLock<Vec<Arc<dyn BlockDevice>>>,
    next_id: AtomicU64,
}

impl BlockDeviceManager {
    pub const fn new() -> Self {
        Self {
            devices: SpinLock::new(Vec::new()),
            next_id: AtomicU64::new(0),
        }
    }
    
    /// Register a block device
    pub fn register_device(&self, device: Arc<dyn BlockDevice>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let mut devices = self.devices.lock();
        devices.push(device);
        id
    }
    
    /// Get a block device by index
    pub fn get_device(&self, index: usize) -> Option<Arc<dyn BlockDevice>> {
        let devices = self.devices.lock();
        devices.get(index).cloned()
    }
    
    /// Get device by name
    pub fn get_device_by_name(&self, name: &str) -> Option<Arc<dyn BlockDevice>> {
        let devices = self.devices.lock();
        devices.iter().find(|dev| dev.name() == name).cloned()
    }
    
    /// List all devices
    pub fn list_devices(&self) -> Vec<Arc<dyn BlockDevice>> {
        let devices = self.devices.lock();
        devices.clone()
    }
    
    /// Get device count
    pub fn device_count(&self) -> usize {
        let devices = self.devices.lock();
        devices.len()
    }
}

/// Global block device manager
static BLOCK_DEVICE_MANAGER: BlockDeviceManager = BlockDeviceManager::new();

/// Get the global block device manager
pub fn block_device_manager() -> &'static BlockDeviceManager {
    &BLOCK_DEVICE_MANAGER
}

/// Initialize block device subsystem
pub fn init_block_devices() {
    crate::serial_println!("[BLOCK] Initializing block device subsystem...");
    
    // Register VirtIO block devices if available
    // This would typically be called from the driver initialization
    
    crate::serial_println!("[BLOCK] Block device subsystem initialized");
}


/// VirtIO block device wrapper
pub struct VirtioBlockDevice {
    device_id: u64,
    name: &'static str,
    sector_size: u32,
    sector_count: u64,
}

impl VirtioBlockDevice {
    pub fn new(device_id: u64, name: &'static str, sector_count: u64) -> Self {
        Self {
            device_id,
            name,
            sector_size: 512, // Standard sector size
            sector_count,
        }
    }
}

impl BlockDevice for VirtioBlockDevice {
    fn sector_size(&self) -> u32 {
        self.sector_size
    }
    
    fn sector_count(&self) -> u64 {
        self.sector_count
    }
    
    fn read_sectors(&self, sector: u64, count: u32, buffer: &mut [u8]) -> Result<(), BlockError> {
        if sector >= self.sector_count {
            return Err(BlockError::InvalidSector);
        }
        
        let required_size = (count as usize) * (self.sector_size as usize);
        if buffer.len() < required_size {
            return Err(BlockError::BufferTooSmall);
        }
        
        // TODO: Implement actual VirtIO block read
        // For now, return zeros as placeholder
        for byte in buffer.iter_mut().take(required_size) {
            *byte = 0;
        }
        
        Ok(())
    }
    
    fn write_sectors(&self, sector: u64, count: u32, buffer: &[u8]) -> Result<(), BlockError> {
        if sector >= self.sector_count {
            return Err(BlockError::InvalidSector);
        }
        
        let required_size = (count as usize) * (self.sector_size as usize);
        if buffer.len() < required_size {
            return Err(BlockError::BufferTooSmall);
        }
        
        // TODO: Implement actual VirtIO block write
        // For now, just succeed as placeholder
        
        Ok(())
    }
    
    fn flush(&self) -> Result<(), BlockError> {
        // TODO: Implement actual flush
        Ok(())
    }
    
    fn name(&self) -> &str {
        self.name
    }
}

/// Register a VirtIO block device
pub fn register_virtio_block_device(device_id: u64, name: &'static str, sector_count: u64) -> u64 {
    let block_dev = Arc::new(VirtioBlockDevice::new(device_id, name, sector_count));
    let id = block_device_manager().register_device(block_dev);
    
    crate::serial_println!("[BLOCK] Registered VirtIO block device '{}' with {} sectors", name, sector_count);
    id
}
