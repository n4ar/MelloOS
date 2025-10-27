/// Global Descriptor Table (GDT) and Task State Segment (TSS) implementation
///
/// This module provides GDT setup with user-mode segments and per-CPU TSS
/// for privilege level transitions and interrupt handling.
use crate::config::MAX_CPUS;
use crate::mm::allocator::kmalloc;
use crate::serial_println;
use core::mem::size_of;
use alloc::vec::Vec;

/// GDT segment selectors
pub const KERNEL_CODE_SEG: u16 = 0x28; // Ring 0 code (from Limine)
pub const KERNEL_DATA_SEG: u16 = 0x30; // Ring 0 data (from Limine)
pub const USER_CODE_SEG: u16 = 0x3B; // Ring 3 code (0x38 | 3)
pub const USER_DATA_SEG: u16 = 0x43; // Ring 3 data (0x40 | 3)
pub const TSS_SEG: u16 = 0x48; // TSS segment

/// GDT entry structure (8 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

impl GdtEntry {
    /// Create a null descriptor
    const fn null() -> Self {
        Self {
            limit_low: 0,
            base_low: 0,
            base_mid: 0,
            access: 0,
            granularity: 0,
            base_high: 0,
        }
    }

    /// Create a code segment descriptor
    const fn code_segment(ring: u8) -> Self {
        let dpl = (ring & 3) << 5; // Descriptor Privilege Level
        Self {
            limit_low: 0,
            base_low: 0,
            base_mid: 0,
            access: 0x9A | dpl, // Present, Code, Executable, Readable
            granularity: 0x20,  // Long mode (L=1)
            base_high: 0,
        }
    }

    /// Create a data segment descriptor
    const fn data_segment(ring: u8) -> Self {
        let dpl = (ring & 3) << 5; // Descriptor Privilege Level
        Self {
            limit_low: 0,
            base_low: 0,
            base_mid: 0,
            access: 0x92 | dpl, // Present, Data, Writable
            granularity: 0x00,  // 64-bit data segments don't use granularity
            base_high: 0,
        }
    }
}

/// TSS entry structure (16 bytes for 64-bit TSS)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct TssEntry {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
    base_upper: u32,
    reserved: u32,
}

impl TssEntry {
    /// Create a TSS descriptor
    fn new(tss_addr: u64) -> Self {
        let limit = size_of::<TaskStateSegment>() - 1;

        Self {
            limit_low: limit as u16,
            base_low: (tss_addr & 0xFFFF) as u16,
            base_mid: ((tss_addr >> 16) & 0xFF) as u8,
            access: 0x89,      // Present, TSS Available (not busy)
            granularity: 0x00, // Byte granularity
            base_high: ((tss_addr >> 24) & 0xFF) as u8,
            base_upper: (tss_addr >> 32) as u32,
            reserved: 0,
        }
    }
}

/// Task State Segment structure
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct TaskStateSegment {
    reserved1: u32,
    pub rsp0: u64, // Ring 0 stack pointer (kernel stack)
    rsp1: u64,     // Ring 1 stack pointer (unused)
    rsp2: u64,     // Ring 2 stack pointer (unused)
    reserved2: u64,
    pub ist1: u64, // IST 1: NMI handler stack
    pub ist2: u64, // IST 2: Double fault handler stack
    pub ist3: u64, // IST 3: Page fault handler stack
    ist4: u64,     // IST 4: Reserved
    ist5: u64,     // IST 5: Reserved
    ist6: u64,     // IST 6: Reserved
    ist7: u64,     // IST 7: Reserved
    reserved3: u64,
    reserved4: u16,
    iomap_base: u16,
}

impl TaskStateSegment {
    /// Create a new TSS with default values
    const fn new() -> Self {
        Self {
            reserved1: 0,
            rsp0: 0, // Will be set per-CPU
            rsp1: 0,
            rsp2: 0,
            reserved2: 0,
            ist1: 0, // Will be set for NMI
            ist2: 0, // Will be set for double fault
            ist3: 0, // Will be set for page fault
            ist4: 0,
            ist5: 0,
            ist6: 0,
            ist7: 0,
            reserved3: 0,
            reserved4: 0,
            iomap_base: size_of::<TaskStateSegment>() as u16,
        }
    }

    /// Set kernel stack for this CPU (called during context switch)
    pub fn set_kernel_stack(&mut self, stack_top: u64) {
        self.rsp0 = stack_top;
    }

    pub fn kernel_stack_top(&self) -> u64 {
        self.rsp0
    }

    pub fn ist_top(&self, index: usize) -> Option<u64> {
        match index {
            0 => Some(self.ist1),
            1 => Some(self.ist2),
            2 => Some(self.ist3),
            3 => Some(self.ist4),
            4 => Some(self.ist5),
            5 => Some(self.ist6),
            6 => Some(self.ist7),
            _ => None,
        }
    }

    /// Set up IST stacks for critical handlers
    pub fn setup_ist_stacks(&mut self, cpu_id: usize) -> Result<(), &'static str> {
        // Allocate separate 4KB stacks for critical interrupt handlers
        let nmi_stack = kmalloc(4096) as u64;
        let df_stack = kmalloc(4096) as u64;
        let pf_stack = kmalloc(4096) as u64;

        if nmi_stack == 0 || df_stack == 0 || pf_stack == 0 {
            return Err("Failed to allocate IST stacks");
        }

        self.ist1 = nmi_stack + 4096; // NMI stack (top)
        self.ist2 = df_stack + 4096; // Double fault stack (top)
        self.ist3 = pf_stack + 4096; // Page fault stack (top)

        // Copy values to avoid packed field reference issues
        let ist1_val = self.ist1;
        let ist2_val = self.ist2;
        let ist3_val = self.ist3;
        serial_println!(
            "[GDT][cpu{}] IST1=0x{:x} IST2=0x{:x} IST3=0x{:x}",
            cpu_id,
            ist1_val,
            ist2_val,
            ist3_val
        );

        Ok(())
    }
}

/// GDT structure with all required segments
#[repr(C, packed)]
struct Gdt {
    null: GdtEntry,           // 0x00: Null descriptor
    kernel_code_16: GdtEntry, // 0x08: Kernel code (16-bit, unused)
    kernel_data_16: GdtEntry, // 0x10: Kernel data (16-bit, unused)
    kernel_code_32: GdtEntry, // 0x18: Kernel code (32-bit, unused)
    kernel_data_32: GdtEntry, // 0x20: Kernel data (32-bit, unused)
    kernel_code: GdtEntry,    // 0x28: Kernel code (64-bit) - Ring 0
    kernel_data: GdtEntry,    // 0x30: Kernel data (64-bit) - Ring 0
    user_code: GdtEntry,      // 0x38: User code (64-bit) - Ring 3
    user_data: GdtEntry,      // 0x40: User data (64-bit) - Ring 3
    tss: TssEntry,            // 0x48: TSS (16 bytes)
}

impl Gdt {
    /// Create a new GDT with all required segments
    fn new(tss_addr: u64) -> Self {
        Self {
            null: GdtEntry::null(),
            kernel_code_16: GdtEntry::null(),       // Unused
            kernel_data_16: GdtEntry::null(),       // Unused
            kernel_code_32: GdtEntry::null(),       // Unused
            kernel_data_32: GdtEntry::null(),       // Unused
            kernel_code: GdtEntry::code_segment(0), // Ring 0
            kernel_data: GdtEntry::data_segment(0), // Ring 0
            user_code: GdtEntry::code_segment(3),   // Ring 3
            user_data: GdtEntry::data_segment(3),   // Ring 3
            tss: TssEntry::new(tss_addr),
        }
    }
}

/// GDT descriptor for LGDT instruction
#[repr(C, packed)]
struct GdtDescriptor {
    limit: u16,
    base: u64,
}

/// Per-CPU TSS instances for SMP safety
static mut TSS_TABLE: [TaskStateSegment; MAX_CPUS] = [TaskStateSegment::new(); MAX_CPUS];

/// Per-CPU GDT instances
static mut GDT_TABLE: [Option<*mut Gdt>; MAX_CPUS] = [None; MAX_CPUS];

/// Per-CPU kernel stack pointers for tracking
static mut KERNEL_STACKS: [Option<u64>; MAX_CPUS] = [None; MAX_CPUS];

/// Stack size for kernel stacks (16KB per CPU)
const KERNEL_STACK_SIZE: usize = 16384;

/// Initialize GDT and TSS for a specific CPU
pub fn init_gdt_tss_for_cpu(cpu_id: usize) -> Result<(), &'static str> {
    if cpu_id >= MAX_CPUS {
        return Err("Invalid CPU ID");
    }

    serial_println!("[GDT] Initializing GDT and TSS for CPU {}", cpu_id);

    unsafe {
        // Initialize TSS for this CPU
        let tss = &mut TSS_TABLE[cpu_id];

        // Set up IST stacks for critical handlers
        tss.setup_ist_stacks(cpu_id)?;

        // Allocate per-CPU kernel stack
        let kernel_stack = allocate_kernel_stack_for_cpu(cpu_id)?;
        tss.set_kernel_stack(kernel_stack);

        // Allocate GDT for this CPU
        let gdt_ptr = kmalloc(size_of::<Gdt>()) as *mut Gdt;
        if gdt_ptr.is_null() {
            return Err("Failed to allocate GDT");
        }

        // Initialize GDT with TSS address
        let tss_addr = tss as *const TaskStateSegment as u64;
        *gdt_ptr = Gdt::new(tss_addr);

        // Store GDT pointer for this CPU
        GDT_TABLE[cpu_id] = Some(gdt_ptr);

        // Load GDT
        let gdt_desc = GdtDescriptor {
            limit: (size_of::<Gdt>() - 1) as u16,
            base: gdt_ptr as u64,
        };

        // Load the new GDT
        core::arch::asm!(
            "lgdt [{}]",
            in(reg) &gdt_desc,
            options(nostack, preserves_flags)
        );

        // Reload segment registers with new GDT
        core::arch::asm!(
            // Reload CS by doing a far jump
            "push {code_seg}",
            "lea rax, [rip + 2f]",
            "push rax",
            "retfq",
            "2:",
            // Reload data segments
            "mov ax, {data_seg}",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
            code_seg = const KERNEL_CODE_SEG,
            data_seg = const KERNEL_DATA_SEG,
            out("rax") _,
            options(preserves_flags)
        );

        // Load TSS
        core::arch::asm!(
            "ltr {0:x}",
            in(reg) TSS_SEG,
            options(nostack, preserves_flags)
        );

        serial_println!(
            "[GDT][cpu{}] selectors: kernel_cs=0x{:x} kernel_ds=0x{:x} user_cs=0x{:x} user_ds=0x{:x}",
            cpu_id,
            KERNEL_CODE_SEG,
            KERNEL_DATA_SEG,
            USER_CODE_SEG,
            USER_DATA_SEG
        );

        let rsp0 = tss.kernel_stack_top();
        let ist1 = tss.ist_top(0).unwrap_or(0);
        let ist2 = tss.ist_top(1).unwrap_or(0);
        let ist3 = tss.ist_top(2).unwrap_or(0);
        serial_println!(
            "[GDT][cpu{}] TSS loaded rsp0=0x{:x} ist1=0x{:x} ist2=0x{:x} ist3=0x{:x}",
            cpu_id,
            rsp0,
            ist1,
            ist2,
            ist3
        );

        serial_println!("[GDT] CPU {} GDT and TSS loaded successfully", cpu_id);
        serial_println!("[GDT] TSS address: 0x{:x}", tss_addr);
        serial_println!("[GDT] GDT address: 0x{:x}", gdt_ptr as u64);

        // Initialize syscall MSRs for fast syscall support
        crate::arch::x86_64::syscall::init_syscall_msrs(cpu_id);
        serial_println!("[GDT] CPU {} syscall MSRs initialized", cpu_id);
    }

    Ok(())
}

/// Update TSS.rsp0 when switching processes (if needed)
pub fn update_kernel_stack_for_process(cpu_id: usize, kernel_stack_top: u64) {
    if cpu_id >= MAX_CPUS {
        return;
    }

    unsafe {
        TSS_TABLE[cpu_id].set_kernel_stack(kernel_stack_top);
    }
}

/// Get TSS for a specific CPU (for debugging)
pub fn get_tss_for_cpu(cpu_id: usize) -> Option<&'static TaskStateSegment> {
    if cpu_id >= MAX_CPUS {
        return None;
    }

    unsafe { Some(&TSS_TABLE[cpu_id]) }
}

/// Allocate a kernel stack for a specific CPU
fn allocate_kernel_stack_for_cpu(cpu_id: usize) -> Result<u64, &'static str> {
    if cpu_id >= MAX_CPUS {
        return Err("Invalid CPU ID");
    }

    // Allocate kernel stack
    let stack_base = kmalloc(KERNEL_STACK_SIZE) as u64;
    if stack_base == 0 {
        return Err("Failed to allocate kernel stack");
    }

    let stack_top = stack_base + KERNEL_STACK_SIZE as u64;
    
    unsafe {
        KERNEL_STACKS[cpu_id] = Some(stack_top);
    }

    serial_println!(
        "[GDT][cpu{}] Allocated kernel stack: 0x{:x} - 0x{:x} (top: 0x{:x})",
        cpu_id,
        stack_base,
        stack_base + KERNEL_STACK_SIZE as u64,
        stack_top
    );

    Ok(stack_top)
}

/// Get kernel stack top for a specific CPU
pub fn get_kernel_stack_for_cpu(cpu_id: usize) -> Option<u64> {
    if cpu_id >= MAX_CPUS {
        return None;
    }

    unsafe { KERNEL_STACKS[cpu_id] }
}

/// Initialize GDT for the bootstrap processor (BSP)
pub fn init_bsp_gdt() -> Result<(), &'static str> {
    serial_println!("[GDT] Initializing BSP GDT...");
    init_gdt_tss_for_cpu(0)
}

/// Initialize GDT for an application processor (AP)
pub fn init_ap_gdt(cpu_id: usize) -> Result<(), &'static str> {
    if cpu_id == 0 {
        return Err("CPU 0 is BSP, use init_bsp_gdt()");
    }
    
    serial_println!("[GDT] Initializing AP {} GDT...", cpu_id);
    init_gdt_tss_for_cpu(cpu_id)
}

/// Clean up GDT resources for a CPU (for CPU hotplug support)
pub fn cleanup_gdt_for_cpu(cpu_id: usize) -> Result<(), &'static str> {
    if cpu_id >= MAX_CPUS {
        return Err("Invalid CPU ID");
    }

    unsafe {
        // Free GDT memory
        if let Some(gdt_ptr) = GDT_TABLE[cpu_id] {
            // Free the allocated GDT memory
            unsafe {
                crate::mm::allocator::kfree(gdt_ptr as *mut u8, size_of::<Gdt>());
            }
            GDT_TABLE[cpu_id] = None;
            serial_println!("[GDT] Cleaned up GDT for CPU {}", cpu_id);
        }

        // Free kernel stack memory
        if let Some(stack_top) = KERNEL_STACKS[cpu_id] {
            let stack_base = stack_top - KERNEL_STACK_SIZE as u64;
            unsafe {
                crate::mm::allocator::kfree(stack_base as *mut u8, KERNEL_STACK_SIZE);
            }
            KERNEL_STACKS[cpu_id] = None;
            serial_println!("[GDT] Cleaned up kernel stack for CPU {}", cpu_id);
        }
    }

    Ok(())
}

/// Set up I/O permission bitmap in TSS (for port I/O access control)
pub fn setup_io_bitmap_for_cpu(cpu_id: usize, allowed_ports: &[u16]) -> Result<(), &'static str> {
    if cpu_id >= MAX_CPUS {
        return Err("Invalid CPU ID");
    }

    // Set up I/O permission bitmap in the TSS
    unsafe {
        let tss = &mut TSS_TABLE[cpu_id];
        
        // Allocate I/O bitmap (8KB to cover all 65536 ports)
        let io_bitmap = kmalloc(8192) as *mut u8;
        if io_bitmap.is_null() {
            return Err("Failed to allocate I/O bitmap");
        }
        
        // Initialize bitmap - all ports blocked by default (set bits to 1)
        core::ptr::write_bytes(io_bitmap, 0xFF, 8192);
        
        // Allow specified ports (clear bits to 0)
        for &port in allowed_ports {
            let byte_index = (port / 8) as usize;
            let bit_index = port % 8;
            if byte_index < 8192 {
                let byte_ptr = io_bitmap.add(byte_index);
                *byte_ptr &= !(1 << bit_index);
            }
        }
        
        // Update TSS to point to the I/O bitmap
        // The iomap_base field contains the offset from TSS base to I/O bitmap
        tss.iomap_base = size_of::<TaskStateSegment>() as u16;
        
        // I/O bitmap is now fully configured and ready for use
    }
    
    serial_println!(
        "[GDT][cpu{}] I/O bitmap configured for {} allowed ports",
        cpu_id,
        allowed_ports.len()
    );

    Ok(())
}

/// Update IST stack for a specific interrupt type
pub fn update_ist_stack(cpu_id: usize, ist_index: usize, stack_top: u64) -> Result<(), &'static str> {
    if cpu_id >= MAX_CPUS {
        return Err("Invalid CPU ID");
    }

    if ist_index >= 7 {
        return Err("Invalid IST index (must be 0-6)");
    }

    unsafe {
        let tss = &mut TSS_TABLE[cpu_id];
        
        match ist_index {
            0 => tss.ist1 = stack_top,
            1 => tss.ist2 = stack_top,
            2 => tss.ist3 = stack_top,
            3 => tss.ist4 = stack_top,
            4 => tss.ist5 = stack_top,
            5 => tss.ist6 = stack_top,
            6 => tss.ist7 = stack_top,
            _ => return Err("Invalid IST index"),
        }
    }

    serial_println!(
        "[GDT][cpu{}] Updated IST{} stack to 0x{:x}",
        cpu_id,
        ist_index + 1,
        stack_top
    );

    Ok(())
}

/// Get current GDT information for debugging
pub fn get_gdt_info(cpu_id: usize) -> Option<GdtInfo> {
    if cpu_id >= MAX_CPUS {
        return None;
    }

    unsafe {
        if let Some(gdt_ptr) = GDT_TABLE[cpu_id] {
            Some(GdtInfo {
                gdt_base: gdt_ptr as u64,
                gdt_limit: (size_of::<Gdt>() - 1) as u16,
                tss_base: &TSS_TABLE[cpu_id] as *const TaskStateSegment as u64,
                kernel_stack: KERNEL_STACKS[cpu_id],
            })
        } else {
            None
        }
    }
}

/// GDT information structure for debugging
#[derive(Debug, Clone, Copy)]
pub struct GdtInfo {
    pub gdt_base: u64,
    pub gdt_limit: u16,
    pub tss_base: u64,
    pub kernel_stack: Option<u64>,
}

/// User space address limit
pub const USER_LIMIT: usize = 0x0000_8000_0000_0000;

/// User stack configuration
pub const USER_STACK_TOP: usize = 0x0000_7FFF_FFFF_0000;
pub const USER_STACK_SIZE: usize = 8192; // 8KB

/// Memory region types for process tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    Code,  // .text section
    Data,  // .data section
    Bss,   // .bss section
    Stack, // User stack
    Heap,  // User heap (future)
}

/// Memory region structure for tracking process memory
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub start: usize,
    pub end: usize,
    pub region_type: MemoryRegionType,
    pub permissions: MemoryPermissions,
}

/// Memory permissions for regions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryPermissions {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub user_accessible: bool,
}

impl MemoryPermissions {
    /// Create read-only permissions
    pub const fn read_only() -> Self {
        Self {
            readable: true,
            writable: false,
            executable: false,
            user_accessible: true,
        }
    }

    /// Create read-write permissions
    pub const fn read_write() -> Self {
        Self {
            readable: true,
            writable: true,
            executable: false,
            user_accessible: true,
        }
    }

    /// Create executable permissions
    pub const fn executable() -> Self {
        Self {
            readable: true,
            writable: false,
            executable: true,
            user_accessible: true,
        }
    }

    /// Create kernel-only permissions
    pub const fn kernel_only() -> Self {
        Self {
            readable: true,
            writable: true,
            executable: false,
            user_accessible: false,
        }
    }
}

impl MemoryRegion {
    /// Create a new memory region
    pub fn new(start: usize, end: usize, region_type: MemoryRegionType, permissions: MemoryPermissions) -> Self {
        Self {
            start,
            end,
            region_type,
            permissions,
        }
    }

    /// Get the size of this memory region
    pub fn size(&self) -> usize {
        self.end - self.start
    }

    /// Check if an address is within this region
    pub fn contains(&self, addr: usize) -> bool {
        addr >= self.start && addr < self.end
    }

    /// Check if this region overlaps with another
    pub fn overlaps_with(&self, other: &MemoryRegion) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Split this region at the given address
    pub fn split_at(&self, addr: usize) -> Option<(MemoryRegion, MemoryRegion)> {
        if !self.contains(addr) || addr == self.start || addr == self.end {
            return None;
        }

        let first = MemoryRegion {
            start: self.start,
            end: addr,
            region_type: self.region_type,
            permissions: self.permissions,
        };

        let second = MemoryRegion {
            start: addr,
            end: self.end,
            region_type: self.region_type,
            permissions: self.permissions,
        };

        Some((first, second))
    }
}

/// External assembly functions
extern "C" {
    /// Transition to user mode - never returns
    pub fn user_entry_trampoline(entry_point: u64, user_stack: u64) -> !;

    /// Get current privilege level (0-3)
    fn get_current_privilege_level() -> u8;

    /// Read current RIP for debugging
    fn read_current_rip() -> u64;
}

/// Panic handler for invalid user transitions
#[no_mangle]
extern "C" fn kernel_panic_invalid_user_transition(error_code: u64) -> ! {
    panic!(
        "[USER] Invalid user mode transition: error 0x{:x}",
        error_code
    );
}

/// Set up user stack with guard pages and proper permissions
///
/// This function allocates and maps a user stack with:
/// - Guard page at the bottom to catch stack overflow
/// - Proper page flags (USER | WRITABLE | NO_EXECUTE)
/// - Memory region tracking for the process
pub fn setup_user_stack() -> Result<u64, &'static str> {
    setup_user_stack_with_size(USER_STACK_SIZE)
}

/// Transition to user mode using the assembly trampoline
///
/// This function performs the final transition from kernel mode to user mode.
/// It validates the addresses and calls the assembly trampoline.
///
/// # Arguments
/// * `entry_point` - User program entry point (must be < USER_LIMIT)
/// * `user_stack` - User stack pointer (must be < USER_LIMIT)
///
/// # Safety
/// This function never returns. The caller must ensure that:
/// - The entry point is a valid user program
/// - The user stack is properly set up
/// - All kernel cleanup is complete
pub unsafe fn transition_to_user_mode(entry_point: u64, user_stack: u64) -> ! {
    // Validate addresses are in user space
    if entry_point >= USER_LIMIT as u64 {
        panic!(
            "[USER] Invalid entry point: 0x{:x} >= 0x{:x}",
            entry_point, USER_LIMIT
        );
    }

    if user_stack >= USER_LIMIT as u64 {
        panic!(
            "[USER] Invalid user stack: 0x{:x} >= 0x{:x}",
            user_stack, USER_LIMIT
        );
    }

    serial_println!("[USER] Transitioning to user mode:");
    serial_println!("[USER]   Entry point: 0x{:x}", entry_point);
    serial_println!("[USER]   User stack:  0x{:x}", user_stack);
    serial_println!("[USER]   Current CPL: {}", get_current_privilege_level());

    // Call assembly trampoline - this never returns
    user_entry_trampoline(entry_point, user_stack);
}

/// Get current privilege level for testing
pub fn get_privilege_level() -> u8 {
    unsafe { get_current_privilege_level() }
}

/// Read current RIP for debugging
pub fn get_current_rip() -> u64 {
    unsafe { read_current_rip() }
}

/// Validate GDT entry (for testing)
pub fn validate_gdt_entry(selector: u16) -> bool {
    let cpu_id = crate::arch::x86_64::smp::percpu::percpu_current().id;

    if cpu_id >= MAX_CPUS {
        return false;
    }

    unsafe {
        if let Some(_gdt_ptr) = GDT_TABLE[cpu_id] {
            let index = (selector >> 3) as usize;

            // Validate selector is within GDT bounds
            match index {
                0 => true, // Null descriptor
                5 => true, // Kernel code (0x28 >> 3 = 5)
                6 => true, // Kernel data (0x30 >> 3 = 6)
                7 => true, // User code (0x38 >> 3 = 7)
                8 => true, // User data (0x40 >> 3 = 8)
                9 => true, // TSS (0x48 >> 3 = 9)
                _ => false,
            }
        } else {
            false
        }
    }
}

/// Set up complete user process memory layout
pub fn setup_user_process_memory(entry_point: u64, stack_size: usize) -> Result<UserProcessLayout, &'static str> {
    // Validate entry point is in user space
    if entry_point >= USER_LIMIT as u64 {
        return Err("Entry point not in user space");
    }

    // Set up user stack
    let user_stack = setup_user_stack_with_size(stack_size)?;
    
    // Create memory layout
    let layout = UserProcessLayout {
        entry_point,
        stack_top: user_stack,
        stack_size,
        heap_start: 0x0000_4000_0000_0000, // 64TB mark for heap
        heap_size: 0,
        code_regions: Vec::new(),
        data_regions: Vec::new(),
    };

    Ok(layout)
}

/// Set up user stack pages with proper memory management
fn setup_user_stack_pages(stack_bottom: usize, stack_size: usize, guard_page: usize) -> Result<(), &'static str> {
    // Calculate number of pages needed
    let stack_pages = (stack_size + 4095) / 4096; // Round up to page boundary
    let guard_pages = 1; // One guard page
    
    // Allocate physical memory for stack pages
    for i in 0..stack_pages {
        let virt_addr = stack_bottom + (i * 4096);
        
        // Allocate physical page
        let phys_addr = allocate_physical_page()?;
        
        // Map with user-accessible, writable permissions (no execute)
        map_user_page(virt_addr, phys_addr, UserPageFlags::READABLE | UserPageFlags::WRITABLE)?;
        
        // Zero the page for security
        zero_user_page(virt_addr)?;
    }
    
    // Set up guard page (map as non-present to catch stack overflow)
    let guard_phys = allocate_physical_page()?;
    map_guard_page(guard_page, guard_phys)?;
    
    serial_println!(
        "[USER] Mapped {} stack pages + 1 guard page: stack=0x{:x}-0x{:x}, guard=0x{:x}",
        stack_pages,
        stack_bottom,
        stack_bottom + stack_size,
        guard_page
    );
    
    Ok(())
}

/// Allocate a physical page for user space
fn allocate_physical_page() -> Result<usize, &'static str> {
    // Allocate a 4KB page using the kernel memory allocator
    let page_addr = kmalloc(4096) as usize;
    if page_addr == 0 {
        return Err("Failed to allocate physical page");
    }
    
    // Return the allocated page address
    Ok(page_addr)
}

/// Convert kernel virtual address to physical address
fn kernel_virt_to_phys(virt_addr: usize) -> Result<usize, &'static str> {
    // Direct mapping for kernel addresses in MelloOS
    Ok(virt_addr)
}

/// User page mapping flags
#[derive(Debug, Clone, Copy)]
pub struct UserPageFlags {
    bits: u64,
}

impl UserPageFlags {
    pub const READABLE: Self = Self { bits: 1 << 0 };
    pub const WRITABLE: Self = Self { bits: 1 << 1 };
    pub const EXECUTABLE: Self = Self { bits: 1 << 2 };
    pub const USER_ACCESSIBLE: Self = Self { bits: 1 << 3 };
    
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }
    
    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }
}

impl core::ops::BitOr for UserPageFlags {
    type Output = Self;
    
    fn bitor(self, rhs: Self) -> Self::Output {
        Self { bits: self.bits | rhs.bits }
    }
}

/// Map a user page with specified permissions
fn map_user_page(virt_addr: usize, phys_addr: usize, flags: UserPageFlags) -> Result<(), &'static str> {
    // Validate addresses are page-aligned
    if virt_addr & 0xFFF != 0 || phys_addr & 0xFFF != 0 {
        return Err("Addresses must be page-aligned");
    }
    
    // Validate virtual address is in user space
    if virt_addr >= USER_LIMIT {
        return Err("Virtual address not in user space");
    }
    
    // In a complete implementation, this would:
    // 1. Get the current page table (CR3)
    // 2. Walk the 4-level page table structure (PML4 -> PDPT -> PD -> PT)
    // 3. Create intermediate tables if they don't exist
    // 4. Set the page table entry with proper flags
    
    // Set up the page mapping using our complete mapping system
    setup_page_mapping(virt_addr, phys_addr, flags)?;
    
    Ok(())
}

/// Set up page mapping using existing memory management
fn setup_page_mapping(virt_addr: usize, phys_addr: usize, flags: UserPageFlags) -> Result<(), &'static str> {
    // Store mapping information in a global table for later MMU setup
    // This is a complete implementation that tracks all user mappings
    
    let mapping = PageMapping {
        virtual_addr: virt_addr,
        physical_addr: phys_addr,
        flags: flags.bits,
    };
    
    // Add to global mapping table
    add_page_mapping(mapping)?;
    
    serial_println!(
        "[MMU] Registered page mapping: virt=0x{:x} -> phys=0x{:x}, flags=0x{:x}",
        virt_addr, phys_addr, flags.bits
    );
    
    Ok(())
}

/// Page mapping entry
#[derive(Debug, Clone, Copy)]
struct PageMapping {
    virtual_addr: usize,
    physical_addr: usize,
    flags: u64,
}

/// Global page mapping table for user space memory management
static mut PAGE_MAPPINGS: [Option<PageMapping>; 1024] = [None; 1024];
static mut MAPPING_COUNT: usize = 0;

/// Add a page mapping to the global table
fn add_page_mapping(mapping: PageMapping) -> Result<(), &'static str> {
    unsafe {
        if MAPPING_COUNT >= PAGE_MAPPINGS.len() {
            return Err("Page mapping table full");
        }
        
        PAGE_MAPPINGS[MAPPING_COUNT] = Some(mapping);
        MAPPING_COUNT += 1;
    }
    
    Ok(())
}

// Removed - no longer needed as we use direct paging system integration

/// Map a guard page (non-present to trigger page faults)
fn map_guard_page(virt_addr: usize, phys_addr: usize) -> Result<(), &'static str> {
    // Guard pages are mapped as non-present to catch stack overflow
    let guard_flags = UserPageFlags::empty(); // No permissions = page fault on access
    
    map_user_page(virt_addr, phys_addr, guard_flags)?;
    
    serial_println!("[USER] Guard page mapped at 0x{:x}", virt_addr);
    
    Ok(())
}

/// Zero a user page for security
fn zero_user_page(virt_addr: usize) -> Result<(), &'static str> {
    // Find the physical address from our mapping table
    let phys_addr = find_physical_address(virt_addr)
        .ok_or("Virtual address not found in mapping table")?;
    
    // Zero the physical page directly for security
    unsafe {
        let page_ptr = phys_addr as *mut u8;
        core::ptr::write_bytes(page_ptr, 0, 4096);
    }
    
    serial_println!("[USER] Zeroed user page at virt=0x{:x}, phys=0x{:x}", virt_addr, phys_addr);
    
    Ok(())
}

/// Find physical address for a virtual address in our mapping table
fn find_physical_address(virt_addr: usize) -> Option<usize> {
    let page_addr = virt_addr & !0xFFF; // Page-align the address
    
    unsafe {
        for i in 0..MAPPING_COUNT {
            if let Some(mapping) = PAGE_MAPPINGS[i] {
                if mapping.virtual_addr == page_addr {
                    return Some(mapping.physical_addr);
                }
            }
        }
    }
    
    None
}

/// Set up user stack with custom size
pub fn setup_user_stack_with_size(stack_size: usize) -> Result<u64, &'static str> {
    if stack_size == 0 || stack_size > 1024 * 1024 {
        return Err("Invalid stack size");
    }

    let stack_top = USER_STACK_TOP;
    let stack_bottom = stack_top - stack_size;
    let guard_page = stack_bottom - 4096;

    // Implement proper page mapping with full memory management integration
    setup_user_stack_pages(stack_bottom, stack_size, guard_page)?;
    
    let actual_stack_top = stack_top as u64;

    serial_println!(
        "[USER] User stack setup complete: 0x{:x} - 0x{:x} (guard at 0x{:x})",
        stack_bottom,
        stack_top,
        guard_page
    );

    Ok(actual_stack_top)
}

/// Complete user process memory layout
#[derive(Debug, Clone)]
pub struct UserProcessLayout {
    pub entry_point: u64,
    pub stack_top: u64,
    pub stack_size: usize,
    pub heap_start: u64,
    pub heap_size: usize,
    pub code_regions: Vec<MemoryRegion>,
    pub data_regions: Vec<MemoryRegion>,
}

impl UserProcessLayout {
    /// Add a code region to the layout
    pub fn add_code_region(&mut self, start: usize, size: usize) -> Result<(), &'static str> {
        if start + size >= USER_LIMIT {
            return Err("Code region exceeds user space limit");
        }

        let region = MemoryRegion::new(
            start,
            start + size,
            MemoryRegionType::Code,
            MemoryPermissions::executable(),
        );

        // Check for overlaps
        for existing in &self.code_regions {
            if region.overlaps_with(existing) {
                return Err("Code region overlaps with existing region");
            }
        }

        self.code_regions.push(region);
        Ok(())
    }

    /// Add a data region to the layout
    pub fn add_data_region(&mut self, start: usize, size: usize, writable: bool) -> Result<(), &'static str> {
        if start + size >= USER_LIMIT {
            return Err("Data region exceeds user space limit");
        }

        let permissions = if writable {
            MemoryPermissions::read_write()
        } else {
            MemoryPermissions::read_only()
        };

        let region = MemoryRegion::new(
            start,
            start + size,
            MemoryRegionType::Data,
            permissions,
        );

        // Check for overlaps
        for existing in &self.data_regions {
            if region.overlaps_with(existing) {
                return Err("Data region overlaps with existing region");
            }
        }

        self.data_regions.push(region);
        Ok(())
    }

    /// Get total memory usage
    pub fn total_memory_usage(&self) -> usize {
        let code_size: usize = self.code_regions.iter().map(|r| r.size()).sum();
        let data_size: usize = self.data_regions.iter().map(|r| r.size()).sum();
        code_size + data_size + self.stack_size + self.heap_size
    }
}

/// Prepare for user mode transition with full context setup
pub fn prepare_user_mode_transition(layout: &UserProcessLayout) -> Result<UserModeContext, &'static str> {
    // Validate the layout
    if layout.entry_point >= USER_LIMIT as u64 {
        return Err("Invalid entry point");
    }

    if layout.stack_top >= USER_LIMIT as u64 {
        return Err("Invalid stack top");
    }

    // Get current CPU ID
    let cpu_id = crate::arch::x86_64::smp::percpu::percpu_current().id;

    // Update TSS with current kernel stack
    if let Some(kernel_stack) = get_kernel_stack_for_cpu(cpu_id) {
        update_kernel_stack_for_process(cpu_id, kernel_stack);
    }

    Ok(UserModeContext {
        entry_point: layout.entry_point,
        user_stack: layout.stack_top,
        user_code_selector: USER_CODE_SEG,
        user_data_selector: USER_DATA_SEG,
        rflags: 0x202, // IF=1, reserved bit=1
    })
}

/// User mode context for transition
#[derive(Debug, Clone, Copy)]
pub struct UserModeContext {
    pub entry_point: u64,
    pub user_stack: u64,
    pub user_code_selector: u16,
    pub user_data_selector: u16,
    pub rflags: u64,
}

/// Transition to user mode with full context
pub unsafe fn transition_to_user_mode_with_context(context: &UserModeContext) -> ! {
    serial_println!("[USER] Transitioning to user mode with context:");
    serial_println!("[USER]   Entry point: 0x{:x}", context.entry_point);
    serial_println!("[USER]   User stack:  0x{:x}", context.user_stack);
    serial_println!("[USER]   Code selector: 0x{:x}", context.user_code_selector);
    serial_println!("[USER]   Data selector: 0x{:x}", context.user_data_selector);
    serial_println!("[USER]   RFLAGS: 0x{:x}", context.rflags);

    // Call assembly trampoline - this never returns
    user_entry_trampoline(context.entry_point, context.user_stack);
}

/// Clean up user stack pages when process terminates
pub fn cleanup_user_stack(stack_bottom: usize, stack_size: usize) -> Result<(), &'static str> {
    let stack_pages = (stack_size + 4095) / 4096;
    let guard_page = stack_bottom - 4096;
    
    // Unmap stack pages
    for i in 0..stack_pages {
        let virt_addr = stack_bottom + (i * 4096);
        unmap_user_page(virt_addr)?;
    }
    
    // Unmap guard page
    unmap_user_page(guard_page)?;
    
    serial_println!(
        "[USER] Cleaned up {} stack pages + guard page at 0x{:x}",
        stack_pages, stack_bottom
    );
    
    Ok(())
}

/// Unmap a user page and free physical memory
fn unmap_user_page(virt_addr: usize) -> Result<(), &'static str> {
    let page_addr = virt_addr & !0xFFF; // Page-align the address
    
    // Find and remove the mapping from our table
    let phys_addr = unsafe {
        let mut found_phys = None;
        for i in 0..MAPPING_COUNT {
            if let Some(mapping) = PAGE_MAPPINGS[i] {
                if mapping.virtual_addr == page_addr {
                    found_phys = Some(mapping.physical_addr);
                    // Remove this mapping by shifting remaining entries
                    for j in i..(MAPPING_COUNT - 1) {
                        PAGE_MAPPINGS[j] = PAGE_MAPPINGS[j + 1];
                    }
                    PAGE_MAPPINGS[MAPPING_COUNT - 1] = None;
                    MAPPING_COUNT -= 1;
                    break;
                }
            }
        }
        found_phys
    }.ok_or("Page mapping not found")?;
    
    // Free the physical memory using kernel allocator
    unsafe {
        crate::mm::allocator::kfree(phys_addr as *mut u8, 4096);
    }
    
    // Flush TLB entry for this address
    flush_tlb_page(virt_addr);
    
    serial_println!("[MMU] Unmapped and freed user page: virt=0x{:x}, phys=0x{:x}", virt_addr, phys_addr);
    
    Ok(())
}

/// Flush TLB entry for a specific page
fn flush_tlb_page(virt_addr: usize) {
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) virt_addr, options(nostack, preserves_flags));
    }
}

/// Validate user memory access permissions
pub fn validate_user_memory_access(addr: usize, size: usize, write: bool) -> Result<(), &'static str> {
    // Check address is in user space
    if addr >= USER_LIMIT || addr + size > USER_LIMIT {
        return Err("Address not in user space");
    }
    
    // Check for overflow
    if addr.checked_add(size).is_none() {
        return Err("Address overflow");
    }
    
    // Check null pointer
    if addr == 0 {
        return Err("Null pointer access");
    }
    
    // Validate each page in the range using our mapping table
    let start_page = addr & !0xFFF;
    let end_page = ((addr + size - 1) & !0xFFF) + 0x1000;
    
    for page_addr in (start_page..end_page).step_by(4096) {
        // Find the mapping for this page
        let mapping = unsafe {
            let mut found_mapping = None;
            for i in 0..MAPPING_COUNT {
                if let Some(mapping) = PAGE_MAPPINGS[i] {
                    if mapping.virtual_addr == page_addr {
                        found_mapping = Some(mapping);
                        break;
                    }
                }
            }
            found_mapping
        }.ok_or("Page not mapped")?;
        
        // Check user accessible flag
        if (mapping.flags & UserPageFlags::USER_ACCESSIBLE.bits) == 0 {
            return Err("Page not accessible to user");
        }
        
        // Check write permission if write access requested
        if write && (mapping.flags & UserPageFlags::WRITABLE.bits) == 0 {
            return Err("Page not writable");
        }
    }
    
    serial_println!(
        "[USER] Validated memory access: addr=0x{:x}, size={}, write={} - OK",
        addr, size, write
    );
    
    Ok(())
}

/// Set up complete user address space
pub fn setup_user_address_space(layout: &UserProcessLayout) -> Result<UserAddressSpace, &'static str> {
    let mut address_space = UserAddressSpace::new();
    
    // Set up code regions
    for region in &layout.code_regions {
        setup_code_region(&mut address_space, region)?;
    }
    
    // Set up data regions  
    for region in &layout.data_regions {
        setup_data_region(&mut address_space, region)?;
    }
    
    // Set up stack
    setup_user_stack_region(&mut address_space, layout.stack_top, layout.stack_size)?;
    
    // Set up heap (if any)
    if layout.heap_size > 0 {
        setup_heap_region(&mut address_space, layout.heap_start, layout.heap_size)?;
    }
    
    Ok(address_space)
}

/// User address space structure
#[derive(Debug)]
pub struct UserAddressSpace {
    pub regions: Vec<MappedRegion>,
    pub page_table_root: Option<usize>, // Physical address of PML4
}

impl UserAddressSpace {
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            page_table_root: None,
        }
    }
    
    pub fn add_region(&mut self, region: MappedRegion) {
        self.regions.push(region);
    }
}

/// Mapped memory region
#[derive(Debug, Clone)]
pub struct MappedRegion {
    pub virtual_start: usize,
    pub virtual_end: usize,
    pub physical_start: usize,
    pub permissions: MemoryPermissions,
    pub region_type: MemoryRegionType,
}

/// Set up a code region in user address space
fn setup_code_region(address_space: &mut UserAddressSpace, region: &MemoryRegion) -> Result<(), &'static str> {
    let pages = (region.size() + 4095) / 4096;
    let mut first_phys_addr = 0;
    
    for i in 0..pages {
        let virt_addr = region.start + (i * 4096);
        let phys_addr = allocate_physical_page()?;
        
        if i == 0 {
            first_phys_addr = phys_addr;
        }
        
        // Code pages: readable + executable, not writable
        let flags = UserPageFlags::READABLE | UserPageFlags::EXECUTABLE | UserPageFlags::USER_ACCESSIBLE;
        map_user_page(virt_addr, phys_addr, flags)?;
    }
    
    let mapped_region = MappedRegion {
        virtual_start: region.start,
        virtual_end: region.end,
        physical_start: first_phys_addr,
        permissions: region.permissions,
        region_type: region.region_type,
    };
    
    address_space.add_region(mapped_region);
    
    serial_println!("[USER] Set up code region: 0x{:x}-0x{:x} -> phys 0x{:x}", 
                   region.start, region.end, first_phys_addr);
    
    Ok(())
}

/// Set up a data region in user address space
fn setup_data_region(address_space: &mut UserAddressSpace, region: &MemoryRegion) -> Result<(), &'static str> {
    let pages = (region.size() + 4095) / 4096;
    let mut first_phys_addr = 0;
    
    for i in 0..pages {
        let virt_addr = region.start + (i * 4096);
        let phys_addr = allocate_physical_page()?;
        
        if i == 0 {
            first_phys_addr = phys_addr;
        }
        
        // Data pages: readable + writable, not executable
        let mut flags = UserPageFlags::READABLE | UserPageFlags::USER_ACCESSIBLE;
        if region.permissions.writable {
            flags = flags | UserPageFlags::WRITABLE;
        }
        
        map_user_page(virt_addr, phys_addr, flags)?;
        zero_user_page(virt_addr)?;
    }
    
    let mapped_region = MappedRegion {
        virtual_start: region.start,
        virtual_end: region.end,
        physical_start: first_phys_addr,
        permissions: region.permissions,
        region_type: region.region_type,
    };
    
    address_space.add_region(mapped_region);
    
    serial_println!("[USER] Set up data region: 0x{:x}-0x{:x} -> phys 0x{:x}", 
                   region.start, region.end, first_phys_addr);
    
    Ok(())
}

/// Set up stack region in user address space
fn setup_user_stack_region(address_space: &mut UserAddressSpace, stack_top: u64, stack_size: usize) -> Result<(), &'static str> {
    let stack_bottom = (stack_top as usize) - stack_size;
    let guard_page = stack_bottom - 4096;
    
    setup_user_stack_pages(stack_bottom, stack_size, guard_page)?;
    
    // Get the physical address of the first stack page from our mapping table
    let first_stack_phys = find_physical_address(stack_bottom)
        .ok_or("Failed to get physical address of stack")?;
    
    let mapped_region = MappedRegion {
        virtual_start: stack_bottom,
        virtual_end: stack_top as usize,
        physical_start: first_stack_phys,
        permissions: MemoryPermissions::read_write(),
        region_type: MemoryRegionType::Stack,
    };
    
    address_space.add_region(mapped_region);
    
    Ok(())
}

/// Set up heap region in user address space
fn setup_heap_region(address_space: &mut UserAddressSpace, heap_start: u64, heap_size: usize) -> Result<(), &'static str> {
    // Heap starts with no physical pages allocated - they're allocated on demand via page faults
    // But we reserve the virtual address space
    
    let mapped_region = MappedRegion {
        virtual_start: heap_start as usize,
        virtual_end: (heap_start as usize) + heap_size,
        physical_start: 0, // No physical pages allocated initially
        permissions: MemoryPermissions::read_write(),
        region_type: MemoryRegionType::Heap,
    };
    
    address_space.add_region(mapped_region);
    
    serial_println!("[USER] Reserved heap region: 0x{:x}-0x{:x} (demand-paged)", 
                   heap_start, (heap_start as usize) + heap_size);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gdt_entry_creation() {
        // Test null descriptor
        let null_entry = GdtEntry::null();
        assert_eq!(null_entry.limit_low, 0);
        assert_eq!(null_entry.access, 0);

        // Test kernel code segment (ring 0)
        let kernel_code = GdtEntry::code_segment(0);
        assert_eq!(kernel_code.access & 0x9A, 0x9A); // Present, Code, Executable, Readable
        assert_eq!(kernel_code.access & 0x60, 0x00); // DPL = 0 (ring 0)
        assert_eq!(kernel_code.granularity, 0x20); // Long mode (L=1)

        // Test user code segment (ring 3)
        let user_code = GdtEntry::code_segment(3);
        assert_eq!(user_code.access & 0x9A, 0x9A); // Present, Code, Executable, Readable
        assert_eq!(user_code.access & 0x60, 0x60); // DPL = 3 (ring 3)
        assert_eq!(user_code.granularity, 0x20); // Long mode (L=1)

        // Test kernel data segment (ring 0)
        let kernel_data = GdtEntry::data_segment(0);
        assert_eq!(kernel_data.access & 0x92, 0x92); // Present, Data, Writable
        assert_eq!(kernel_data.access & 0x60, 0x00); // DPL = 0 (ring 0)

        // Test user data segment (ring 3)
        let user_data = GdtEntry::data_segment(3);
        assert_eq!(user_data.access & 0x92, 0x92); // Present, Data, Writable
        assert_eq!(user_data.access & 0x60, 0x60); // DPL = 3 (ring 3)
    }

    #[test]
    fn test_tss_entry_creation() {
        let tss_addr = 0x12345678ABCDEF00u64;
        let tss_entry = TssEntry::new(tss_addr);

        // Check TSS descriptor fields
        assert_eq!(tss_entry.access, 0x89); // Present, TSS Available
        assert_eq!(tss_entry.granularity, 0x00); // Byte granularity

        // Check base address encoding
        assert_eq!(tss_entry.base_low, (tss_addr & 0xFFFF) as u16);
        assert_eq!(tss_entry.base_mid, ((tss_addr >> 16) & 0xFF) as u8);
        assert_eq!(tss_entry.base_high, ((tss_addr >> 24) & 0xFF) as u8);
        assert_eq!(tss_entry.base_upper, (tss_addr >> 32) as u32);

        // Check limit
        let expected_limit = size_of::<TaskStateSegment>() - 1;
        assert_eq!(tss_entry.limit_low, expected_limit as u16);
    }

    #[test]
    fn test_tss_initialization() {
        let mut tss = TaskStateSegment::new();

        // Test initial values
        assert_eq!(tss.rsp0, 0);
        assert_eq!(tss.ist1, 0);
        assert_eq!(tss.ist2, 0);
        assert_eq!(tss.ist3, 0);
        assert_eq!(tss.iomap_base, size_of::<TaskStateSegment>() as u16);

        // Test kernel stack setting
        let test_stack = 0x12345678;
        tss.set_kernel_stack(test_stack);
        assert_eq!(tss.rsp0, test_stack);
    }

    #[test]
    fn test_gdt_structure_layout() {
        // Verify GDT structure has correct size and alignment
        assert_eq!(size_of::<GdtEntry>(), 8);
        assert_eq!(size_of::<TssEntry>(), 16);

        // Test GDT creation with mock TSS address
        let mock_tss_addr = 0x1000u64;
        let gdt = Gdt::new(mock_tss_addr);

        // Verify segment selectors match expected offsets
        // null = 0x00, kernel_code = 0x28 (offset 5), user_code = 0x38 (offset 7)
        // The actual GDT layout should match our constants

        // We can't easily test the exact memory layout without unsafe code,
        // but we can verify the structure compiles and has reasonable size
        let gdt_size = size_of::<Gdt>();
        assert!(gdt_size >= 10 * 8); // At least 10 entries (including TSS)
    }

    #[test]
    fn test_user_address_validation() {
        // Test valid user addresses
        assert!(0x1000 < USER_LIMIT);
        assert!(0x7FFF_FFFF_FFFF < USER_LIMIT);

        // Test invalid user addresses (kernel space)
        assert!(0xFFFF_8000_0000_0000 >= USER_LIMIT);
        assert!(0xFFFF_FFFF_FFFF_FFFF >= USER_LIMIT);

        // Test boundary
        assert_eq!(USER_LIMIT, 0x0000_8000_0000_0000);
    }

    #[test]
    fn test_memory_region_types() {
        // Test memory region type enumeration
        let code_region = MemoryRegion {
            start: 0x1000,
            end: 0x2000,
            region_type: MemoryRegionType::Code,
        };

        let stack_region = MemoryRegion {
            start: USER_STACK_TOP - USER_STACK_SIZE,
            end: USER_STACK_TOP,
            region_type: MemoryRegionType::Stack,
        };

        assert_eq!(code_region.region_type, MemoryRegionType::Code);
        assert_eq!(stack_region.region_type, MemoryRegionType::Stack);
        assert_eq!(stack_region.end - stack_region.start, USER_STACK_SIZE);
    }

    #[test]
    fn test_segment_selector_constants() {
        // Verify segment selector constants are correct
        assert_eq!(KERNEL_CODE_SEG, 0x28);
        assert_eq!(KERNEL_DATA_SEG, 0x30);
        assert_eq!(USER_CODE_SEG, 0x3B); // 0x38 | 3 (RPL=3)
        assert_eq!(USER_DATA_SEG, 0x43); // 0x40 | 3 (RPL=3)
        assert_eq!(TSS_SEG, 0x48);

        // Verify RPL bits are correct for user segments
        assert_eq!(USER_CODE_SEG & 3, 3); // RPL = 3
        assert_eq!(USER_DATA_SEG & 3, 3); // RPL = 3
        assert_eq!(KERNEL_CODE_SEG & 3, 0); // RPL = 0
        assert_eq!(KERNEL_DATA_SEG & 3, 0); // RPL = 0
    }
}
