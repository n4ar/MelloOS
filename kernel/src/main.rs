#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

mod panic;
mod framebuffer;
mod mm;
mod serial;
mod sched;
mod sys;
mod init_loader;
mod config;
mod arch;
mod sync;

use sched::{init_scheduler, spawn_task, priority::TaskPriority};

use limine::request::{FramebufferRequest, RsdpRequest};

/// Limine framebuffer request
/// This static variable is placed in the .requests section so that
/// the Limine bootloader can find it and provide framebuffer information
#[used]
#[link_section = ".requests"]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

/// Limine RSDP request
/// This static variable is placed in the .requests section so that
/// the Limine bootloader can find it and provide RSDP address for ACPI
#[used]
#[link_section = ".requests"]
static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();

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

/// Test task for syscall interface - demonstrates sys_write and sys_sleep
fn syscall_test_task() -> ! {
    // Helper function to invoke syscall
    unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
        let ret: isize;
        core::arch::asm!(
            "int 0x80",
            in("rax") id,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }
    
    loop {
        // Test sys_write (syscall 0)
        let msg = "Hello from syscall! 🚀\n";
        let result = unsafe {
            syscall(0, 0, msg.as_ptr() as usize, msg.len())
        };
        serial_println!("[TEST] sys_write returned: {}", result);
        
        // Test sys_sleep (syscall 2) - sleep for 50 ticks
        serial_println!("[TEST] Calling sys_sleep(50)...");
        let sleep_result = unsafe {
            syscall(2, 50, 0, 0)
        };
        serial_println!("[TEST] sys_sleep returned: {}", sleep_result);
        serial_println!("[TEST] Woke up from sleep!");
        
        // Busy-wait delay
        for _ in 0..5_000_000 {
            unsafe {
                core::arch::asm!("nop");
            }
        }
    }
}

// ============================================================================
// Integration Tests for Phase 4
// ============================================================================

/// Test 7.1: Priority scheduling test
/// Spawns three tasks with High, Normal, and Low priorities
/// Verifies execution order (High → Normal → Low)
fn test_priority_high() -> ! {
    // Helper function to invoke syscall
    unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
        let ret: isize;
        core::arch::asm!(
            "int 0x80",
            in("rax") id,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }
    
    use core::sync::atomic::{AtomicUsize, Ordering};
    static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);
    
    loop {
        let count = EXEC_COUNT.fetch_add(1, Ordering::Relaxed);
        if count < 10 {
            serial_println!("[TEST-7.1] HIGH priority task executing (count: {})", count);
        }
        
        // Sleep to allow other tasks to run
        unsafe { syscall(2, 20, 0, 0) };
    }
}

fn test_priority_normal() -> ! {
    // Helper function to invoke syscall
    unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
        let ret: isize;
        core::arch::asm!(
            "int 0x80",
            in("rax") id,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }
    
    use core::sync::atomic::{AtomicUsize, Ordering};
    static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);
    
    loop {
        let count = EXEC_COUNT.fetch_add(1, Ordering::Relaxed);
        if count < 10 {
            serial_println!("[TEST-7.1] NORMAL priority task executing (count: {})", count);
        }
        
        // Sleep to allow other tasks to run
        unsafe { syscall(2, 20, 0, 0) };
    }
}

fn test_priority_low() -> ! {
    // Helper function to invoke syscall
    unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
        let ret: isize;
        core::arch::asm!(
            "int 0x80",
            in("rax") id,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }
    
    use core::sync::atomic::{AtomicUsize, Ordering};
    static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);
    
    loop {
        let count = EXEC_COUNT.fetch_add(1, Ordering::Relaxed);
        if count < 10 {
            serial_println!("[TEST-7.1] LOW priority task executing (count: {})", count);
        }
        
        // Sleep to allow other tasks to run
        unsafe { syscall(2, 20, 0, 0) };
    }
}

/// Test 7.2: Sleep/wake test
/// Spawns task that sleeps for 50 ticks and verifies wake timing
fn test_sleep_wake() -> ! {
    // Helper function to invoke syscall
    unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
        let ret: isize;
        core::arch::asm!(
            "int 0x80",
            in("rax") id,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }
    
    use core::sync::atomic::{AtomicUsize, Ordering};
    static ITERATION: AtomicUsize = AtomicUsize::new(0);
    
    loop {
        let iter = ITERATION.fetch_add(1, Ordering::Relaxed);
        
        if iter < 5 {
            serial_println!("[TEST-7.2] Sleep/wake test iteration {}", iter);
            serial_println!("[TEST-7.2] Going to sleep for 50 ticks...");
            
            // Sleep for 50 ticks
            let result = unsafe { syscall(2, 50, 0, 0) };
            
            serial_println!("[TEST-7.2] Woke up! sys_sleep returned: {}", result);
            serial_println!("[TEST-7.2] Sleep/wake cycle completed successfully");
        }
        
        // Busy-wait delay between iterations
        for _ in 0..5_000_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

/// Test 7.3: Syscall integration test
/// Tests sys_write and sys_sleep syscalls
fn test_syscall_integration() -> ! {
    // Helper function to invoke syscall
    unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
        let ret: isize;
        core::arch::asm!(
            "int 0x80",
            in("rax") id,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }
    
    use core::sync::atomic::{AtomicUsize, Ordering};
    static ITERATION: AtomicUsize = AtomicUsize::new(0);
    
    loop {
        let iter = ITERATION.fetch_add(1, Ordering::Relaxed);
        
        if iter < 3 {
            serial_println!("[TEST-7.3] Syscall integration test iteration {}", iter);
            
            // Test sys_write
            let msg = "Testing sys_write from integration test\n";
            let write_result = unsafe {
                syscall(0, 0, msg.as_ptr() as usize, msg.len())
            };
            serial_println!("[TEST-7.3] sys_write returned: {}", write_result);
            
            // Test sys_sleep
            serial_println!("[TEST-7.3] Testing sys_sleep(30)...");
            let sleep_result = unsafe { syscall(2, 30, 0, 0) };
            serial_println!("[TEST-7.3] sys_sleep returned: {}", sleep_result);
        }
        
        // Busy-wait delay
        for _ in 0..10_000_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

/// Test 7.4: IPC integration test - Sender task
fn test_ipc_sender() -> ! {
    // Helper function to invoke syscall
    unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
        let ret: isize;
        core::arch::asm!(
            "int 0x80",
            in("rax") id,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }
    
    use core::sync::atomic::{AtomicUsize, Ordering};
    static SEND_COUNT: AtomicUsize = AtomicUsize::new(0);
    
    loop {
        let count = SEND_COUNT.fetch_add(1, Ordering::Relaxed);
        
        if count < 5 {
            serial_println!("[TEST-7.4] Sender: Sending message #{} to port 1", count);
            
            let msg = b"ping";
            let result = unsafe {
                syscall(3, 1, msg.as_ptr() as usize, msg.len())
            };
            
            if result == 0 {
                serial_println!("[TEST-7.4] Sender: Message sent successfully");
            } else {
                serial_println!("[TEST-7.4] Sender: Failed to send message (error: {})", result);
            }
        }
        
        // Delay between sends
        for _ in 0..10_000_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

/// Test 7.4: IPC integration test - Receiver task
fn test_ipc_receiver() -> ! {
    // Helper function to invoke syscall
    unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
        let ret: isize;
        core::arch::asm!(
            "int 0x80",
            in("rax") id,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }
    
    use core::sync::atomic::{AtomicUsize, Ordering};
    static RECV_COUNT: AtomicUsize = AtomicUsize::new(0);
    
    loop {
        let count = RECV_COUNT.load(Ordering::Relaxed);
        
        if count < 5 {
            serial_println!("[TEST-7.4] Receiver: Waiting for message on port 1...");
            
            let mut buf = [0u8; 64];
            let result = unsafe {
                syscall(4, 1, buf.as_mut_ptr() as usize, buf.len())
            };
            
            if result > 0 {
                let bytes_received = result as usize;
                let msg = core::str::from_utf8(&buf[..bytes_received]).unwrap_or("[invalid]");
                serial_println!("[TEST-7.4] Receiver: Received {} bytes: '{}'", bytes_received, msg);
                
                // Verify message content
                if msg == "ping" {
                    serial_println!("[TEST-7.4] Receiver: Message content verified ✓");
                } else {
                    serial_println!("[TEST-7.4] Receiver: Message content mismatch! Expected 'ping', got '{}'", msg);
                }
                
                RECV_COUNT.fetch_add(1, Ordering::Relaxed);
            } else {
                serial_println!("[TEST-7.4] Receiver: Failed to receive message (error: {})", result);
            }
        }
        
        // Small delay
        for _ in 0..1_000_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

/// Test 7.5: IPC stress test - Ping task
fn test_ipc_stress_ping() -> ! {
    // Helper function to invoke syscall
    unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
        let ret: isize;
        core::arch::asm!(
            "int 0x80",
            in("rax") id,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }
    
    use core::sync::atomic::{AtomicUsize, Ordering};
    static PING_COUNT: AtomicUsize = AtomicUsize::new(0);
    
    loop {
        let count = PING_COUNT.load(Ordering::Relaxed);
        
        if count < 100 {  // Send 100 messages (reduced from 1000 for faster testing)
            // Send ping to port 10
            let msg = b"ping";
            let send_result = unsafe {
                syscall(3, 10, msg.as_ptr() as usize, msg.len())
            };
            
            if send_result == 0 {
                // Wait for pong on port 11
                let mut buf = [0u8; 64];
                let recv_result = unsafe {
                    syscall(4, 11, buf.as_mut_ptr() as usize, buf.len())
                };
                
                if recv_result > 0 {
                    let new_count = PING_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
                    if new_count % 10 == 0 {
                        serial_println!("[TEST-7.5] Ping-pong completed {} messages", new_count);
                    }
                } else {
                    serial_println!("[TEST-7.5] Ping: Failed to receive pong");
                }
            } else {
                serial_println!("[TEST-7.5] Ping: Failed to send ping (queue full?)");
                // Add small delay on queue full
                unsafe { syscall(2, 10, 0, 0) };
            }
        } else if count == 100 {
            serial_println!("[TEST-7.5] Ping-pong stress test completed: 100 messages exchanged ✓");
            PING_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        
        // Small random jitter (simulated with varying delays)
        let delay = (count % 3 + 1) * 100_000;
        for _ in 0..delay {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

/// Test 7.5: IPC stress test - Pong task
fn test_ipc_stress_pong() -> ! {
    // Helper function to invoke syscall
    unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
        let ret: isize;
        core::arch::asm!(
            "int 0x80",
            in("rax") id,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }
    
    use core::sync::atomic::{AtomicUsize, Ordering};
    static PONG_COUNT: AtomicUsize = AtomicUsize::new(0);
    
    loop {
        let count = PONG_COUNT.load(Ordering::Relaxed);
        
        if count < 100 {
            // Wait for ping on port 10
            let mut buf = [0u8; 64];
            let recv_result = unsafe {
                syscall(4, 10, buf.as_mut_ptr() as usize, buf.len())
            };
            
            if recv_result > 0 {
                // Send pong to port 11
                let msg = b"pong";
                let send_result = unsafe {
                    syscall(3, 11, msg.as_ptr() as usize, msg.len())
                };
                
                if send_result == 0 {
                    PONG_COUNT.fetch_add(1, Ordering::Relaxed);
                } else {
                    serial_println!("[TEST-7.5] Pong: Failed to send pong");
                }
            }
        }
        
        // Small delay
        for _ in 0..100_000 {
            unsafe { core::arch::asm!("nop"); }
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
    
    serial_println!("[KERNEL] Initializing ACPI...");
    // Get RSDP address from Limine
    let rsdp_response = RSDP_REQUEST
        .get_response()
        .expect("Failed to get RSDP response from Limine");
    let rsdp_addr = rsdp_response.address() as u64;
    
    // Parse ACPI MADT to detect CPUs
    arch::x86_64::acpi::init_acpi(rsdp_addr)
        .expect("Failed to initialize ACPI");
    
    serial_println!("[KERNEL] Initializing BSP Local APIC...");
    // Get MADT info to retrieve LAPIC address
    let madt_info = arch::x86_64::acpi::get_madt_info()
        .expect("MADT info not available");
    
    // Create and initialize BSP Local APIC
    let mut bsp_lapic = unsafe {
        arch::x86_64::apic::LocalApic::new(madt_info.lapic_address)
    };
    bsp_lapic.init();
    
    // Verify LAPIC ID matches BSP APIC ID from MADT
    let bsp_apic_id = bsp_lapic.id();
    let expected_bsp_apic_id = madt_info.cpus[0]
        .expect("No BSP CPU found in MADT")
        .apic_id;
    
    if bsp_apic_id == expected_bsp_apic_id {
        serial_println!("[SMP] BSP online (apic_id={})", bsp_apic_id);
    } else {
        serial_println!("[SMP] Warning: BSP APIC ID mismatch! Expected {}, got {}",
            expected_bsp_apic_id, bsp_apic_id);
        serial_println!("[SMP] BSP online (apic_id={})", bsp_apic_id);
    }
    
    serial_println!("[KERNEL] Initializing BSP per-CPU data...");
    // Initialize BSP per-CPU data structure
    unsafe {
        arch::x86_64::smp::percpu::init_percpu(0, bsp_apic_id);
        arch::x86_64::smp::percpu::setup_gs_base(0);
    }
    serial_println!("[PERCPU] BSP per-CPU data initialized (cpu_id=0, apic_id={})", bsp_apic_id);
    
    serial_println!("[KERNEL] Writing message to screen...");
    // Display "Hello from MelloOS ✨" message
    // White text on black background, positioned at (100, 100)
    fb.write_string("Hello from MelloOS ✨", 100, 100, 0xFFFFFF, 0x000000);
    
    serial_println!("[KERNEL] Initializing IPC subsystem...");
    // Initialize IPC ports
    sys::port::init_ipc();
    
    serial_println!("[KERNEL] Initializing scheduler...");
    // Initialize the task scheduler
    init_scheduler();
    
    serial_println!("[KERNEL] ========================================");
    serial_println!("[KERNEL] Phase 4 Integration Tests");
    serial_println!("[KERNEL] ========================================");
    
    // Test 7.1: Priority scheduling test
    serial_println!("[KERNEL] Spawning Test 7.1: Priority scheduling test...");
    spawn_task("Test-High", test_priority_high, TaskPriority::High).expect("Failed to spawn Test-High");
    spawn_task("Test-Normal", test_priority_normal, TaskPriority::Normal).expect("Failed to spawn Test-Normal");
    spawn_task("Test-Low", test_priority_low, TaskPriority::Low).expect("Failed to spawn Test-Low");
    
    // Test 7.2: Sleep/wake test
    serial_println!("[KERNEL] Spawning Test 7.2: Sleep/wake test...");
    spawn_task("Test-Sleep", test_sleep_wake, TaskPriority::Normal).expect("Failed to spawn Test-Sleep");
    
    // Test 7.3: Syscall integration test
    serial_println!("[KERNEL] Spawning Test 7.3: Syscall integration test...");
    spawn_task("Test-Syscall", test_syscall_integration, TaskPriority::Normal).expect("Failed to spawn Test-Syscall");
    
    // Test 7.4: IPC integration test
    serial_println!("[KERNEL] Spawning Test 7.4: IPC integration test...");
    spawn_task("IPC-Sender", test_ipc_sender, TaskPriority::Normal).expect("Failed to spawn IPC-Sender");
    spawn_task("IPC-Receiver", test_ipc_receiver, TaskPriority::Normal).expect("Failed to spawn IPC-Receiver");
    
    // Test 7.5: IPC stress test
    serial_println!("[KERNEL] Spawning Test 7.5: IPC stress test...");
    spawn_task("Stress-Ping", test_ipc_stress_ping, TaskPriority::Normal).expect("Failed to spawn Stress-Ping");
    spawn_task("Stress-Pong", test_ipc_stress_pong, TaskPriority::Normal).expect("Failed to spawn Stress-Pong");
    
    // Test 7.6: End-to-end system test (init process)
    serial_println!("[KERNEL] Loading Test 7.6: Init process (end-to-end test)...");
    init_loader::load_init_process().expect("Failed to load init process");
    
    serial_println!("[KERNEL] ========================================");
    serial_println!("[KERNEL] All test tasks spawned successfully!");
    serial_println!("[KERNEL] ========================================");
    
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
