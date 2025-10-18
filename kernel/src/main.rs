#![no_std]
#![no_main]

mod panic;
mod framebuffer;

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
    // Get framebuffer response from Limine
    let framebuffer_response = FRAMEBUFFER_REQUEST
        .get_response()
        .expect("Failed to get framebuffer response from Limine");
    
    // Get the first framebuffer (there's usually only one)
    let limine_framebuffer = framebuffer_response
        .framebuffers()
        .next()
        .expect("No framebuffer available");
    
    // Create our Framebuffer instance from Limine response
    let mut fb = framebuffer::Framebuffer::new(&limine_framebuffer);
    
    // Clear the screen with black color
    fb.clear(0x000000);
    
    // Display "Hello from my kernel ✨" message
    // White text on black background, positioned at (100, 100)
    fb.write_string("Hello from my kernel ✨", 100, 100, 0xFFFFFF, 0x000000);
    
    // Infinite loop to prevent kernel from returning
    loop {
        // Halt instruction to reduce CPU usage
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
