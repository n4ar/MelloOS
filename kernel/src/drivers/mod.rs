//! Driver Manager Core
//!
//! This module provides the core driver management infrastructure for MelloOS.
//! It defines the driver and device abstractions, manages driver and device registries,
//! and handles driver probing and initialization.

use crate::sync::SpinLock;

// Driver modules
pub mod input;
pub mod serial;

/// Represents a device driver
#[derive(Copy, Clone)]
pub struct Driver {
    pub name: &'static str,
    pub probe: fn(&Device) -> bool,
    pub init: fn(&Device) -> Result<(), DriverError>,
    pub shutdown: fn(&Device) -> Result<(), DriverError>,
}

/// Represents a hardware device
#[derive(Copy, Clone)]
pub struct Device {
    pub name: &'static str,
    pub bus: BusType,
    pub io_base: u64,
    pub irq: Option<u8>,
    pub irq_affinity: Option<u8>, // Target CPU core for IRQ routing
    pub driver: Option<&'static str>,
    pub state: DeviceState,
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

/// Bus types supported by the system
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BusType {
    Platform,   // Platform devices (built-in)
    PS2,        // PS/2 keyboard/mouse
    PCI,        // PCI/PCIe devices
    Virtio,     // Paravirtualized devices
}

/// Driver-specific errors
#[derive(Debug)]
pub enum DriverError {
    ProbeFailure,
    InitFailure,
    IoError,
    ResourceUnavailable,
    NotSupported,
}

/// Maximum number of drivers supported
const MAX_DRIVERS: usize = 32;

/// Maximum number of devices supported
const MAX_DEVICES: usize = 64;

/// Driver registry structure
struct DriverRegistry {
    drivers: [Option<Driver>; MAX_DRIVERS],
    count: usize,
}

impl DriverRegistry {
    const fn new() -> Self {
        DriverRegistry {
            drivers: [None; MAX_DRIVERS],
            count: 0,
        }
    }
    
    fn register(&mut self, driver: Driver) -> Result<(), &'static str> {
        if self.count >= MAX_DRIVERS {
            return Err("Driver registry full");
        }
        self.drivers[self.count] = Some(driver);
        self.count += 1;
        Ok(())
    }
    
    fn iter(&self) -> impl Iterator<Item = &Driver> {
        self.drivers[..self.count].iter().filter_map(|d| d.as_ref())
    }
}

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
    
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Device> {
        self.devices[..self.count].iter_mut().filter_map(|d| d.as_mut())
    }
    
    fn iter(&self) -> impl Iterator<Item = &Device> {
        self.devices[..self.count].iter().filter_map(|d| d.as_ref())
    }
}

/// Global driver registry
static DRIVER_REGISTRY: SpinLock<DriverRegistry> = SpinLock::new(DriverRegistry::new());

/// Global device registry
static DEVICE_REGISTRY: SpinLock<DeviceRegistry> = SpinLock::new(DeviceRegistry::new());

/// Register a driver in the driver registry
pub fn driver_register(driver: Driver) {
    let mut registry = DRIVER_REGISTRY.lock();
    match registry.register(driver) {
        Ok(()) => {
            crate::serial_println!("[DRIVER] ✓ Registered driver: {}", driver.name);
        }
        Err(e) => {
            crate::serial_println!("[DRIVER] ✗ Failed to register driver {}: {}", driver.name, e);
        }
    }
}

/// Register a device in the device registry
pub fn device_register(device: Device) {
    let mut registry = DEVICE_REGISTRY.lock();
    match registry.register(device) {
        Ok(()) => {
            if let Some(irq) = device.irq {
                crate::serial_println!(
                    "[DRIVER] ✓ Registered device: {} (bus={:?}, io_base=0x{:X}, irq={})",
                    device.name,
                    device.bus,
                    device.io_base,
                    irq
                );
            } else {
                crate::serial_println!(
                    "[DRIVER] ✓ Registered device: {} (bus={:?}, io_base=0x{:X}, irq=none)",
                    device.name,
                    device.bus,
                    device.io_base
                );
            }
        }
        Err(e) => {
            crate::serial_println!("[DRIVER] ✗ Failed to register device {}: {}", device.name, e);
        }
    }
}

/// Probe all registered drivers against all registered devices
pub fn driver_probe_all() {
    crate::serial_println!("[DRIVER] Starting driver probing...");
    
    // We need to collect driver info first to avoid holding both locks
    let driver_count = DRIVER_REGISTRY.lock().count;
    
    if driver_count == 0 {
        crate::serial_println!("[DRIVER] No drivers registered, skipping probe");
        return;
    }
    
    // Process each device
    let mut devices = DEVICE_REGISTRY.lock();
    let device_count = devices.count;
    
    if device_count == 0 {
        crate::serial_println!("[DRIVER] No devices detected, skipping probe");
        return;
    }
    
    let mut matched_count = 0;
    let mut failed_count = 0;
    
    for device in devices.iter_mut() {
        if device.driver.is_some() {
            continue; // Already has a driver
        }
        
        crate::serial_println!("[DRIVER] Probing device: {}", device.name);
        device.state = DeviceState::Initializing;
        
        // Try each driver
        let drivers = DRIVER_REGISTRY.lock();
        let mut found_match = false;
        
        for driver in drivers.iter() {
            if (driver.probe)(device) {
                crate::serial_println!("[DRIVER]   ✓ Driver '{}' matched device '{}'", driver.name, device.name);
                match (driver.init)(device) {
                    Ok(()) => {
                        device.driver = Some(driver.name);
                        device.state = DeviceState::Active;
                        crate::serial_println!("[DRIVER]   ✓ Driver '{}' initialized successfully", driver.name);
                        matched_count += 1;
                        found_match = true;
                        break;
                    }
                    Err(e) => {
                        crate::serial_println!("[DRIVER]   ✗ Driver '{}' init failed: {:?}", driver.name, e);
                        device.state = DeviceState::Failed;
                        failed_count += 1;
                        found_match = true;
                        break;
                    }
                }
            }
        }
        drop(drivers); // Release driver lock
        
        if !found_match {
            device.state = DeviceState::Detected;
            crate::serial_println!("[DRIVER]   ⚠ No driver found for device '{}'", device.name);
        }
    }
    
    crate::serial_println!("[DRIVER] Probing complete: {} matched, {} failed, {} unmatched", 
                          matched_count, failed_count, device_count - matched_count - failed_count);
}

/// Get the number of registered devices
pub fn device_count() -> usize {
    DEVICE_REGISTRY.lock().count
}

/// Get the number of registered drivers
pub fn driver_count() -> usize {
    DRIVER_REGISTRY.lock().count
}

/// Iterate over all registered devices (for debugging/introspection)
pub fn for_each_device<F>(mut f: F)
where
    F: FnMut(&Device),
{
    let registry = DEVICE_REGISTRY.lock();
    for device in registry.iter() {
        f(device);
    }
}

/// Iterate over all registered drivers (for debugging/introspection)
pub fn for_each_driver<F>(mut f: F)
where
    F: FnMut(&Driver),
{
    let registry = DRIVER_REGISTRY.lock();
    for driver in registry.iter() {
        f(driver);
    }
}

/// Register all built-in drivers
/// This function registers keyboard, serial, and virtio-blk drivers
fn register_builtin_drivers() {
    crate::serial_println!("[DRIVER] Registering built-in drivers");
    
    // Register keyboard driver
    driver_register(crate::drivers::input::keyboard::KEYBOARD_DRIVER);
    
    // Register serial driver
    driver_register(crate::drivers::serial::SERIAL_DRIVER);
    
    // Note: Additional drivers will be registered here once they are implemented
    // in task 8:
    // - driver_register(crate::drivers::block::virtio_blk::VIRTIO_BLK_DRIVER);
    
    let count = driver_count();
    crate::serial_println!("[DRIVER] Built-in driver registration complete ({} drivers registered)", count);
}

/// Initialize the driver subsystem
/// This is the main entry point called during kernel boot
pub fn init_drivers() {
    crate::serial_println!("[DRIVER] ========================================");
    crate::serial_println!("[DRIVER] Initializing Driver Subsystem (Phase 7)");
    crate::serial_println!("[DRIVER] ========================================");
    
    // 1. Initialize IOAPIC routing
    crate::serial_println!("[DRIVER] Step 1: Initializing IOAPIC routing");
    crate::io::irq::init_ioapic_routing();
    
    // 2. Register all built-in drivers
    crate::serial_println!("[DRIVER] Step 2: Registering built-in drivers");
    register_builtin_drivers();
    
    // 3. Scan buses in deterministic order
    crate::serial_println!("[DRIVER] Step 3: Scanning buses for devices");
    crate::io::devtree::scan_platform_bus();
    crate::io::devtree::scan_ps2_bus();
    crate::io::devtree::scan_pci_bus();
    crate::io::devtree::scan_virtio_bus();
    
    let device_count = device_count();
    let driver_count = driver_count();
    crate::serial_println!("[DRIVER] Found {} devices, {} drivers registered", device_count, driver_count);
    
    // 4. Probe and initialize drivers
    crate::serial_println!("[DRIVER] Step 4: Probing and initializing drivers");
    driver_probe_all();
    
    // 5. Report initialization status
    crate::serial_println!("[DRIVER] ========================================");
    crate::serial_println!("[DRIVER] Driver Subsystem Initialization Complete");
    crate::serial_println!("[DRIVER] ========================================");
    
    // Log device status
    crate::serial_println!("[DRIVER] Device Status Summary:");
    for_each_device(|device| {
        let driver_name = device.driver.unwrap_or("none");
        let state_str = match device.state {
            DeviceState::Detected => "detected",
            DeviceState::Initializing => "initializing",
            DeviceState::Active => "active",
            DeviceState::Failed => "FAILED",
            DeviceState::Shutdown => "shutdown",
        };
        crate::serial_println!(
            "[DRIVER]   - {} ({:?}): driver={}, state={}",
            device.name,
            device.bus,
            driver_name,
            state_str
        );
    });
}


