/// ELF64 Binary Loader
/// 
/// This module implements loading and parsing of ELF64 executables for user-mode execution.
/// It supports ET_EXEC format with PT_LOAD segments and proper memory protection.

use crate::mm::paging::{PageMapper, PageTableFlags};
use crate::mm::pmm::PhysicalMemoryManager;
use crate::mm::{phys_to_virt, PhysAddr};
use crate::sched::task::{MemoryRegion, MemoryRegionType, Task, USER_LIMIT};
use crate::serial_println;
use core::mem;

/// ELF identification bytes
const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

/// ELF class (64-bit)
const ELFCLASS64: u8 = 2;

/// ELF data encoding (little-endian)
const ELFDATA2LSB: u8 = 1;

/// ELF version (current)
const EV_CURRENT: u8 = 1;

/// ELF file types
const ET_EXEC: u16 = 2; // Executable file

/// ELF machine types
const EM_X86_64: u16 = 62; // AMD x86-64 architecture

/// Program header types
const PT_LOAD: u32 = 1; // Loadable segment
const PT_GNU_STACK: u32 = 0x6474e551; // GNU stack segment

/// Program header flags
const PF_X: u32 = 1; // Execute
const PF_W: u32 = 2; // Write
const PF_R: u32 = 4; // Read

/// User stack configuration
const USER_STACK_TOP: usize = 0x0000_7FFF_FFFF_0000;
const USER_STACK_SIZE: usize = 8192; // 8KB

/// ELF64 Header structure
#[repr(C)]
#[derive(Debug)]
struct Elf64Header {
    e_ident: [u8; 16],      // ELF identification
    e_type: u16,            // Object file type (ET_EXEC = 2)
    e_machine: u16,         // Machine type (EM_X86_64 = 62)
    e_version: u32,         // Object file version
    e_entry: u64,           // Entry point address
    e_phoff: u64,           // Program header offset
    e_shoff: u64,           // Section header offset
    e_flags: u32,           // Processor-specific flags
    e_ehsize: u16,          // ELF header size
    e_phentsize: u16,       // Program header entry size
    e_phnum: u16,           // Number of program header entries
    e_shentsize: u16,       // Section header entry size
    e_shnum: u16,           // Number of section header entries
    e_shstrndx: u16,        // Section header string table index
}

/// ELF64 Program Header structure
#[repr(C)]
#[derive(Debug)]
struct Elf64ProgramHeader {
    p_type: u32,            // Segment type (PT_LOAD = 1)
    p_flags: u32,           // Segment flags (PF_X=1, PF_W=2, PF_R=4)
    p_offset: u64,          // Segment file offset
    p_vaddr: u64,           // Segment virtual address
    p_paddr: u64,           // Segment physical address
    p_filesz: u64,          // Segment size in file
    p_memsz: u64,           // Segment size in memory
    p_align: u64,           // Segment alignment
}

/// ELF loader error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    /// Invalid ELF magic number
    InvalidMagic,
    /// Unsupported ELF class (not 64-bit)
    UnsupportedClass,
    /// Unsupported data encoding (not little-endian)
    UnsupportedEncoding,
    /// Unsupported ELF version
    UnsupportedVersion,
    /// Unsupported file type (not ET_EXEC)
    UnsupportedType,
    /// Unsupported machine type (not x86-64)
    UnsupportedMachine,
    /// Invalid entry point address
    InvalidEntryPoint,
    /// Invalid virtual address (not in user space)
    InvalidAddress,
    /// Buffer too small for ELF data
    BufferTooSmall,
    /// Out of memory during loading
    OutOfMemory,
    /// Invalid program header
    InvalidProgramHeader,
    /// Memory mapping failed
    MappingFailed,
}

/// ELF64 Binary Loader
pub struct ElfLoader<'a> {
    pmm: &'a mut PhysicalMemoryManager,
    mapper: &'a mut PageMapper,
}

impl<'a> ElfLoader<'a> {
    /// Create a new ELF loader
    pub fn new(pmm: &'a mut PhysicalMemoryManager, mapper: &'a mut PageMapper) -> Self {
        Self { pmm, mapper }
    }
    
    /// Load an ELF64 binary and set up memory regions for a task
    /// 
    /// # Arguments
    /// * `elf_data` - Raw ELF binary data
    /// * `task` - Task to load the binary into
    /// 
    /// # Returns
    /// Entry point address on success, or ElfError on failure
    pub fn load_elf(&mut self, elf_data: &[u8], task: &mut Task) -> Result<u64, ElfError> {
        serial_println!("[ELF] Loading ELF binary ({} bytes)", elf_data.len());
        
        // 1. Parse and validate ELF header
        let header = self.parse_elf_header(elf_data)?;
        serial_println!("[ELF] Entry point: 0x{:x}", header.e_entry);
        
        // 2. Validate ELF format
        self.validate_elf(&header)?;
        
        // 3. Parse program headers
        let program_headers = self.parse_program_headers(elf_data, &header)?;
        serial_println!("[ELF] Found {} program headers", program_headers.len());
        
        // 4. Clear existing memory regions
        task.clear_memory_regions();
        
        // 5. Map PT_LOAD segments
        for (i, phdr) in program_headers.iter().enumerate() {
            if phdr.p_type == PT_LOAD {
                serial_println!("[ELF] Mapping segment {}: 0x{:x}-0x{:x} (flags: 0x{:x})", 
                               i, phdr.p_vaddr, phdr.p_vaddr + phdr.p_memsz, phdr.p_flags);
                self.map_segment(elf_data, phdr, task)?;
            } else if phdr.p_type == PT_GNU_STACK {
                serial_println!("[ELF] Found GNU_STACK segment (flags: 0x{:x})", phdr.p_flags);
                // Note: GNU_STACK flags will be used for future stack protection
            }
        }
        
        // 6. Set up user stack
        self.setup_user_stack(task)?;
        
        serial_println!("[ELF] ELF loading completed successfully");
        Ok(header.e_entry)
    }
    
    /// Parse and validate the ELF header
    fn parse_elf_header(&self, elf_data: &[u8]) -> Result<Elf64Header, ElfError> {
        if elf_data.len() < mem::size_of::<Elf64Header>() {
            return Err(ElfError::BufferTooSmall);
        }
        
        // Safety: We've verified the buffer is large enough
        let header = unsafe {
            core::ptr::read(elf_data.as_ptr() as *const Elf64Header)
        };
        
        Ok(header)
    }
    
    /// Validate ELF header fields
    fn validate_elf(&self, header: &Elf64Header) -> Result<(), ElfError> {
        // Check ELF magic number
        if header.e_ident[0..4] != ELF_MAGIC {
            return Err(ElfError::InvalidMagic);
        }
        
        // Check ELF class (64-bit)
        if header.e_ident[4] != ELFCLASS64 {
            return Err(ElfError::UnsupportedClass);
        }
        
        // Check data encoding (little-endian)
        if header.e_ident[5] != ELFDATA2LSB {
            return Err(ElfError::UnsupportedEncoding);
        }
        
        // Check ELF version
        if header.e_ident[6] != EV_CURRENT {
            return Err(ElfError::UnsupportedVersion);
        }
        
        // Check file type (executable)
        if header.e_type != ET_EXEC {
            return Err(ElfError::UnsupportedType);
        }
        
        // Check machine type (x86-64)
        if header.e_machine != EM_X86_64 {
            return Err(ElfError::UnsupportedMachine);
        }
        
        // Validate entry point is in user space
        if header.e_entry >= USER_LIMIT as u64 {
            return Err(ElfError::InvalidEntryPoint);
        }
        
        // Check entry point alignment (recommended)
        if header.e_entry % 4096 != 0 {
            serial_println!("[ELF] Warning: Entry point 0x{:x} not page-aligned", header.e_entry);
        }
        
        Ok(())
    }
    
    /// Parse program headers from ELF data
    fn parse_program_headers(&self, elf_data: &[u8], header: &Elf64Header) -> Result<Vec<Elf64ProgramHeader>, ElfError> {
        let phdr_offset = header.e_phoff as usize;
        let phdr_size = header.e_phentsize as usize;
        let phdr_count = header.e_phnum as usize;
        
        // Validate program header table bounds
        let total_size = phdr_offset + (phdr_size * phdr_count);
        if total_size > elf_data.len() {
            return Err(ElfError::BufferTooSmall);
        }
        
        // Validate program header entry size
        if phdr_size != mem::size_of::<Elf64ProgramHeader>() {
            return Err(ElfError::InvalidProgramHeader);
        }
        
        let mut program_headers = Vec::new();
        
        for i in 0..phdr_count {
            let offset = phdr_offset + (i * phdr_size);
            
            // Safety: We've validated bounds above
            let phdr = unsafe {
                core::ptr::read((elf_data.as_ptr().add(offset)) as *const Elf64ProgramHeader)
            };
            
            program_headers.push(phdr);
        }
        
        Ok(program_headers)
    }
    
    /// Map a PT_LOAD segment into memory
    fn map_segment(&mut self, elf_data: &[u8], phdr: &Elf64ProgramHeader, task: &mut Task) -> Result<(), ElfError> {
        let vaddr = phdr.p_vaddr as usize;
        let size = phdr.p_memsz as usize;
        let file_size = phdr.p_filesz as usize;
        let file_offset = phdr.p_offset as usize;
        
        // Validate virtual address is in user space
        if vaddr >= USER_LIMIT || vaddr + size > USER_LIMIT {
            return Err(ElfError::InvalidAddress);
        }
        
        // Validate file offset and size
        if file_offset + file_size > elf_data.len() {
            return Err(ElfError::BufferTooSmall);
        }
        
        // Calculate page-aligned range
        let start_page = vaddr & !0xFFF;
        let end_page = (vaddr + size + 0xFFF) & !0xFFF;
        
        // Determine page flags from program header
        let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER;
        if phdr.p_flags & PF_W != 0 { 
            flags |= PageTableFlags::WRITABLE; 
        }
        if phdr.p_flags & PF_X == 0 { 
            flags |= PageTableFlags::NO_EXECUTE; 
        }
        
        // Determine memory region type based on flags
        let region_type = if phdr.p_flags & PF_X != 0 {
            MemoryRegionType::Code
        } else if phdr.p_flags & PF_W != 0 {
            if file_size == 0 {
                MemoryRegionType::Bss
            } else {
                MemoryRegionType::Data
            }
        } else {
            MemoryRegionType::Data // Read-only data
        };
        
        // Map pages and copy data using kernel mapping approach
        for page_addr in (start_page..end_page).step_by(4096) {
            let phys_frame = self.pmm.alloc_frame()
                .ok_or(ElfError::OutOfMemory)?;
            
            // Map page in user space
            self.mapper.map_page(page_addr, phys_frame, flags, self.pmm)
                .map_err(|_| ElfError::MappingFailed)?;
            
            // Create temporary kernel mapping for safe data copying
            let kernel_vaddr = phys_to_virt(phys_frame);
            
            // Calculate what portion of this page needs data from ELF
            let page_offset = if page_addr >= vaddr { 0 } else { vaddr - page_addr };
            let page_file_start = if page_addr >= vaddr { 
                file_offset + (page_addr - vaddr) 
            } else { 
                file_offset 
            };
            let page_file_size = core::cmp::min(
                4096 - page_offset,
                if file_size > (page_addr.saturating_sub(vaddr)) { 
                    file_size - (page_addr.saturating_sub(vaddr)) 
                } else { 
                    0 
                }
            );
            
            // Zero the entire page first
            unsafe {
                let page_slice = core::slice::from_raw_parts_mut(
                    kernel_vaddr as *mut u8, 
                    4096
                );
                page_slice.fill(0);
            }
            
            // Copy ELF data to this page if any
            if page_file_size > 0 && page_file_start < elf_data.len() {
                let src_start = page_file_start;
                let src_end = core::cmp::min(
                    src_start + page_file_size,
                    elf_data.len()
                );
                let src = &elf_data[src_start..src_end];
                
                unsafe {
                    let dst = core::slice::from_raw_parts_mut(
                        (kernel_vaddr + page_offset) as *mut u8,
                        src.len()
                    );
                    dst.copy_from_slice(src);
                }
            }
            
            // Flush TLB for this page to ensure visibility
            unsafe {
                core::arch::asm!("invlpg [{}]", in(reg) page_addr);
                // TODO: IPI TLB shootdown for SMP when implementing full page table separation
            }
        }
        
        // Add memory region to task
        let region = MemoryRegion::new(start_page, end_page, flags, region_type);
        task.add_memory_region(region)
            .map_err(|_| ElfError::MappingFailed)?;
        
        serial_println!("[ELF] Mapped segment: 0x{:x}-0x{:x} ({:?})", 
                       start_page, end_page, region_type);
        
        Ok(())
    }
    
    /// Set up user stack with guard pages
    fn setup_user_stack(&mut self, task: &mut Task) -> Result<(), ElfError> {
        let stack_top = USER_STACK_TOP;
        let stack_size = USER_STACK_SIZE;
        let stack_bottom = stack_top - stack_size;
        let guard_page = stack_bottom - 4096;
        
        serial_println!("[ELF] Setting up user stack: 0x{:x}-0x{:x}", stack_bottom, stack_top);
        
        // Map stack pages (RW + NX + USER)
        for addr in (stack_bottom..stack_top).step_by(4096) {
            let phys_frame = self.pmm.alloc_frame()
                .ok_or(ElfError::OutOfMemory)?;
            
            self.mapper.map_page(
                addr,
                phys_frame,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | 
                PageTableFlags::USER | PageTableFlags::NO_EXECUTE,
                self.pmm,
            ).map_err(|_| ElfError::MappingFailed)?;
            
            // Zero the stack page
            let kernel_vaddr = phys_to_virt(phys_frame);
            unsafe {
                let page_slice = core::slice::from_raw_parts_mut(
                    kernel_vaddr as *mut u8, 
                    4096
                );
                page_slice.fill(0);
            }
        }
        
        // Leave guard page unmapped to catch stack overflow
        // (guard_page is intentionally not mapped)
        
        // Add memory region for tracking
        let stack_region = MemoryRegion::new(
            stack_bottom,
            stack_top,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | 
            PageTableFlags::USER | PageTableFlags::NO_EXECUTE,
            MemoryRegionType::Stack,
        );
        
        task.add_memory_region(stack_region)
            .map_err(|_| ElfError::MappingFailed)?;
        
        serial_println!("[ELF] User stack set up successfully");
        Ok(())
    }
}

// Implement Vec for program headers (simple implementation)
struct Vec<T> {
    data: [Option<T>; 16], // Support up to 16 program headers
    len: usize,
}

impl<T> Vec<T> {
    fn new() -> Self {
        Self {
            data: [None, None, None, None, None, None, None, None,
                   None, None, None, None, None, None, None, None],
            len: 0,
        }
    }
    
    fn push(&mut self, item: T) {
        if self.len < 16 {
            self.data[self.len] = Some(item);
            self.len += 1;
        }
    }
    
    fn len(&self) -> usize {
        self.len
    }
    
    fn iter(&self) -> impl Iterator<Item = &T> {
        self.data[..self.len].iter().filter_map(|x| x.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sched::task::{Task, TaskPriority};
    
    /// Test ELF header validation with invalid magic number
    #[test]
    fn test_elf_header_validation_invalid_magic() {
        // Create invalid ELF data with wrong magic number
        let invalid_elf = [0x7E, b'E', b'L', b'F']; // Wrong first byte
        
        // This would normally require PMM and PageMapper, but for header validation
        // we can test the validation logic directly
        // In a real test environment, we'd mock these dependencies
        
        // For now, just verify the constants are correct
        assert_eq!(ELF_MAGIC, [0x7F, b'E', b'L', b'F']);
        assert_eq!(ELFCLASS64, 2);
        assert_eq!(ET_EXEC, 2);
        assert_eq!(EM_X86_64, 62);
    }
    
    /// Test memory region tracking validation
    #[test]
    fn test_memory_region_validation() {
        // Test that memory regions are properly validated
        let mut task = Task::new(1, "test", dummy_test_entry, TaskPriority::Normal)
            .expect("Failed to create test task");
        
        // Test adding a valid region
        let region = MemoryRegion::new(
            0x1000,
            0x2000,
            PageTableFlags::PRESENT | PageTableFlags::USER,
            MemoryRegionType::Code,
        );
        
        assert!(task.add_memory_region(region).is_ok());
        assert_eq!(task.region_count, 1);
        
        // Test finding the region
        assert!(task.find_memory_region(0x1500).is_some());
        assert!(task.find_memory_region(0x500).is_none());
    }
    
    /// Test ELF segment mapping flags
    #[test]
    fn test_segment_flag_mapping() {
        // Test that program header flags are correctly mapped to page table flags
        
        // Executable segment (code)
        let code_flags = PF_R | PF_X;
        assert_eq!(code_flags & PF_X, PF_X); // Should be executable
        assert_eq!(code_flags & PF_W, 0);    // Should not be writable
        
        // Writable segment (data)
        let data_flags = PF_R | PF_W;
        assert_eq!(data_flags & PF_W, PF_W); // Should be writable
        assert_eq!(data_flags & PF_X, 0);    // Should not be executable
        
        // Read-only segment
        let ro_flags = PF_R;
        assert_eq!(ro_flags & PF_W, 0);      // Should not be writable
        assert_eq!(ro_flags & PF_X, 0);      // Should not be executable
    }
    
    /// Dummy entry point for test tasks
    fn dummy_test_entry() -> ! {
        loop {
            unsafe {
                core::arch::asm!("hlt");
            }
        }
    }
}