#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

mod panic;
mod framebuffer;
mod mm;
mod serial;
mod sched;

use sched::{init_scheduler, spawn_task};

use limine::request::FramebufferRequest;

/// Limine framebuffer request
/// This static variable is placed in the .requests section so that
/// the Limine bootloader can find it and provide framebuffer information
#[used]
#[link_section = ".requests"]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

/// Demonstration task A - prints "A" in a loop
fn task_a() -> ! {
    loop {
        serial_println!("A");
        // Busy-wait delay to make output visible
        for _ in 0..1_000_000 {
            unsafe {
                core::arch::asm!("nop");
            }
        }
    }
}

/// Demonstration task B - prints "B" in a loop
fn task_b() -> ! {
    loop {
        serial_println!("B");
        // Busy-wait delay to make output visible
        for _ in 0..1_000_000 {
            unsafe {
                core::arch::asm!("nop");
            }
        }
    }
}

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
    
    serial_println!("[KERNEL] Initializing scheduler...");
    // Initialize the task scheduler
    init_scheduler();
    
    serial_println!("[KERNEL] Spawning demonstration tasks...");
    // Spawn demonstration tasks
    spawn_task("Task A", task_a).expect("Failed to spawn Task A");
    spawn_task("Task B", task_b).expect("Failed to spawn Task B");
    
    serial_println!("[KERNEL] Initializing timer interrupt...");
    // Initialize timer interrupt at 100 Hz (10ms per tick)
    unsafe {
        sched::timer::init_timer(100);
    }
    
    serial_println!("[KERNEL] Enabling interrupts...");
    // Enable interrupts to start task switching
    unsafe {
        core::arch::asm!("sti");
    }
    
    serial_println!("[KERNEL] Scheduler initialization complete!");
    serial_println!("[KERNEL] Boot complete! Entering idle loop...");
    
    // Infinite loop to prevent kernel from returning
    // The scheduler will preempt this loop and switch to tasks
    loop {
        // Halt instruction to reduce CPU usage
        // The CPU will wake up on the next interrupt
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
