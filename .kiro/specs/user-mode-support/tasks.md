# Implementation Plan

Convert the user-mode support design into a series of prompts for a code-generation LLM that will implement each step with incremental progress. Make sure that each prompt builds on the previous prompts, and ends with wiring things together. There should be no hanging or orphaned code that isn't integrated into a previous step. Focus ONLY on tasks that involve writing, modifying, or testing code.

## Phase 6.1: Ring Transition Infrastructure

- [x] 1. Set up GDT with user segments and TSS
  - Create `kernel/src/arch/x86_64/gdt.rs` with new GDT containing USER_CODE_SEG (0x3B), USER_DATA_SEG (0x43), and TSS_SEG (0x48)
  - Implement TSS structure with per-CPU instances and IST stack setup for NMI/Double Fault handlers
  - Add GDT installation and TSS loading functions that replace Limine's GDT with our extended version
  - _Requirements: 1.1, 1.2, 1.5_

- [x] 1.1 Implement per-CPU TSS management
  - Add TSS initialization in `init_tss_for_cpu()` with proper kernel stack (rsp0) and IST stacks
  - Integrate TSS setup with existing SMP initialization in `arch::x86_64::smp::init_smp()`
  - Add `update_kernel_stack_for_process()` function for context switching
  - _Requirements: 1.1, 1.2_

- [x] 1.2 Create user entry trampoline assembly
  - Write `kernel/src/arch/x86_64/user_entry.S` with canonical address validation and IRET transition
  - Implement `user_entry_trampoline()` function that sets up IRET frame and transitions to ring 3
  - Add `setup_user_stack()` helper that maps user stack with guard pages and proper NX flags
  - _Requirements: 1.1, 1.5_

- [x] 1.3 Write unit tests for GDT and TSS setup
  - Test GDT entry creation and validation
  - Test TSS initialization and per-CPU setup
  - Test user entry trampoline with mock addresses
  - _Requirements: 8.1, 8.2_

## Phase 6.2: Syscall Interface

- [x] 2. Implement fast syscall mechanism
  - Create `kernel/src/arch/x86_64/syscall/mod.rs` with MSR configuration (EFER.SCE, STAR, LSTAR, SFMASK)
  - Add per-CPU GS base setup in `init_syscall_msrs()` for SWAPGS support
  - Implement syscall/sysret enablement that works with existing interrupt infrastructure
  - _Requirements: 2.1, 2.2, 2.7_

- [x] 2.1 Create fast syscall entry assembly stub
  - Write `kernel/src/arch/x86_64/syscall/entry.S` with SWAPGS, safe stack switching, and canonical address validation
  - Implement register save/restore following R10-based calling convention (not RCX for arg4)
  - Add error handling for non-canonical return addresses in `handle_bad_syscall_return()`
  - _Requirements: 2.1, 2.2, 2.4_

- [x] 2.2 Enhance syscall dispatcher with new syscalls
  - Extend existing `kernel/src/sys/syscall.rs` dispatcher to support SYS_FORK, SYS_EXEC, SYS_WAIT, SYS_YIELD, SYS_GETPID
  - Add detailed logging with CPU ID, PID, and RIP for debugging SMP issues
  - Implement user pointer validation in `is_user_pointer_valid()` and `copy_from_user()`/`copy_to_user()` functions
  - _Requirements: 2.3, 2.4, 2.5, 2.6_

- [x] 2.3 Implement basic syscalls for testing
  - Enhance `sys_write()` with user pointer validation and fastpath optimization for small writes
  - Implement `sys_yield()` that marks current task as Ready and calls scheduler
  - Add `sys_getpid()` that returns current process ID for debugging
  - _Requirements: 2.3, 2.5_

- [x] 2.4 Write syscall integration tests
  - Test syscall entry/exit mechanism with mock user programs
  - Test user pointer validation and error handling
  - Test syscall performance and register preservation
  - _Requirements: 8.3, 8.4_

## Phase 6.3: ELF Loader and Init Process

- [x] 3. Implement ELF64 binary loader
  - Create `kernel/src/user/elf.rs` with ELF64 header parsing and validation (ET_EXEC, EM_X86_64)
  - Implement program header parsing and PT_LOAD segment mapping with proper page flags (USER, NX, WRITABLE)
  - Add segment data copying using kernel mapping approach (not direct user pointers) with TLB flush
  - _Requirements: 3.1, 3.2, 3.3, 3.5, 3.6_

- [x] 3.1 Add memory region tracking for processes
  - Extend existing Task structure with memory regions (Code, Data, BSS, Stack) in `kernel/src/sched/task.rs`
  - Implement memory region validation to prevent overlaps and ensure user space limits
  - Add region lookup functions for page fault handling and debugging
  - _Requirements: 3.4, 3.5, 6.4, 6.5_

- [x] 3.2 Create ramdisk integration for init binary
  - Modify `kernel/src/init_loader.rs` to load ELF binary instead of running kernel task
  - Embed init.elf binary in kernel image or load from ramdisk
  - Integrate ELF loader with existing init process creation
  - _Requirements: 5.3, 5.4, 5.6_

- [x] 3.3 Implement init process creation
  - Create init process (PID 1) using ELF loader in `load_init_process()`
  - Set up user stack with guard pages and proper memory layout
  - Transition to user mode using `user_entry_trampoline()` and verify ring 3 execution
  - _Requirements: 5.1, 5.2, 5.7, 8.1, 8.2_

- [x] 3.4 Write ELF loader tests
  - Test ELF header validation and error handling
  - Test segment mapping with various flag combinations
  - Test memory region tracking and validation
  - _Requirements: 3.7, 8.5_

## Phase 6.4: Process Management

- [x] 4. Implement process control block and table
  - Create `kernel/src/user/process.rs` with Process structure containing PID, state, page table, memory regions
  - Implement fine-grained process table with per-process locks using ProcessGuard/ProcessSlotGuard pattern
  - Add atomic PID allocation and process table management functions
  - _Requirements: 4.7, 4.8, 7.1, 7.2_

- [x] 4.1 Implement sys_fork() system call
  - Create child process with new PID and copy parent's memory space (mark TODO for copy-on-write)
  - Duplicate page table and memory regions from parent process
  - Set up child context to return 0, parent context to return child PID
  - _Requirements: 4.1, 4.2, 4.9_

- [x] 4.2 Implement sys_exec() system call
  - Clear current process memory space and load new ELF binary
  - Reset CPU context with new entry point and user stack
  - Update process memory regions and page table for new program
  - _Requirements: 4.3, 4.4_

- [x] 4.3 Implement sys_exit() and sys_wait() system calls
  - Mark process as zombie state and set exit code in `sys_exit()`
  - Implement parent waiting and child cleanup in `sys_wait()`
  - Add process resource cleanup and zombie process removal
  - _Requirements: 4.5, 4.6, 4.8_

- [x] 4.4 Add page fault handler for memory protection
  - Create page fault handler in `kernel/src/arch/x86_64/fault.rs` with IST stack usage
  - Implement user space fault detection and process termination
  - Add detailed fault logging with CR2, error code, and process context
  - _Requirements: 6.3, 6.4, 6.7, 6.9_

- [x] 4.5 Integrate process management with scheduler
  - Modify existing scheduler to work with Process structures instead of Task structures
  - Update context switching to handle process page table switching and TLB flush
  - Add process state management (Ready, Running, Sleeping, Blocked, Zombie, Terminated)
  - _Requirements: 4.8, 7.4, 7.5_

- [x] 4.6 Write process management tests
  - Test fork/exec/exit/wait cycle with multiple processes
  - Test process table management and PID allocation
  - Test page fault handling and process termination
  - _Requirements: 8.4, 8.6, 8.7_

## Phase 6.5: Integration and Testing

- [x] 5. Create comprehensive integration tests
  - Write automated QEMU tests that expect "Hello from userland!" message from init process
  - Implement stress test creating fork chain of 10 processes to test zombie cleanup
  - Add privilege level validation using `get_current_privilege_level()` helper
  - _Requirements: 8.1, 8.2, 8.6, 8.10_

- [x] 5.1 Add SMP safety verification
  - Test user-mode support on multi-core systems with process migration
  - Verify syscall handling works correctly across different CPU cores
  - Test process creation and termination under SMP load
  - _Requirements: 7.1, 7.2, 7.6, 8.10_

- [x] 5.2 Implement performance monitoring and debugging
  - Add syscall performance counters and CPU/PID logging for all major operations
  - Implement process lifecycle logging for debugging multi-process scenarios
  - Add memory usage tracking and process resource monitoring
  - _Requirements: 8.7, 8.9_

- [x] 5.3 Write comprehensive test suite
  - Test all syscalls with valid and invalid arguments
  - Test memory protection boundaries and error handling
  - Test process lifecycle edge cases and error conditions
  - _Requirements: 8.5, 8.8_

- [ ] 5.4 Document SMP issues and solutions
  - Create `docs/troubleshooting/smp-issues.md` documenting the AP boot issues encountered
  - Document the LAPIC address corruption bug and fix (trampoline register preservation)
  - Document the CPU_COUNT synchronization issue between scheduler and SMP module
  - Document the syscall MSR initialization fix (passing cpu_id parameter)
  - Include code examples and debugging techniques used
  - _Requirements: 8.9, 8.10_

- [ ] 5.5 Update architecture documentation
  - Update `docs/architecture/smp.md` with final SMP implementation details
  - Add section on load balancing algorithm and task distribution
  - Document IPI usage for cross-CPU communication
  - Add diagrams showing task flow across multiple CPUs
  - Document per-CPU data structures and GS.BASE usage
  - _Requirements: 8.9, 8.10_

- [ ] 5.6 Create user-mode and SMP integration guide
  - Create `docs/integration/user-mode-smp.md` explaining how user-mode works with SMP
  - Document how syscalls work across different CPU cores
  - Explain process migration and cross-CPU fork behavior
  - Document memory protection in multi-core environment
  - Add examples of user processes running on multiple cores
  - _Requirements: 8.9, 8.10_

- [ ] 5.7 Update main README with achievements
  - Update `README.md` with SMP multi-core support feature
  - Add section describing user-mode process support
  - Update feature list with completed Phase 6 and 6.5 items
  - Add performance characteristics (2-core load balancing, IPI latency)
  - Include test results summary showing SMP functionality
  - _Requirements: 8.9, 8.10_

## Integration Notes

Each task builds incrementally on previous work:
- Phase 6.1 establishes the hardware foundation (GDT, TSS, ring transitions)
- Phase 6.2 adds the syscall interface using the ring transition infrastructure
- Phase 6.3 implements ELF loading and creates the first user process using syscalls
- Phase 6.4 adds full process management using all previous components
- Phase 6.5 provides comprehensive testing of the complete system

The implementation integrates with existing MelloOS infrastructure:
- Uses existing scheduler framework and extends it for process management
- Builds on existing memory management (PMM, paging) for user space allocation
- Extends existing syscall infrastructure with new user-mode syscalls
- Integrates with existing SMP support for multi-core process execution
- Uses existing interrupt handling framework for page faults and syscalls