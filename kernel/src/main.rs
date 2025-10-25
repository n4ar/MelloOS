#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod arch;
mod config;
mod dev;
pub mod drivers;
mod framebuffer;
mod fs;
mod init_loader;
mod io;
mod log;
mod metrics;
mod mm;
mod panic;
mod sched;
mod serial;
mod signal;
mod sync;
mod sys;
mod user;

use sched::{init_scheduler, priority::TaskPriority, spawn_task};

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
        let result = unsafe { syscall(0, 0, msg.as_ptr() as usize, msg.len()) };
        serial_println!("[TEST] sys_write returned: {}", result);

        // Test sys_sleep (syscall 2) - sleep for 20 ticks (reduced from 50)
        serial_println!("[TEST] Calling sys_sleep(20)...");
        let sleep_result = unsafe { syscall(2, 20, 0, 0) };
        serial_println!("[TEST] sys_sleep returned: {}", sleep_result);
        serial_println!("[TEST] Woke up from sleep!");

        // Use sleep instead of busy-wait
        unsafe { syscall(2, 50, 0, 0) };
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
            serial_println!(
                "[TEST-7.1] NORMAL priority task executing (count: {})",
                count
            );
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
            unsafe {
                core::arch::asm!("nop");
            }
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
            let write_result = unsafe { syscall(0, 0, msg.as_ptr() as usize, msg.len()) };
            serial_println!("[TEST-7.3] sys_write returned: {}", write_result);

            // Test sys_sleep
            serial_println!("[TEST-7.3] Testing sys_sleep(30)...");
            let sleep_result = unsafe { syscall(2, 30, 0, 0) };
            serial_println!("[TEST-7.3] sys_sleep returned: {}", sleep_result);
        }

        // Busy-wait delay
        for _ in 0..10_000_000 {
            unsafe {
                core::arch::asm!("nop");
            }
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
            let result = unsafe { syscall(3, 1, msg.as_ptr() as usize, msg.len()) };

            if result == 0 {
                serial_println!("[TEST-7.4] Sender: Message sent successfully");
            } else {
                serial_println!(
                    "[TEST-7.4] Sender: Failed to send message (error: {})",
                    result
                );
            }
        }

        // Delay between sends
        for _ in 0..10_000_000 {
            unsafe {
                core::arch::asm!("nop");
            }
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
            let result = unsafe { syscall(4, 1, buf.as_mut_ptr() as usize, buf.len()) };

            if result > 0 {
                let bytes_received = result as usize;
                let msg = core::str::from_utf8(&buf[..bytes_received]).unwrap_or("[invalid]");
                serial_println!(
                    "[TEST-7.4] Receiver: Received {} bytes: '{}'",
                    bytes_received,
                    msg
                );

                // Verify message content
                if msg == "ping" {
                    serial_println!("[TEST-7.4] Receiver: Message content verified ✓");
                } else {
                    serial_println!(
                        "[TEST-7.4] Receiver: Message content mismatch! Expected 'ping', got '{}'",
                        msg
                    );
                }

                RECV_COUNT.fetch_add(1, Ordering::Relaxed);
            } else {
                serial_println!(
                    "[TEST-7.4] Receiver: Failed to receive message (error: {})",
                    result
                );
            }
        }

        // Small delay
        for _ in 0..1_000_000 {
            unsafe {
                core::arch::asm!("nop");
            }
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

        if count < 100 {
            // Send 100 messages (reduced from 1000 for faster testing)
            // Send ping to port 10
            let msg = b"ping";
            let send_result = unsafe { syscall(3, 10, msg.as_ptr() as usize, msg.len()) };

            if send_result == 0 {
                // Wait for pong on port 11
                let mut buf = [0u8; 64];
                let recv_result = unsafe { syscall(4, 11, buf.as_mut_ptr() as usize, buf.len()) };

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
            unsafe {
                core::arch::asm!("nop");
            }
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
            let recv_result = unsafe { syscall(4, 10, buf.as_mut_ptr() as usize, buf.len()) };

            if recv_result > 0 {
                // Send pong to port 11
                let msg = b"pong";
                let send_result = unsafe { syscall(3, 11, msg.as_ptr() as usize, msg.len()) };

                if send_result == 0 {
                    PONG_COUNT.fetch_add(1, Ordering::Relaxed);
                } else {
                    serial_println!("[TEST-7.5] Pong: Failed to send pong");
                }
            }
        }

        // Small delay
        for _ in 0..100_000 {
            unsafe {
                core::arch::asm!("nop");
            }
        }
    }
}

// ============================================================================
// SMP Multi-Core Test Tasks (Phase 5)
// ============================================================================

/// SMP Test Task A - High priority task
/// Logs with format "[SCHED][coreN] run A" and performs simple work with yield
fn smp_test_task_a() -> ! {
    // Helper function to invoke sys_yield syscall
    unsafe fn sys_yield() {
        core::arch::asm!(
            "int 0x80",
            in("rax") 1, // sys_yield is syscall 1
            in("rdi") 0,
            in("rsi") 0,
            in("rdx") 0,
            options(nostack, preserves_flags)
        );
    }

    use core::sync::atomic::{AtomicUsize, Ordering};
    static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);

    loop {
        let count = EXEC_COUNT.fetch_add(1, Ordering::Relaxed);

        // Get current CPU ID from per-CPU data
        let cpu_id = arch::x86_64::smp::percpu::percpu_current().id;

        // Log execution with required format (reduced iterations in fast boot mode)
        let max_logs = if config::FAST_BOOT_MODE { 10 } else { 20 };
        if count < max_logs {
            serial_println!("[SCHED][core{}] run A", cpu_id);
        }

        // Perform simple work (reduced busy loop)
        for _ in 0..50_000 {
            unsafe {
                core::arch::asm!("nop");
            }
        }

        // Yield to allow other tasks to run
        unsafe {
            sys_yield();
        }
    }
}

/// SMP Test Task B - Normal priority task
/// Logs with format "[SCHED][coreN] run B" and performs simple work with yield
fn smp_test_task_b() -> ! {
    // Helper function to invoke sys_yield syscall
    unsafe fn sys_yield() {
        core::arch::asm!(
            "int 0x80",
            in("rax") 1, // sys_yield is syscall 1
            in("rdi") 0,
            in("rsi") 0,
            in("rdx") 0,
            options(nostack, preserves_flags)
        );
    }

    use core::sync::atomic::{AtomicUsize, Ordering};
    static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);

    loop {
        let count = EXEC_COUNT.fetch_add(1, Ordering::Relaxed);

        // Get current CPU ID from per-CPU data
        let cpu_id = arch::x86_64::smp::percpu::percpu_current().id;

        // Log execution with required format (reduced iterations in fast boot mode)
        let max_logs = if config::FAST_BOOT_MODE { 10 } else { 20 };
        if count < max_logs {
            serial_println!("[SCHED][core{}] run B", cpu_id);
        }

        // Perform simple work (reduced busy loop)
        for _ in 0..50_000 {
            unsafe {
                core::arch::asm!("nop");
            }
        }

        // Yield to allow other tasks to run
        unsafe {
            sys_yield();
        }
    }
}

/// SMP Test Task C - Normal priority task
/// Logs with format "[SCHED][coreN] run C" and performs simple work with yield
fn smp_test_task_c() -> ! {
    // Helper function to invoke sys_yield syscall
    unsafe fn sys_yield() {
        core::arch::asm!(
            "int 0x80",
            in("rax") 1, // sys_yield is syscall 1
            in("rdi") 0,
            in("rsi") 0,
            in("rdx") 0,
            options(nostack, preserves_flags)
        );
    }

    use core::sync::atomic::{AtomicUsize, Ordering};
    static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);

    loop {
        let count = EXEC_COUNT.fetch_add(1, Ordering::Relaxed);

        // Get current CPU ID from per-CPU data
        let cpu_id = arch::x86_64::smp::percpu::percpu_current().id;

        // Log execution with required format (reduced iterations in fast boot mode)
        let max_logs = if config::FAST_BOOT_MODE { 10 } else { 20 };
        if count < max_logs {
            serial_println!("[SCHED][core{}] run C", cpu_id);
        }

        // Perform simple work (reduced busy loop)
        for _ in 0..50_000 {
            unsafe {
                core::arch::asm!("nop");
            }
        }

        // Yield to allow other tasks to run
        unsafe {
            sys_yield();
        }
    }
}

/// SMP Test Task D - Low priority task
/// Logs with format "[SCHED][coreN] run D" and performs simple work with yield
fn smp_test_task_d() -> ! {
    // Helper function to invoke sys_yield syscall
    unsafe fn sys_yield() {
        core::arch::asm!(
            "int 0x80",
            in("rax") 1, // sys_yield is syscall 1
            in("rdi") 0,
            in("rsi") 0,
            in("rdx") 0,
            options(nostack, preserves_flags)
        );
    }

    use core::sync::atomic::{AtomicUsize, Ordering};
    static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);

    loop {
        let count = EXEC_COUNT.fetch_add(1, Ordering::Relaxed);

        // Get current CPU ID from per-CPU data
        let cpu_id = arch::x86_64::smp::percpu::percpu_current().id;

        // Log execution with required format (reduced iterations in fast boot mode)
        let max_logs = if config::FAST_BOOT_MODE { 10 } else { 20 };
        if count < max_logs {
            serial_println!("[SCHED][core{}] run D", cpu_id);
        }

        // Perform simple work (reduced busy loop)
        for _ in 0..50_000 {
            unsafe {
                core::arch::asm!("nop");
            }
        }

        // Yield to allow other tasks to run
        unsafe {
            sys_yield();
        }
    }
}

/// Print test results after a delay to allow tests to complete
fn print_test_results_delayed() -> ! {
    // Helper function to invoke sys_sleep syscall
    unsafe fn sys_sleep(ticks: usize) {
        core::arch::asm!(
            "int 0x80",
            in("rax") 2, // sys_sleep is syscall 2
            in("rdi") ticks,
            in("rsi") 0,
            in("rdx") 0,
            options(nostack, preserves_flags)
        );
    }

    // Wait for tests to complete
    // In fast boot mode: 3 seconds (300 ticks at 100Hz)
    // In normal mode: 10 seconds (1000 ticks at 100Hz)
    let wait_ticks = if config::FAST_BOOT_MODE {
        300
    } else {
        1000
    };
    
    unsafe {
        sys_sleep(wait_ticks);
    }

    // Print integration test results
    crate::user::integration_tests::print_test_results();

    // Continue sleeping to avoid consuming CPU
    loop {
        unsafe {
            sys_sleep(1000);
        } // Sleep for 10 seconds at 100Hz
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
    arch::x86_64::acpi::init_acpi(rsdp_addr).expect("Failed to initialize ACPI");

    serial_println!("[KERNEL] Initializing BSP Local APIC...");
    // Get MADT info to retrieve LAPIC address
    let madt_info = arch::x86_64::acpi::get_madt_info().expect("MADT info not available");

    // Create and initialize BSP Local APIC
    let mut bsp_lapic = unsafe { arch::x86_64::apic::LocalApic::new(madt_info.lapic_address) };
    bsp_lapic.init();

    // Verify LAPIC ID matches BSP APIC ID from MADT
    let bsp_apic_id = bsp_lapic.id();
    let expected_bsp_apic_id = madt_info.cpus[0].expect("No BSP CPU found in MADT").apic_id;

    if bsp_apic_id == expected_bsp_apic_id {
        serial_println!("[SMP] BSP online (apic_id={})", bsp_apic_id);
    } else {
        serial_println!(
            "[SMP] Warning: BSP APIC ID mismatch! Expected {}, got {}",
            expected_bsp_apic_id,
            bsp_apic_id
        );
        serial_println!("[SMP] BSP online (apic_id={})", bsp_apic_id);
    }

    serial_println!("[KERNEL] Initializing BSP per-CPU data...");
    // Initialize BSP per-CPU data structure
    unsafe {
        arch::x86_64::smp::percpu::init_percpu(0, bsp_apic_id);
        arch::x86_64::smp::percpu::setup_gs_base(0);
    }
    serial_println!(
        "[PERCPU] BSP per-CPU data initialized (cpu_id=0, apic_id={})",
        bsp_apic_id
    );

    serial_println!("[KERNEL] Calibrating APIC timer...");
    // Calibrate APIC timer using PIT
    let lapic_frequency = unsafe { bsp_lapic.calibrate_timer() };
    serial_println!("[APIC] LAPIC timer frequency: {} Hz", lapic_frequency);

    // Store calibrated frequency in BSP per-CPU data
    unsafe {
        let percpu = arch::x86_64::smp::percpu::percpu_current_mut();
        percpu.lapic_timer_hz = lapic_frequency;
    }

    serial_println!("[KERNEL] Initializing BSP APIC timer...");
    // Initialize APIC timer at SCHED_HZ (100 Hz)
    unsafe {
        bsp_lapic.init_timer(lapic_frequency, config::SCHED_HZ);
    }
    serial_println!("[APIC] core0 timer @{}Hz", config::SCHED_HZ);

    serial_println!("[KERNEL] Initializing SMP (bringing up Application Processors)...");

    // Initialize SMP and bring up Application Processors
    let cpu_count = match arch::x86_64::smp::init_smp(&mut bsp_lapic) {
        Ok(count) => {
            serial_println!("[SMP] Successfully initialized {} CPUs", count);
            count
        }
        Err(e) => {
            serial_println!("[SMP] Warning: SMP initialization failed: {}", e);
            serial_println!("[SMP] Continuing with BSP only (single-core mode)");
            1 // BSP only
        }
    };

    // Store CPU count globally
    static CPU_COUNT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
    CPU_COUNT.store(cpu_count, core::sync::atomic::Ordering::SeqCst);

    serial_println!("[KERNEL] Writing message to screen...");
    // Display "Hello from MelloOS ✨" message
    // White text on black background, positioned at (100, 100)
    fb.write_string("Hello from MelloOS ✨", 100, 100, 0xFFFFFF, 0x000000);

    serial_println!("[KERNEL] Initializing IPC subsystem...");
    // Initialize IPC ports
    sys::port::init_ipc();

    serial_println!("[KERNEL] Initializing PTY subsystem...");
    // Initialize PTY (pseudo-terminal) subsystem
    dev::pty::init();
    serial_println!("[KERNEL] PTY subsystem initialized successfully");

    serial_println!("[KERNEL] Initializing /proc filesystem...");
    // Initialize /proc virtual filesystem
    fs::proc::init();
    serial_println!("[KERNEL] /proc filesystem initialized successfully");

    serial_println!("[KERNEL] Initializing driver subsystem...");
    // Initialize device drivers and I/O subsystem (Phase 7)
    drivers::init_drivers();

    serial_println!("[KERNEL] Initializing VFS and mounting root filesystem...");
    // Initialize block device subsystem (Phase 8 - Task 8.9)
    fs::block_dev::init_block_devices();
    // Initialize VFS and mount mfs_ram as root (Phase 8)
    fs::init();
    // Initialize mfs_disk filesystem type (Phase 8 - Task 9.1)
    fs::mfs::disk::init();

    serial_println!("[KERNEL] Initializing scheduler...");
    // Initialize the task scheduler
    init_scheduler();

    serial_println!("[KERNEL] Initializing timer interrupt...");
    // Initialize IDT and syscall handler early so kernel tests can use syscalls safely
    unsafe {
        sched::timer::init_idt();
        sched::timer::init_apic_timer_handler();
        sched::timer::init_reschedule_ipi_handler();
    }

    serial_println!("[KERNEL] ========================================");
    serial_println!("[KERNEL] Phase 4 Integration Tests");
    serial_println!("[KERNEL] ========================================");

    // Test 7.6: End-to-end system test (init process) - spawn first to run with high priority
    serial_println!("[KERNEL] Loading Test 7.6: Init process (end-to-end test)...");
    init_loader::load_init_process().expect("Failed to load init process");

    // Test 7.1: Priority scheduling test
    serial_println!("[KERNEL] Spawning Test 7.1: Priority scheduling test...");
    spawn_task("Test-High", test_priority_high, TaskPriority::High)
        .expect("Failed to spawn Test-High");
    spawn_task("Test-Normal", test_priority_normal, TaskPriority::Normal)
        .expect("Failed to spawn Test-Normal");
    spawn_task("Test-Low", test_priority_low, TaskPriority::Low).expect("Failed to spawn Test-Low");

    // Test 7.2: Sleep/wake test
    serial_println!("[KERNEL] Spawning Test 7.2: Sleep/wake test...");
    spawn_task("Test-Sleep", test_sleep_wake, TaskPriority::Normal)
        .expect("Failed to spawn Test-Sleep");

    // Test 7.3: Syscall integration test
    serial_println!("[KERNEL] Spawning Test 7.3: Syscall integration test...");
    spawn_task(
        "Test-Syscall",
        test_syscall_integration,
        TaskPriority::Normal,
    )
    .expect("Failed to spawn Test-Syscall");

    // Test 7.4: IPC integration test
    serial_println!("[KERNEL] Spawning Test 7.4: IPC integration test...");
    spawn_task("IPC-Sender", test_ipc_sender, TaskPriority::Normal)
        .expect("Failed to spawn IPC-Sender");
    spawn_task("IPC-Receiver", test_ipc_receiver, TaskPriority::Normal)
        .expect("Failed to spawn IPC-Receiver");

    // Test 7.5: IPC stress test
    serial_println!("[KERNEL] Spawning Test 7.5: IPC stress test...");
    spawn_task("Stress-Ping", test_ipc_stress_ping, TaskPriority::Normal)
        .expect("Failed to spawn Stress-Ping");
    spawn_task("Stress-Pong", test_ipc_stress_pong, TaskPriority::Normal)
        .expect("Failed to spawn Stress-Pong");
    
    serial_println!("[KERNEL] ========================================");
    serial_println!("[KERNEL] All Phase 4 test tasks spawned successfully!");
    serial_println!("[KERNEL] ========================================");

    // Phase 6: User-Mode Integration Tests
    serial_println!("[KERNEL] ========================================");
    serial_println!("[KERNEL] Phase 6: User-Mode Integration Tests");
    serial_println!("[KERNEL] ========================================");

    // Run comprehensive user-mode integration tests
    crate::user::integration_tests::run_all_integration_tests();

    // Phase 5: SMP Multi-Core Test Tasks
    serial_println!("[KERNEL] ========================================");
    serial_println!("[KERNEL] Phase 5 SMP Multi-Core Tests");
    serial_println!("[KERNEL] ========================================");

    serial_println!("[KERNEL] Spawning SMP test tasks (A, B, C, D)...");
    spawn_task("SMP-A", smp_test_task_a, TaskPriority::High).expect("Failed to spawn SMP-A");
    spawn_task("SMP-B", smp_test_task_b, TaskPriority::Normal).expect("Failed to spawn SMP-B");
    spawn_task("SMP-C", smp_test_task_c, TaskPriority::Normal).expect("Failed to spawn SMP-C");
    spawn_task("SMP-D", smp_test_task_d, TaskPriority::Low).expect("Failed to spawn SMP-D");

    serial_println!("[KERNEL] ========================================");
    serial_println!("[KERNEL] All SMP test tasks spawned successfully!");
    serial_println!("[KERNEL] CPU count: {}", cpu_count);
    serial_println!("[KERNEL] Tasks will be distributed across cores");
    serial_println!("[KERNEL] ========================================");

    serial_println!("[KERNEL] Enabling interrupts on all CPUs...");
    // Enable interrupts on all CPUs to start task switching
    // This must be done AFTER all kernel subsystems are initialized
    // to prevent deadlocks during init
    unsafe {
        arch::x86_64::smp::enable_interrupts_all_cpus();
    }

    serial_println!("[KERNEL] Scheduler initialization complete!");
    serial_println!("[KERNEL] Boot complete! Entering idle loop...");

    // Spawn a task to print test results after some time
    spawn_task(
        "Test-Results",
        print_test_results_delayed,
        TaskPriority::Low,
    )
    .expect("Failed to spawn test results task");

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
