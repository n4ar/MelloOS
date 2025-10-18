// Paging System
// Manages virtual memory mapping with 4-level page tables

#![allow(dead_code)]

use crate::mm::pmm::PhysicalMemoryManager;
use crate::mm::{phys_to_virt, PhysAddr, VirtAddr};

/// Page table entry flags
/// These flags control the behavior and permissions of mapped pages
pub struct PageTableFlags;

impl PageTableFlags {
    pub const PRESENT: u64 = 1 << 0;        // Page is present in memory
    pub const WRITABLE: u64 = 1 << 1;       // Page is writable
    pub const USER: u64 = 1 << 2;           // Page is accessible from user mode
    pub const WRITE_THROUGH: u64 = 1 << 3;  // Write-through caching
    pub const NO_CACHE: u64 = 1 << 4;       // Disable caching
    pub const ACCESSED: u64 = 1 << 5;       // Page has been accessed
    pub const DIRTY: u64 = 1 << 6;          // Page has been written to
    pub const HUGE: u64 = 1 << 7;           // Huge page (2MB or 1GB)
    pub const GLOBAL: u64 = 1 << 8;         // Global page (not flushed from TLB)
    pub const NO_EXECUTE: u64 = 1 << 63;    // Page is not executable (requires NXE bit)
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
    pub fn set(&mut self, addr: PhysAddr, flags: u64) {
        // Ensure address is 4KB aligned by masking lower 12 bits
        let addr_masked = (addr as u64) & 0x000F_FFFF_FFFF_F000;
        self.0 = addr_masked | flags;
    }
    
    /// Check if entry is present
    pub fn is_present(&self) -> bool {
        (self.0 & PageTableFlags::PRESENT) != 0
    }
    
    /// Clear entry (set to zero)
    pub fn clear(&mut self) {
        self.0 = 0;
    }
    
    /// Get the raw entry value
    pub fn raw(&self) -> u64 {
        self.0
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
        flags: u64,
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
        let pml4_entry = self.pml4.get_entry_mut(pml4_index);
        let pdpt_phys = if pml4_entry.is_present() {
            pml4_entry.addr()
        } else {
            let new_table = pmm.alloc_frame()
                .ok_or("Out of physical memory")?;
            let table_virt = phys_to_virt(new_table);
            let table = unsafe { &mut *(table_virt as *mut PageTable) };
            table.zero();
            pml4_entry.set(new_table, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
            new_table
        };
        
        // Get or create PD from PDPT
        let pdpt_virt = phys_to_virt(pdpt_phys);
        let pdpt = unsafe { &mut *(pdpt_virt as *mut PageTable) };
        let pdpt_entry = pdpt.get_entry_mut(pdpt_index);
        let pd_phys = if pdpt_entry.is_present() {
            pdpt_entry.addr()
        } else {
            let new_table = pmm.alloc_frame()
                .ok_or("Out of physical memory")?;
            let table_virt = phys_to_virt(new_table);
            let table = unsafe { &mut *(table_virt as *mut PageTable) };
            table.zero();
            pdpt_entry.set(new_table, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
            new_table
        };
        
        // Get or create PT from PD
        let pd_virt = phys_to_virt(pd_phys);
        let pd = unsafe { &mut *(pd_virt as *mut PageTable) };
        let pd_entry = pd.get_entry_mut(pd_index);
        let pt_phys = if pd_entry.is_present() {
            pd_entry.addr()
        } else {
            let new_table = pmm.alloc_frame()
                .ok_or("Out of physical memory")?;
            let table_virt = phys_to_virt(new_table);
            let table = unsafe { &mut *(table_virt as *mut PageTable) };
            table.zero();
            pd_entry.set(new_table, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
            new_table
        };
        
        // Set final PT entry
        let pt_virt = phys_to_virt(pt_phys);
        let pt = unsafe { &mut *(pt_virt as *mut PageTable) };
        let entry = pt.get_entry_mut(pt_index);
        entry.set(phys_addr, flags);
        
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
        
        // Invalidate TLB
        invlpg(virt_addr);
        
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
        if (pdpt_entry.raw() & PageTableFlags::HUGE) != 0 {
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
        if (pd_entry.raw() & PageTableFlags::HUGE) != 0 {
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
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE | PageTableFlags::GLOBAL,
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
        flags: u64,
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
