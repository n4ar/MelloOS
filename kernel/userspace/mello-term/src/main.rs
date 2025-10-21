//! Mello-Term - Terminal Emulator for MelloOS
//!
//! This is a placeholder for the terminal emulator implementation.
//! It will be implemented in later tasks.

#![no_std]
#![no_main]

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Placeholder entry point
    loop {}
}
