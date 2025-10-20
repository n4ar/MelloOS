/// SMP (Symmetric Multi-Processing) support
/// This module provides CPU core discovery, AP (Application Processor) bringup,
/// and per-CPU data structures.

pub mod percpu;

use crate::arch::x86_64::apic::LocalApic;
use crate::arch::x86_64::acpi::get_madt_info;
use crate::config::MAX_CPUS;
use crate::serial_println;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Array of flags indicating which CPUs are online
static CPU_ONLINE: [AtomicBool; MAX_CPUS] = {
    const INIT: AtomicBool = AtomicBool::new(false);
    [INIT; MAX_CPUS]
};

/// Counter for the number of CPUs that have come online
static CPU_COUNT: AtomicUsize = AtomicUsize::new(0);

/// 64-bit entry point for Application Processors
/// 
/// This function is called by the AP trampoline code after the AP has
/// transitioned to 64-bit long mode. It performs the following steps:
/// 
/// 1. Reads the CPU ID from the trampoline structure
/// 2. Initializes the AP's PerCpu structure
/// 3. Configures the AP's GS.BASE MSR
/// 4. Initializes the AP's Local APIC
/// 5. Signals the BSP that the AP is online
/// 6. Enters the scheduler idle loop
/// 
/// # Arguments
/// * `cpu_id` - Logical CPU ID assigned by the BSP
/// 
/// # Safety
/// This function is called from assembly code and must not return.
/// It assumes that the trampoline has properly set up the stack and
/// page tables.
#[no_mangle]
pub extern "C" fn ap_entry64(cpu_id: usize, apic_id: u8, lapic_address: u64) -> ! {
    // ULTRA-EARLY debug: write '1' to serial before any Rust code
    unsafe {
        core::arch::asm!(
            "mov al, '1'",
            "mov dx, 0x3F8",
            "out dx, al",
            options(nostack, nomem)
        );
    }
    
    // Disable interrupts during initialization
    unsafe {
        core::arch::asm!("cli", options(nostack, nomem));
    }
    
    // Debug: '2' after cli
    unsafe {
        core::arch::asm!(
            "mov al, '2'",
            "mov dx, 0x3F8",
            "out dx, al",
            options(nostack, nomem)
        );
    }
    
    // TEMPORARILY COMMENTED: serial_println may deadlock if BSP holds lock
    // serial_println!("[SMP] AP#{} entered Rust (cpu_id={}, apic_id={}, lapic=0x{:X})", 
    //                cpu_id, cpu_id, apic_id, lapic_address);
    
    // Debug: '3' before init_percpu
    unsafe {
        core::arch::asm!(
            "mov al, '3'",
            "mov dx, 0x3F8",
            "out dx, al",
            options(nostack, nomem)
        );
    }

    // Match BSP feature setup so NX-marked pages are valid on this core
    crate::mm::enable_nx_bit();
    crate::mm::enable_write_protect();
    
    // Initialize PerCpu structure for this AP
    unsafe {
        percpu::init_percpu(cpu_id, apic_id);
    }
    
    // Debug: '4' after init_percpu
    unsafe {
        core::arch::asm!(
            "mov al, '4'",
            "mov dx, 0x3F8",
            "out dx, al",
            options(nostack, nomem)
        );
    }
    
    // Debug: '5' before setup_gs_base
    unsafe {
        core::arch::asm!(
            "mov al, '5'",
            "mov dx, 0x3F8",
            "out dx, al",
            options(nostack, nomem)
        );
    }
    
    // Configure GS.BASE MSR to point to our PerCpu structure
    unsafe {
        percpu::setup_gs_base(cpu_id);
    }
    
    // Debug: '6' after setup_gs_base
    unsafe {
        core::arch::asm!(
            "mov al, '6'",
            "mov dx, 0x3F8",
            "out dx, al",
            options(nostack, nomem)
        );
    }
    
    // TEMPORARILY COMMENTED: serial_println may deadlock
    // serial_println!("[SMP] AP#{} PerCpu and GS.BASE initialized", cpu_id);
    
    // Initialize GDT and TSS for this AP
    if let Err(e) = crate::arch::x86_64::gdt::init_gdt_tss_for_cpu(cpu_id) {
        serial_println!("[SMP] AP#{} failed to initialize GDT/TSS: {}", cpu_id, e);
        loop { unsafe { core::arch::asm!("hlt"); } }
    }
    
    // Initialize Local APIC for this AP using passed address
    let mut lapic = unsafe { LocalApic::new(lapic_address) };
    lapic.init();
    
    // Verify LAPIC ID matches expected APIC ID
    let actual_apic_id = lapic.id();
    if actual_apic_id != apic_id {
        serial_println!("[SMP] AP#{} warning: LAPIC ID mismatch (expected {}, got {})",
                       cpu_id, apic_id, actual_apic_id);
    }
    
    // Calibrate APIC timer for this AP
    let lapic_frequency = unsafe { lapic.calibrate_timer() };
    
    // Store calibrated frequency in AP per-CPU data
    unsafe {
        let percpu = percpu::percpu_current_mut();
        percpu.lapic_timer_hz = lapic_frequency;
    }
    
    // Initialize APIC timer at SCHED_HZ (100 Hz)
    unsafe {
        lapic.init_timer(lapic_frequency, crate::config::SCHED_HZ);
    }
    serial_println!("[APIC] core{} timer @{}Hz", cpu_id, crate::config::SCHED_HZ);
    
    // Signal BSP that we are online
    CPU_ONLINE[cpu_id].store(true, Ordering::Release);
    CPU_COUNT.fetch_add(1, Ordering::SeqCst);
    
    // Log that we are online
    serial_println!("[SMP] AP#{} online", cpu_id);
    
    // Enable interrupts
    unsafe {
        core::arch::asm!("sti", options(nostack, nomem));
    }
    
    // Enter idle loop - for now just halt
    // TODO: Call scheduler when scheduler is SMP-ready
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nostack, nomem));
        }
    }
}

/// Check if a specific CPU is online
/// 
/// # Arguments
/// * `cpu_id` - Logical CPU ID
/// 
/// # Returns
/// `true` if the CPU is online, `false` otherwise
pub fn is_cpu_online(cpu_id: usize) -> bool {
    if cpu_id >= MAX_CPUS {
        return false;
    }
    CPU_ONLINE[cpu_id].load(Ordering::Acquire)
}

/// Get the total number of CPUs that are currently online
/// 
/// # Returns
/// The number of online CPUs
pub fn get_cpu_count() -> usize {
    CPU_COUNT.load(Ordering::SeqCst)
}

/// Get the current CPU's logical ID
/// 
/// # Returns
/// The logical CPU ID (0 for BSP, 1..N for APs)
pub fn get_current_cpu_id() -> usize {
    percpu::percpu_current().id
}

/// Trampoline memory layout constants
const TRAMPOLINE_BASE: usize = 0x8000;
const TRAMPOLINE_SIZE: usize = 0x1000; // 4KB
const TRAMPOLINE_STACK_PTR: usize = 0x8300;
const TRAMPOLINE_ENTRY_PTR: usize = 0x8308;
const TRAMPOLINE_CR3: usize = 0x8310;
const TRAMPOLINE_CPU_ID: usize = 0x8318;
const TRAMPOLINE_APIC_ID: usize = 0x8320;
const TRAMPOLINE_LAPIC_ADDR: usize = 0x8328;

/// AP stack size (16KB per AP)
const AP_STACK_SIZE: usize = 16 * 1024;

/// AP stacks physical base (starts at 0x100000 = 1MB mark)
/// This is within the identity-mapped region (0x0-0x1FFFFF)
const AP_STACK_PHYS_BASE: usize = 0x100000;

/// Trampoline binary data
/// This is the compiled AP boot code that will be copied to 0x8000
static TRAMPOLINE_CODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/boot_ap.bin"));

/// Initialize GDT and TSS for BSP (CPU 0)
/// This should be called early in kernel initialization
pub fn init_bsp_gdt_tss() -> Result<(), &'static str> {
    serial_println!("[SMP] Initializing GDT and TSS for BSP (CPU 0)");
    
    // Initialize BSP's PerCpu structure first
    unsafe {
        percpu::init_percpu(0, 0); // BSP is CPU 0, APIC ID will be set later
        percpu::setup_gs_base(0);
    }
    
    // Initialize GDT and TSS for BSP
    crate::arch::x86_64::gdt::init_gdt_tss_for_cpu(0)?;
    
    serial_println!("[SMP] BSP GDT and TSS initialized successfully");
    Ok(())
}

/// Initialize SMP and bring up Application Processors
/// 
/// This function performs the following steps:
/// 1. Sets up the AP trampoline code at physical address 0x8000
/// 2. Creates identity mapping for the trampoline region (0x0000-0x9FFF)
/// 3. For each AP in the MADT:
///    - Allocates a stack
///    - Writes trampoline parameters (stack, entry point, CR3, CPU ID)
///    - Sends INIT IPI
///    - Waits 10ms
///    - Sends two SIPI IPIs with 200μs delay
///    - Waits up to 100ms for AP to come online
/// 4. Returns the total number of online CPUs
/// 
/// # Arguments
/// * `lapic` - Mutable reference to the BSP's Local APIC
/// 
/// # Returns
/// The total number of online CPUs (including BSP)
/// 
/// # Errors
/// Returns an error string if initialization fails
pub fn init_smp(lapic: &mut LocalApic) -> Result<usize, &'static str> {
    use crate::mm::paging::{PageMapper, PageTableFlags};
    use crate::mm::pmm::PhysicalMemoryManager;
    
    serial_println!("[SMP] Initializing SMP...");
    
    // Get MADT info
    let madt_info = get_madt_info()
        .ok_or("Failed to get MADT info")?;
    
    // Copy trampoline code to 0x8000
    serial_println!("[SMP] Copying trampoline to 0x{:X} ({} bytes)", 
                   TRAMPOLINE_BASE, TRAMPOLINE_CODE.len());
    
    unsafe {
        let trampoline_dest = TRAMPOLINE_BASE as *mut u8;
        core::ptr::copy_nonoverlapping(
            TRAMPOLINE_CODE.as_ptr(),
            trampoline_dest,
            TRAMPOLINE_CODE.len()
        );
    }
    
    // Create identity mapping for trampoline region (0x0000-0x9FFF)
    // This allows the AP to access the trampoline code in real mode
    serial_println!("[SMP] Creating identity mapping for 0x0000-0x9FFF");
    
    // Identity map low memory directly by modifying page tables
    // We'll do this manually without PMM to avoid complexity
    unsafe {
        identity_map_low_memory()?;
    }
    
    serial_println!("[SMP] Identity mapping created for low memory");
    
    // Verify that higher-half kernel mappings exist in page table
    unsafe {
        use crate::mm::phys_to_virt;
        let test_cr3: u64;
        core::arch::asm!(
            "mov {}, cr3",
            out(reg) test_cr3,
            options(nostack, preserves_flags)
        );
        let pml4_phys = (test_cr3 & 0x000F_FFFF_FFFF_F000) as usize;
        let pml4_virt = phys_to_virt(pml4_phys);
        let pml4 = &*(pml4_virt as *const [u64; 512]);
        
        // Check PML4 entry for higher-half kernel (0xFFFF800000000000+)
        // Kernel is typically at index 256 (0x100)
        let kernel_pml4_idx = 256;
        serial_println!("[SMP] DEBUG: PML4[{}] (higher-half) = 0x{:016X}", 
                       kernel_pml4_idx, pml4[kernel_pml4_idx]);
        
        if (pml4[kernel_pml4_idx] & 0x1) == 0 {
            serial_println!("[SMP] ERROR: No higher-half kernel mapping in page table!");
            serial_println!("[SMP] ERROR: Entry point 0x{:X} cannot be accessed!", ap_entry64 as u64);
            return Err("Missing higher-half kernel mappings");
        }
        serial_println!("[SMP] Higher-half kernel mappings verified");
    }
    
    // Get current CR3 value to pass to APs
    let cr3 = unsafe {
        let cr3_val: u64;
        core::arch::asm!(
            "mov {}, cr3",
            out(reg) cr3_val,
            options(nostack, preserves_flags)
        );
        cr3_val
    };
    
    // Write entry point address to trampoline
    unsafe {
        let entry_ptr = TRAMPOLINE_ENTRY_PTR as *mut u64;
        *entry_ptr = ap_entry64 as u64;
        
        // Write CR3 to trampoline
        let cr3_ptr = TRAMPOLINE_CR3 as *mut u64;
        *cr3_ptr = cr3;
        
        // Debug output
        serial_println!("[SMP] DEBUG: CR3 = 0x{:016X}", cr3);
        serial_println!("[SMP] DEBUG: Entry point = 0x{:016X}", ap_entry64 as u64);
    }
    
    // Count enabled CPUs (excluding BSP)
    let ap_count = madt_info.cpus.iter()
        .filter_map(|cpu| *cpu)
        .filter(|cpu| cpu.enabled && cpu.apic_id != lapic.id())
        .count();
    
    serial_println!("[SMP] Found {} APs to initialize", ap_count);
    
    // Initialize each AP
    let mut cpu_id = 1; // BSP is CPU 0
    for cpu_opt in &madt_info.cpus {
        // Skip None entries
        let cpu_info = match cpu_opt {
            Some(info) => info,
            None => continue,
        };
        
        // Skip BSP and disabled CPUs
        if !cpu_info.enabled || cpu_info.apic_id == lapic.id() {
            continue;
        }
        
        serial_println!("[SMP] Initializing AP#{} (apic_id={})", cpu_id, cpu_info.apic_id);
        
        // Allocate stack for this AP in identity-mapped region
        // Stack grows downward, so we place each AP's stack at a different offset
        // AP#1 at 0x100000-0x104000, AP#2 at 0x104000-0x108000, etc.
        let stack_phys_base = AP_STACK_PHYS_BASE + ((cpu_id - 1) * AP_STACK_SIZE);
        let stack_top = stack_phys_base + AP_STACK_SIZE;
        
        // Verify stack is within identity-mapped region (0x0-0x1FFFFF)
        if stack_top > 0x1FFFFF {
            serial_println!("[SMP] ERROR: AP#{} stack at 0x{:X} exceeds identity-mapped region", 
                           cpu_id, stack_top);
            return Err("AP stack outside identity-mapped region");
        }
        
        // Write stack pointer and CPU ID to trampoline
        unsafe {
            let stack_ptr = TRAMPOLINE_STACK_PTR as *mut u64;
            *stack_ptr = stack_top as u64;
            
            let cpu_id_ptr = TRAMPOLINE_CPU_ID as *mut u64;
            *cpu_id_ptr = cpu_id as u64;
            
            // Write APIC ID to trampoline
            let apic_id_ptr = TRAMPOLINE_APIC_ID as *mut u64;
            *apic_id_ptr = cpu_info.apic_id as u64;
            
            // Write LAPIC address to trampoline
            let lapic_addr_ptr = TRAMPOLINE_LAPIC_ADDR as *mut u64;
            *lapic_addr_ptr = madt_info.lapic_address;
            
            // Debug output
            serial_println!("[SMP] DEBUG: AP#{} stack = 0x{:016X} (identity-mapped physical)", 
                           cpu_id, stack_top);
        }
        
        // Send INIT IPI
        serial_println!("[SMP] Sending INIT IPI to AP#{}", cpu_id);
        if !lapic.send_init_ipi(cpu_info.apic_id) {
            serial_println!("[SMP] Failed to send INIT IPI to AP#{}", cpu_id);
            continue;
        }
        
        // Wait 10ms
        busy_wait_ms(10);
        
        // Send first SIPI
        serial_println!("[SMP] Sending SIPI #1 to AP#{}", cpu_id);
        let start_page = (TRAMPOLINE_BASE >> 12) as u8; // 0x8000 >> 12 = 0x08
        if !lapic.send_sipi(cpu_info.apic_id, start_page) {
            serial_println!("[SMP] Failed to send SIPI #1 to AP#{}", cpu_id);
            continue;
        }
        
        // Wait 200μs
        busy_wait_us(200);
        
        // Send second SIPI
        serial_println!("[SMP] Sending SIPI #2 to AP#{}", cpu_id);
        if !lapic.send_sipi(cpu_info.apic_id, start_page) {
            serial_println!("[SMP] Failed to send SIPI #2 to AP#{}", cpu_id);
            continue;
        }
        
        // Wait up to 100ms for AP to come online
        let mut timeout = 100;
        while timeout > 0 && !is_cpu_online(cpu_id) {
            busy_wait_ms(1);
            timeout -= 1;
        }
        
        if is_cpu_online(cpu_id) {
            serial_println!("[SMP] AP#{} came online successfully", cpu_id);
        } else {
            serial_println!("[SMP] AP#{} failed to come online (timeout)", cpu_id);
        }
        
        cpu_id += 1;
    }
    
    let total_cpus = get_cpu_count();
    serial_println!("[SMP] SMP initialization complete: {} CPUs online", total_cpus);
    
    Ok(total_cpus)
}

/// Busy-wait for approximately the specified number of milliseconds
/// 
/// This is a simple busy-wait loop that uses the CPU's pause instruction
/// to reduce power consumption and bus contention.
/// 
/// # Arguments
/// * `ms` - Number of milliseconds to wait
fn busy_wait_ms(ms: u64) {
    // Approximate: 1ms = 1,000,000 iterations with pause
    // This is very rough and depends on CPU speed
    for _ in 0..(ms * 100_000) {
        unsafe {
            core::arch::asm!("pause", options(nostack, nomem));
        }
    }
}

/// Busy-wait for approximately the specified number of microseconds
/// 
/// # Arguments
/// * `us` - Number of microseconds to wait
fn busy_wait_us(us: u64) {
    // Approximate: 1μs = 1,000 iterations with pause
    for _ in 0..(us * 100) {
        unsafe {
            core::arch::asm!("pause", options(nostack, nomem));
        }
    }
}

/// Identity map low memory (0x0000-0x1FFFFF) for AP trampoline using 2MB huge page
/// 
/// This function creates an identity mapping for the first 2MB of physical memory
/// using a huge page, which is simpler and more reliable than using 4KB pages.
/// 
/// # Safety
/// This function directly manipulates page tables and must be called during
/// SMP initialization before APs are started.
unsafe fn identity_map_low_memory() -> Result<(), &'static str> {
    use crate::mm::{phys_to_virt, virt_to_phys, allocator::kmalloc};
    
    // Get current CR3 (PML4 address)
    let cr3: u64;
    core::arch::asm!(
        "mov {}, cr3",
        out(reg) cr3,
        options(nostack, preserves_flags)
    );
    let pml4_phys = (cr3 & 0x000F_FFFF_FFFF_F000) as usize;
    let pml4_virt = phys_to_virt(pml4_phys);
    
    // PML4 entry 0 covers virtual addresses 0x0000_0000_0000_0000 - 0x0000_007F_FFFF_FFFF
    let pml4 = &mut *(pml4_virt as *mut [u64; 512]);
    
    // Check if PML4[0] is already present
    let pdpt_phys = if (pml4[0] & 0x1) != 0 {
        // Already present, use existing PDPT
        (pml4[0] & 0x000F_FFFF_FFFF_F000) as usize
    } else {
        // Allocate new PDPT
        let pdpt_virt = kmalloc(4096) as usize;
        if pdpt_virt == 0 {
            return Err("Failed to allocate PDPT");
        }
        // Zero the new table
        core::ptr::write_bytes(pdpt_virt as *mut u8, 0, 4096);
        // Convert virtual address to physical
        let pdpt_phys = virt_to_phys(pdpt_virt);
        // Set PML4[0] to point to new PDPT (present + writable)
        pml4[0] = (pdpt_phys as u64) | 0x3;
        pdpt_phys
    };
    
    let pdpt_virt = phys_to_virt(pdpt_phys);
    let pdpt = &mut *(pdpt_virt as *mut [u64; 512]);
    
    // Check if PDPT[0] is already present
    let pd_phys = if (pdpt[0] & 0x1) != 0 {
        // Already present, use existing PD
        (pdpt[0] & 0x000F_FFFF_FFFF_F000) as usize
    } else {
        // Allocate new PD
        let pd_virt = kmalloc(4096) as usize;
        if pd_virt == 0 {
            return Err("Failed to allocate PD");
        }
        // Zero the new table
        core::ptr::write_bytes(pd_virt as *mut u8, 0, 4096);
        // Convert virtual address to physical
        let pd_phys = virt_to_phys(pd_virt);
        // Set PDPT[0] to point to new PD (present + writable)
        pdpt[0] = (pd_phys as u64) | 0x3;
        pd_phys
    };
    
    let pd_virt = phys_to_virt(pd_phys);
    let pd = &mut *(pd_virt as *mut [u64; 512]);
    
    // Use 2MB huge page for identity mapping (0x0000-0x1FFFFF)
    // This covers the trampoline at 0x8000 and more
    // PD[0] entry: physical address 0x0 | huge page (bit 7) | present | writable
    pd[0] = 0x0 | 0x83; // 0x83 = present (bit 0) + writable (bit 1) + huge page (bit 7)
    
    // Flush TLB for the mapped region
    core::arch::asm!(
        "invlpg [{}]",
        in(reg) 0,
        options(nostack, preserves_flags)
    );
    
    serial_println!("[SMP] Identity mapped 0x0-0x1FFFFF using 2MB huge page");
    
    Ok(())
}
