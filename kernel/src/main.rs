#![no_std]
#![no_main]

mod panic;
mod framebuffer;
mod mm;
mod serial;

use limine::request::FramebufferRequest;

/// Limine framebuffer request
/// This static variable is placed in the .requests section so that
/// the Limine bootloader can find it and provide framebuffer information
#[used]
#[link_section = ".requests"]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

/// Kernel entry point called by the Limine bootloader
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Initialize serial port for debugging
    serial::SERIAL.lock().init();
    serial_println!("[KERNEL] MelloOS starting...");
    
    serial_println!("[KERNEL] Getting framebuffer response...");
    // Get framebuffer response from Limine
    let framebuffer_response = FRAMEBUFFER_REQUEST
        .get_response()
        .expect("Failed to get framebuffer response from Limine");
    
    serial_println!("[KERNEL] Getting framebuffer...");
    // Get the first framebuffer (there's usually only one)
    let limine_framebuffer = framebuffer_response
        .framebuffers()
        .next()
        .expect("No framebuffer available");
    
    serial_println!("[KERNEL] Creating framebuffer instance...");
    // Create our Framebuffer instance from Limine response
    let mut fb = framebuffer::Framebuffer::new(&limine_framebuffer);
    
    serial_println!("[KERNEL] Clearing screen...");
    // Clear the screen with black color
    fb.clear(0x000000);
    
    serial_println!("[KERNEL] Initializing memory management...");
    // Initialize memory management system
    // This must be called after framebuffer setup but before any dynamic memory allocation
    mm::init_memory();
    
    serial_println!("[KERNEL] Writing message to screen...");
    // Display "Hello from MelloOS ✨" message
    // White text on black background, positioned at (100, 100)
    fb.write_string("Hello from MelloOS ✨", 100, 100, 0xFFFFFF, 0x000000);
    
    serial_println!("[KERNEL] Boot complete! Entering idle loop...");
    
    // Infinite loop to prevent kernel from returning
    loop {
        // Halt instruction to reduce CPU usage
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
