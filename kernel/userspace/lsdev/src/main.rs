#![no_std]
#![no_main]
#![allow(static_mut_refs)]

extern crate alloc;

use alloc::format;
use core::panic::PanicInfo;

mod allocator;
mod syscalls;

use syscalls::{exit, write, get_device_list, DeviceInfo};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    allocator::init_heap();
    main();
    exit(0);
}

fn main() {
    // Print header
    write_str("Device Tree:\n");
    write_str("NAME              BUS       IO_BASE    IRQ    DRIVER\n");
    write_str("--------------------------------------------------------\n");
    
    // Query device list
    let mut devices = [DeviceInfo {
        name: [0; 32],
        bus_type: 0,
        io_base: 0,
        irq: 0xFFFFFFFF,
        state: 0,
        has_driver: 0,
    }; 16]; // Support up to 16 devices
    
    let count = get_device_list(&mut devices);
    
    if count < 0 {
        write_str("Error: Failed to query device list\n");
        return;
    }
    
    if count == 0 {
        write_str("No devices found\n");
        return;
    }
    
    // Display each device
    for i in 0..(count as usize) {
        let dev = &devices[i];
        
        // Extract device name (null-terminated)
        let name = extract_name(&dev.name);
        
        // Convert bus type to string
        let bus = match dev.bus_type {
            0 => "Platform",
            1 => "PS2",
            2 => "PCI",
            3 => "Virtio",
            _ => "Unknown",
        };
        
        // Format IRQ
        let irq_str = if dev.irq == 0xFFFFFFFF {
            format!("N/A")
        } else {
            format!("{}", dev.irq)
        };
        
        // Determine driver status
        let driver = if dev.has_driver != 0 {
            "loaded"
        } else {
            "none"
        };
        
        // Print device info in table format
        let line = format!(
            "{:<16} {:<9} 0x{:08X} {:>3}    {}\n",
            name, bus, dev.io_base, irq_str, driver
        );
        write_str(&line);
    }
}

/// Extract null-terminated string from byte array
fn extract_name(bytes: &[u8; 32]) -> &str {
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(32);
    core::str::from_utf8(&bytes[..len]).unwrap_or("<invalid>")
}

fn write_str(s: &str) {
    for byte in s.bytes() {
        write(1, &[byte]);
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
