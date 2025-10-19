# Changelog

All notable changes to MelloOS will be documented in this file.

## [Unreleased]

### Added - Task Scheduler (Phase 3 - Major Update)

#### Task Scheduler Core
- Preemptive multitasking with Round-Robin scheduling algorithm
- Task Control Block (TCB) structure with unique task IDs
- Task states: Ready, Running, Sleeping
- Circular runqueue for O(1) task selection
- Thread-safe task table with mutex protection
- Idle task (ID 0) for when no tasks are available
- Task spawning API: `spawn_task(name, entry_point)`

#### Context Switching
- Assembly-optimized context switch routine
- Full CPU context save/restore (callee-saved registers)
- Per-task 8KB stacks allocated from kernel heap
- Entry trampoline for new task initialization
- Follows x86_64 System V ABI calling convention
- Tail-switch optimization (no return to interrupt handler)

#### Timer Interrupt System
- PIT (Programmable Interval Timer) configuration at 100 Hz
- PIC (Programmable Interrupt Controller) remapping to vectors 32-47
- IDT (Interrupt Descriptor Table) setup with timer handler
- Automatic EOI (End of Interrupt) handling
- Timer tick counter for debugging and testing

#### Scheduler Logging
- Logging macros with `[SCHED]` prefix
- Context switch logging with throttling (first 10, then every 100)
- Task spawn/destroy logging
- Error and warning messages for debugging

#### Testing
- End-to-end integration test with two demo tasks
- Manual test functions for scheduler components
- Context switching verification
- Timer interrupt verification
- Round-Robin algorithm verification

### Added - Memory Management System (Phase 2 - Major Update)

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

- Updated kernel entry point to initialize task scheduler
- Modified boot sequence to spawn demo tasks (Task A and Task B)
- Enhanced project structure with `kernel/src/sched/` subsystem
- Kernel now runs in multitasking mode with visible task switching
- Interrupts are now enabled after scheduler initialization
- Updated kernel idle loop to use HLT instruction

### Breaking Changes

- Kernel no longer runs in single-threaded mode
- Timer interrupts are now enabled and will preempt code execution
- Stack usage must be carefully managed (8KB per task limit)

### Documentation

- Comprehensive README update with:
  - Task scheduler architecture and components
  - Round-Robin scheduling explanation
  - Context switching mechanism details
  - Timer interrupt configuration
  - Interrupt vector mapping
  - Memory management architecture
  - Security features documentation
  - Technical details (memory layout, page flags, buddy orders)
  - API usage examples
  - Current capabilities and limitations
  - Performance characteristics (context switch < 1μs)
  - Updated roadmap with Phase 3 completion
- Added `.kiro/specs/task-scheduler/` specification with:
  - Requirements document (EARS format)
  - Design document with architecture diagrams
  - Implementation tasks and completion tracking
- Inline documentation for all scheduler functions
- Assembly code comments explaining context switch
- Safety documentation for unsafe code
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

