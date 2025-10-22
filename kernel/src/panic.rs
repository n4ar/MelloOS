use core::panic::PanicInfo;

/// Panic handler for the kernel
/// This function is called when a panic occurs in no_std environment
/// 
/// Dumps comprehensive system state including:
/// - CPU ID and panic message
/// - Current task state (PID, PGID, SID, TTY)
/// - Register state (RIP, RSP, CR2)
/// - Stack trace (if available)
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use crate::serial_println;
    
    // Disable interrupts to prevent further issues
    unsafe {
        core::arch::asm!("cli");
    }

    // Get current CPU ID (safe even during panic)
    let cpu_id = {
        let percpu = crate::arch::x86_64::smp::percpu::percpu_current();
        percpu.id
    };

    serial_println!("================================================================================");
    serial_println!("KERNEL PANIC on CPU {}", cpu_id);
    serial_println!("================================================================================");
    
    // Print panic message
    if let Some(location) = info.location() {
        serial_println!("Location: {}:{}:{}", location.file(), location.line(), location.column());
    }
    
    serial_println!("Message: {}", info.message());

    serial_println!("--------------------------------------------------------------------------------");
    
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

    serial_println!("--------------------------------------------------------------------------------");
    
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

    serial_println!("--------------------------------------------------------------------------------");
    
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

    serial_println!("================================================================================");
    serial_println!("System halted. Please reboot.");
    serial_println!("================================================================================");

    // Halt all CPUs
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
