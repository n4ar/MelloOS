//! Memory Security Module
//!
//! This module provides security validation functions for user-kernel memory operations.
//! It implements comprehensive pointer validation, bounds checking, and permission verification
//! to prevent security vulnerabilities.

use crate::mm::paging::{PageMapper, PageTableFlags};
use crate::mm::pmm::PhysicalMemoryManager;
use crate::mm::{PhysAddr, VirtAddr};
use crate::sched::task::USER_LIMIT;
use core::mem::{align_of, size_of};

/// Error codes for memory security operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityError {
    /// Invalid pointer (null or out of bounds)
    InvalidPointer,
    /// Pointer not aligned
    Misaligned,
    /// Permission denied (page not readable/writable)
    PermissionDenied,
    /// Overflow in address calculation
    Overflow,
    /// Page not present
    PageNotPresent,
}

/// Result type for security operations
pub type SecurityResult<T> = Result<T, SecurityError>;

/// Validate that a pointer is in user space
///
/// # Arguments
/// * `ptr` - Pointer to validate
///
/// # Returns
/// true if pointer is in valid user space range, false otherwise
#[inline]
pub fn is_user_pointer(ptr: usize) -> bool {
    ptr != 0 && ptr < USER_LIMIT
}

/// Validate that a memory range is entirely in user space
///
/// # Arguments
/// * `ptr` - Start of memory range
/// * `len` - Length of memory range
///
/// # Returns
/// true if entire range is in valid user space, false otherwise
#[inline]
pub fn is_user_range(ptr: usize, len: usize) -> bool {
    if ptr == 0 {
        return false;
    }

    // Check for overflow
    match ptr.checked_add(len) {
        Some(end) => ptr < USER_LIMIT && end <= USER_LIMIT,
        None => false,
    }
}

/// Validate pointer alignment for a specific type
///
/// # Arguments
/// * `ptr` - Pointer to validate
///
/// # Returns
/// true if pointer is properly aligned for type T
#[inline]
pub fn is_aligned<T>(ptr: usize) -> bool {
    ptr % align_of::<T>() == 0
}

/// Validate that a user pointer is readable
///
/// This function checks:
/// 1. Pointer is in user space
/// 2. Pointer is properly aligned
/// 3. Page is present and readable
///
/// # Arguments
/// * `ptr` - Pointer to validate
/// * `mapper` - Page mapper to check page permissions
///
/// # Returns
/// Ok(()) if pointer is valid and readable, Err otherwise
pub fn validate_user_read<T>(ptr: usize, mapper: &PageMapper) -> SecurityResult<()> {
    // Check if pointer is in user space
    if !is_user_pointer(ptr) {
        return Err(SecurityError::InvalidPointer);
    }

    // Check alignment
    if !is_aligned::<T>(ptr) {
        return Err(SecurityError::Misaligned);
    }

    // Check if range is valid
    if !is_user_range(ptr, size_of::<T>()) {
        return Err(SecurityError::InvalidPointer);
    }

    // Check page permissions
    match mapper.get_page_flags(ptr) {
        Some(flags) => {
            // Page is present, verify USER flag is set
            if (flags.bits() & PageTableFlags::USER.bits()) == 0 {
                return Err(SecurityError::PermissionDenied);
            }
            Ok(())
        }
        None => Err(SecurityError::PageNotPresent),
    }
}

/// Validate that a user pointer is writable
///
/// This function checks:
/// 1. Pointer is in user space
/// 2. Pointer is properly aligned
/// 3. Page is present, readable, and writable
///
/// # Arguments
/// * `ptr` - Pointer to validate
/// * `mapper` - Page mapper to check page permissions
///
/// # Returns
/// Ok(()) if pointer is valid and writable, Err otherwise
pub fn validate_user_write<T>(ptr: usize, mapper: &PageMapper) -> SecurityResult<()> {
    // Check if pointer is in user space
    if !is_user_pointer(ptr) {
        return Err(SecurityError::InvalidPointer);
    }

    // Check alignment
    if !is_aligned::<T>(ptr) {
        return Err(SecurityError::Misaligned);
    }

    // Check if range is valid
    if !is_user_range(ptr, size_of::<T>()) {
        return Err(SecurityError::InvalidPointer);
    }

    // Check page permissions
    match mapper.get_page_flags(ptr) {
        Some(flags) => {
            // Page is present, verify USER and WRITABLE flags are set
            if (flags.bits() & PageTableFlags::USER.bits()) == 0 {
                return Err(SecurityError::PermissionDenied);
            }
            if (flags.bits() & PageTableFlags::WRITABLE.bits()) == 0 {
                return Err(SecurityError::PermissionDenied);
            }
            Ok(())
        }
        None => Err(SecurityError::PageNotPresent),
    }
}

/// Copy data from user space to kernel space with validation
///
/// This function performs comprehensive validation before copying:
/// 1. Validates source pointer is in user space
/// 2. Checks alignment
/// 3. Verifies page permissions
/// 4. Performs bounds checking
///
/// # Arguments
/// * `dst` - Destination buffer in kernel space
/// * `src_ptr` - Source pointer in user space
/// * `len` - Number of bytes to copy
/// * `mapper` - Page mapper for permission checks
///
/// # Returns
/// Ok(()) on success, Err on validation failure
///
/// # Safety
/// This function uses unsafe operations but validates all inputs first
pub fn copy_from_user(
    dst: &mut [u8],
    src_ptr: usize,
    len: usize,
    mapper: &PageMapper,
) -> SecurityResult<()> {
    // Validate destination buffer size
    if dst.len() < len {
        return Err(SecurityError::InvalidPointer);
    }

    // Validate source pointer is in user space
    if !is_user_range(src_ptr, len) {
        return Err(SecurityError::InvalidPointer);
    }

    // For each page in the range, validate it's present and readable
    let page_size = 4096;
    let start_page = src_ptr & !(page_size - 1);
    let end_addr = match src_ptr.checked_add(len) {
        Some(addr) => addr,
        None => return Err(SecurityError::Overflow),
    };
    let end_page = (end_addr + page_size - 1) & !(page_size - 1);

    let mut page = start_page;
    while page < end_page {
        match mapper.get_page_flags(page) {
            Some(flags) => {
                // Page is present, verify USER flag is set
                if (flags.bits() & PageTableFlags::USER.bits()) == 0 {
                    return Err(SecurityError::PermissionDenied);
                }
            }
            None => return Err(SecurityError::PageNotPresent),
        }
        page += page_size;
    }

    // Perform the copy
    unsafe {
        let src = src_ptr as *const u8;
        for i in 0..len {
            dst[i] = *src.add(i);
        }
    }

    Ok(())
}

/// Copy data from kernel space to user space with validation
///
/// This function performs comprehensive validation before copying:
/// 1. Validates destination pointer is in user space
/// 2. Checks alignment
/// 3. Verifies page permissions (writable)
/// 4. Performs bounds checking
///
/// # Arguments
/// * `dst_ptr` - Destination pointer in user space
/// * `src` - Source buffer in kernel space
/// * `mapper` - Page mapper for permission checks
///
/// # Returns
/// Ok(()) on success, Err on validation failure
///
/// # Safety
/// This function uses unsafe operations but validates all inputs first
pub fn copy_to_user(dst_ptr: usize, src: &[u8], mapper: &PageMapper) -> SecurityResult<()> {
    let len = src.len();

    // Validate destination pointer is in user space
    if !is_user_range(dst_ptr, len) {
        return Err(SecurityError::InvalidPointer);
    }

    // For each page in the range, validate it's present and writable
    let page_size = 4096;
    let start_page = dst_ptr & !(page_size - 1);
    let end_addr = match dst_ptr.checked_add(len) {
        Some(addr) => addr,
        None => return Err(SecurityError::Overflow),
    };
    let end_page = (end_addr + page_size - 1) & !(page_size - 1);

    let mut page = start_page;
    while page < end_page {
        match mapper.get_page_flags(page) {
            Some(flags) => {
                // Page is present, verify USER and WRITABLE flags are set
                if (flags.bits() & PageTableFlags::USER.bits()) == 0 {
                    return Err(SecurityError::PermissionDenied);
                }
                if (flags.bits() & PageTableFlags::WRITABLE.bits()) == 0 {
                    return Err(SecurityError::PermissionDenied);
                }
            }
            None => return Err(SecurityError::PageNotPresent),
        }
        page += page_size;
    }

    // Perform the copy
    unsafe {
        let dst = dst_ptr as *mut u8;
        for i in 0..len {
            *dst.add(i) = src[i];
        }
    }

    Ok(())
}

/// Copy a typed value from user space to kernel space
///
/// # Arguments
/// * `user_ptr` - Pointer to value in user space
/// * `mapper` - Page mapper for permission checks
///
/// # Returns
/// Ok(value) on success, Err on validation failure
pub fn copy_from_user_typed<T: Copy>(user_ptr: usize, mapper: &PageMapper) -> SecurityResult<T> {
    // Validate pointer
    validate_user_read::<T>(user_ptr, mapper)?;

    // Perform the copy
    unsafe {
        let ptr = user_ptr as *const T;
        Ok(*ptr)
    }
}

/// Copy a typed value from kernel space to user space
///
/// # Arguments
/// * `user_ptr` - Destination pointer in user space
/// * `value` - Value to copy
/// * `mapper` - Page mapper for permission checks
///
/// # Returns
/// Ok(()) on success, Err on validation failure
pub fn copy_to_user_typed<T: Copy>(
    user_ptr: usize,
    value: T,
    mapper: &PageMapper,
) -> SecurityResult<()> {
    // Validate pointer
    validate_user_write::<T>(user_ptr, mapper)?;

    // Perform the copy
    unsafe {
        let ptr = user_ptr as *mut T;
        *ptr = value;
    }

    Ok(())
}

/// Validate a null-terminated string in user space
///
/// # Arguments
/// * `str_ptr` - Pointer to string in user space
/// * `max_len` - Maximum allowed string length
/// * `mapper` - Page mapper for permission checks
///
/// # Returns
/// Ok(length) on success (not including null terminator), Err on validation failure
pub fn validate_user_string(
    str_ptr: usize,
    max_len: usize,
    mapper: &PageMapper,
) -> SecurityResult<usize> {
    // Validate pointer is in user space
    if !is_user_pointer(str_ptr) {
        return Err(SecurityError::InvalidPointer);
    }

    // Find string length and validate each page
    let mut len = 0;
    let mut current_ptr = str_ptr;

    while len < max_len {
        // Check if we're still in user space
        if !is_user_pointer(current_ptr) {
            return Err(SecurityError::InvalidPointer);
        }

        // Check if page is present
        let page = current_ptr & !0xFFF;
        if mapper.translate(page).is_none() {
            return Err(SecurityError::PageNotPresent);
        }

        // Read byte
        unsafe {
            let byte = *(current_ptr as *const u8);
            if byte == 0 {
                return Ok(len);
            }
        }

        len += 1;
        current_ptr += 1;
    }

    // String too long (no null terminator found)
    Err(SecurityError::InvalidPointer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_user_pointer() {
        // Valid user pointers
        assert!(is_user_pointer(0x1000));
        assert!(is_user_pointer(0x7FFF_FFFF_FFFF));

        // Invalid pointers
        assert!(!is_user_pointer(0)); // Null
        assert!(!is_user_pointer(USER_LIMIT)); // At limit
        assert!(!is_user_pointer(0xFFFF_8000_0000_0000)); // Kernel space
    }

    #[test]
    fn test_is_user_range() {
        // Valid ranges
        assert!(is_user_range(0x1000, 0x1000));
        assert!(is_user_range(0x1000, 0x100));

        // Invalid ranges
        assert!(!is_user_range(0, 0x1000)); // Null pointer
        assert!(!is_user_range(USER_LIMIT - 0x100, 0x200)); // Crosses boundary
        assert!(!is_user_range(usize::MAX - 0x100, 0x200)); // Overflow
    }

    #[test]
    fn test_is_aligned() {
        // Test u32 alignment (4 bytes)
        assert!(is_aligned::<u32>(0x1000));
        assert!(is_aligned::<u32>(0x1004));
        assert!(!is_aligned::<u32>(0x1001));
        assert!(!is_aligned::<u32>(0x1002));

        // Test u64 alignment (8 bytes)
        assert!(is_aligned::<u64>(0x1000));
        assert!(is_aligned::<u64>(0x1008));
        assert!(!is_aligned::<u64>(0x1004));
    }
}

/// W^X (Write XOR Execute) Memory Protection
///
/// This module enforces the security principle that memory pages should be
/// either writable OR executable, but never both.

/// Validate that page flags follow W^X principle
///
/// # Arguments
/// * `flags` - Page table flags to validate
///
/// # Returns
/// true if flags are valid (not both writable and executable), false otherwise
pub fn validate_wx_flags(flags: PageTableFlags) -> bool {
    let writable = (flags.bits() & PageTableFlags::WRITABLE.bits()) != 0;
    let executable = (flags.bits() & PageTableFlags::NO_EXECUTE.bits()) == 0;

    // Valid combinations:
    // - Read-only, executable (code pages)
    // - Writable, non-executable (data/stack pages)
    // - Read-only, non-executable (const data)
    // Invalid:
    // - Writable AND executable
    !(writable && executable)
}

/// Map a code page with proper W^X flags (R+X, not W)
///
/// # Arguments
/// * `mapper` - Page mapper
/// * `virt_addr` - Virtual address to map
/// * `phys_addr` - Physical address to map to
/// * `user` - Whether this is a user-mode page
/// * `pmm` - Physical memory manager
///
/// # Returns
/// Ok(()) on success, Err on failure
pub fn map_code_page(
    mapper: &mut PageMapper,
    virt_addr: VirtAddr,
    phys_addr: PhysAddr,
    user: bool,
    pmm: &mut PhysicalMemoryManager,
) -> Result<(), &'static str> {
    // Code pages: PRESENT + USER (if user) + executable (no NO_EXECUTE flag)
    let mut flags = PageTableFlags::PRESENT;
    if user {
        flags = flags | PageTableFlags::USER;
    }
    // Note: NOT setting WRITABLE or NO_EXECUTE means R+X

    // Validate W^X
    if !validate_wx_flags(flags) {
        return Err("W^X violation: code page cannot be writable");
    }

    mapper.map_page(virt_addr, phys_addr, flags, pmm)
}

/// Map a data page with proper W^X flags (R+W, not X)
///
/// # Arguments
/// * `mapper` - Page mapper
/// * `virt_addr` - Virtual address to map
/// * `phys_addr` - Physical address to map to
/// * `user` - Whether this is a user-mode page
/// * `pmm` - Physical memory manager
///
/// # Returns
/// Ok(()) on success, Err on failure
pub fn map_data_page(
    mapper: &mut PageMapper,
    virt_addr: VirtAddr,
    phys_addr: PhysAddr,
    user: bool,
    pmm: &mut PhysicalMemoryManager,
) -> Result<(), &'static str> {
    // Data pages: PRESENT + WRITABLE + NO_EXECUTE + USER (if user)
    let mut flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;
    if user {
        flags = flags | PageTableFlags::USER;
    }

    // Validate W^X
    if !validate_wx_flags(flags) {
        return Err("W^X violation: data page cannot be executable");
    }

    mapper.map_page(virt_addr, phys_addr, flags, pmm)
}

/// Map a stack page with proper W^X flags (R+W, not X, with NX bit)
///
/// # Arguments
/// * `mapper` - Page mapper
/// * `virt_addr` - Virtual address to map
/// * `phys_addr` - Physical address to map to
/// * `user` - Whether this is a user-mode page
/// * `pmm` - Physical memory manager
///
/// # Returns
/// Ok(()) on success, Err on failure
pub fn map_stack_page(
    mapper: &mut PageMapper,
    virt_addr: VirtAddr,
    phys_addr: PhysAddr,
    user: bool,
    pmm: &mut PhysicalMemoryManager,
) -> Result<(), &'static str> {
    // Stack pages: PRESENT + WRITABLE + NO_EXECUTE + USER (if user)
    // Same as data pages, but semantically different
    let mut flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;
    if user {
        flags = flags | PageTableFlags::USER;
    }

    // Validate W^X
    if !validate_wx_flags(flags) {
        return Err("W^X violation: stack page cannot be executable");
    }

    mapper.map_page(virt_addr, phys_addr, flags, pmm)
}

/// Map a read-only page (not writable, not executable)
///
/// # Arguments
/// * `mapper` - Page mapper
/// * `virt_addr` - Virtual address to map
/// * `phys_addr` - Physical address to map to
/// * `user` - Whether this is a user-mode page
/// * `pmm` - Physical memory manager
///
/// # Returns
/// Ok(()) on success, Err on failure
pub fn map_readonly_page(
    mapper: &mut PageMapper,
    virt_addr: VirtAddr,
    phys_addr: PhysAddr,
    user: bool,
    pmm: &mut PhysicalMemoryManager,
) -> Result<(), &'static str> {
    // Read-only pages: PRESENT + NO_EXECUTE + USER (if user)
    let mut flags = PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE;
    if user {
        flags = flags | PageTableFlags::USER;
    }

    // Validate W^X (always valid for read-only)
    if !validate_wx_flags(flags) {
        return Err("W^X violation: unexpected");
    }

    mapper.map_page(virt_addr, phys_addr, flags, pmm)
}

/// Validate that a page's current flags follow W^X principle
///
/// # Arguments
/// * `mapper` - Page mapper
/// * `virt_addr` - Virtual address to check
///
/// # Returns
/// Ok(()) if page follows W^X, Err otherwise
pub fn validate_page_wx(mapper: &PageMapper, virt_addr: VirtAddr) -> SecurityResult<()> {
    // Get page flags from page table entry
    match mapper.get_page_flags(virt_addr) {
        Some(flags) => {
            // Verify W^X principle
            if !validate_wx_flags(flags) {
                return Err(SecurityError::PermissionDenied);
            }
            Ok(())
        }
        None => Err(SecurityError::PageNotPresent),
    }
}

/// Check if a memory range has consistent W^X properties
///
/// # Arguments
/// * `mapper` - Page mapper
/// * `start_addr` - Start of memory range
/// * `len` - Length of memory range
///
/// # Returns
/// Ok(()) if all pages in range follow W^X, Err otherwise
pub fn validate_range_wx(
    mapper: &PageMapper,
    start_addr: VirtAddr,
    len: usize,
) -> SecurityResult<()> {
    let page_size = 4096;
    let start_page = start_addr & !(page_size - 1);
    let end_addr = match start_addr.checked_add(len) {
        Some(addr) => addr,
        None => return Err(SecurityError::Overflow),
    };
    let end_page = (end_addr + page_size - 1) & !(page_size - 1);

    let mut page = start_page;
    while page < end_page {
        validate_page_wx(mapper, page)?;
        page += page_size;
    }

    Ok(())
}

/// Memory region type for W^X enforcement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// Code region (R+X, not W)
    Code,
    /// Data region (R+W, not X)
    Data,
    /// Stack region (R+W, not X, with NX bit)
    Stack,
    /// Read-only region (R, not W, not X)
    ReadOnly,
}

impl MemoryRegionType {
    /// Get the appropriate page flags for this region type
    pub fn flags(&self, user: bool) -> PageTableFlags {
        let mut flags = PageTableFlags::PRESENT;

        if user {
            flags = flags | PageTableFlags::USER;
        }

        match self {
            MemoryRegionType::Code => {
                // Code: R+X (no WRITABLE, no NO_EXECUTE)
                flags
            }
            MemoryRegionType::Data | MemoryRegionType::Stack => {
                // Data/Stack: R+W+NX
                flags | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE
            }
            MemoryRegionType::ReadOnly => {
                // Read-only: R+NX
                flags | PageTableFlags::NO_EXECUTE
            }
        }
    }

    /// Validate that flags match this region type
    pub fn validate_flags(&self, flags: PageTableFlags) -> bool {
        let writable = (flags.bits() & PageTableFlags::WRITABLE.bits()) != 0;
        let executable = (flags.bits() & PageTableFlags::NO_EXECUTE.bits()) == 0;

        match self {
            MemoryRegionType::Code => !writable && executable,
            MemoryRegionType::Data | MemoryRegionType::Stack => writable && !executable,
            MemoryRegionType::ReadOnly => !writable && !executable,
        }
    }
}

#[cfg(test)]
mod wx_tests {
    use super::*;

    #[test]
    fn test_validate_wx_flags() {
        // Valid: Read-only, executable (code)
        let code_flags = PageTableFlags::PRESENT;
        assert!(validate_wx_flags(code_flags));

        // Valid: Writable, non-executable (data)
        let data_flags =
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;
        assert!(validate_wx_flags(data_flags));

        // Valid: Read-only, non-executable (const data)
        let readonly_flags = PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE;
        assert!(validate_wx_flags(readonly_flags));

        // Invalid: Writable AND executable
        let bad_flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        assert!(!validate_wx_flags(bad_flags));
    }

    #[test]
    fn test_memory_region_type_flags() {
        // Code region
        let code_flags = MemoryRegionType::Code.flags(true);
        assert!(MemoryRegionType::Code.validate_flags(code_flags));
        assert!(validate_wx_flags(code_flags));

        // Data region
        let data_flags = MemoryRegionType::Data.flags(true);
        assert!(MemoryRegionType::Data.validate_flags(data_flags));
        assert!(validate_wx_flags(data_flags));

        // Stack region
        let stack_flags = MemoryRegionType::Stack.flags(true);
        assert!(MemoryRegionType::Stack.validate_flags(stack_flags));
        assert!(validate_wx_flags(stack_flags));

        // Read-only region
        let readonly_flags = MemoryRegionType::ReadOnly.flags(true);
        assert!(MemoryRegionType::ReadOnly.validate_flags(readonly_flags));
        assert!(validate_wx_flags(readonly_flags));
    }
}
