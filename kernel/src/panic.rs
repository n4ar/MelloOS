//! Kernel panic handler with ASCII art display
//!
//! This module provides a comprehensive panic handler that displays:
//! - Fun ASCII art on both serial output and framebuffer screen
//! - CPU ID and panic location
//! - Current task information
//! - Register state (RIP, RSP, CR2)
//! - Stack trace
//!
//! # Testing
//!
//! To test the panic screen, you can temporarily add a panic call in kernel code:
//! ```rust,ignore
//! panic!("Test panic message");
//! ```
//!
//! Or trigger a panic through invalid memory access, division by zero, etc.

use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};

/// Global flag to prevent recursive panics
static PANIC_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// ASCII art for kernel panic display
const PANIC_ART: &str = r#"
   ___  _____      
  .'/,-Y"     "~-.  
  l.Y             ^.
  /\               _\_
 i            ___/"   "\
 |          /"   "\   o !
 l         ]     o !__./
  \ _  _    \.___./    "~\
   X \/ \            ___./
  ( \ ___.   _..--~~"   ~`-.
   ` Z,--   /               \
     \__.  (   /       ______)
        \   l  /-----~~" /
         Y   \          /
         |    "x______.^
         |           \
         j            Y
"#;

/// Display panic information on the framebuffer screen
fn display_panic_screen(info: &PanicInfo, cpu_id: usize) {
    // Try to get the global framebuffer
    // This is a best-effort attempt - if it fails, we just skip screen output
    // Safety: We're in a panic handler, normal rules don't apply. We're the only code running.
    #[allow(static_mut_refs)]
    if let Some(fb) = unsafe { crate::GLOBAL_FRAMEBUFFER.as_mut() } {
        // Clear screen to red background (panic color!)
        fb.clear(0x8B0000); // Dark red

        // Draw ASCII art at the top
        let art_color = 0xFFFFFF; // White
        let bg_color = 0x8B0000; // Dark red
        fb.write_string(PANIC_ART, 10, 10, art_color, bg_color);

        // Draw "KERNEL PANICCCCCCC!" message
        let panic_msg = "KERNEL PANICCCCCCC!";
        fb.write_string(panic_msg, 10, 200, 0xFFFF00, bg_color); // Yellow text

        // Draw CPU info
        let mut y_pos = 230;
        let line_height = 10;

        // Format CPU ID
        let cpu_text = alloc::format!("CPU: {}", cpu_id);
        fb.write_string(&cpu_text, 10, y_pos, art_color, bg_color);
        y_pos += line_height;

        // Draw location if available
        if let Some(location) = info.location() {
            let loc_text = alloc::format!("Location: {}:{}", location.file(), location.line());
            fb.write_string(&loc_text, 10, y_pos, art_color, bg_color);
            y_pos += line_height;
        }

        // Draw panic message
        let msg_text = alloc::format!("Message: {}", info.message());
        fb.write_string(&msg_text, 10, y_pos, art_color, bg_color);
        y_pos += line_height * 2;

        // Draw halt message
        fb.write_string(
            "System halted. Please reboot.",
            10,
            y_pos,
            0xFF0000,
            bg_color,
        ); // Red text
    }
}

/// Panic handler for the kernel
/// This function is called when a panic occurs in no_std environment
///
/// Dumps comprehensive system state including:
/// - CPU ID and panic message
/// - Current task state (PID, PGID, SID, TTY)
/// - Register state (RIP, RSP, CR2)
/// - Stack trace (if available)
///
/// Also displays a fun ASCII art on both serial and screen!
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use crate::serial_println;

    // Prevent recursive panics
    if PANIC_IN_PROGRESS.swap(true, Ordering::SeqCst) {
        // Already panicking, just halt
        serial_println!("RECURSIVE PANIC DETECTED! Halting immediately.");
        loop {
            unsafe {
                core::arch::asm!("cli; hlt");
            }
        }
    }

    // Disable interrupts to prevent further issues
    unsafe {
        core::arch::asm!("cli");
    }

    // Get current CPU ID (safe even during panic)
    let cpu_id = {
        let percpu = crate::arch::x86_64::smp::percpu::percpu_current();
        percpu.id
    };

    // Display panic on screen if framebuffer is available
    display_panic_screen(info, cpu_id);

    // Print ASCII art and panic info to serial
    serial_println!(
        "================================================================================"
    );
    serial_println!("{}", PANIC_ART);
    serial_println!("                    KERNEL PANICCCCCCC!");
    serial_println!(
        "================================================================================"
    );
    serial_println!("KERNEL PANIC on CPU {}", cpu_id);
    serial_println!(
        "================================================================================"
    );

    // Print panic message
    if let Some(location) = info.location() {
        serial_println!(
            "Location: {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    }

    serial_println!("Message: {}", info.message());

    serial_println!(
        "--------------------------------------------------------------------------------"
    );

    // Dump current task state
    let percpu = crate::arch::x86_64::smp::percpu::percpu_current();
    let current_task_id = percpu.current_task;

    if let Some(task_id) = current_task_id {
        if let Some(task) = crate::sched::get_task_by_id(task_id) {
            serial_println!("Current Task:");
            serial_println!("  PID:  {}", task.pid);
            serial_println!("  PGID: {}", task.pgid);
            serial_println!("  SID:  {}", task.sid);
            serial_println!("  TTY:  {:?}", task.tty);
            serial_println!("  Name: {}", task.name);
            serial_println!("  State: {:?}", task.state);

            // Print last syscall if available
            if let Some(last_syscall) = task.last_syscall {
                serial_println!("  Last syscall: {}", last_syscall);
            }
        } else {
            serial_println!("Current Task: ID {} (task not found)", task_id);
        }
    } else {
        serial_println!("Current Task: None (idle or early boot)");
    }

    serial_println!(
        "--------------------------------------------------------------------------------"
    );

    // Dump register state
    serial_println!("Register State:");

    // Read CR2 (page fault address)
    let cr2: u64;
    unsafe {
        core::arch::asm!("mov {}, cr2", out(reg) cr2);
    }
    serial_println!("  CR2 (fault addr): {:#018x}", cr2);

    // Get RIP and RSP from current stack frame
    let rip: u64;
    let rsp: u64;
    unsafe {
        core::arch::asm!(
            "lea {}, [rip]",
            "mov {}, rsp",
            out(reg) rip,
            out(reg) rsp,
        );
    }
    serial_println!("  RIP: {:#018x}", rip);
    serial_println!("  RSP: {:#018x}", rsp);

    serial_println!(
        "--------------------------------------------------------------------------------"
    );

    // Print stack trace (simple version - just print a few stack frames)
    serial_println!("Stack Trace:");
    unsafe {
        let mut rbp: *const u64;
        core::arch::asm!("mov {}, rbp", out(reg) rbp);

        for i in 0..10 {
            if rbp.is_null() || (rbp as u64) < 0x1000 {
                break;
            }

            // Read return address from stack frame
            let ret_addr = rbp.offset(1).read();
            serial_println!("  #{}: {:#018x}", i, ret_addr);

            // Move to previous frame
            rbp = (*rbp) as *const u64;
        }
    }

    serial_println!(
        "================================================================================"
    );
    serial_println!("System halted. Please reboot.");
    serial_println!(
        "================================================================================"
    );

    // Halt all CPUs
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
