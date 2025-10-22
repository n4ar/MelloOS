//! Driver Manager Core
//!
//! This module provides the core driver management infrastructure for MelloOS.
//! It defines the driver and device abstractions, manages driver and device registries,
//! and handles driver probing and initialization.

use crate::sync::SpinLock;

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
    crate::serial_println!("[DRIVER] Registering driver: {}", driver.name);
    registry.register(driver).expect("Failed to register driver");
}

/// Register a device in the device registry
pub fn device_register(device: Device) {
    let mut registry = DEVICE_REGISTRY.lock();
    crate::serial_println!("[DRIVER] Registering device: {} on {:?} bus", device.name, device.bus);
    registry.register(device).expect("Failed to register device");
}

/// Probe all registered drivers against all registered devices
pub fn driver_probe_all() {
    // We need to collect driver info first to avoid holding both locks
    let driver_count = DRIVER_REGISTRY.lock().count;
    
    // Process each device
    let mut devices = DEVICE_REGISTRY.lock();
    for device in devices.iter_mut() {
        if device.driver.is_some() {
            continue; // Already has a driver
        }
        
        device.state = DeviceState::Initializing;
        
        // Try each driver
        let drivers = DRIVER_REGISTRY.lock();
        for driver in drivers.iter() {
            if (driver.probe)(device) {
                crate::serial_println!("[DRIVER] Driver {} matched device {}", driver.name, device.name);
                match (driver.init)(device) {
                    Ok(()) => {
                        device.driver = Some(driver.name);
                        device.state = DeviceState::Active;
                        crate::serial_println!("[DRIVER] Driver {} initialized successfully", driver.name);
                        break;
                    }
                    Err(e) => {
                        crate::serial_println!("[DRIVER] Driver {} init failed: {:?}", driver.name, e);
                        device.state = DeviceState::Failed;
                    }
                }
            }
        }
        drop(drivers); // Release driver lock
        
        if device.driver.is_none() {
            device.state = DeviceState::Detected;
            crate::serial_println!("[DRIVER] No driver found for device {}", device.name);
        }
    }
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


