# Implementation Plan

- [x] 1. Setup project dependencies and module structure
  - Add `spin` crate to Cargo.toml for Mutex support
  - Add `x86_64` crate (optional, recommended for paging)
  - Create `kernel/src/mm/` directory structure
  - Create empty module files: `mod.rs`, `pmm.rs`, `paging.rs`, `allocator.rs`
  - _Requirements: 4.1, 4.2_

- [x] 2. Implement HHDM and address conversion utilities
  - Create `kernel/src/mm/mod.rs` with HHDM offset management
  - Implement `init_hhdm()` to read offset from Limine
  - Implement `phys_to_virt()` and `virt_to_phys()` functions using AtomicUsize
  - Add Limine requests: `HhdmRequest`, `MemoryMapRequest`, `KernelAddressRequest`
  - _Requirements: 1.1, 2.5_

- [x] 3. Implement Physical Memory Manager (PMM)
  - [x] 3.1 Create PMM data structures in `pmm.rs`
    - Define `PhysicalMemoryManager` struct with bitmap, frame counts, and last_alloc
    - Define constants: `FRAME_SIZE = 4096`
    - _Requirements: 1.3_
  
  - [x] 3.2 Implement PMM initialization
    - Parse Limine memory map and filter for Usable memory only
    - Calculate total frames and allocate bitmap
    - Mark kernel image and page tables as used
    - Log total and usable memory in MB
    - _Requirements: 1.1, 1.2, 1.3_
  
  - [x] 3.3 Implement frame allocation
    - Implement `alloc_frame()` with bitmap scanning from last_alloc
    - Zero allocated frame for security
    - Update free_frames counter
    - Log allocation with "[MM]" prefix
    - _Requirements: 1.4, 1.6_
  
  - [x] 3.4 Implement frame deallocation
    - Implement `free_frame()` to mark frame as free
    - Update free_frames counter
    - Log deallocation with "[MM]" prefix
    - _Requirements: 1.5, 1.6_
  
  - [x] 3.5 Implement contiguous allocation for DMA
    - Implement `alloc_contiguous()` to find aligned contiguous frames
    - _Requirements: 1.4_

- [x] 4. Implement CPU setup for memory protection
  - [x] 4.1 Enable NX bit support
    - Read EFER MSR (0xC0000080)
    - Set bit 11 (NXE)
    - Write back to EFER
    - _Requirements: 2.6_
  
  - [x] 4.2 Enable write protection
    - Read CR0 register
    - Set bit 16 (WP)
    - Write back to CR0
    - _Requirements: 2.6_

- [x] 5. Implement paging system
  - [x] 5.1 Create page table data structures in `paging.rs`
    - Define `PageTableEntry` with flags (PRESENT, WRITABLE, NO_EXECUTE, etc.)
    - Define `PageTable` with 512 entries, 4KB aligned
    - Define `PageMapper` struct
    - _Requirements: 2.1_
  
  - [x] 5.2 Implement page table entry operations
    - Implement `PageTableEntry::addr()` to extract physical address
    - Implement `PageTableEntry::set()` to set address and flags
    - Implement `PageTableEntry::is_present()` check
    - Implement `PageTableEntry::clear()`
    - _Requirements: 2.1, 2.6_
  
  - [x] 5.3 Implement page mapping
    - Implement `map_page()` to create 4-level page table entries
    - Traverse PML4 → PDPT → PD → PT, creating tables as needed
    - Set appropriate flags for each mapping
    - _Requirements: 2.2, 2.6_
  
  - [x] 5.4 Implement page unmapping and TLB invalidation
    - Implement `unmap_page()` to clear page table entry
    - Implement `invlpg()` assembly instruction for TLB invalidation
    - _Requirements: 2.3_
  
  - [x] 5.5 Implement address translation
    - Implement `translate()` to walk page tables
    - Return physical address or None if not mapped
    - _Requirements: 2.4_
  
  - [x] 5.6 Implement kernel section mapping
    - Get kernel section addresses from Limine
    - Map .text section with RX flags (PRESENT | GLOBAL)
    - Map .rodata section with R flags (PRESENT | NO_EXECUTE | GLOBAL)
    - Map .data/.bss with RW+NX flags (PRESENT | WRITABLE | NO_EXECUTE | GLOBAL)
    - _Requirements: 2.1, 2.2, 2.6_
  
  - [x] 5.7 Add guard pages for stack/heap protection
    - Unmap pages around kernel stack
    - Unmap pages around heap boundaries
    - _Requirements: 2.2_

- [x] 6. Implement kernel heap allocator (Buddy System)
  - [x] 6.1 Create allocator data structures in `allocator.rs`
    - Define `BuddyAllocator` struct with free lists array
    - Define `FreeBlock` struct for free list nodes
    - Define constants: MIN_BLOCK_SIZE=64, MAX_BLOCK_SIZE=1MB, NUM_ORDERS=15
    - Create global `ALLOCATOR` wrapped in `Mutex`
    - _Requirements: 3.1, 3.6, 3.7_
  
  - [x] 6.2 Implement buddy allocator initialization
    - Implement `BuddyAllocator::init()` with heap range
    - Initialize free lists
    - Add initial heap memory to appropriate free list
    - _Requirements: 3.1_
  
  - [x] 6.3 Implement memory allocation
    - Implement `alloc()` to find or split blocks
    - Round up size to power of 2
    - Search free lists for appropriate order
    - Split larger blocks if needed
    - Zero allocated memory for security
    - _Requirements: 3.1, 3.6, 3.7_
  
  - [x] 6.4 Implement memory deallocation
    - Implement `free()` to return blocks to free lists
    - Find buddy block using XOR
    - Merge with buddy if both free
    - _Requirements: 3.2, 3.7_
  
  - [x] 6.5 Implement public API with logging
    - Implement `kmalloc()` with Mutex locking
    - Log allocation with size and address
    - Implement `kfree()` with Mutex locking
    - Log deallocation with size and address
    - Handle out-of-memory by returning null and logging error
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 7. Implement memory management initialization coordinator
  - Create `init_memory()` function in `mm/mod.rs`
  - Get HHDM offset from Limine and call `init_hhdm()`
  - Get memory map and kernel addresses from Limine
  - Call CPU setup functions (enable_nx_bit, enable_write_protect)
  - Initialize PMM with memory map and kernel bounds
  - Initialize paging system and map kernel sections
  - Add guard pages
  - Initialize kernel heap allocator
  - Log initialization summary with memory statistics
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [x] 8. Integrate memory management into kernel boot
  - Update `kernel/src/main.rs` to call `mm::init_memory()`
  - Call after initial framebuffer setup but before "Hello" message
  - Ensure kernel still displays "Hello from MelloOS ✨" after MM init
  - _Requirements: 4.1, 5.6_

- [x] 9. Implement memory management tests
  - [x] 9.1 Create PMM test function
    - Test frame allocation returns valid address
    - Test multiple allocations return different frames
    - Test free and reallocation reuses frame
    - Log test results with "[MM] ✓" prefix
    - _Requirements: 5.1, 5.2, 5.4_
  
  - [x] 9.2 Create paging test function
    - Test page mapping and translation
    - Test page unmapping
    - Log test results with "[MM] ✓" prefix
    - _Requirements: 5.1, 5.2, 5.4_
  
  - [x] 9.3 Create allocator test function
    - Test kmalloc(1024) returns non-null pointer
    - Test memory write and read
    - Test kfree() completes without error
    - Test multiple allocations (10x 64 bytes)
    - Test multiple frees
    - Log test results with "[MM] ✓" prefix
    - _Requirements: 5.1, 5.2, 5.3, 5.4_
  
  - [x] 9.4 Create test runner
    - Implement `run_memory_tests()` function
    - Call all test functions in sequence
    - Print test header and footer with "=========="
    - Print "All tests passed! ✨" on success
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.7_
  
  - [x] 9.5 Integrate tests into initialization
    - Call `run_memory_tests()` from `init_memory()`
    - Run tests after all MM components are initialized
    - _Requirements: 5.1_

- [x] 10. Add logging utilities for memory management
  - Create macro or function for "[MM]" prefixed logging
  - Ensure all addresses are displayed in hexadecimal format (0x...)
  - Ensure memory sizes are displayed in appropriate units (bytes, KB, MB)
  - _Requirements: 5.4, 5.5_
