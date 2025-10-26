// Paging System
// Manages virtual memory mapping with 4-level page tables

#![allow(dead_code)]

use crate::mm::pmm::PhysicalMemoryManager;
use crate::mm::{phys_to_virt, PhysAddr, VirtAddr};
use core::sync::atomic::{AtomicUsize, Ordering};

/// Page table entry flags
/// These flags control the behavior and permissions of mapped pages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageTableFlags(pub u64);

impl PageTableFlags {
    pub const PRESENT: PageTableFlags = PageTableFlags(1 << 0); // Page is present in memory
    pub const WRITABLE: PageTableFlags = PageTableFlags(1 << 1); // Page is writable
    pub const USER: PageTableFlags = PageTableFlags(1 << 2); // Page is accessible from user mode
    pub const WRITE_THROUGH: PageTableFlags = PageTableFlags(1 << 3); // Write-through caching
    pub const NO_CACHE: PageTableFlags = PageTableFlags(1 << 4); // Disable caching
    pub const ACCESSED: PageTableFlags = PageTableFlags(1 << 5); // Page has been accessed
    pub const DIRTY: PageTableFlags = PageTableFlags(1 << 6); // Page has been written to
    pub const HUGE: PageTableFlags = PageTableFlags(1 << 7); // Huge page (2MB or 1GB)
    pub const GLOBAL: PageTableFlags = PageTableFlags(1 << 8); // Global page (not flushed from TLB)
    pub const COW: PageTableFlags = PageTableFlags(1 << 9); // Copy-on-Write page (available bit)
    pub const NO_EXECUTE: PageTableFlags = PageTableFlags(1 << 63); // Page is not executable (requires NXE bit)
}

impl core::ops::BitOr for PageTableFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        PageTableFlags(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for PageTableFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl core::ops::BitAnd for PageTableFlags {
    type Output = u64;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.0 & rhs.0
    }
}

impl core::ops::BitAnd<PageTableFlags> for u64 {
    type Output = u64;

    fn bitand(self, rhs: PageTableFlags) -> Self::Output {
        self & rhs.0
    }
}

impl From<PageTableFlags> for u64 {
    fn from(flags: PageTableFlags) -> Self {
        flags.0
    }
}

impl PageTableFlags {
    pub fn bits(&self) -> u64 {
        self.0
    }
}

/// Page table entry
/// Represents a single entry in a page table
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

/// Page table
/// Contains 512 entries, aligned to 4KB boundary
#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

/// Page table with reference counting for COW and shared mappings
///
/// This wrapper adds atomic reference counting to page tables,
/// enabling safe sharing between processes (e.g., for fork with COW).
pub struct PageTableRef {
    /// Physical address of the page table
    phys_addr: PhysAddr,

    /// Atomic reference count for safe sharing in SMP
    refcount: AtomicUsize,
}

/// Page mapper
/// Manages virtual memory mappings using 4-level page tables
pub struct PageMapper {
    pml4: &'static mut PageTable,
}

impl PageTableEntry {
    /// Create a new empty page table entry
    pub const fn new() -> Self {
        PageTableEntry(0)
    }

    /// Extract physical address from entry
    /// Masks bits 12-51 to get the physical address
    pub fn addr(&self) -> PhysAddr {
        (self.0 & 0x000F_FFFF_FFFF_F000) as usize
    }

    /// Set physical address and flags
    /// The address must be 4KB aligned
    pub fn set(&mut self, addr: PhysAddr, flags: PageTableFlags) {
        // Ensure address is 4KB aligned by masking lower 12 bits
        let addr_masked = (addr as u64) & 0x000F_FFFF_FFFF_F000;
        self.0 = addr_masked | flags.bits();
    }

    /// Check if entry is present
    pub fn is_present(&self) -> bool {
        (self.0 & PageTableFlags::PRESENT.bits()) != 0
    }

    /// Clear entry (set to zero)
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Get the raw entry value
    pub fn raw(&self) -> u64 {
        self.0
    }

    /// Set COW flag and clear WRITABLE flag
    ///
    /// This marks the page as copy-on-write, making it read-only until
    /// a write fault occurs.
    pub fn set_cow(&mut self) {
        self.0 |= PageTableFlags::COW.bits();
        self.0 &= !PageTableFlags::WRITABLE.bits();
    }

    /// Clear COW flag
    ///
    /// This removes the copy-on-write marking from the page.
    pub fn clear_cow(&mut self) {
        self.0 &= !PageTableFlags::COW.bits();
    }

    /// Check if entry is marked as COW
    pub fn is_cow(&self) -> bool {
        (self.0 & PageTableFlags::COW.bits()) != 0
    }

    /// Check if entry is writable
    pub fn is_writable(&self) -> bool {
        (self.0 & PageTableFlags::WRITABLE.bits()) != 0
    }

    /// Set writable flag
    pub fn set_writable(&mut self, writable: bool) {
        if writable {
            self.0 |= PageTableFlags::WRITABLE.bits();
        } else {
            self.0 &= !PageTableFlags::WRITABLE.bits();
        }
    }
}

impl PageTable {
    /// Create a new empty page table
    pub const fn new() -> Self {
        PageTable {
            entries: [PageTableEntry::new(); 512],
        }
    }

    /// Get a reference to an entry at the given index
    pub fn get_entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }

    /// Get a mutable reference to an entry at the given index
    pub fn get_entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }

    /// Zero all entries in the page table
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.clear();
        }
    }
}

impl PageMapper {
    /// Create a new page mapper using the current PML4
    /// Reads CR3 to get the current page table root
    pub fn new() -> Self {
        let pml4_phys = unsafe {
            let cr3: u64;
            core::arch::asm!(
                "mov {}, cr3",
                out(reg) cr3,
                options(nostack, preserves_flags)
            );
            (cr3 & 0x000F_FFFF_FFFF_F000) as usize
        };

        let pml4_virt = phys_to_virt(pml4_phys);
        let pml4 = unsafe { &mut *(pml4_virt as *mut PageTable) };

        PageMapper { pml4 }
    }

    /// Map a virtual address to a physical address with specified flags
    /// Creates intermediate page tables as needed
    ///
    /// # Arguments
    /// * `virt_addr` - Virtual address to map (must be 4KB aligned)
    /// * `phys_addr` - Physical address to map to (must be 4KB aligned)
    /// * `flags` - Page table flags (PRESENT, WRITABLE, etc.)
    /// * `pmm` - Physical memory manager for allocating new page tables
    pub fn map_page(
        &mut self,
        virt_addr: VirtAddr,
        phys_addr: PhysAddr,
        flags: PageTableFlags,
        pmm: &mut PhysicalMemoryManager,
    ) -> Result<(), &'static str> {
        // Validate alignment
        if virt_addr % 4096 != 0 || phys_addr % 4096 != 0 {
            return Err("Address not aligned to 4KB");
        }

        // Extract indices from virtual address
        // Virtual address structure (48-bit):
        // [47:39] PML4 index (9 bits)
        // [38:30] PDPT index (9 bits)
        // [29:21] PD index (9 bits)
        // [20:12] PT index (9 bits)
        // [11:0]  Offset (12 bits)
        let pml4_index = (virt_addr >> 39) & 0x1FF;
        let pdpt_index = (virt_addr >> 30) & 0x1FF;
        let pd_index = (virt_addr >> 21) & 0x1FF;
        let pt_index = (virt_addr >> 12) & 0x1FF;

        // Get or create PDPT from PML4
        let user_flag = if (flags.bits() & PageTableFlags::USER.bits()) != 0 {
            PageTableFlags::USER
        } else {
            PageTableFlags(0)
        };

        let table_flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | user_flag;

        let pml4_entry = self.pml4.get_entry_mut(pml4_index);
        let pdpt_phys = if pml4_entry.is_present() {
            pml4_entry.addr()
        } else {
            let new_table = pmm.alloc_frame().ok_or("Out of physical memory")?;
            let table_virt = phys_to_virt(new_table);
            let table = unsafe { &mut *(table_virt as *mut PageTable) };
            table.zero();
            pml4_entry.set(new_table, table_flags);
            new_table
        };

        // Get or create PD from PDPT
        let pdpt_virt = phys_to_virt(pdpt_phys);
        let pdpt = unsafe { &mut *(pdpt_virt as *mut PageTable) };
        let pdpt_entry = pdpt.get_entry_mut(pdpt_index);
        let pd_phys = if pdpt_entry.is_present() {
            pdpt_entry.addr()
        } else {
            let new_table = pmm.alloc_frame().ok_or("Out of physical memory")?;
            let table_virt = phys_to_virt(new_table);
            let table = unsafe { &mut *(table_virt as *mut PageTable) };
            table.zero();
            pdpt_entry.set(new_table, table_flags);
            new_table
        };

        // Get or create PT from PD
        let pd_virt = phys_to_virt(pd_phys);
        let pd = unsafe { &mut *(pd_virt as *mut PageTable) };
        let pd_entry = pd.get_entry_mut(pd_index);
        let pt_phys = if pd_entry.is_present() {
            pd_entry.addr()
        } else {
            let new_table = pmm.alloc_frame().ok_or("Out of physical memory")?;
            let table_virt = phys_to_virt(new_table);
            let table = unsafe { &mut *(table_virt as *mut PageTable) };
            table.zero();
            pd_entry.set(new_table, table_flags);
            new_table
        };

        // Set final PT entry
        let pt_virt = phys_to_virt(pt_phys);
        let pt = unsafe { &mut *(pt_virt as *mut PageTable) };
        let entry = pt.get_entry_mut(pt_index);

        // Check if we're remapping an existing page
        let was_present = entry.is_present();

        entry.set(phys_addr, flags);

        // If we remapped an existing page, perform TLB shootdown
        if was_present {
            unsafe {
                crate::mm::tlb::tlb_shootdown(virt_addr, virt_addr + 4096);
            }
        }

        Ok(())
    }
}

/// Invalidate TLB entry for a single page
/// Uses the invlpg instruction to flush the TLB entry for the given virtual address
fn invlpg(virt_addr: VirtAddr) {
    unsafe {
        core::arch::asm!(
            "invlpg [{}]",
            in(reg) virt_addr,
            options(nostack, preserves_flags)
        );
    }
}

impl PageMapper {
    /// Unmap a virtual address
    /// Clears the page table entry and invalidates the TLB
    ///
    /// # Arguments
    /// * `virt_addr` - Virtual address to unmap (must be 4KB aligned)
    pub fn unmap_page(&mut self, virt_addr: VirtAddr) -> Result<(), &'static str> {
        // Validate alignment
        if virt_addr % 4096 != 0 {
            return Err("Address not aligned to 4KB");
        }

        // Extract indices from virtual address
        let pml4_index = (virt_addr >> 39) & 0x1FF;
        let pdpt_index = (virt_addr >> 30) & 0x1FF;
        let pd_index = (virt_addr >> 21) & 0x1FF;
        let pt_index = (virt_addr >> 12) & 0x1FF;

        // Traverse page tables to find the entry
        let pml4_entry = self.pml4.get_entry(pml4_index);
        if !pml4_entry.is_present() {
            return Err("Page not mapped (PML4)");
        }

        let pdpt_phys = pml4_entry.addr();
        let pdpt_virt = phys_to_virt(pdpt_phys);
        let pdpt = unsafe { &mut *(pdpt_virt as *mut PageTable) };

        let pdpt_entry = pdpt.get_entry(pdpt_index);
        if !pdpt_entry.is_present() {
            return Err("Page not mapped (PDPT)");
        }

        let pd_phys = pdpt_entry.addr();
        let pd_virt = phys_to_virt(pd_phys);
        let pd = unsafe { &mut *(pd_virt as *mut PageTable) };

        let pd_entry = pd.get_entry(pd_index);
        if !pd_entry.is_present() {
            return Err("Page not mapped (PD)");
        }

        let pt_phys = pd_entry.addr();
        let pt_virt = phys_to_virt(pt_phys);
        let pt = unsafe { &mut *(pt_virt as *mut PageTable) };

        let entry = pt.get_entry_mut(pt_index);
        if !entry.is_present() {
            return Err("Page not mapped (PT)");
        }

        // Clear the entry
        entry.clear();

        // Perform TLB shootdown on all CPUs
        // This ensures all CPUs flush their TLB entries for this page
        unsafe {
            crate::mm::tlb::tlb_shootdown(virt_addr, virt_addr + 4096);
        }

        Ok(())
    }
}

impl PageMapper {
    /// Translate virtual address to physical address
    /// Walks the page tables to find the physical address
    ///
    /// # Arguments
    /// * `virt_addr` - Virtual address to translate
    ///
    /// # Returns
    /// * `Some(PhysAddr)` - Physical address if the page is mapped
    /// * `None` - If the page is not mapped
    pub fn translate(&self, virt_addr: VirtAddr) -> Option<PhysAddr> {
        // Extract indices from virtual address
        let pml4_index = (virt_addr >> 39) & 0x1FF;
        let pdpt_index = (virt_addr >> 30) & 0x1FF;
        let pd_index = (virt_addr >> 21) & 0x1FF;
        let pt_index = (virt_addr >> 12) & 0x1FF;
        let offset = virt_addr & 0xFFF;

        // Traverse PML4
        let pml4_entry = self.pml4.get_entry(pml4_index);
        if !pml4_entry.is_present() {
            return None;
        }

        // Traverse PDPT
        let pdpt_phys = pml4_entry.addr();
        let pdpt_virt = phys_to_virt(pdpt_phys);
        let pdpt = unsafe { &*(pdpt_virt as *const PageTable) };

        let pdpt_entry = pdpt.get_entry(pdpt_index);
        if !pdpt_entry.is_present() {
            return None;
        }

        // Check for 1GB huge page
        if (pdpt_entry.raw() & PageTableFlags::HUGE.bits()) != 0 {
            let page_offset = virt_addr & 0x3FFF_FFFF; // 1GB offset
            return Some(pdpt_entry.addr() + page_offset);
        }

        // Traverse PD
        let pd_phys = pdpt_entry.addr();
        let pd_virt = phys_to_virt(pd_phys);
        let pd = unsafe { &*(pd_virt as *const PageTable) };

        let pd_entry = pd.get_entry(pd_index);
        if !pd_entry.is_present() {
            return None;
        }

        // Check for 2MB huge page
        if (pd_entry.raw() & PageTableFlags::HUGE.bits()) != 0 {
            let page_offset = virt_addr & 0x1F_FFFF; // 2MB offset
            return Some(pd_entry.addr() + page_offset);
        }

        // Traverse PT
        let pt_phys = pd_entry.addr();
        let pt_virt = phys_to_virt(pt_phys);
        let pt = unsafe { &*(pt_virt as *const PageTable) };

        let pt_entry = pt.get_entry(pt_index);
        if !pt_entry.is_present() {
            return None;
        }

        // Return physical address with offset
        Some(pt_entry.addr() + offset)
    }

    /// Get page flags for a virtual address
    /// Walks the page tables to find the page table entry and extract its flags
    ///
    /// # Arguments
    /// * `virt_addr` - Virtual address to get flags for
    ///
    /// # Returns
    /// * `Some(PageTableFlags)` - Flags if the page is mapped
    /// * `None` - If the page is not mapped
    pub fn get_page_flags(&self, virt_addr: VirtAddr) -> Option<PageTableFlags> {
        // Extract indices from virtual address
        let pml4_index = (virt_addr >> 39) & 0x1FF;
        let pdpt_index = (virt_addr >> 30) & 0x1FF;
        let pd_index = (virt_addr >> 21) & 0x1FF;
        let pt_index = (virt_addr >> 12) & 0x1FF;

        // Traverse PML4
        let pml4_entry = self.pml4.get_entry(pml4_index);
        if !pml4_entry.is_present() {
            return None;
        }

        // Traverse PDPT
        let pdpt_phys = pml4_entry.addr();
        let pdpt_virt = phys_to_virt(pdpt_phys);
        let pdpt = unsafe { &*(pdpt_virt as *const PageTable) };

        let pdpt_entry = pdpt.get_entry(pdpt_index);
        if !pdpt_entry.is_present() {
            return None;
        }

        // Check for 1GB huge page
        if (pdpt_entry.raw() & PageTableFlags::HUGE.bits()) != 0 {
            return Some(PageTableFlags(pdpt_entry.raw() & 0xFFF));
        }

        // Traverse PD
        let pd_phys = pdpt_entry.addr();
        let pd_virt = phys_to_virt(pd_phys);
        let pd = unsafe { &*(pd_virt as *const PageTable) };

        let pd_entry = pd.get_entry(pd_index);
        if !pd_entry.is_present() {
            return None;
        }

        // Check for 2MB huge page
        if (pd_entry.raw() & PageTableFlags::HUGE.bits()) != 0 {
            return Some(PageTableFlags(pd_entry.raw() & 0xFFF));
        }

        // Traverse PT
        let pt_phys = pd_entry.addr();
        let pt_virt = phys_to_virt(pt_phys);
        let pt = unsafe { &*(pt_virt as *const PageTable) };

        let pt_entry = pt.get_entry(pt_index);
        if !pt_entry.is_present() {
            return None;
        }

        // Return flags (lower 12 bits of entry)
        Some(PageTableFlags(pt_entry.raw() & 0xFFF))
    }
}

impl PageMapper {
    /// Map kernel sections with appropriate permissions
    /// Maps .text (RX), .rodata (R), .data/.bss (RW+NX)
    ///
    /// # Arguments
    /// * `kernel_addr_response` - Limine kernel address response containing section addresses
    /// * `pmm` - Physical memory manager for allocating page tables
    pub fn map_kernel_sections(
        &mut self,
        kernel_addr_response: &limine::response::ExecutableAddressResponse,
        pmm: &mut PhysicalMemoryManager,
    ) -> Result<(), &'static str> {
        let kernel_base_virt = kernel_addr_response.virtual_base() as usize;
        let kernel_base_phys = kernel_addr_response.physical_base() as usize;

        // Get kernel section addresses from linker symbols
        // These are defined in the linker script
        extern "C" {
            static __text_start: u8;
            static __text_end: u8;
            static __rodata_start: u8;
            static __rodata_end: u8;
            static __data_start: u8;
            static __data_end: u8;
        }

        let text_start = unsafe { &__text_start as *const u8 as usize };
        let text_end = unsafe { &__text_end as *const u8 as usize };
        let rodata_start = unsafe { &__rodata_start as *const u8 as usize };
        let rodata_end = unsafe { &__rodata_end as *const u8 as usize };
        let data_start = unsafe { &__data_start as *const u8 as usize };
        let data_end = unsafe { &__data_end as *const u8 as usize };

        // Map .text section: Read + Execute (no write)
        // PRESENT | GLOBAL (no WRITABLE, no NO_EXECUTE)
        self.map_range(
            text_start,
            text_end,
            kernel_base_virt,
            kernel_base_phys,
            PageTableFlags::PRESENT | PageTableFlags::GLOBAL,
            pmm,
        )?;

        // Map .rodata section: Read only (no write, no execute)
        // PRESENT | NO_EXECUTE | GLOBAL
        self.map_range(
            rodata_start,
            rodata_end,
            kernel_base_virt,
            kernel_base_phys,
            PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE | PageTableFlags::GLOBAL,
            pmm,
        )?;

        // Map .data/.bss section: Read + Write (no execute)
        // PRESENT | WRITABLE | NO_EXECUTE | GLOBAL
        self.map_range(
            data_start,
            data_end,
            kernel_base_virt,
            kernel_base_phys,
            PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::NO_EXECUTE
                | PageTableFlags::GLOBAL,
            pmm,
        )?;

        Ok(())
    }

    /// Map a range of virtual addresses to physical addresses
    /// Helper function for mapping kernel sections
    fn map_range(
        &mut self,
        start_virt: VirtAddr,
        end_virt: VirtAddr,
        kernel_base_virt: VirtAddr,
        kernel_base_phys: PhysAddr,
        flags: PageTableFlags,
        pmm: &mut PhysicalMemoryManager,
    ) -> Result<(), &'static str> {
        // Align start down to page boundary
        let start = start_virt & !0xFFF;
        // Align end up to page boundary
        let end = (end_virt + 0xFFF) & !0xFFF;

        // Map each page in the range
        let mut virt = start;
        while virt < end {
            // Calculate corresponding physical address
            let offset = virt - kernel_base_virt;
            let phys = kernel_base_phys + offset;

            // Map the page
            self.map_page(virt, phys, flags, pmm)?;

            virt += 4096;
        }

        Ok(())
    }
}

impl PageMapper {
    /// Add guard pages around kernel stack and heap
    /// Unmaps pages to create protection zones that will trigger page faults
    ///
    /// # Arguments
    /// * `stack_bottom` - Bottom address of the kernel stack
    /// * `heap_start` - Start address of the kernel heap
    /// * `heap_end` - End address of the kernel heap
    pub fn add_guard_pages(
        &mut self,
        stack_bottom: VirtAddr,
        heap_start: VirtAddr,
        heap_end: VirtAddr,
    ) -> Result<(), &'static str> {
        // Add guard page below kernel stack
        // This will catch stack overflow
        let stack_guard = stack_bottom - 4096;
        if let Err(_) = self.unmap_page(stack_guard) {
            // Page might not be mapped, which is fine
        }

        // Add guard page before heap start
        // This will catch underflow access to heap
        if heap_start >= 4096 {
            let heap_guard_start = heap_start - 4096;
            if let Err(_) = self.unmap_page(heap_guard_start) {
                // Page might not be mapped, which is fine
            }
        }

        // Add guard page after heap end
        // This will catch overflow access to heap
        let heap_guard_end = heap_end;
        if let Err(_) = self.unmap_page(heap_guard_end) {
            // Page might not be mapped, which is fine
        }

        Ok(())
    }
}

/// Page Table Management Functions
///
/// These functions provide allocation, deallocation, and cloning of page tables
/// with proper reference counting for COW and process isolation.

impl PageTableRef {
    /// Create a new page table reference with refcount 1
    ///
    /// # Arguments
    /// * `phys_addr` - Physical address of the page table
    ///
    /// # Returns
    /// A new PageTableRef with refcount initialized to 1
    pub fn new(phys_addr: PhysAddr) -> Self {
        Self {
            phys_addr,
            refcount: AtomicUsize::new(1),
        }
    }

    /// Get the physical address of this page table
    pub fn phys_addr(&self) -> PhysAddr {
        self.phys_addr
    }

    /// Increment the reference count
    ///
    /// Returns the new reference count
    pub fn inc_ref(&self) -> usize {
        self.refcount.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Decrement the reference count
    ///
    /// Returns the new reference count (0 means the page table should be freed)
    pub fn dec_ref(&self) -> usize {
        let old_count = self.refcount.fetch_sub(1, Ordering::AcqRel);
        if old_count == 0 {
            panic!("PageTableRef: refcount underflow!");
        }
        old_count - 1
    }

    /// Get the current reference count
    pub fn refcount(&self) -> usize {
        self.refcount.load(Ordering::Acquire)
    }
}

/// Allocate a new page table
///
/// Allocates a physical frame for a page table and zeros it.
///
/// # Arguments
/// * `pmm` - Physical memory manager for frame allocation
///
/// # Returns
/// Ok(phys_addr) with the physical address of the new page table, or an error
pub fn alloc_page_table(pmm: &mut PhysicalMemoryManager) -> Result<PhysAddr, &'static str> {
    // Allocate a physical frame
    let phys_addr = pmm.alloc_frame().ok_or("Out of physical memory")?;

    // Zero the page table
    let virt_addr = phys_to_virt(phys_addr);
    let page_table = unsafe { &mut *(virt_addr as *mut PageTable) };
    page_table.zero();

    crate::serial_println!("[PAGING] Allocated page table at phys={:#x}", phys_addr);

    Ok(phys_addr)
}

/// Free a page table
///
/// Frees the physical frame used by a page table.
/// Should only be called when the reference count reaches zero.
///
/// # Arguments
/// * `phys_addr` - Physical address of the page table to free
/// * `pmm` - Physical memory manager for frame deallocation
///
/// # Safety
/// Caller must ensure no references to this page table exist
pub fn free_page_table(phys_addr: PhysAddr, pmm: &mut PhysicalMemoryManager) {
    crate::serial_println!("[PAGING] Freeing page table at phys={:#x}", phys_addr);
    pmm.free_frame(phys_addr);
}

/// Clone a page table for fork()
///
/// Creates a copy of a page table, copying all entries.
/// For COW implementation, this will mark pages as read-only and increment refcounts.
///
/// # Arguments
/// * `src_phys` - Physical address of the source page table
/// * `pmm` - Physical memory manager for allocating the new page table
///
/// # Returns
/// Ok(phys_addr) with the physical address of the cloned page table, or an error
pub fn clone_page_table(
    src_phys: PhysAddr,
    pmm: &mut PhysicalMemoryManager,
) -> Result<PhysAddr, &'static str> {
    // Allocate new page table
    let dst_phys = alloc_page_table(pmm)?;

    // Get virtual addresses for source and destination
    let src_virt = phys_to_virt(src_phys);
    let dst_virt = phys_to_virt(dst_phys);

    let src_table = unsafe { &*(src_virt as *const PageTable) };
    let dst_table = unsafe { &mut *(dst_virt as *mut PageTable) };

    // Copy all entries
    for i in 0..512 {
        let src_entry = src_table.get_entry(i);
        let dst_entry = dst_table.get_entry_mut(i);

        // Copy the raw entry value
        *dst_entry = *src_entry;
    }

    crate::serial_println!(
        "[PAGING] Cloned page table: src={:#x} -> dst={:#x}",
        src_phys,
        dst_phys
    );

    Ok(dst_phys)
}

/// Clone a full 4-level page table hierarchy
///
/// Recursively clones PML4, PDPT, PD, and PT levels.
/// This creates a complete copy of the address space for fork().
///
/// # Arguments
/// * `src_pml4_phys` - Physical address of the source PML4
/// * `pmm` - Physical memory manager for allocations
///
/// # Returns
/// Ok(phys_addr) with the physical address of the cloned PML4, or an error
pub fn clone_page_table_hierarchy(
    src_pml4_phys: PhysAddr,
    pmm: &mut PhysicalMemoryManager,
) -> Result<PhysAddr, &'static str> {
    // Allocate new PML4
    let dst_pml4_phys = alloc_page_table(pmm)?;

    let src_pml4_virt = phys_to_virt(src_pml4_phys);
    let dst_pml4_virt = phys_to_virt(dst_pml4_phys);

    let src_pml4 = unsafe { &*(src_pml4_virt as *const PageTable) };
    let dst_pml4 = unsafe { &mut *(dst_pml4_virt as *mut PageTable) };

    // Clone each PML4 entry (only user space: lower half)
    for i in 0..256 {
        // Only clone lower half (user space)
        let src_entry = src_pml4.get_entry(i);

        if src_entry.is_present() {
            // Clone PDPT level
            let src_pdpt_phys = src_entry.addr();
            let dst_pdpt_phys = clone_pdpt_level(src_pdpt_phys, pmm)?;

            // Set PML4 entry to point to cloned PDPT
            let dst_entry = dst_pml4.get_entry_mut(i);
            dst_entry.set(dst_pdpt_phys, PageTableFlags(src_entry.raw() & 0xFFF));
        }
    }

    // Copy kernel mappings (upper half) directly without cloning
    for i in 256..512 {
        let src_entry = src_pml4.get_entry(i);
        let dst_entry = dst_pml4.get_entry_mut(i);
        *dst_entry = *src_entry;
    }

    crate::serial_println!(
        "[PAGING] Cloned page table hierarchy: src_pml4={:#x} -> dst_pml4={:#x}",
        src_pml4_phys,
        dst_pml4_phys
    );

    Ok(dst_pml4_phys)
}

/// Clone PDPT level
fn clone_pdpt_level(
    src_pdpt_phys: PhysAddr,
    pmm: &mut PhysicalMemoryManager,
) -> Result<PhysAddr, &'static str> {
    let dst_pdpt_phys = alloc_page_table(pmm)?;

    let src_pdpt_virt = phys_to_virt(src_pdpt_phys);
    let dst_pdpt_virt = phys_to_virt(dst_pdpt_phys);

    let src_pdpt = unsafe { &*(src_pdpt_virt as *const PageTable) };
    let dst_pdpt = unsafe { &mut *(dst_pdpt_virt as *mut PageTable) };

    for i in 0..512 {
        let src_entry = src_pdpt.get_entry(i);

        if src_entry.is_present() {
            // Check for huge page (1GB)
            if (src_entry.raw() & PageTableFlags::HUGE.bits()) != 0 {
                // Copy huge page entry directly
                let dst_entry = dst_pdpt.get_entry_mut(i);
                *dst_entry = *src_entry;
            } else {
                // Clone PD level
                let src_pd_phys = src_entry.addr();
                let dst_pd_phys = clone_pd_level(src_pd_phys, pmm)?;

                let dst_entry = dst_pdpt.get_entry_mut(i);
                dst_entry.set(dst_pd_phys, PageTableFlags(src_entry.raw() & 0xFFF));
            }
        }
    }

    Ok(dst_pdpt_phys)
}

/// Clone PD level
fn clone_pd_level(
    src_pd_phys: PhysAddr,
    pmm: &mut PhysicalMemoryManager,
) -> Result<PhysAddr, &'static str> {
    let dst_pd_phys = alloc_page_table(pmm)?;

    let src_pd_virt = phys_to_virt(src_pd_phys);
    let dst_pd_virt = phys_to_virt(dst_pd_phys);

    let src_pd = unsafe { &*(src_pd_virt as *const PageTable) };
    let dst_pd = unsafe { &mut *(dst_pd_virt as *mut PageTable) };

    for i in 0..512 {
        let src_entry = src_pd.get_entry(i);

        if src_entry.is_present() {
            // Check for huge page (2MB)
            if (src_entry.raw() & PageTableFlags::HUGE.bits()) != 0 {
                // Copy huge page entry directly
                let dst_entry = dst_pd.get_entry_mut(i);
                *dst_entry = *src_entry;
            } else {
                // Clone PT level
                let src_pt_phys = src_entry.addr();
                let dst_pt_phys = clone_page_table(src_pt_phys, pmm)?;

                let dst_entry = dst_pd.get_entry_mut(i);
                dst_entry.set(dst_pt_phys, PageTableFlags(src_entry.raw() & 0xFFF));
            }
        }
    }

    Ok(dst_pd_phys)
}

/// Get the current CR3 value (current page table physical address)
pub fn get_current_cr3() -> PhysAddr {
    unsafe {
        let cr3: u64;
        core::arch::asm!(
            "mov {}, cr3",
            out(reg) cr3,
            options(nostack, preserves_flags)
        );
        (cr3 & 0x000F_FFFF_FFFF_F000) as usize
    }
}

/// Free a complete page table hierarchy
///
/// Recursively frees all page tables in the hierarchy (PML4, PDPT, PD, PT).
/// Only frees user space page tables (lower half), as kernel space is shared.
///
/// # Arguments
/// * `pml4_phys` - Physical address of the PML4 (root page table)
/// * `pmm` - Physical memory manager for frame deallocation
///
/// # Safety
/// Caller must ensure:
/// - The page table is not currently in use (not loaded in any CPU's CR3)
/// - No other references to this page table exist
/// - This is not the kernel's page table
pub fn free_page_table_hierarchy(pml4_phys: PhysAddr, pmm: &mut PhysicalMemoryManager) {
    crate::serial_println!(
        "[PAGING] Freeing page table hierarchy at PML4={:#x}",
        pml4_phys
    );

    let pml4_virt = phys_to_virt(pml4_phys);
    let pml4 = unsafe { &*(pml4_virt as *const PageTable) };

    // Only free user space page tables (lower half: indices 0-255)
    // Kernel space (upper half: 256-511) is shared and should not be freed
    for pml4_idx in 0..256 {
        let pml4_entry = pml4.get_entry(pml4_idx);
        if !pml4_entry.is_present() {
            continue;
        }

        let pdpt_phys = pml4_entry.addr();
        free_pdpt_level(pdpt_phys, pmm);
    }

    // Finally, free the PML4 itself
    free_page_table(pml4_phys, pmm);
    crate::serial_println!("[PAGING] Freed PML4 at {:#x}", pml4_phys);
}

/// Free PDPT level and all child page tables
fn free_pdpt_level(pdpt_phys: PhysAddr, pmm: &mut PhysicalMemoryManager) {
    let pdpt_virt = phys_to_virt(pdpt_phys);
    let pdpt = unsafe { &*(pdpt_virt as *const PageTable) };

    for pdpt_idx in 0..512 {
        let pdpt_entry = pdpt.get_entry(pdpt_idx);
        if !pdpt_entry.is_present() {
            continue;
        }

        // Check for huge page (1GB)
        if (pdpt_entry.raw() & PageTableFlags::HUGE.bits()) != 0 {
            // Huge pages don't have child page tables, skip
            continue;
        }

        let pd_phys = pdpt_entry.addr();
        free_pd_level(pd_phys, pmm);
    }

    // Free the PDPT itself
    free_page_table(pdpt_phys, pmm);
}

/// Free PD level and all child page tables
fn free_pd_level(pd_phys: PhysAddr, pmm: &mut PhysicalMemoryManager) {
    let pd_virt = phys_to_virt(pd_phys);
    let pd = unsafe { &*(pd_virt as *const PageTable) };

    for pd_idx in 0..512 {
        let pd_entry = pd.get_entry(pd_idx);
        if !pd_entry.is_present() {
            continue;
        }

        // Check for huge page (2MB)
        if (pd_entry.raw() & PageTableFlags::HUGE.bits()) != 0 {
            // Huge pages don't have child page tables, skip
            continue;
        }

        let pt_phys = pd_entry.addr();
        // Free the PT (no need to recurse further, PT is the lowest level)
        free_page_table(pt_phys, pmm);
    }

    // Free the PD itself
    free_page_table(pd_phys, pmm);
}

/// Mark all writable user pages as COW in a page table hierarchy
///
/// This function walks through all page table levels and marks writable user pages
/// as copy-on-write. It also increments the reference count for each COW page.
///
/// # Arguments
/// * `pml4_phys` - Physical address of the PML4 (root page table)
///
/// # Returns
/// Ok(count) with the number of pages marked as COW, or an error
pub fn mark_user_pages_cow(pml4_phys: PhysAddr) -> Result<usize, &'static str> {
    use crate::mm::refcount::PAGE_REFCOUNT;

    let pml4_virt = phys_to_virt(pml4_phys);
    let pml4 = unsafe { &mut *(pml4_virt as *mut PageTable) };

    let mut cow_count = 0;

    // Only process user space (lower half): indices 0-255
    for pml4_idx in 0..256 {
        let pml4_entry = pml4.get_entry_mut(pml4_idx);
        if !pml4_entry.is_present() {
            continue;
        }

        let pdpt_phys = pml4_entry.addr();
        let pdpt_virt = phys_to_virt(pdpt_phys);
        let pdpt = unsafe { &mut *(pdpt_virt as *mut PageTable) };

        for pdpt_idx in 0..512 {
            let pdpt_entry = pdpt.get_entry_mut(pdpt_idx);
            if !pdpt_entry.is_present() {
                continue;
            }

            // Check for 1GB huge page
            if (pdpt_entry.raw() & PageTableFlags::HUGE.bits()) != 0 {
                if pdpt_entry.is_writable() {
                    let page_phys = pdpt_entry.addr();
                    pdpt_entry.set_cow();
                    PAGE_REFCOUNT.inc_refcount(page_phys);
                    cow_count += 1;
                }
                continue;
            }

            let pd_phys = pdpt_entry.addr();
            let pd_virt = phys_to_virt(pd_phys);
            let pd = unsafe { &mut *(pd_virt as *mut PageTable) };

            for pd_idx in 0..512 {
                let pd_entry = pd.get_entry_mut(pd_idx);
                if !pd_entry.is_present() {
                    continue;
                }

                // Check for 2MB huge page
                if (pd_entry.raw() & PageTableFlags::HUGE.bits()) != 0 {
                    if pd_entry.is_writable() {
                        let page_phys = pd_entry.addr();
                        pd_entry.set_cow();
                        PAGE_REFCOUNT.inc_refcount(page_phys);
                        cow_count += 1;
                    }
                    continue;
                }

                let pt_phys = pd_entry.addr();
                let pt_virt = phys_to_virt(pt_phys);
                let pt = unsafe { &mut *(pt_virt as *mut PageTable) };

                for pt_idx in 0..512 {
                    let pt_entry = pt.get_entry_mut(pt_idx);
                    if !pt_entry.is_present() {
                        continue;
                    }

                    // Mark writable user pages as COW
                    if pt_entry.is_writable() {
                        let page_phys = pt_entry.addr();
                        pt_entry.set_cow();
                        PAGE_REFCOUNT.inc_refcount(page_phys);
                        cow_count += 1;
                    }
                }
            }
        }
    }

    crate::serial_println!(
        "[PAGING] Marked {} user pages as COW in page table at phys={:#x}",
        cow_count,
        pml4_phys
    );

    Ok(cow_count)
}

/// Set CR3 to switch to a different page table
///
/// # Arguments
/// * `phys_addr` - Physical address of the new page table (PML4)
///
/// # Safety
/// Caller must ensure the page table is valid and properly initialized
pub unsafe fn set_cr3(phys_addr: PhysAddr) {
    core::arch::asm!(
        "mov cr3, {}",
        in(reg) phys_addr as u64,
        options(nostack, preserves_flags)
    );
}

/// Kernel Page Table Template
///
/// This is a template page table that contains all kernel mappings.
/// New process page tables copy the upper half (kernel space) from this template.
use spin::Once;

static KERNEL_PAGE_TABLE_TEMPLATE: Once<PhysAddr> = Once::new();

/// Initialize the kernel page table template
///
/// This function should be called during kernel initialization to set up
/// the shared kernel mappings that all processes will inherit.
///
/// # Arguments
/// * `current_cr3` - Physical address of the current (boot) page table
///
/// # Returns
/// Ok(()) if initialization succeeded, or an error
pub fn init_kernel_template(current_cr3: PhysAddr) -> Result<(), &'static str> {
    // Store the current page table as the kernel template
    // All kernel mappings (upper half) are already set up in this table
    KERNEL_PAGE_TABLE_TEMPLATE.call_once(|| current_cr3);

    crate::serial_println!(
        "[PAGING] Kernel page table template initialized at phys={:#x}",
        current_cr3
    );

    Ok(())
}

/// Get the kernel page table template physical address
///
/// # Returns
/// Some(phys_addr) if the template has been initialized, None otherwise
pub fn get_kernel_template() -> Option<PhysAddr> {
    KERNEL_PAGE_TABLE_TEMPLATE.get().copied()
}

/// Copy kernel mappings from template to a new page table
///
/// Copies the upper half (kernel space) entries from the kernel template
/// to a new process page table. This ensures all processes share the same
/// kernel mappings.
///
/// # Arguments
/// * `dest_pml4_phys` - Physical address of the destination PML4
///
/// # Returns
/// Ok(()) if copy succeeded, or an error
pub fn copy_kernel_mappings(dest_pml4_phys: PhysAddr) -> Result<(), &'static str> {
    let template_phys = get_kernel_template().ok_or("Kernel template not initialized")?;

    let template_virt = phys_to_virt(template_phys);
    let dest_virt = phys_to_virt(dest_pml4_phys);

    let template_pml4 = unsafe { &*(template_virt as *const PageTable) };
    let dest_pml4 = unsafe { &mut *(dest_virt as *mut PageTable) };

    // Copy upper half (kernel space) entries: indices 256-511
    for i in 256..512 {
        let template_entry = template_pml4.get_entry(i);
        let dest_entry = dest_pml4.get_entry_mut(i);
        *dest_entry = *template_entry;
    }

    crate::serial_println!(
        "[PAGING] Copied kernel mappings to PML4 at phys={:#x}",
        dest_pml4_phys
    );

    Ok(())
}

/// Allocate a new page table with kernel mappings
///
/// This is a convenience function that allocates a new page table and
/// automatically copies the kernel mappings from the template.
///
/// # Arguments
/// * `pmm` - Physical memory manager for allocation
///
/// # Returns
/// Ok(phys_addr) with the physical address of the new page table, or an error
pub fn alloc_page_table_with_kernel_mappings(
    pmm: &mut PhysicalMemoryManager,
) -> Result<PhysAddr, &'static str> {
    // Allocate new PML4
    let pml4_phys = alloc_page_table(pmm)?;

    // Copy kernel mappings
    copy_kernel_mappings(pml4_phys)?;

    crate::serial_println!(
        "[PAGING] Allocated new page table with kernel mappings at phys={:#x}",
        pml4_phys
    );

    Ok(pml4_phys)
}

/// Update PageMapper::new() to use kernel template
impl PageMapper {
    /// Create a new page mapper for a process
    ///
    /// If a kernel template exists, this creates a new page table with kernel mappings.
    /// Otherwise, it uses the current page table (for kernel initialization).
    ///
    /// # Arguments
    /// * `pmm` - Physical memory manager for allocation (optional)
    ///
    /// # Returns
    /// A new PageMapper instance
    pub fn new_for_process(pmm: Option<&mut PhysicalMemoryManager>) -> Result<Self, &'static str> {
        if let Some(pmm_ref) = pmm {
            // Allocate new page table with kernel mappings
            let pml4_phys = alloc_page_table_with_kernel_mappings(pmm_ref)?;
            let pml4_virt = phys_to_virt(pml4_phys);
            let pml4 = unsafe { &mut *(pml4_virt as *mut PageTable) };

            Ok(PageMapper { pml4 })
        } else {
            // Use current page table (for kernel initialization)
            Ok(PageMapper::new())
        }
    }
}
