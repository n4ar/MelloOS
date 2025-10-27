//! exec() system call implementation
//!
//! This module implements the exec() family of system calls, which replace the current
//! process image with a new program loaded from an ELF binary file.
//!
//! # Overview
//!
//! The exec() operation performs the following steps:
//! 1. Load and validate the ELF binary from the filesystem
//! 2. Parse ELF headers and program segments
//! 3. Clear the old process image (memory mappings)
//! 4. Load new program segments into memory
//! 5. Setup new stack with argc/argv/envp
//! 6. Close file descriptors marked with O_CLOEXEC
//! 7. Jump to the new program's entry point
//!
//! On success, exec() never returns (execution continues at new program).
//! On failure, the original process image is preserved and an error is returned.
//!
//! # Security
//!
//! This module implements comprehensive security validation for user space pointers:
//! - All pointers from user space are validated before dereferencing
//! - NULL pointers are rejected
//! - Kernel space addresses are rejected
//! - String lengths are enforced (max 4KB per string)
//! - Array sizes are enforced (max 1024 elements)
//!
//! These validations prevent:
//! - NULL pointer dereferences
//! - Kernel memory access from user space
//! - Buffer overflows from excessively long strings
//! - Resource exhaustion from too many arguments

use crate::sched::task::Task;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

/// Context for executing a new program via exec()
///
/// This structure holds all the information needed to replace the current
/// process image with a new program.
pub struct ExecContext {
    /// Path to the executable file
    pub path: String,

    /// Command-line arguments (argv)
    /// argv[0] is typically the program name
    pub argv: Vec<String>,

    /// Environment variables (envp)
    /// Each string is in the form "KEY=VALUE"
    pub envp: Vec<String>,

    /// Reference to the current task being replaced
    pub task: Arc<Task>,
}

/// Saved memory state for rollback on exec failure
///
/// This structure contains the process's memory layout before exec,
/// allowing us to restore it if the exec operation fails.
#[derive(Debug, Clone)]
pub struct SavedMemoryState {
    /// Saved memory regions (with per-page mappings)
    pub regions: Vec<SavedRegionState>,

    /// Saved heap pointers (heap_start, heap_end) if applicable
    pub heap_pointer: Option<(usize, usize)>,
}

/// Saved region with its page mappings
#[derive(Debug, Clone)]
pub struct SavedRegionState {
    /// Region metadata
    pub region: crate::sched::task::MemoryRegion,

    /// Physical frames backing each mapped page in the region
    pub pages: Vec<SavedPageState>,
}

/// Saved page mapping information
#[derive(Debug, Clone)]
pub struct SavedPageState {
    /// Page-aligned virtual address
    pub virt_addr: usize,

    /// Physical frame backing this page (page-aligned)
    pub phys_addr: usize,
}

impl ExecContext {
    /// Create a new execution context
    ///
    /// # Arguments
    /// * `path` - Path to the executable file
    /// * `argv` - Command-line arguments
    /// * `envp` - Environment variables
    /// * `task` - Current task to replace
    pub fn new(path: String, argv: Vec<String>, envp: Vec<String>, task: Arc<Task>) -> Self {
        Self {
            path,
            argv,
            envp,
            task,
        }
    }

    /// Execute a new program in the current process
    ///
    /// This is the main exec() implementation that orchestrates all the steps
    /// required to replace the current process image with a new program.
    ///
    /// # Process
    /// 1. Load ELF binary from filesystem
    /// 2. Parse and validate ELF headers
    /// 3. Save current memory state (for rollback on error)
    /// 4. Clear old process image
    /// 5. Load new program segments
    /// 6. Setup new stack with argc/argv/envp
    /// 7. Close O_CLOEXEC file descriptors
    /// 8. Update task state
    /// 9. Jump to userspace (never returns)
    ///
    /// # Returns
    /// Never returns on success (execution continues at new program)
    /// Returns Err on failure (original process image is preserved)
    ///
    /// # Errors
    /// * FileNotFound - Executable file does not exist
    /// * PermissionDenied - File is not executable
    /// * InvalidFormat - Not a valid ELF binary
    /// * OutOfMemory - Failed to allocate memory
    /// * InvalidArgument - Invalid parameters or addresses
    /// * IoError - I/O error reading file
    ///
    /// # Requirements
    /// Implements R1.1-R1.8, R7.5:
    /// - R1.1: Load ELF binary from specified path
    /// - R1.2: Replace current process image with new program
    /// - R1.3: Preserve process ID and file descriptors
    /// - R1.4: Never return on success
    /// - R1.5: Return error and preserve process on failure
    /// - R1.6: Close O_CLOEXEC file descriptors
    /// - R1.7: Pass command-line arguments to new program
    /// - R1.8: Pass environment variables to new program
    /// - R7.5: Preserve original process image on error
    ///
    /// # Security
    /// - All user space pointers are validated before use
    /// - Memory is zeroed before use to prevent information leakage
    /// - W^X policy is enforced (no writable+executable pages)
    /// - Stack is non-executable
    /// - On error, original process state is fully restored
    ///
    /// # Example
    /// ```rust
    /// let ctx = ExecContext::new(
    ///     "/bin/sh".to_string(),
    ///     vec!["/bin/sh".to_string()],
    ///     vec!["PATH=/bin".to_string()],
    ///     current_task,
    /// );
    ///
    /// // This never returns on success
    /// ctx.exec(pmm)?;
    /// ```
    pub fn exec(
        self,
        pmm: &mut crate::mm::pmm::PhysicalMemoryManager,
    ) -> Result<core::convert::Infallible, ExecError> {
        use crate::serial_println;

        serial_println!("[EXEC] Starting exec: path={}", self.path);
        serial_println!("[EXEC] Arguments: {:?}", self.argv);
        serial_println!("[EXEC] Environment: {} variables", self.envp.len());

        // Step 1: Load ELF binary from filesystem
        serial_println!("[EXEC] Step 1: Loading ELF from filesystem...");
        let elf_data = self.load_elf_from_fs().map_err(|e| {
            serial_println!("[EXEC] Failed to load ELF: {:?}", e);
            e
        })?;
        serial_println!("[EXEC] Loaded {} bytes", elf_data.len());

        // Step 2: Parse and validate ELF
        serial_println!("[EXEC] Step 2: Parsing ELF...");
        let elf_info = self.parse_elf(&elf_data).map_err(|e| {
            serial_println!("[EXEC] Failed to parse ELF: {:?}", e);
            e
        })?;
        serial_println!(
            "[EXEC] Parsed ELF: entry=0x{:016x}, {} segments",
            elf_info.entry,
            elf_info.segments.len()
        );

        // Step 3: Save current memory state for rollback
        serial_println!("[EXEC] Step 3: Saving current memory state...");
        let saved_state = self.clear_old_image().map_err(|e| {
            serial_println!("[EXEC] Failed to save/clear old image: {:?}", e);
            e
        })?;
        serial_println!("[EXEC] Saved {} memory regions", saved_state.regions.len());

        // From this point on, if we encounter an error, we need to rollback
        // We'll use a closure to handle rollback on error
        let rollback_on_error = |error: ExecError| -> ExecError {
            serial_println!("[EXEC] Error occurred, rolling back: {:?}", error);

            // Attempt to restore the old process image
            if let Err(restore_err) = self.restore_old_image(saved_state.clone()) {
                serial_println!(
                    "[EXEC] WARNING: Failed to restore old image: {:?}",
                    restore_err
                );
                // If rollback fails, we're in a bad state
                // The process is likely corrupted, but we return the original error
            } else {
                serial_println!("[EXEC] Successfully rolled back to old image");
            }

            error
        };

        // Step 4: Load new program segments
        serial_println!("[EXEC] Step 4: Loading program segments...");
        if let Err(e) = self.load_segments(&elf_info, &elf_data, pmm) {
            return Err(rollback_on_error(e));
        }
        serial_println!("[EXEC] Loaded {} segments", elf_info.segments.len());

        // Step 5: Setup new stack with argc/argv/envp
        serial_println!("[EXEC] Step 5: Setting up stack...");
        let stack_pointer = match self.setup_stack(pmm) {
            Ok(sp) => sp,
            Err(e) => return Err(rollback_on_error(e)),
        };
        serial_println!("[EXEC] Stack setup complete: sp=0x{:016x}", stack_pointer);

        // Step 6: Close O_CLOEXEC file descriptors
        serial_println!("[EXEC] Step 6: Closing O_CLOEXEC file descriptors...");
        if let Err(e) = self.close_cloexec_fds() {
            return Err(rollback_on_error(e));
        }
        serial_println!("[EXEC] Closed O_CLOEXEC file descriptors");

        // Step 7: Update task state
        // At this point, we've successfully loaded the new program
        // The task's context will be set up when we jump to userspace
        // The CpuContext only contains callee-saved registers (r15-r12, rbp, rbx, rsp)
        // which are used for context switching between kernel tasks.
        // For exec(), we directly jump to userspace with sysretq, so we don't
        // need to update the CpuContext. Instead, we'll set up the registers
        // in jump_to_userspace().
        serial_println!("[EXEC] Step 7: Task state ready for userspace transition");
        serial_println!("[EXEC] Step 8: Jumping to userspace...");

        // Step 8: Jump to userspace
        // This never returns on success
        // The new program starts executing at the entry point
        self.jump_to_userspace(elf_info.entry, stack_pointer)
    }

    /// Clear the old process image
    ///
    /// This method:
    /// 1. Saves current memory state for rollback on error
    /// 2. Unmaps all user space memory regions
    /// 3. Preserves kernel stack and task structures
    /// 4. Resets heap pointer (future implementation)
    /// 5. Clears old page tables (user space only)
    ///
    /// # Returns
    /// Ok(SavedMemoryState) containing the saved state for rollback, Err on failure
    ///
    /// # Errors
    /// * OutOfMemory - Failed to allocate memory for saved state
    ///
    /// # Requirements
    /// Implements R3.1, R3.2:
    /// - Save current memory state for rollback
    /// - Unmap all user space memory regions
    /// - Preserve kernel stack and task structures
    /// - Reset heap pointer
    /// - Clear old page tables
    ///
    /// # Safety
    /// This function modifies the process's memory layout. The caller must ensure
    /// that no user space code is executing when this is called. The kernel stack
    /// and task structures are preserved to allow the kernel to continue execution.
    pub fn clear_old_image(&self) -> Result<SavedMemoryState, ExecError> {
        use crate::mm::paging::PageMapper;

        // 1. Save current memory state for rollback
        // We need to save the memory regions and their backing pages so we can restore
        let mut saved_regions: Vec<SavedRegionState> = Vec::new();

        // Access task's memory regions through interior mutability pattern
        // Since we have Arc<Task>, we need to be careful about concurrent access
        // For now, we'll work with the assumption that the task is not being
        // modified by other threads during exec (single-threaded per process)

        // Clone the memory regions for rollback
        // Note: In a full implementation, we would also save the actual page table
        // entries and their contents. For now, we just save the region descriptors.
        let mut mapper = PageMapper::new();

        for i in 0..self.task.region_count {
            if let Some(region) = &self.task.memory_regions[i] {
                // Capture per-page mappings for this region
                let start_page = region.start & !0xFFF;
                let end_page = (region
                    .end
                    .checked_add(0xFFF)
                    .ok_or(ExecError::InvalidArgument)?
                    & !0xFFF)
                    .max(start_page);

                let mut pages = Vec::new();
                let mut addr = start_page;
                while addr < end_page {
                    if let Some(phys_addr) = mapper.translate(addr) {
                        pages.push(SavedPageState {
                            virt_addr: addr,
                            phys_addr: phys_addr & !0xFFF,
                        });
                    } else {
                        crate::serial_println!(
                            "[EXEC] WARNING: Page 0x{:016x} not mapped while saving region",
                            addr
                        );
                    }
                    addr += 4096;
                }

                saved_regions.push(SavedRegionState {
                    region: region.clone(),
                    pages,
                });
            }
        }

        // Save the current heap pointer for rollback
        // We save both heap_start and heap_end so we can restore them if exec fails
        let saved_heap_pointer = if self.task.heap_start != 0 {
            Some((self.task.heap_start, self.task.heap_end))
        } else {
            None
        };

        // 2. Unmap all user space memory regions
        // We need to walk through all user space pages and unmap them
        // This is done by clearing the page table entries for user space

        // Unmap each memory region
        // We iterate through the saved regions (not task.memory_regions) to avoid
        // borrowing issues, since we'll be modifying the task's regions
        for region in &saved_regions {
            // Unmap all pages in this region
            let start_page = region.region.start & !0xFFF; // Align down to page boundary
            let end_page = (region
                .region
                .end
                .checked_add(0xFFF)
                .ok_or(ExecError::InvalidArgument)?
                & !0xFFF)
                .max(start_page); // Align up to page boundary

            let mut addr = start_page;
            while addr < end_page {
                // Unmap the page
                // We ignore errors here because some pages might not be mapped
                // (e.g., if they were demand-paged and never accessed)
                let _ = mapper.unmap_page(addr);
                addr += 4096;
            }
        }

        // 3. Clear the task's memory region tracking
        // This is safe because we've saved the regions for rollback
        // We need to use unsafe here to get mutable access through Arc
        // In a real implementation, we would use proper synchronization
        unsafe {
            let task_ptr = Arc::as_ptr(&self.task) as *mut Task;
            (*task_ptr).clear_memory_regions();
        }

        // 4. Reset heap pointer
        // Clear the heap pointers since we're replacing the process image
        // The new program will set up its own heap via brk/sbrk syscalls
        unsafe {
            let task_ptr = Arc::as_ptr(&self.task) as *mut Task;
            (*task_ptr).heap_start = 0;
            (*task_ptr).heap_end = 0;
        }

        // 5. Page tables are already cleared by unmapping the regions
        // The kernel space mappings are preserved because we only unmapped
        // user space addresses (below USER_LIMIT)

        // Return the saved state for potential rollback
        Ok(SavedMemoryState {
            regions: saved_regions,
            heap_pointer: saved_heap_pointer,
        })
    }

    /// Restore the old process image from saved state
    ///
    /// This method is called when exec fails and we need to rollback to the
    /// previous process state. It restores the memory regions and heap pointer.
    ///
    /// # Arguments
    /// * `saved_state` - The saved memory state to restore
    ///
    /// # Returns
    /// Ok(()) if restoration succeeded, Err on failure
    ///
    /// # Safety
    /// This function modifies the process's memory layout. It should only be
    /// called after clear_old_image() has been called and exec has failed.
    pub fn restore_old_image(&self, saved_state: SavedMemoryState) -> Result<(), ExecError> {
        use crate::mm::paging::PageMapper;

        // Note: In a full implementation with proper page table isolation,
        // we would need to:
        // 1. Remap all the physical pages that were unmapped
        // 2. Restore the page table entries with correct flags
        // 3. Restore the actual page contents from a saved copy
        //
        // For now, since we're using a shared address space model where
        // user and kernel share the same page tables, the unmapping in
        // clear_old_image() doesn't actually free the physical pages.
        // They remain allocated but unmapped. So restoration is simpler.

        // Remap each saved page back into the address space
        let mut mapper = PageMapper::new();
        let mut pmm_guard = crate::mm::pmm::get_global_pmm();
        let pmm = pmm_guard.as_mut().ok_or(ExecError::OutOfMemory)?;

        for saved_region in &saved_state.regions {
            let flags = saved_region.region.flags;
            for page in &saved_region.pages {
                mapper
                    .map_page(page.virt_addr, page.phys_addr, flags, pmm)
                    .map_err(|_| ExecError::OutOfMemory)?;
            }
        }
        drop(pmm_guard);

        // Restore the task's memory region tracking
        unsafe {
            let task_ptr = Arc::as_ptr(&self.task) as *mut Task;
            (*task_ptr).clear_memory_regions();

            for region in &saved_state.regions {
                let _ = (*task_ptr).add_memory_region(region.region.clone());
            }
        }

        // Restore heap pointer
        if let Some((heap_start, heap_end)) = saved_state.heap_pointer {
            // Restore the saved heap pointers to their original values
            unsafe {
                let task_ptr = Arc::as_ptr(&self.task) as *mut Task;
                (*task_ptr).heap_start = heap_start;
                (*task_ptr).heap_end = heap_end;
            }
        }

        Ok(())
    }

    /// Load ELF binary from filesystem
    ///
    /// This method:
    /// 1. Resolves the path using VFS
    /// 2. Opens the file and reads its contents
    /// 3. Returns the file data as a Vec<u8>
    ///
    /// # Returns
    /// Ok(Vec<u8>) containing the ELF file data, Err on failure
    ///
    /// # Errors
    /// * FileNotFound - File does not exist (ENOENT)
    /// * PermissionDenied - File is not readable (EACCES)
    /// * IoError - I/O error reading file (EIO)
    ///
    /// # Requirements
    /// Implements R2.1, R2.2, R2.6, R7.1, R7.2:
    /// - Use VFS path resolution to find file
    /// - Open file and read contents into Vec<u8>
    /// - Handle file not found error (ENOENT)
    /// - Handle permission denied error (EACCES)
    /// - Close file descriptor after reading (automatic via RAII)
    pub fn load_elf_from_fs(&self) -> Result<Vec<u8>, ExecError> {
        use crate::fs::vfs::path;
        use crate::fs::vfs::superblock::FsError;

        // Resolve the path to an inode
        // Use None for current_dir to force absolute path resolution from root
        let inode = path::resolve_path(&self.path, None).map_err(|e| match e {
            FsError::NotFound => ExecError::FileNotFound,
            FsError::PermissionDenied => ExecError::PermissionDenied,
            FsError::NotADirectory => ExecError::FileNotFound,
            FsError::InvalidArgument => ExecError::InvalidArgument,
            _ => ExecError::IoError,
        })?;

        // Check if it's a regular file
        if !inode.mode().is_file() {
            return Err(ExecError::PermissionDenied);
        }

        // Get file size
        let file_size = inode.size();

        // Sanity check: ELF files should be at least 64 bytes (ELF header size)
        if file_size < 64 {
            return Err(ExecError::InvalidFormat);
        }

        // Sanity check: Limit file size to 16MB to prevent memory exhaustion
        const MAX_ELF_SIZE: u64 = 16 * 1024 * 1024;
        if file_size > MAX_ELF_SIZE {
            return Err(ExecError::OutOfMemory);
        }

        // Allocate buffer for file contents
        let mut buffer = Vec::with_capacity(file_size as usize);
        buffer.resize(file_size as usize, 0);

        // Read entire file into buffer
        let bytes_read = inode.read_at(0, &mut buffer).map_err(|e| match e {
            FsError::PermissionDenied => ExecError::PermissionDenied,
            FsError::IoError => ExecError::IoError,
            _ => ExecError::IoError,
        })?;

        // Verify we read the expected amount
        if bytes_read != file_size as usize {
            return Err(ExecError::IoError);
        }

        // File descriptor is automatically closed (inode is dropped here)
        Ok(buffer)
    }

    /// Parse and validate ELF binary
    ///
    /// This method:
    /// 1. Validates ELF magic number (0x7F 'E' 'L' 'F')
    /// 2. Parses ELF header (entry point, program header offset)
    /// 3. Parses program headers (PT_LOAD segments)
    /// 4. Validates architecture (x86_64)
    /// 5. Returns ENOEXEC for invalid ELF files
    ///
    /// # Arguments
    /// * `data` - Raw ELF file data
    ///
    /// # Returns
    /// Ok(ElfInfo) containing parsed ELF information, Err on validation failure
    ///
    /// # Errors
    /// * InvalidFormat - Not a valid ELF file (ENOEXEC)
    /// * InvalidArgument - Wrong architecture or invalid headers
    ///
    /// # Requirements
    /// Implements R2.3, R2.4, R7.3:
    /// - Validate ELF magic number (0x7F 'E' 'L' 'F')
    /// - Parse ELF header (entry point, program header offset)
    /// - Parse program headers (PT_LOAD segments)
    /// - Validate architecture (x86_64)
    /// - Return ENOEXEC for invalid ELF files
    pub fn parse_elf(&self, data: &[u8]) -> Result<ElfInfo, ExecError> {
        // Minimum ELF header size is 64 bytes for 64-bit ELF
        if data.len() < 64 {
            return Err(ExecError::InvalidFormat);
        }

        // Validate ELF magic number: 0x7F 'E' 'L' 'F'
        if data[0] != 0x7F || data[1] != b'E' || data[2] != b'L' || data[3] != b'F' {
            return Err(ExecError::InvalidFormat);
        }

        // Validate ELF class (64-bit)
        const ELFCLASS64: u8 = 2;
        if data[4] != ELFCLASS64 {
            return Err(ExecError::InvalidFormat);
        }

        // Validate endianness (little-endian)
        const ELFDATA2LSB: u8 = 1;
        if data[5] != ELFDATA2LSB {
            return Err(ExecError::InvalidFormat);
        }

        // Validate ELF version
        if data[6] != 1 {
            return Err(ExecError::InvalidFormat);
        }

        // Validate ELF type (executable)
        const ET_EXEC: u16 = 2;
        const ET_DYN: u16 = 3; // Position-independent executable
        let elf_type = u16::from_le_bytes([data[16], data[17]]);
        if elf_type != ET_EXEC && elf_type != ET_DYN {
            return Err(ExecError::InvalidFormat);
        }

        // Validate machine architecture (x86_64)
        const EM_X86_64: u16 = 62;
        let machine = u16::from_le_bytes([data[18], data[19]]);
        if machine != EM_X86_64 {
            return Err(ExecError::InvalidFormat);
        }

        // Parse entry point address (offset 24, 8 bytes)
        let entry = u64::from_le_bytes([
            data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
        ]);

        // Validate entry point is in user space
        use crate::sched::task::USER_LIMIT;
        if entry == 0 || entry >= USER_LIMIT as u64 {
            return Err(ExecError::InvalidFormat);
        }

        // Parse program header offset (offset 32, 8 bytes)
        let phoff = u64::from_le_bytes([
            data[32], data[33], data[34], data[35], data[36], data[37], data[38], data[39],
        ]);

        // Parse program header entry size (offset 54, 2 bytes)
        let phentsize = u16::from_le_bytes([data[54], data[55]]);

        // Parse program header count (offset 56, 2 bytes)
        let phnum = u16::from_le_bytes([data[56], data[57]]);

        // Validate program header table is within file bounds
        let ph_table_size = phentsize as u64 * phnum as u64;
        if phoff.checked_add(ph_table_size).is_none()
            || (phoff + ph_table_size) as usize > data.len()
        {
            return Err(ExecError::InvalidFormat);
        }

        // Parse program headers
        let mut segments = Vec::new();

        for i in 0..phnum {
            let ph_offset = (phoff + (i as u64 * phentsize as u64)) as usize;

            // Ensure we have enough data for this program header
            if ph_offset + 56 > data.len() {
                return Err(ExecError::InvalidFormat);
            }

            // Parse program header fields
            let seg_type = u32::from_le_bytes([
                data[ph_offset],
                data[ph_offset + 1],
                data[ph_offset + 2],
                data[ph_offset + 3],
            ]);

            let flags = u32::from_le_bytes([
                data[ph_offset + 4],
                data[ph_offset + 5],
                data[ph_offset + 6],
                data[ph_offset + 7],
            ]);

            let offset = u64::from_le_bytes([
                data[ph_offset + 8],
                data[ph_offset + 9],
                data[ph_offset + 10],
                data[ph_offset + 11],
                data[ph_offset + 12],
                data[ph_offset + 13],
                data[ph_offset + 14],
                data[ph_offset + 15],
            ]);

            let vaddr = u64::from_le_bytes([
                data[ph_offset + 16],
                data[ph_offset + 17],
                data[ph_offset + 18],
                data[ph_offset + 19],
                data[ph_offset + 20],
                data[ph_offset + 21],
                data[ph_offset + 22],
                data[ph_offset + 23],
            ]);

            // Skip paddr (physical address) at offset 24

            let filesz = u64::from_le_bytes([
                data[ph_offset + 32],
                data[ph_offset + 33],
                data[ph_offset + 34],
                data[ph_offset + 35],
                data[ph_offset + 36],
                data[ph_offset + 37],
                data[ph_offset + 38],
                data[ph_offset + 39],
            ]);

            let memsz = u64::from_le_bytes([
                data[ph_offset + 40],
                data[ph_offset + 41],
                data[ph_offset + 42],
                data[ph_offset + 43],
                data[ph_offset + 44],
                data[ph_offset + 45],
                data[ph_offset + 46],
                data[ph_offset + 47],
            ]);

            let align = u64::from_le_bytes([
                data[ph_offset + 48],
                data[ph_offset + 49],
                data[ph_offset + 50],
                data[ph_offset + 51],
                data[ph_offset + 52],
                data[ph_offset + 53],
                data[ph_offset + 54],
                data[ph_offset + 55],
            ]);

            // Only process PT_LOAD segments
            if seg_type == PT_LOAD {
                // Validate segment is within file bounds
                if filesz > 0 {
                    if offset.checked_add(filesz).is_none()
                        || (offset + filesz) as usize > data.len()
                    {
                        return Err(ExecError::InvalidFormat);
                    }
                }

                // Validate virtual address is in user space
                if vaddr >= USER_LIMIT as u64 {
                    return Err(ExecError::InvalidFormat);
                }

                // Validate memsz >= filesz
                if memsz < filesz {
                    return Err(ExecError::InvalidFormat);
                }

                // Validate segment doesn't overflow virtual address space
                if vaddr.checked_add(memsz).is_none() {
                    return Err(ExecError::InvalidFormat);
                }

                // Check that segment end is still in user space
                if vaddr + memsz > USER_LIMIT as u64 {
                    return Err(ExecError::InvalidFormat);
                }

                segments.push(ProgramSegment {
                    seg_type,
                    vaddr,
                    filesz,
                    memsz,
                    flags,
                    offset,
                    align,
                });
            }
        }

        // Validate we found at least one loadable segment
        if segments.is_empty() {
            return Err(ExecError::InvalidFormat);
        }

        Ok(ElfInfo { entry, segments })
    }
}

/// Errors that can occur during exec()
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecError {
    /// File not found (ENOENT)
    FileNotFound,

    /// Permission denied (EACCES)
    PermissionDenied,

    /// Invalid ELF format (ENOEXEC)
    InvalidFormat,

    /// Out of memory (ENOMEM)
    OutOfMemory,

    /// Invalid argument (EINVAL)
    InvalidArgument,

    /// I/O error (EIO)
    IoError,

    /// Argument list too long (E2BIG)
    ArgumentListTooLong,

    /// Text file busy (ETXTBSY)
    TextFileBusy,
}

impl ExecError {
    /// Convert ExecError to errno value
    ///
    /// Returns the negative errno value that should be returned to userspace.
    pub fn to_errno(self) -> isize {
        match self {
            ExecError::FileNotFound => -2,        // ENOENT
            ExecError::PermissionDenied => -13,   // EACCES
            ExecError::InvalidFormat => -8,       // ENOEXEC
            ExecError::OutOfMemory => -12,        // ENOMEM
            ExecError::InvalidArgument => -22,    // EINVAL
            ExecError::IoError => -5,             // EIO
            ExecError::ArgumentListTooLong => -7, // E2BIG
            ExecError::TextFileBusy => -26,       // ETXTBSY
        }
    }
}

impl core::fmt::Display for ExecError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ExecError::FileNotFound => write!(f, "File not found"),
            ExecError::PermissionDenied => write!(f, "Permission denied"),
            ExecError::InvalidFormat => write!(f, "Invalid ELF format"),
            ExecError::OutOfMemory => write!(f, "Out of memory"),
            ExecError::InvalidArgument => write!(f, "Invalid argument"),
            ExecError::IoError => write!(f, "I/O error"),
            ExecError::ArgumentListTooLong => write!(f, "Argument list too long"),
            ExecError::TextFileBusy => write!(f, "Text file busy"),
        }
    }
}

/// Information parsed from an ELF binary
///
/// This structure contains the essential information extracted from an ELF file
/// that is needed to load and execute the program.
#[derive(Debug, Clone)]
pub struct ElfInfo {
    /// Entry point address where execution should begin
    pub entry: u64,

    /// Program segments to be loaded into memory
    pub segments: Vec<ProgramSegment>,
}

/// A single program segment from an ELF file
///
/// Represents a PT_LOAD segment that should be mapped into the process's
/// virtual address space.
#[derive(Debug, Clone)]
pub struct ProgramSegment {
    /// Segment type (PT_LOAD, PT_DYNAMIC, etc.)
    pub seg_type: u32,

    /// Virtual address where segment should be loaded
    pub vaddr: u64,

    /// Size of segment in the file
    pub filesz: u64,

    /// Size of segment in memory (may be larger than filesz for BSS)
    pub memsz: u64,

    /// Segment flags (read, write, execute permissions)
    pub flags: u32,

    /// Offset in the ELF file where segment data starts
    pub offset: u64,

    /// Alignment requirement for the segment
    pub align: u64,
}

impl ProgramSegment {
    /// Check if segment is readable
    pub fn is_readable(&self) -> bool {
        const PF_R: u32 = 0x4;
        (self.flags & PF_R) != 0
    }

    /// Check if segment is writable
    pub fn is_writable(&self) -> bool {
        const PF_W: u32 = 0x2;
        (self.flags & PF_W) != 0
    }

    /// Check if segment is executable
    pub fn is_executable(&self) -> bool {
        const PF_X: u32 = 0x1;
        (self.flags & PF_X) != 0
    }
}

// ELF constants
pub const PT_LOAD: u32 = 1;
pub const PF_X: u32 = 0x1;
pub const PF_W: u32 = 0x2;
pub const PF_R: u32 = 0x4;

// Maximum string length for exec arguments (4KB)
const MAX_STRING_LENGTH: usize = 4096;

// Maximum number of arguments/environment variables
const MAX_ARG_COUNT: usize = 1024;

/// Validate a user space pointer
///
/// This function checks:
/// 1. Pointer is not NULL
/// 2. Pointer is in user space (below USER_LIMIT)
/// 3. Pointer does not point to kernel space
///
/// # Arguments
/// * `ptr` - Pointer to validate
///
/// # Returns
/// Ok(()) if pointer is valid, Err(ExecError::InvalidArgument) otherwise
///
/// # Requirements
/// Implements R8.1: Validate that the path pointer is in user space
pub fn validate_user_pointer(ptr: usize) -> Result<(), ExecError> {
    use crate::sched::task::USER_LIMIT;

    // Check for NULL pointer
    if ptr == 0 {
        return Err(ExecError::InvalidArgument);
    }

    // Check for kernel space addresses
    // User space is 0x0000_0000_0000_0001 to USER_LIMIT (exclusive)
    if ptr >= USER_LIMIT {
        return Err(ExecError::InvalidArgument);
    }

    Ok(())
}

/// Validate a user space pointer range
///
/// This function checks:
/// 1. Start pointer is valid
/// 2. Range does not overflow
/// 3. End of range is still in user space
///
/// # Arguments
/// * `ptr` - Start of memory range
/// * `len` - Length of memory range
///
/// # Returns
/// Ok(()) if range is valid, Err(ExecError::InvalidArgument) otherwise
pub fn validate_user_range(ptr: usize, len: usize) -> Result<(), ExecError> {
    use crate::sched::task::USER_LIMIT;

    // Validate start pointer
    validate_user_pointer(ptr)?;

    // Check for overflow
    let end = ptr.checked_add(len).ok_or(ExecError::InvalidArgument)?;

    // Check that end is still in user space
    if end > USER_LIMIT {
        return Err(ExecError::InvalidArgument);
    }

    Ok(())
}

/// Validate that a memory region is mapped and accessible
///
/// This function checks:
/// 1. Pointer is in user space
/// 2. Memory is mapped (page is present)
///
/// # Arguments
/// * `ptr` - Pointer to validate
/// * `len` - Length of memory region
///
/// # Returns
/// Ok(()) if memory is mapped, Err otherwise
///
/// # Requirements
/// Implements R8.3: Check for unmapped memory
pub fn validate_user_memory_mapped(ptr: usize, len: usize) -> Result<(), ExecError> {
    use crate::mm::paging::{PageMapper, PageTableFlags};
    use crate::mm::security::is_user_range;

    // First validate the range is in user space
    if !is_user_range(ptr, len) {
        return Err(ExecError::InvalidArgument);
    }

    // Walk each page in the range and ensure it's mapped for userspace
    let mapper = PageMapper::new();
    let page_size = 4096usize;
    let start_page = ptr & !(page_size - 1);
    let end_addr = ptr.checked_add(len).ok_or(ExecError::InvalidArgument)?;
    let end_page = end_addr
        .checked_add(page_size - 1)
        .ok_or(ExecError::InvalidArgument)?
        & !(page_size - 1);

    let mut current = start_page;
    while current < end_page {
        match mapper.get_page_flags(current) {
            Some(flags) => {
                if (flags.bits() & PageTableFlags::USER.bits()) == 0 {
                    return Err(ExecError::InvalidArgument);
                }
            }
            None => return Err(ExecError::InvalidArgument),
        }
        current += page_size;
    }

    Ok(())
}

/// Copy a null-terminated string from user space
///
/// This function:
/// 1. Validates the string pointer is in user space
/// 2. Finds the null terminator
/// 3. Enforces maximum string length
/// 4. Copies the string to kernel space
///
/// # Arguments
/// * `user_ptr` - Pointer to null-terminated string in user space
///
/// # Returns
/// Ok(String) containing the copied string, Err on validation failure
///
/// # Requirements
/// Implements R5.1, R5.5, R8.4: String copying with length limits
pub fn copy_string_from_user(user_ptr: usize) -> Result<String, ExecError> {
    // Validate pointer
    validate_user_pointer(user_ptr)?;

    // Find string length and validate each byte
    let mut len = 0;
    let mut current_ptr = user_ptr;

    while len < MAX_STRING_LENGTH {
        // Validate current pointer is still in user space
        validate_user_pointer(current_ptr)?;

        // Read byte (unsafe but validated)
        let byte = unsafe { *(current_ptr as *const u8) };

        // Check for null terminator
        if byte == 0 {
            break;
        }

        len += 1;
        current_ptr += 1;
    }

    // Check if we hit the limit without finding null terminator
    if len >= MAX_STRING_LENGTH {
        return Err(ExecError::ArgumentListTooLong);
    }

    // Allocate string and copy data
    let mut string = String::with_capacity(len);
    unsafe {
        let src = user_ptr as *const u8;
        for i in 0..len {
            string.push(*src.add(i) as char);
        }
    }

    Ok(string)
}

/// Copy an array of string pointers from user space
///
/// This function:
/// 1. Validates the array pointer is in user space
/// 2. Reads each string pointer from the array
/// 3. Validates each string pointer
/// 4. Copies each string to kernel space
/// 5. Stops at NULL pointer (array terminator)
///
/// # Arguments
/// * `array_ptr` - Pointer to array of string pointers (NULL-terminated)
///
/// # Returns
/// Ok(Vec<String>) containing all strings, Err on validation failure
///
/// # Requirements
/// Implements R8.2: Validation for string arrays (argv, envp)
pub fn copy_string_array_from_user(array_ptr: usize) -> Result<Vec<String>, ExecError> {
    // Handle NULL array pointer (means empty array)
    if array_ptr == 0 {
        return Ok(Vec::new());
    }

    // Validate array pointer
    validate_user_pointer(array_ptr)?;

    let mut strings = Vec::new();
    let mut index = 0;

    while index < MAX_ARG_COUNT {
        // Calculate pointer to current array element
        let element_ptr = array_ptr + (index * core::mem::size_of::<usize>());

        // Validate element pointer is in user space
        validate_user_pointer(element_ptr)?;

        // Read string pointer from array (unsafe but validated)
        let string_ptr = unsafe { *(element_ptr as *const usize) };

        // NULL pointer terminates the array
        if string_ptr == 0 {
            break;
        }

        // Validate and copy the string
        let string = copy_string_from_user(string_ptr)?;
        strings.push(string);

        index += 1;
    }

    // Check if we hit the limit without finding NULL terminator
    if index >= MAX_ARG_COUNT {
        return Err(ExecError::ArgumentListTooLong);
    }

    Ok(strings)
}

impl ExecContext {
    /// Setup new stack with argc, argv, and envp
    ///
    /// This method creates a new user stack and populates it with the program's
    /// arguments and environment variables according to the System V ABI specification.
    ///
    /// Stack layout (from high to low addresses):
    /// ```text
    /// 0x7FFF_FFFF_0000 (STACK_TOP)
    /// |  [environment strings]  |
    /// |  [argument strings]     |
    /// |  [padding for alignment]|
    /// |  [NULL]                 | <- envp array terminator
    /// |  [envp[n-1]]            |
    /// |  ...                    |
    /// |  [envp[0]]              |
    /// |  [NULL]                 | <- argv array terminator
    /// |  [argv[n-1]]            |
    /// |  ...                    |
    /// |  [argv[0]]              |
    /// |  [argc]                 | <- Stack pointer (RSP)
    /// ```
    ///
    /// # Arguments
    /// * `pmm` - Physical memory manager for allocating stack frames
    ///
    /// # Returns
    /// Ok(u64) containing the final stack pointer (RSP value), Err on failure
    ///
    /// # Errors
    /// * OutOfMemory - Failed to allocate physical memory for stack
    /// * ArgumentListTooLong - Too many arguments or environment variables
    ///
    /// # Requirements
    /// Implements R3.4, R3.5, R5.1-R5.8:
    /// - Allocate 8MB user stack at 0x7FFF_FFFF_0000
    /// - Copy environment strings to stack
    /// - Copy argument strings to stack
    /// - Build envp pointer array (NULL-terminated)
    /// - Build argv pointer array (NULL-terminated)
    /// - Push argc (argument count)
    /// - Align stack to 16-byte boundary
    /// - Return final stack pointer
    ///
    /// # Security
    /// - Stack is allocated with read/write permissions only (no execute)
    /// - All strings are copied to prevent TOCTOU attacks
    /// - Stack pointer is 16-byte aligned as required by x86_64 ABI
    pub fn setup_stack(
        &self,
        pmm: &mut crate::mm::pmm::PhysicalMemoryManager,
    ) -> Result<u64, ExecError> {
        use crate::mm::paging::{PageMapper, PageTableFlags};
        use crate::mm::phys_to_virt;
        use crate::sched::task::{MemoryRegion, MemoryRegionType};

        // Stack configuration
        const STACK_SIZE: usize = 8 * 1024 * 1024; // 8MB
        const STACK_TOP: u64 = 0x0000_7FFF_FFFF_0000;

        // Calculate stack bottom (start address)
        let stack_bottom = STACK_TOP - STACK_SIZE as u64;

        // Allocate and map stack pages
        let mut mapper = PageMapper::new();
        let stack_flags = PageTableFlags::PRESENT
            | PageTableFlags::WRITABLE
            | PageTableFlags::USER
            | PageTableFlags::NO_EXECUTE; // Stack should not be executable (W^X)

        // Map all pages for the stack
        let mut current_addr = stack_bottom as usize;
        let stack_end = STACK_TOP as usize;

        while current_addr < stack_end {
            // Allocate a physical frame
            let phys_frame = pmm.alloc_frame().ok_or(ExecError::OutOfMemory)?;

            // Map the virtual page to the physical frame
            mapper
                .map_page(current_addr, phys_frame, stack_flags, pmm)
                .map_err(|_| ExecError::OutOfMemory)?;

            // Zero the page for security
            let page_virt = phys_to_virt(phys_frame);
            unsafe {
                core::ptr::write_bytes(page_virt as *mut u8, 0, 4096);
            }

            current_addr += 4096;
        }

        // Track stack region in task's memory regions
        let stack_region = MemoryRegion::new(
            stack_bottom as usize,
            STACK_TOP as usize,
            stack_flags,
            MemoryRegionType::Stack,
        );

        unsafe {
            let task_ptr = Arc::as_ptr(&self.task) as *mut Task;
            (*task_ptr)
                .add_memory_region(stack_region)
                .map_err(|_| ExecError::OutOfMemory)?;
        }

        // Now build the stack contents
        // We'll work from the top down, pushing data onto the stack
        let mut sp = STACK_TOP;

        // Step 1: Copy environment strings to stack and collect their addresses
        let mut envp_addrs = Vec::with_capacity(self.envp.len());

        for env_str in self.envp.iter().rev() {
            // Push the string (including null terminator)
            let str_bytes = env_str.as_bytes();
            let str_len = str_bytes.len() + 1; // +1 for null terminator

            // Move stack pointer down to make room for string
            sp -= str_len as u64;

            // Copy string to stack
            unsafe {
                core::ptr::copy_nonoverlapping(str_bytes.as_ptr(), sp as *mut u8, str_bytes.len());
                // Add null terminator
                *((sp + str_bytes.len() as u64) as *mut u8) = 0;
            }

            // Save the address of this string
            envp_addrs.push(sp);
        }

        // Reverse envp_addrs since we pushed in reverse order
        envp_addrs.reverse();

        // Step 2: Copy argument strings to stack and collect their addresses
        let mut argv_addrs = Vec::with_capacity(self.argv.len());

        for arg_str in self.argv.iter().rev() {
            // Push the string (including null terminator)
            let str_bytes = arg_str.as_bytes();
            let str_len = str_bytes.len() + 1; // +1 for null terminator

            // Move stack pointer down to make room for string
            sp -= str_len as u64;

            // Copy string to stack
            unsafe {
                core::ptr::copy_nonoverlapping(str_bytes.as_ptr(), sp as *mut u8, str_bytes.len());
                // Add null terminator
                *((sp + str_bytes.len() as u64) as *mut u8) = 0;
            }

            // Save the address of this string
            argv_addrs.push(sp);
        }

        // Reverse argv_addrs since we pushed in reverse order
        argv_addrs.reverse();

        // Step 3: Align stack pointer to 16-byte boundary
        // This is required by the x86_64 System V ABI
        sp = sp & !0xF;

        // Step 4: Push envp array (NULL-terminated array of pointers)
        // First push NULL terminator
        sp -= 8;
        unsafe {
            *((sp) as *mut u64) = 0;
        }

        // Then push environment pointers in reverse order
        for &env_addr in envp_addrs.iter().rev() {
            sp -= 8;
            unsafe {
                *((sp) as *mut u64) = env_addr;
            }
        }

        // Save envp pointer for later (points to start of envp array)
        let envp_ptr = sp;

        // Step 5: Push argv array (NULL-terminated array of pointers)
        // First push NULL terminator
        sp -= 8;
        unsafe {
            *((sp) as *mut u64) = 0;
        }

        // Then push argument pointers in reverse order
        for &arg_addr in argv_addrs.iter().rev() {
            sp -= 8;
            unsafe {
                *((sp) as *mut u64) = arg_addr;
            }
        }

        // Save argv pointer for later (points to start of argv array)
        let argv_ptr = sp;

        // Step 6: Push argc (argument count)
        sp -= 8;
        unsafe {
            *((sp) as *mut u64) = self.argv.len() as u64;
        }

        // Step 7: Final alignment check
        // The stack pointer should be 16-byte aligned at this point
        // (required by x86_64 ABI before function calls)
        if sp & 0xF != 0 {
            // Adjust if needed
            sp = sp & !0xF;
        }

        // Log stack setup for debugging
        crate::serial_println!(
            "[EXEC] Stack setup complete: sp=0x{:016x}, argc={}, argv=0x{:016x}, envp=0x{:016x}",
            sp,
            self.argv.len(),
            argv_ptr,
            envp_ptr
        );

        // Return the final stack pointer
        // The program will start with:
        // - RSP pointing to argc
        // - RDI = argc (first argument)
        // - RSI = argv (second argument)
        // - RDX = envp (third argument)
        Ok(sp)
    }

    /// Load program segments from ELF into memory
    ///
    /// This method:
    /// 1. Iterates through PT_LOAD segments
    /// 2. Allocates memory at segment virtual addresses
    /// 3. Copies segment data from ELF file
    /// 4. Zeros BSS sections (memsz > filesz)
    /// 5. Sets memory permissions (read/write/execute)
    /// 6. Enforces W^X policy (no write+execute)
    ///
    /// # Arguments
    /// * `elf_info` - Parsed ELF information containing segments
    /// * `elf_data` - Raw ELF file data for copying segment contents
    /// * `pmm` - Physical memory manager for allocating frames
    ///
    /// # Returns
    /// Ok(()) if all segments loaded successfully, Err on failure
    ///
    /// # Errors
    /// * OutOfMemory - Failed to allocate physical memory for segments
    /// * InvalidArgument - Segment addresses overlap kernel space or violate W^X
    ///
    /// # Requirements
    /// Implements R2.5, R3.3, R8.5, R8.6:
    /// - Iterate through PT_LOAD segments
    /// - Allocate memory at segment virtual addresses
    /// - Copy segment data from ELF file
    /// - Zero BSS sections (memsz > filesz)
    /// - Set memory permissions (read, write, execute)
    /// - Enforce W^X policy (no write+execute)
    ///
    /// # Security
    /// - Enforces W^X (Write XOR Execute) policy: segments cannot be both
    ///   writable and executable. If a segment has both flags, the execute
    ///   permission is removed.
    /// - Validates all segment addresses are in user space
    /// - Prevents segments from overlapping kernel space
    pub fn load_segments(
        &self,
        elf_info: &ElfInfo,
        elf_data: &[u8],
        pmm: &mut crate::mm::pmm::PhysicalMemoryManager,
    ) -> Result<(), ExecError> {
        use crate::mm::paging::PageMapper;
        use crate::mm::phys_to_virt;
        use crate::sched::task::{MemoryRegion, USER_LIMIT};

        // Get the page mapper for the current address space
        let mut mapper = PageMapper::new();

        // Iterate through all PT_LOAD segments
        for segment in &elf_info.segments {
            // Only process PT_LOAD segments (already filtered in parse_elf)
            if segment.seg_type != PT_LOAD {
                continue;
            }

            // Validate segment is in user space
            if segment.vaddr >= USER_LIMIT as u64 {
                return Err(ExecError::InvalidArgument);
            }

            // Validate segment end is in user space
            let segment_end = segment
                .vaddr
                .checked_add(segment.memsz)
                .ok_or(ExecError::InvalidArgument)?;

            if segment_end > USER_LIMIT as u64 {
                return Err(ExecError::InvalidArgument);
            }

            // Convert segment flags to page table flags
            let page_flags = self.segment_flags_to_page_flags(segment.flags)?;

            // Calculate page-aligned start and end addresses
            let start_page = (segment.vaddr as usize) & !0xFFF; // Align down to page boundary
            let end_page = ((segment.vaddr as usize + segment.memsz as usize) + 0xFFF) & !0xFFF; // Align up

            // Allocate and map pages for the entire segment
            let mut current_addr = start_page;
            while current_addr < end_page {
                // Allocate a physical frame
                let phys_frame = pmm.alloc_frame().ok_or(ExecError::OutOfMemory)?;

                // Map the virtual page to the physical frame
                mapper
                    .map_page(current_addr, phys_frame, page_flags, pmm)
                    .map_err(|_| ExecError::OutOfMemory)?;

                // Zero the entire page for security
                // This ensures no data from previous processes leaks
                let page_virt = phys_to_virt(phys_frame);
                unsafe {
                    core::ptr::write_bytes(page_virt as *mut u8, 0, 4096);
                }

                current_addr += 4096;
            }

            // Copy segment data from ELF file (if filesz > 0)
            if segment.filesz > 0 {
                // Validate offset and size are within ELF file bounds
                let file_start = segment.offset as usize;
                let file_end = file_start
                    .checked_add(segment.filesz as usize)
                    .ok_or(ExecError::InvalidFormat)?;

                if file_end > elf_data.len() {
                    return Err(ExecError::InvalidFormat);
                }

                // Get source data from ELF file
                let src_data = &elf_data[file_start..file_end];

                // Copy data to the mapped virtual address
                // We need to copy byte by byte because the segment might not be page-aligned
                let dest_addr = segment.vaddr as usize;
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        src_data.as_ptr(),
                        dest_addr as *mut u8,
                        segment.filesz as usize,
                    );
                }
            }

            // Zero BSS section (memsz > filesz)
            // BSS is the uninitialized data section that should be zeroed
            if segment.memsz > segment.filesz {
                let bss_start = (segment.vaddr + segment.filesz) as usize;
                let bss_size = (segment.memsz - segment.filesz) as usize;

                // Zero the BSS section
                unsafe {
                    core::ptr::write_bytes(bss_start as *mut u8, 0, bss_size);
                }
            }

            // Track this memory region in the task
            // This allows us to manage and clean up the memory later
            let region = MemoryRegion::new(
                segment.vaddr as usize,
                (segment.vaddr + segment.memsz) as usize,
                page_flags,
                self.segment_type_from_flags(segment.flags),
            );

            // Add region to task's memory tracking
            // We need unsafe here to get mutable access through Arc
            unsafe {
                let task_ptr = Arc::as_ptr(&self.task) as *mut Task;
                (*task_ptr)
                    .add_memory_region(region)
                    .map_err(|_| ExecError::OutOfMemory)?;
            }
        }

        Ok(())
    }

    /// Convert ELF segment flags to page table flags
    ///
    /// This method enforces the W^X (Write XOR Execute) security policy:
    /// - If a segment is both writable and executable, the execute permission is removed
    /// - This prevents code injection attacks
    ///
    /// # Arguments
    /// * `elf_flags` - ELF segment flags (PF_R, PF_W, PF_X)
    ///
    /// # Returns
    /// Ok(PageTableFlags) with appropriate flags set, Err if invalid
    ///
    /// # Security
    /// Implements R8.6: Enforce W^X policy (no write+execute)
    fn segment_flags_to_page_flags(
        &self,
        elf_flags: u32,
    ) -> Result<crate::mm::paging::PageTableFlags, ExecError> {
        use crate::mm::paging::PageTableFlags;

        // Start with base flags: PRESENT and USER
        let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER;

        // Check readable flag (PF_R = 0x4)
        // Note: x86_64 doesn't have a separate read flag, pages are readable if present
        let _is_readable = (elf_flags & PF_R) != 0;

        // Check writable flag (PF_W = 0x2)
        let is_writable = (elf_flags & PF_W) != 0;
        if is_writable {
            flags = flags | PageTableFlags::WRITABLE;
        }

        // Check executable flag (PF_X = 0x1)
        let is_executable = (elf_flags & PF_X) != 0;

        // Enforce W^X policy: cannot be both writable and executable
        // If both flags are set, we remove the execute permission
        if is_writable && is_executable {
            // Remove execute permission by setting NO_EXECUTE
            flags = flags | PageTableFlags::NO_EXECUTE;

            // Log a warning about W^X violation
            crate::serial_println!(
                "[EXEC] W^X violation detected - removing execute permission from writable segment"
            );
        } else if !is_executable {
            // If not executable, set NO_EXECUTE flag
            flags = flags | PageTableFlags::NO_EXECUTE;
        }
        // If executable but not writable, don't set NO_EXECUTE (allow execution)

        Ok(flags)
    }

    /// Determine memory region type from ELF segment flags
    ///
    /// This is a heuristic to classify segments into region types:
    /// - Executable + Readable = Code
    /// - Writable + Readable = Data
    /// - Otherwise = Data (default)
    ///
    /// # Arguments
    /// * `elf_flags` - ELF segment flags (PF_R, PF_W, PF_X)
    ///
    /// # Returns
    /// MemoryRegionType classification
    fn segment_type_from_flags(&self, elf_flags: u32) -> crate::sched::task::MemoryRegionType {
        use crate::sched::task::MemoryRegionType;

        let is_readable = (elf_flags & PF_R) != 0;
        let is_writable = (elf_flags & PF_W) != 0;
        let is_executable = (elf_flags & PF_X) != 0;

        // Classify based on flags
        if is_executable && is_readable && !is_writable {
            MemoryRegionType::Code
        } else if is_writable && is_readable {
            MemoryRegionType::Data
        } else {
            // Default to Data for other combinations
            MemoryRegionType::Data
        }
    }

    /// Close file descriptors marked with O_CLOEXEC flag
    ///
    /// This method iterates through the process's file descriptor table and closes
    /// all file descriptors that have the O_CLOEXEC (close-on-exec) flag set.
    ///
    /// Standard file descriptors (stdin/stdout/stderr - FDs 0, 1, 2) are preserved
    /// regardless of their flags, as they are typically inherited across exec().
    /// Other file descriptors are preserved only if they don't have O_CLOEXEC set.
    ///
    /// # Returns
    /// Ok(()) if successful, Err on failure
    ///
    /// # Requirements
    /// Implements R6.1, R6.2, R6.3, R6.4:
    /// - R6.1: Preserve all open file descriptors by default
    /// - R6.2: Close file descriptors with O_CLOEXEC flag set
    /// - R6.3: Maintain file positions and flags for preserved FDs
    /// - R6.4: Preserve stdin/stdout/stderr (FDs 0, 1, 2)
    ///
    /// # Security
    /// - Prevents file descriptor leaks to new programs
    /// - Allows programs to control which FDs are inherited
    /// - Maintains process isolation
    ///
    /// # Example
    /// ```
    /// // Before exec():
    /// // FD 0: stdin (no O_CLOEXEC) -> preserved
    /// // FD 1: stdout (no O_CLOEXEC) -> preserved
    /// // FD 2: stderr (no O_CLOEXEC) -> preserved
    /// // FD 3: file.txt (no O_CLOEXEC) -> preserved
    /// // FD 4: secret.key (O_CLOEXEC) -> closed
    /// // FD 5: log.txt (O_CLOEXEC) -> closed
    ///
    /// // After exec():
    /// // FD 0: stdin (preserved)
    /// // FD 1: stdout (preserved)
    /// // FD 2: stderr (preserved)
    /// // FD 3: file.txt (preserved)
    /// // FD 4: closed
    /// // FD 5: closed
    /// ```
    pub fn close_cloexec_fds(&self) -> Result<(), ExecError> {
        // Get the task's file descriptor table
        // The FdTable is wrapped in Arc<SpinLock<...>> for thread-safety
        let mut fd_table = self.task.fd_table.lock();

        // Close all file descriptors with O_CLOEXEC flag set
        // The FdTable::close_cloexec_fds() method handles the iteration
        // and closing logic, preserving all FDs without the flag
        fd_table.close_cloexec_fds();

        // Log the operation for debugging
        crate::serial_println!(
            "[EXEC] Closed O_CLOEXEC file descriptors, {} FDs remain open",
            fd_table.count()
        );

        Ok(())
    }

    /// Jump to userspace and start executing the new program
    ///
    /// This method performs the final step of exec() by transitioning from kernel
    /// mode to user mode and starting execution at the new program's entry point.
    ///
    /// This method:
    /// 1. Updates task instruction pointer (RIP) to entry point
    /// 2. Updates task stack pointer (RSP) to new stack
    /// 3. Sets up initial register state (argc in RDI, argv in RSI, envp in RDX)
    /// 4. Switches to user mode (ring 3)
    /// 5. Executes sysretq to jump to new program
    ///
    /// # Arguments
    /// * `entry_point` - Virtual address of the program's entry point
    /// * `stack_pointer` - Virtual address of the top of the new stack
    ///
    /// # Returns
    /// Never returns on success (execution continues at new program)
    ///
    /// # Requirements
    /// Implements R1.4, R3.6:
    /// - R1.4: When exec() succeeds, the system shall not return to the caller
    /// - R3.6: Update the process's instruction pointer to the new entry point
    ///
    /// # Security
    /// - Validates entry point and stack pointer are in user space
    /// - Clears all registers except argc/argv/envp to prevent information leakage
    /// - Uses sysretq for safe transition to user mode
    ///
    /// # Safety
    /// This function is unsafe because it:
    /// - Directly manipulates CPU registers
    /// - Changes privilege level from ring 0 to ring 3
    /// - Never returns (changes execution flow)
    ///
    /// # Notes
    /// According to the System V ABI for x86_64:
    /// - RDI = argc (first argument)
    /// - RSI = argv (second argument)
    /// - RDX = envp (third argument)
    /// - Stack must be 16-byte aligned
    /// - RBP should be 0 (no previous frame)
    ///
    /// The program entry point expects:
    /// ```c
    /// void _start(int argc, char **argv, char **envp);
    /// ```
    pub fn jump_to_userspace(&self, entry_point: u64, stack_pointer: u64) -> ! {
        use crate::sched::task::USER_LIMIT;

        // Validate entry point is in user space
        if entry_point == 0 || entry_point >= USER_LIMIT as u64 {
            panic!("[EXEC] Invalid entry point: 0x{:016x}", entry_point);
        }

        // Validate stack pointer is in user space
        if stack_pointer == 0 || stack_pointer >= USER_LIMIT as u64 {
            panic!("[EXEC] Invalid stack pointer: 0x{:016x}", stack_pointer);
        }

        // Calculate argc, argv, and envp from the stack
        // The stack layout we created in setup_stack() is:
        // [argc] [argv array] [NULL] [envp array] [NULL] [strings...]
        //  ^sp
        //
        // So:
        // - argc is at stack_pointer
        // - argv is at stack_pointer + 8
        // - envp is at stack_pointer + 8 + (argc + 1) * 8

        let argc = self.argv.len() as u64;
        let argv_ptr = stack_pointer + 8; // Points to start of argv array
        let envp_ptr = stack_pointer + 8 + ((argc + 1) * 8); // Points to start of envp array

        crate::serial_println!(
            "[EXEC] Jumping to userspace: entry=0x{:016x}, sp=0x{:016x}",
            entry_point,
            stack_pointer
        );
        crate::serial_println!(
            "[EXEC] Initial registers: argc={}, argv=0x{:016x}, envp=0x{:016x}",
            argc,
            argv_ptr,
            envp_ptr
        );

        // Cache user-mode stack pointer and mark task as userspace before transition
        unsafe {
            let task_ptr = Arc::as_ptr(&self.task) as *mut Task;
            (*task_ptr).user_stack_pointer = stack_pointer;
            (*task_ptr).creds.is_kernel_thread = false;
        }

        // Perform the jump to userspace using sysretq
        // This is done in inline assembly because we need precise control
        // over register state and privilege level transition
        unsafe {
            core::arch::asm!(
                // Set up user code and data segments for sysretq
                // STAR MSR configures these, but we need to ensure they're correct

                // Set up registers for the new program
                // According to System V ABI:
                // RDI = argc, RSI = argv, RDX = envp
                "mov rdi, {argc}",           // argc in RDI
                "mov rsi, {argv}",           // argv in RSI
                "mov rdx, {envp}",           // envp in RDX

                // Set up RCX and R11 for sysretq
                // sysretq uses these registers:
                // RCX = user RIP (entry point)
                // R11 = user RFLAGS (with IF=1 for interrupts enabled)
                "mov rcx, {entry}",          // Entry point in RCX
                "mov r11, 0x202",            // RFLAGS: IF=1 (bit 9), reserved bit 1 always set

                // Set up user stack pointer
                // We'll use RSP directly since sysretq doesn't touch it
                "mov rsp, {stack}",          // User stack pointer

                // Clear frame pointer (no previous frame)
                "xor rbp, rbp",

                // Clear other registers for security (prevent information leakage)
                "xor rax, rax",
                "xor rbx, rbx",
                "xor r8, r8",
                "xor r9, r9",
                "xor r10, r10",
                "xor r12, r12",
                "xor r13, r13",
                "xor r14, r14",
                "xor r15, r15",

                // Switch to user GS base (if we were using kernel GS)
                // This is needed if we came from a syscall context
                // Check if we need to swap by testing if we're using kernel GS
                // For now, we'll always swap to be safe
                "swapgs",

                // Execute sysretq to jump to user mode
                // This will:
                // - Load CS from STAR[63:48] + 16 (user code segment with RPL=3)
                // - Load SS from STAR[63:48] + 8 (user data segment with RPL=3)
                // - Load RIP from RCX (entry point)
                // - Load RFLAGS from R11 (with IF=1)
                // - Set CPL=3 (user mode)
                "sysretq",

                entry = in(reg) entry_point,
                stack = in(reg) stack_pointer,
                argc = in(reg) argc,
                argv = in(reg) argv_ptr,
                envp = in(reg) envp_ptr,
                options(noreturn)
            );
        }
    }
}
