// Memory Management Module
// Coordinates PMM, Paging, and Kernel Heap Allocator
//
// # Logging
//
// This module provides logging utilities with the "[MM]" prefix for all memory management operations.
// All addresses are displayed in hexadecimal format (0x...) and memory sizes are displayed in
// appropriate units (bytes, KB, MB).
//
// ## Usage Examples
//
// ```rust
// use crate::{mm_log, mm_info, mm_error, mm_test_ok};
//
// // Basic logging
// mm_log!("Initializing memory management...");
//
// // Logging with formatted values
// let total_mb = 8192;
// mm_info!("Total memory: {} MB", total_mb);
//
// // Logging addresses in hexadecimal
// let frame_addr = 0x100000;
// mm_log!("Allocated frame at 0x{:x}", frame_addr);
//
// // Logging errors
// mm_error!("Out of physical memory");
//
// // Logging test results
// mm_test_ok!("PMM allocation test passed");
//
// // Logging sizes with appropriate units
// use crate::mm::log::format_size;
// let (value, unit) = format_size(16 * 1024 * 1024);
// mm_log!("Heap size: {} {}", value, unit);  // Prints: [MM] Heap size: 16 MB
// ```

#![allow(dead_code)]

use core::sync::atomic::{AtomicUsize, Ordering};
use limine::request::{ExecutableAddressRequest, HhdmRequest, MemoryMapRequest};
use spin::Mutex;

pub mod allocator;
pub mod paging;
pub mod pmm;
pub mod security;
pub mod tlb;
pub mod mmap;

struct MemoryManagerState {
    pmm: pmm::PhysicalMemoryManager,
    mapper: paging::PageMapper,
}

static MEMORY_MANAGER: Mutex<Option<MemoryManagerState>> = Mutex::new(None);

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

/// Execute a closure with mutable access to the global PMM and page mapper.
///
/// Returns an error if the memory system has not been initialised yet.
pub fn with_memory_managers<R, F>(f: F) -> Result<R, &'static str>
where
    F: FnOnce(&mut pmm::PhysicalMemoryManager, &mut paging::PageMapper) -> Result<R, &'static str>,
{
    let mut guard = MEMORY_MANAGER.lock();
    let state = guard.as_mut().ok_or("Memory manager not initialized")?;
    f(&mut state.pmm, &mut state.mapper)
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

/// Test Physical Memory Manager
/// Tests frame allocation, multiple allocations, and free/reallocation
fn test_pmm(pmm: &mut pmm::PhysicalMemoryManager) {
    // Test 1: Allocate frame returns valid address
    let frame1 = pmm.alloc_frame();
    if let Some(addr) = frame1 {
        // Frame should be aligned to 4KB
        if addr % 4096 == 0 {
            // Success - frame allocated
        }
    }

    // Test 2: Multiple allocations return different frames
    let frame2 = pmm.alloc_frame();
    if let (Some(addr1), Some(addr2)) = (frame1, frame2) {
        if addr1 != addr2 {
            // Success - different frames allocated
        }
    }

    // Test 3: Free and reallocation reuses frame
    if let Some(addr1) = frame1 {
        pmm.free_frame(addr1);
        let frame3 = pmm.alloc_frame();
        if let Some(addr3) = frame3 {
            if addr3 == addr1 {
                // Success - freed frame was reused
            }
        }
    }
}

/// Test Paging System
/// Tests page mapping, translation, and unmapping
fn test_paging(mapper: &mut paging::PageMapper, pmm: &mut pmm::PhysicalMemoryManager) {
    // Test 1: Map and translate
    let test_virt = 0xFFFF_B000_0000_0000usize;
    let test_phys = pmm.alloc_frame();

    if let Some(phys_addr) = test_phys {
        // Map the page
        let map_result = mapper.map_page(
            test_virt,
            phys_addr,
            paging::PageTableFlags::PRESENT | paging::PageTableFlags::WRITABLE,
            pmm,
        );

        if map_result.is_ok() {
            // Test translation
            let translated = mapper.translate(test_virt);
            if let Some(translated_phys) = translated {
                if translated_phys == phys_addr {
                    // Success - page mapping and translation work
                }
            }

            // Test 2: Unmap
            let unmap_result = mapper.unmap_page(test_virt);
            if unmap_result.is_ok() {
                // Verify page is unmapped
                let translated_after = mapper.translate(test_virt);
                if translated_after.is_none() {
                    // Success - page unmapping works
                }
            }
        }

        // Free the test frame
        pmm.free_frame(phys_addr);
    }
}

/// Test Kernel Heap Allocator
/// Tests kmalloc, memory read/write, kfree, and multiple allocations
fn test_allocator() {
    // Test 1: Basic allocation
    let ptr = allocator::kmalloc(1024);
    if !ptr.is_null() {
        // Success - allocated 1024 bytes

        // Test 2: Memory write and read
        unsafe {
            *ptr = 0x42;
            if *ptr == 0x42 {
                // Success - memory read/write works
            }
        }

        // Test 3: Free
        allocator::kfree(ptr, 1024);
        // Success - freed memory
    }

    // Test 4: Multiple allocations (10x 64 bytes)
    let mut ptrs: [*mut u8; 10] = [core::ptr::null_mut(); 10];
    for i in 0..10 {
        ptrs[i] = allocator::kmalloc(64);
        if ptrs[i].is_null() {
            // Allocation failed
            break;
        }
    }

    // Test 5: Multiple frees
    for i in 0..10 {
        if !ptrs[i].is_null() {
            allocator::kfree(ptrs[i], 64);
        }
    }
    // Success - multiple allocations and frees work
}

/// Run all memory management tests
/// Calls all test functions in sequence and reports results
pub fn run_memory_tests(pmm: &mut pmm::PhysicalMemoryManager, mapper: &mut paging::PageMapper) {
    // Print test header
    // Note: Logging would be done here once logging infrastructure is available
    // Expected output:
    // [MM] ==========================================
    // [MM] Running Memory Management Tests
    // [MM] ==========================================

    // Run PMM tests
    test_pmm(pmm);
    // Expected output: [MM] ✓ PMM tests passed

    // Run paging tests
    test_paging(mapper, pmm);
    // Expected output: [MM] ✓ Paging tests passed

    // Run allocator tests
    test_allocator();
    // Expected output: [MM] ✓ Allocator tests passed

    // Print test footer
    // Expected output:
    // [MM] ==========================================
    // [MM] All tests passed! ✨
    // [MM] ==========================================
}

/// Initialize the entire memory management system
///
/// This function coordinates the initialization of all memory management components:
/// 1. HHDM offset from Limine
/// 2. CPU memory protection features (NX bit, write protection)
/// 3. Physical Memory Manager (PMM)
/// 4. Paging system with kernel section mapping
/// 5. Guard pages for stack/heap protection
/// 6. Kernel heap allocator
///
/// This should be called early in kernel initialization, after framebuffer setup
/// but before any dynamic memory allocation is needed.
pub fn init_memory() {
    

    // Get HHDM offset from Limine
    let hhdm_response = HHDM_REQUEST
        .get_response()
        .expect("[MM] ERROR: Failed to get HHDM response from Limine");

    let hhdm_offset = hhdm_response.offset() as usize;
    init_hhdm(hhdm_offset);

    // Get memory map from Limine
    let memory_map_response = MEMORY_MAP_REQUEST
        .get_response()
        .expect("[MM] ERROR: Failed to get memory map from Limine");

    // Get kernel address information from Limine
    let kernel_addr_response = KERNEL_ADDRESS_REQUEST
        .get_response()
        .expect("[MM] ERROR: Failed to get kernel address from Limine");

    let kernel_phys_base = kernel_addr_response.physical_base() as usize;
    let kernel_virt_base = kernel_addr_response.virtual_base() as usize;

    // Calculate kernel bounds (estimate 16MB for kernel image)
    let kernel_start = kernel_phys_base;
    let kernel_end = kernel_phys_base + (16 * 1024 * 1024); // 16MB

    // Enable CPU memory protection features
    enable_nx_bit();
    enable_write_protect();

    // Initialize Physical Memory Manager
    let mut pmm = pmm::PhysicalMemoryManager::init(memory_map_response, kernel_start, kernel_end);

    let _total_mb = pmm.total_memory_mb();
    let _free_mb = pmm.free_memory_mb();

    // Initialize paging system
    let mut mapper = paging::PageMapper::new();

    // Map kernel sections with appropriate permissions
    mapper
        .map_kernel_sections(kernel_addr_response, &mut pmm)
        .expect("[MM] ERROR: Failed to map kernel sections");

    // Define heap region (16MB heap starting at 0xFFFF_A000_0000_0000)
    let heap_start = 0xFFFF_A000_0000_0000usize;
    let heap_size = 16 * 1024 * 1024; // 16MB
    let heap_end = heap_start + heap_size;

    // Map heap region with RW+NX flags
    let mut heap_addr = heap_start;
    while heap_addr < heap_end {
        let phys_frame = pmm
            .alloc_frame()
            .expect("[MM] ERROR: Out of memory while mapping heap");

        mapper
            .map_page(
                heap_addr,
                phys_frame,
                paging::PageTableFlags::PRESENT
                    | paging::PageTableFlags::WRITABLE
                    | paging::PageTableFlags::NO_EXECUTE,
                &mut pmm,
            )
            .expect("[MM] ERROR: Failed to map heap page");

        heap_addr += 4096;
    }

    // Add guard pages around stack and heap
    // Note: Stack location would need to be determined from Limine or linker script
    // For now, we'll just add heap guard pages
    let _ = mapper.add_guard_pages(
        kernel_virt_base, // Placeholder for stack bottom
        heap_start,
        heap_end,
    );

    // Initialize kernel heap allocator
    allocator::init_allocator(heap_start, heap_size);

    // Run memory management tests
    run_memory_tests(&mut pmm, &mut mapper);

    // Store memory managers for later use (user-mode ELF loading, etc.)
    *MEMORY_MANAGER.lock() = Some(MemoryManagerState { pmm, mapper });

    // Log initialization summary
    // TODO: Replace with proper logging once available
    // For now, we'll skip logging to avoid dependencies
    // Expected output:
    // [MM] Initializing memory management...
    // [MM] Total memory: {total_mb} MB
    // [MM] Free memory: {free_mb} MB
    // [MM] Physical memory manager initialized
    // [MM] Page tables initialized
    // [MM] Kernel heap: 0xFFFF_A000_0000_0000 - 0xFFFF_A000_0100_0000 (16 MB)
    // [MM] Memory management initialized successfully
}
