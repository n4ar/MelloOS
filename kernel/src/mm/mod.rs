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
