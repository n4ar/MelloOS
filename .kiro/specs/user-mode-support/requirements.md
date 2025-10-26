# Requirements Document

## Introduction

This document specifies the requirements for implementing user-mode support in MelloOS, transitioning from kernel-mode (ring 0) to user-mode (ring 3) execution. This feature enables the kernel to run user programs safely in a restricted privilege level, providing a syscall interface for kernel services, an ELF binary loader for executing programs from storage, and basic process management capabilities. This is a foundational feature that establishes the user-kernel boundary and enables true multi-user, multi-process operating system functionality.

## Glossary

- **Ring 0**: Kernel mode - highest privilege level with full hardware access
- **Ring 3**: User mode - restricted privilege level for user applications
- **Syscall**: System call - mechanism for user programs to request kernel services
- **ELF**: Executable and Linkable Format - standard binary format for executables
- **Process**: An instance of a running program with its own memory space and resources
- **PID**: Process Identifier - unique numeric identifier for each process
- **GDT**: Global Descriptor Table - defines memory segments and privilege levels
- **TSS**: Task State Segment - stores processor state during privilege transitions
- **IDT**: Interrupt Descriptor Table - maps interrupts to handler functions
- **MSR**: Model Specific Register - processor-specific configuration registers
- **IRET**: Interrupt Return instruction - returns from kernel to user mode
- **SYSRET**: System Return instruction - fast return from syscall handler
- **Page Table**: Memory mapping structure for virtual memory management
- **Ramdisk**: In-memory file system for storing initial programs
- **Init Process**: First user process (PID 1) started by the kernel

## Requirements

### Requirement 1: Ring Transition Infrastructure

**User Story:** As a kernel developer, I want to implement privilege level transitions from ring 0 to ring 3, so that user programs can run in a restricted environment while the kernel maintains full control.

#### Acceptance Criteria

1. THE Kernel SHALL configure the GDT with USER_CODE_SEG (0x1B) and USER_DATA_SEG (0x23) descriptors for ring 3 execution
2. THE Kernel SHALL implement a TSS (Task State Segment) to handle privilege level transitions
3. WHEN transitioning to user mode, THE Kernel SHALL set up a separate user stack at privilege level 3
4. THE Kernel SHALL implement user_entry assembly trampoline to perform IRET transition to ring 3
5. WHEN returning from user mode, THE Kernel SHALL safely restore kernel context and privilege level

### Requirement 2: Syscall Interface Implementation

**User Story:** As a user program developer, I want to invoke kernel services through system calls, so that my program can perform privileged operations like I/O and process management.

#### Acceptance Criteria

1. THE Kernel SHALL implement syscall entry mechanism using either syscall/sysret or int 0x80 instruction
2. WHEN using syscall/sysret, THE Kernel SHALL configure MSR registers (STAR, LSTAR, SFMASK) to point to syscall_entry handler
3. THE Kernel SHALL implement a syscall dispatcher that validates syscall numbers and routes to appropriate handlers
4. THE Kernel SHALL support at least 6 basic syscalls: sys_write, sys_exit, sys_fork, sys_exec, sys_wait, and sys_yield
5. THE Kernel SHALL define calling convention with rax=syscall_id and rdi-r9 as arguments
6. WHEN sys_write is called, THE Kernel SHALL output data from user buffer to console/serial port
7. WHEN sys_yield is called, THE Kernel SHALL cooperatively yield CPU to the scheduler
8. WHEN an invalid syscall number is provided, THE Kernel SHALL return an error code to the user program
9. THE Kernel SHALL preserve user program state across syscall invocations

### Requirement 3: ELF Binary Loader

**User Story:** As a kernel developer, I want to load and execute ELF64 binaries from storage, so that the system can run compiled user programs.

#### Acceptance Criteria

1. THE Kernel SHALL implement an ELF64 loader that supports ET_EXEC (executable) format
2. THE ELF Loader SHALL parse ELF headers and validate the binary format including e_entry alignment
3. THE ELF Loader SHALL read Program Headers (PHDR) and map PT_LOAD segments to virtual memory
4. THE ELF Loader SHALL support PT_GNU_STACK program header for future stack flag compatibility
5. WHEN loading an ELF binary, THE ELF Loader SHALL set up the program's virtual memory layout
6. THE ELF Loader SHALL configure user stack, entry point, and segment registers before transitioning to user mode
7. THE ELF Loader SHALL handle memory permissions correctly (read, write, execute) for different segments
8. WHEN ELF loading fails, THE Kernel SHALL return an appropriate error code

### Requirement 4: Basic Process Management

**User Story:** As a kernel developer, I want to implement fundamental process management operations, so that the system can create, manage, and terminate user processes.

#### Acceptance Criteria

1. THE Kernel SHALL implement sys_fork() to create a copy of the current process with a new PID
2. WHEN fork() is called, THE Kernel SHALL duplicate the process page table and context (with TODO for future copy-on-write optimization)
3. THE Kernel SHALL implement sys_exec() to replace the current process image with a new ELF binary
4. WHEN exec() is called, THE Kernel SHALL clear the current address space and load the new ELF binary
5. THE Kernel SHALL implement sys_exit() to terminate the current process and release its resources
6. THE Kernel SHALL implement sys_wait() to allow parent processes to wait for child process completion
7. THE Kernel SHALL assign unique PIDs using an atomic counter and maintain a process table protected by spinlocks
8. THE Kernel SHALL handle process state transitions (running, ready, zombie, terminated) with proper zombie cleanup
9. THE Kernel SHALL maintain per-CPU current process pointers for SMP safety

### Requirement 5: Init Process (PID 1)

**User Story:** As a kernel developer, I want to create and launch the first user process after boot, so that the system can demonstrate user-mode functionality.

#### Acceptance Criteria

1. THE Kernel SHALL create an init process with PID 1 after completing boot initialization
2. THE Init Process SHALL execute in user mode (ring 3) with restricted privileges
3. THE Init Process SHALL be loaded from a ramdisk or embedded binary
4. WHEN the Init Process runs, THE Process SHALL demonstrate syscall functionality by calling sys_write()
5. THE Init Process SHALL output "Hello from userland!" message to verify user-mode execution
6. THE Init Process SHALL call sys_fork() to spawn a child process for testing process management
7. THE Kernel SHALL log "Launching init (PID 1)..." during boot for debugging purposes
8. THE Kernel SHALL log the successful creation and execution of the Init Process
9. WHEN the Init Process encounters an error, THE Kernel SHALL handle it gracefully without crashing

### Requirement 6: Memory Protection and Safety

**User Story:** As a kernel developer, I want to ensure memory protection between user and kernel space, so that user programs cannot compromise kernel integrity.

#### Acceptance Criteria

1. THE Kernel SHALL enable NX-bit in paging to prevent code execution on stack and data pages
2. THE Kernel SHALL validate all user memory accesses through copy_from_user() and copy_to_user() functions
3. THE Kernel SHALL define USER_LIMIT constant (0x0000_8000_0000_0000) for virtual address range validation
4. WHEN a user program accesses invalid memory, THE Kernel SHALL generate a page fault and terminate the process
5. THE Kernel SHALL log page fault details (CR2 register and error code) before terminating the process
6. THE Kernel SHALL maintain separate page tables for each process to ensure memory isolation
7. THE Kernel SHALL perform TLB flush operations when switching between processes
8. THE Kernel SHALL protect kernel memory from user access through proper page table permissions
9. WHEN user programs attempt to access kernel memory, THE Kernel SHALL deny access and signal the process

### Requirement 7: SMP Safety and Synchronization

**User Story:** As a kernel developer, I want user-mode support to work correctly on multi-core systems, so that processes can run safely across multiple CPU cores.

#### Acceptance Criteria

1. THE Kernel SHALL protect the process table with appropriate spinlocks for SMP safety
2. WHEN performing context switches, THE Kernel SHALL ensure atomic updates to process state
3. THE Kernel SHALL handle IPI (Inter-Processor Interrupt) for TLB flush operations across cores
4. THE Kernel SHALL maintain per-CPU process scheduling queues to minimize lock contention
5. WHEN a process faults on one core, THE Kernel SHALL handle it without affecting processes on other cores
6. THE Kernel SHALL ensure syscall handlers are reentrant and SMP-safe
7. THE Kernel SHALL log process operations with CPU core identification for debugging

### Requirement 8: Testing and Verification

**User Story:** As a kernel developer, I want comprehensive testing of user-mode functionality, so that I can verify the system works correctly and safely.

#### Acceptance Criteria

1. THE Kernel SHALL boot successfully and display "Switching to user mode..." message
2. THE Init Process SHALL successfully transition to ring 3 and execute user code
3. THE sys_write() syscall SHALL function correctly and output user messages to console
4. THE Kernel SHALL demonstrate fork/exec/exit/wait functionality with test processes
5. THE Kernel SHALL include automated QEMU tests that expect "Hello from userland!" message
6. THE Kernel SHALL provide stress test creating fork chain of 10 processes to test zombie cleanup
7. THE Kernel SHALL handle user program faults without crashing the kernel
8. THE Kernel SHALL maintain system stability during continuous process creation and termination
9. THE Kernel SHALL log all major user-mode operations with CPU core ID for debugging and verification
10. WHEN running on SMP systems, THE Kernel SHALL demonstrate stable multi-core process execution

## Resource Limits

- Maximum processes: 1024
- Maximum open file descriptors per process: 256
- User stack size: 8KB per process
- Maximum ELF binary size: 16MB
- Maximum syscall arguments: 6 registers
- Process table size: 4KB (fixed allocation)

## Security Considerations

This implementation provides basic user-kernel separation but is not intended for production security. Future enhancements should include:
- Address Space Layout Randomization (ASLR)
- Stack canaries and buffer overflow protection
- Capability-based security model
- Secure boot and code signing verification
- Resource quotas and limits enforcement

## Performance Requirements

- Context switch latency: < 10 microseconds
- Syscall overhead: < 1 microsecond
- ELF loading time: < 100 milliseconds for typical binaries
- Process creation time: < 5 milliseconds
- Memory allocation overhead: < 5% of total system memory
## Implem
entation Phases

To manage complexity and enable incremental testing, this feature should be implemented in the following sub-phases:

### Phase 6.1: Ring Transition Infrastructure
- GDT configuration with user segments
- TSS setup with per-CPU kernel stacks
- Assembly trampoline for user_entry
- Basic ring 0 ↔ ring 3 transitions

### Phase 6.2: Syscall Interface
- MSR configuration for syscall/sysret
- Syscall dispatcher and routing
- Implementation of sys_write and sys_yield
- Basic syscall testing framework

### Phase 6.3: ELF Loader and Init Process
- ELF64 parser and loader
- Ramdisk integration
- Init process creation and execution
- User-mode "Hello from userland!" demonstration

### Phase 6.4: Process Management
- sys_fork, sys_exec, sys_exit, sys_wait implementation
- Process table and PID management
- Process state transitions and cleanup
- Multi-process testing and stress tests

This phased approach allows for testing and debugging at each stage while building toward the complete user-mode support system.