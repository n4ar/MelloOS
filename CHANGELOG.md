# Changelog

All notable changes to MelloOS will be documented in this file.

## [Unreleased]

### Added - Memory Management System (Major Update)

#### Physical Memory Manager (PMM)
- Bitmap-based frame allocator for 4KB physical frames
- Automatic memory zeroing for security
- Support for contiguous frame allocation (DMA)
- Memory statistics tracking (total/free memory)
- Integration with Limine memory map

#### Paging System
- 4-level page table implementation (PML4 → PDPT → PD → PT)
- Per-section memory permissions:
  - .text: Read + Execute (RX)
  - .rodata: Read only (R)
  - .data/.bss: Read + Write + No Execute (RW+NX)
- Guard pages for stack/heap overflow protection
- TLB invalidation support
- Virtual address translation

#### Kernel Heap Allocator
- Buddy System algorithm (64B to 1MB blocks)
- Thread-safe allocation with Mutex
- `kmalloc()` and `kfree()` API
- Automatic block splitting and merging
- 16MB kernel heap at 0xFFFF_A000_0000_0000

#### Security Features
- NX (No Execute) bit support via EFER MSR
- Write protection via CR0 register
- Memory zeroing on allocation
- Guard pages around critical regions

#### Memory Management Logging
- Logging utilities with `[MM]` prefix
- Macros: `mm_log!`, `mm_info!`, `mm_error!`, `mm_test_ok!`
- Automatic size formatting (bytes, KB, MB)
- Hexadecimal address formatting

#### Testing
- Automated PMM tests (allocation, free, reallocation)
- Automated paging tests (mapping, translation, unmapping)
- Automated allocator tests (kmalloc/kfree, multiple allocations)
- All tests run automatically during kernel initialization

### Changed

- Updated kernel entry point to initialize memory management
- Modified framebuffer message to "Hello from MelloOS ✨"
- Enhanced project structure with `kernel/src/mm/` subsystem
- Updated dependencies: added `spin` (0.9) and `x86_64` (0.15)

### Documentation

- Comprehensive README update with:
  - Memory management architecture
  - Security features documentation
  - Technical details (memory layout, page flags, buddy orders)
  - API usage examples
  - Current capabilities and limitations
  - Performance characteristics
  - Roadmap
- Added `docs/memory-management-logging.md`
- Added `.kiro/specs/memory-management/` specification
- Updated `.github/BRANCH_PROTECTION.md`

### CI/CD

- GitHub Actions workflow for automated testing on develop branch
- Build verification scripts
- Automated release workflow with ISO artifacts

## [0.1.0] - Initial Release

### Added

- Basic kernel boot with Limine bootloader
- UEFI firmware support
- Framebuffer driver with pixel-level access
- 8x8 bitmap font rendering
- Character and string drawing functions
- Panic handler
- Build system with Makefile
- QEMU testing environment

