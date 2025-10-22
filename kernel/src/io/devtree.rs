//! Device Tree and Bus Scanning Infrastructure
//!
//! This module provides device discovery and registration functionality.
//! It scans various buses (Platform, PS/2, PCI, virtio) to detect hardware devices.

use crate::sync::SpinLock;

/// Bus types supported by the system
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BusType {
    Platform,   // Platform devices (built-in)
    PS2,        // PS/2 keyboard/mouse
    PCI,        // PCI/PCIe devices
    Virtio,     // Paravirtualized devices
}

/// Device state tracking
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceState {
    Detected,      // Device found but not initialized
    Initializing,  // Driver init in progress
    Active,        // Device operational
    Failed,        // Initialization failed
    Shutdown,      // Device shut down
}

/// Represents a hardware device
#[derive(Debug, Clone, Copy)]
pub struct Device {
    pub name: &'static str,
    pub bus: BusType,
    pub io_base: u64,
    pub irq: Option<u8>,
    pub irq_affinity: Option<u8>, // Target CPU core for IRQ routing
    pub driver: Option<&'static str>,
    pub state: DeviceState,
}

/// Maximum number of devices supported
const MAX_DEVICES: usize = 64;

/// Device registry structure
struct DeviceRegistry {
    devices: [Option<Device>; MAX_DEVICES],
    count: usize,
}

impl DeviceRegistry {
    const fn new() -> Self {
        DeviceRegistry {
            devices: [None; MAX_DEVICES],
            count: 0,
        }
    }

    fn register(&mut self, device: Device) -> Result<(), &'static str> {
        if self.count >= MAX_DEVICES {
            return Err("Device registry full");
        }
        self.devices[self.count] = Some(device);
        self.count += 1;
        Ok(())
    }

    fn get_all(&self) -> impl Iterator<Item = &Device> {
        self.devices[..self.count]
            .iter()
            .filter_map(|d| d.as_ref())
    }

    fn find_by_name(&self, name: &str) -> Option<&Device> {
        self.get_all().find(|d| d.name == name)
    }

    fn find_by_bus(&self, bus: BusType) -> impl Iterator<Item = &Device> {
        self.get_all().filter(move |d| d.bus == bus)
    }

    fn len(&self) -> usize {
        self.count
    }
}

/// Global device registry
static DEVICE_TREE: SpinLock<DeviceRegistry> = SpinLock::new(DeviceRegistry::new());

/// Register a device in the device tree
pub fn device_register(device: Device) {
    let mut tree = DEVICE_TREE.lock();
    crate::serial_println!(
        "[IO] Registering device: {} on {:?} bus (io_base: 0x{:x}, irq: {:?})",
        device.name,
        device.bus,
        device.io_base,
        device.irq
    );
    if let Err(e) = tree.register(device) {
        crate::serial_println!("[IO] Warning: Failed to register device: {}", e);
    }
}

/// Get the number of registered devices
pub fn device_count() -> usize {
    DEVICE_TREE.lock().len()
}

/// Find a device by name
pub fn find_device_by_name(name: &str) -> Option<Device> {
    DEVICE_TREE.lock().find_by_name(name).cloned()
}

/// Iterate over all devices with a callback
pub fn for_each_device<F>(mut f: F)
where
    F: FnMut(&Device),
{
    let tree = DEVICE_TREE.lock();
    for device in tree.get_all() {
        f(device);
    }
}

/// Iterate over devices on a specific bus with a callback
pub fn for_each_device_on_bus<F>(bus: BusType, mut f: F)
where
    F: FnMut(&Device),
{
    let tree = DEVICE_TREE.lock();
    for device in tree.find_by_bus(bus) {
        f(device);
    }
}

/// Scan platform bus for built-in devices
pub fn scan_platform_bus() {
    crate::serial_println!("[IO] Scanning platform bus");
    
    // Platform devices are typically hardcoded
    // Examples: framebuffer, timer, ACPI devices
    // For now, this is a placeholder for future platform device registration
    
    #[cfg(debug_assertions)]
    crate::serial_println!("[IO] Platform bus scan complete");
}

/// Scan PS/2 bus for keyboard/mouse
pub fn scan_ps2_bus() {
    crate::serial_println!("[IO] Scanning PS/2 bus");
    
    // Check if PS/2 controller exists
    if ps2_controller_present() {
        crate::serial_println!("[IO] PS/2 controller detected");
        
        // Register keyboard device
        let kbd_device = Device {
            name: "ps2-keyboard",
            bus: BusType::PS2,
            io_base: 0x60,
            irq: Some(1),
            irq_affinity: None, // Let system decide
            driver: None,
            state: DeviceState::Detected,
        };
        device_register(kbd_device);
        
        // Note: PS/2 mouse (IRQ 12) can be added here in the future
        // For now, we only register the keyboard
    } else {
        crate::serial_println!("[IO] Warning: PS/2 controller not detected");
    }
    
    #[cfg(debug_assertions)]
    crate::serial_println!("[IO] PS/2 bus scan complete");
}

/// Scan PCI bus for devices
pub fn scan_pci_bus() {
    crate::serial_println!("[IO] Scanning PCI bus");
    
    // PCI enumeration will be implemented in future
    // This involves:
    // 1. Reading PCI configuration space
    // 2. Enumerating buses, devices, and functions
    // 3. Reading vendor ID, device ID, class codes
    // 4. Registering detected PCI devices
    
    // For now, this is a placeholder
    #[cfg(debug_assertions)]
    crate::serial_println!("[IO] PCI bus scan complete (placeholder)");
}

/// Scan virtio bus for paravirtualized devices
pub fn scan_virtio_bus() {
    crate::serial_println!("[IO] Scanning virtio bus");
    
    // Detect virtio-blk devices
    // In a full implementation, this would:
    // 1. Use PCI scanning to find virtio devices (vendor ID 0x1AF4)
    // 2. Check device ID to determine device type
    // 3. Read BAR (Base Address Register) for MMIO base
    // 4. Register each detected virtio device
    
    // For now, assume virtio-blk at standard location for QEMU
    // This is a simplified detection for development purposes
    
    let virtio_blk = Device {
        name: "virtio-blk",
        bus: BusType::Virtio,
        io_base: 0, // Will be determined by PCI BAR in full implementation
        irq: None,  // Will be determined by PCI in full implementation
        irq_affinity: None, // Let system decide
        driver: None,
        state: DeviceState::Detected,
    };
    device_register(virtio_blk);
    
    #[cfg(debug_assertions)]
    crate::serial_println!("[IO] virtio bus scan complete");
}

/// Check if PS/2 controller is present
///
/// This function performs a simple check by reading from the PS/2 status port.
/// If we get 0xFF, the controller likely doesn't exist.
pub fn ps2_controller_present() -> bool {
    unsafe {
        let status = crate::io::port::inb(0x64);
        
        // If we get 0xFF, controller likely doesn't exist
        // This is a common pattern for non-existent hardware
        if status == 0xFF {
            return false;
        }
        
        // Additional check: try to read the configuration byte
        // Send "Read Configuration Byte" command
        crate::io::port::outb(0x64, 0x20);
        
        // Small delay to let controller respond
        for _ in 0..100 {
            core::hint::spin_loop();
        }
        
        // Check if data is available
        let status = crate::io::port::inb(0x64);
        if status & 0x01 != 0 {
            // Data available, controller exists
            let _config = crate::io::port::inb(0x60);
            return true;
        }
        
        // If no data after command, controller might not exist
        // But we'll be lenient and assume it exists if status wasn't 0xFF
        true
    }
}

/// Initialize device tree and scan all buses
pub fn init_device_tree() {
    crate::serial_println!("[IO] Initializing device tree");
    
    // Scan buses in deterministic order
    scan_platform_bus();
    scan_ps2_bus();
    scan_pci_bus();
    scan_virtio_bus();
    
    let count = device_count();
    crate::serial_println!("[IO] Device tree initialized: {} devices detected", count);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_registration() {
        let device = Device {
            name: "test-device",
            bus: BusType::Platform,
            io_base: 0x1000,
            irq: Some(5),
            irq_affinity: None,
            driver: None,
            state: DeviceState::Detected,
        };
        
        device_register(device.clone());
        
        let found = find_device_by_name("test-device");
        assert!(found.is_some());
        
        let found_device = found.unwrap();
        assert_eq!(found_device.name, "test-device");
        assert_eq!(found_device.bus, BusType::Platform);
        assert_eq!(found_device.io_base, 0x1000);
    }

    #[test]
    fn test_find_devices_by_bus() {
        let device1 = Device {
            name: "ps2-kbd",
            bus: BusType::PS2,
            io_base: 0x60,
            irq: Some(1),
            irq_affinity: None,
            driver: None,
            state: DeviceState::Detected,
        };
        
        let device2 = Device {
            name: "virtio-blk",
            bus: BusType::Virtio,
            io_base: 0,
            irq: None,
            irq_affinity: None,
            driver: None,
            state: DeviceState::Detected,
        };
        
        device_register(device1);
        device_register(device2);
        
        let mut ps2_count = 0;
        for_each_device_on_bus(BusType::PS2, |_| ps2_count += 1);
        assert!(ps2_count >= 1);
        
        let mut virtio_count = 0;
        for_each_device_on_bus(BusType::Virtio, |_| virtio_count += 1);
        assert!(virtio_count >= 1);
    }
}
