use core::panic::PanicInfo;

/// Panic handler for the kernel
/// This function is called when a panic occurs in no_std environment
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // TODO: Display panic message on framebuffer when available
    // For now, just halt the CPU

    loop {
        // Halt instruction to reduce CPU usage while in panic state
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
