// Memory Management Module
// Coordinates PMM, Paging, and Kernel Heap Allocator

use core::sync::atomic::{AtomicUsize, Ordering};
use limine::request::{HhdmRequest, MemoryMapRequest, ExecutableAddressRequest};

pub mod pmm;
pub mod paging;
pub mod allocator;

/// Limine HHDM (Higher Half Direct Map) request
/// This provides the offset for direct physical memory mapping
#[used]
#[link_section = ".requests"]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

/// Limine Memory Map request
/// This provides information about available physical memory regions
#[used]
#[link_section = ".requests"]
static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

/// Limine Kernel Address request
/// This provides the virtual and physical addresses of the kernel
#[used]
#[link_section = ".requests"]
static KERNEL_ADDRESS_REQUEST: ExecutableAddressRequest = ExecutableAddressRequest::new();

/// HHDM offset - stores the offset for converting physical to virtual addresses
/// Initialized from Limine bootloader, NOT hardcoded
static HHDM_OFFSET: AtomicUsize = AtomicUsize::new(0);

/// Physical address type
pub type PhysAddr = usize;

/// Virtual address type
pub type VirtAddr = usize;

/// Initialize HHDM offset from Limine bootloader
/// This MUST be called before using phys_to_virt() or virt_to_phys()
pub fn init_hhdm(offset: usize) {
    HHDM_OFFSET.store(offset, Ordering::Relaxed);
}

/// Convert physical address to virtual address using HHDM
/// Uses the direct mapping provided by Limine bootloader
pub fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
    phys + HHDM_OFFSET.load(Ordering::Relaxed)
}

/// Convert virtual address to physical address using HHDM
/// Assumes the address is in the direct-mapped region
pub fn virt_to_phys(virt: VirtAddr) -> PhysAddr {
    virt - HHDM_OFFSET.load(Ordering::Relaxed)
}

/// Enable NX (No Execute) bit support in the CPU
/// This allows marking pages as non-executable for security
/// Sets the NXE bit (bit 11) in the EFER MSR (Model Specific Register)
pub fn enable_nx_bit() {
    unsafe {
        use core::arch::asm;
        
        // EFER MSR number
        const EFER_MSR: u32 = 0xC0000080;
        const NXE_BIT: u64 = 1 << 11;
        
        // Read EFER MSR
        let mut efer: u64;
        asm!(
            "rdmsr",
            in("ecx") EFER_MSR,
            out("eax") _,
            out("edx") _,
            options(nostack, preserves_flags)
        );
        
        // Combine EAX (low 32 bits) and EDX (high 32 bits)
        let eax: u32;
        let edx: u32;
        asm!(
            "rdmsr",
            in("ecx") EFER_MSR,
            out("eax") eax,
            out("edx") edx,
            options(nostack, preserves_flags)
        );
        efer = ((edx as u64) << 32) | (eax as u64);
        
        // Set NXE bit (bit 11)
        efer |= NXE_BIT;
        
        // Write back to EFER MSR
        let new_eax = efer as u32;
        let new_edx = (efer >> 32) as u32;
        asm!(
            "wrmsr",
            in("ecx") EFER_MSR,
            in("eax") new_eax,
            in("edx") new_edx,
            options(nostack, preserves_flags)
        );
    }
}

/// Enable write protection in the CPU
/// This makes the kernel respect page-level write protection
/// Sets the WP bit (bit 16) in the CR0 register
pub fn enable_write_protect() {
    unsafe {
        use core::arch::asm;
        
        const WP_BIT: u64 = 1 << 16;
        
        // Read CR0 register
        let mut cr0: u64;
        asm!(
            "mov {}, cr0",
            out(reg) cr0,
            options(nostack, preserves_flags)
        );
        
        // Set WP bit (bit 16)
        cr0 |= WP_BIT;
        
        // Write back to CR0
        asm!(
            "mov cr0, {}",
            in(reg) cr0,
            options(nostack, preserves_flags)
        );
    }
}
