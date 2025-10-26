# Implementation Tasks: exec() and Shell Launch

## Overview

This task list breaks down the implementation of exec() syscall and shell launch into manageable, incremental steps. Each task builds on previous work to create a working interactive shell environment.

**Goal:** Boot MelloOS → Init exec()s shell → User sees "mello$ " prompt

---

## Task List

- [x] 1. Create exec() Infrastructure
  - Create `kernel/src/user/exec.rs` module
  - Define ExecContext struct
  - Define ExecError enum with errno mapping
  - Define ElfInfo and ProgramSegment structs
  - Export exec module from `kernel/src/user/mod.rs`
  - _Requirements: R1.1, R7.1-R7.6_

- [x] 2. Implement User Pointer Validation
  - Create `validate_user_pointer()` function
  - Check for NULL pointers
  - Check for kernel space addresses
  - Check for unmapped memory
  - Add validation for string arrays (argv, envp)
  - _Requirements: R8.1, R8.2, R8.3_

- [x] 3. Implement String Copying from User Space
  - Create `copy_string_from_user()` function
  - Enforce maximum string length (4096 bytes)
  - Handle NULL-terminated strings safely
  - Create `copy_string_array_from_user()` for argv/envp
  - Return Vec<String> for kernel use
  - _Requirements: R5.1, R5.5, R8.4_

- [x] 4. Implement ELF Loading from Filesystem
  - Create `load_elf_from_fs()` method in ExecContext
  - Use VFS path resolution to find file
  - Open file and read contents into Vec<u8>
  - Handle file not found error (ENOENT)
  - Handle permission denied error (EACCES)
  - Close file descriptor after reading
  - _Requirements: R2.1, R2.2, R2.6, R7.1, R7.2_

- [x] 5. Implement ELF Parsing and Validation
  - Create `parse_elf()` method in ExecContext
  - Validate ELF magic number (0x7F 'E' 'L' 'F')
  - Parse ELF header (entry point, program header offset)
  - Parse program headers (PT_LOAD segments)
  - Validate architecture (x86_64)
  - Return ENOEXEC for invalid ELF files
  - _Requirements: R2.3, R2.4, R7.3_

- [x] 6. Implement Process Image Clearing
  - Create `clear_old_image()` method in ExecContext
  - Save current memory state for rollback
  - Unmap all user space memory regions
  - Preserve kernel stack and task structures
  - Reset heap pointer
  - Clear old page tables
  - _Requirements: R3.1, R3.2_

- [x] 7. Implement Segment Loading
  - Create `load_segments()` method in ExecContext
  - Iterate through PT_LOAD segments
  - Allocate memory at segment virtual addresses
  - Copy segment data from ELF file
  - Zero BSS sections (memsz > filesz)
  - Set memory permissions (read/write/execute)
  - Enforce W^X policy (no write+execute)
  - _Requirements: R2.5, R3.3, R8.5, R8.6_

- [x] 8. Implement Stack Setup with Arguments
  - Create `setup_stack()` method in ExecContext
  - Allocate 8MB user stack at 0x7FFF_FFFF_0000
  - Copy environment strings to stack
  - Copy argument strings to stack
  - Build envp pointer array (NULL-terminated)
  - Build argv pointer array (NULL-terminated)
  - Push argc (argument count)
  - Align stack to 16-byte boundary
  - Return final stack pointer
  - _Requirements: R3.4, R3.5, R5.1-R5.8_

- [x] 9. Implement File Descriptor Handling
  - Create `close_cloexec_fds()` method in ExecContext
  - Iterate through process FD table
  - Close FDs with O_CLOEXEC flag set
  - Preserve stdin/stdout/stderr (FDs 0, 1, 2)
  - Preserve other FDs without O_CLOEXEC
  - _Requirements: R6.1, R6.2, R6.3, R6.4_

- [x] 10. Implement Userspace Jump
  - Create `jump_to_userspace()` method in ExecContext
  - Update task instruction pointer (RIP) to entry point
  - Update task stack pointer (RSP) to new stack
  - Set up initial register state (argc in RDI, argv in RSI, envp in RDX)
  - Switch to user mode (ring 3)
  - Execute sysretq to jump to new program
  - _Requirements: R1.4, R3.6_

- [x] 11. Implement Main exec() Function
  - Create `ExecContext::exec()` method
  - Call load_elf_from_fs()
  - Call parse_elf()
  - Call clear_old_image()
  - Call load_segments()
  - Call setup_stack()
  - Call close_cloexec_fds()
  - Update task state
  - Call jump_to_userspace()
  - Implement rollback on any error
  - _Requirements: R1.1-R1.8, R7.5_

- [x] 12. Implement sys_exec() Syscall Handler
  - Add SYS_EXEC to syscall table (syscall number 9)
  - Create `sys_exec()` function in `kernel/src/sys/syscall.rs`
  - Validate path pointer
  - Validate argv pointer array
  - Validate envp pointer array
  - Copy path string from user space
  - Copy argv array from user space
  - Copy envp array from user space
  - Create ExecContext
  - Call ExecContext::exec()
  - Return error code on failure (never returns on success)
  - _Requirements: R1.1-R1.8, R8.1-R8.3_

- [x] 13. Add Binary Extraction to Filesystem
  - Create `extract_binaries_to_fs()` function in `kernel/src/main.rs`
  - Create /bin directory in VFS
  - Extract mello-sh binary to /bin/sh
  - Set executable permissions (chmod 0755)
  - Extract mellobox binary to /bin/mellobox
  - Create symlinks for mellobox commands (ls, cat, echo, ps, etc.)
  - Call during kernel initialization (after VFS init)
  - _Requirements: R10.1, R10.2_

- [x] 14. Update Init Process to Launch Shell
  - Modify `kernel/userspace/init/src/main.rs`
  - Add sys_exec() syscall wrapper
  - Create argv array with shell path
  - Create envp array with PATH and HOME
  - Call sys_exec("/bin/sh", argv, envp)
  - Add fallback to test mode if exec fails
  - Print error message on exec failure
  - _Requirements: R4.1, R4.2, R4.3, R10.3, R10.4_

- [ ] 15. Verify Shell Integration
  - Ensure mello-sh displays prompt on startup
  - Verify shell reads from stdin (FD 0)
  - Verify shell writes to stdout (FD 1)
  - Test basic commands (echo, ls, ps)
  - Verify shell can fork and exec other programs
  - Test Ctrl+C signal handling
  - Test Ctrl+D EOF handling
  - _Requirements: R9.1-R9.7_

- [ ] 16. Implement Error Handling Tests
  - Test exec with non-existent file (expect ENOENT)
  - Test exec with non-executable file (expect EACCES)
  - Test exec with invalid ELF (expect ENOEXEC)
  - Test exec with NULL path (expect EINVAL)
  - Test exec with invalid argv pointer (expect EINVAL)
  - Verify process state preserved on error
  - Verify no memory leaks on error
  - _Requirements: R7.1-R7.6_

- [ ] 17. Implement Security Validation Tests
  - Test exec with kernel space path pointer (expect EINVAL)
  - Test exec with kernel space argv pointer (expect EINVAL)
  - Test exec with overlapping segments (expect EINVAL)
  - Test exec with executable stack (verify W^X enforcement)
  - Test exec with writable+executable segment (verify W^X enforcement)
  - Verify all user pointers validated before use
  - _Requirements: R8.1-R8.6_

- [ ] 18. Performance Testing and Optimization
  - Measure exec() latency (target < 10ms)
  - Measure shell startup time (target < 100ms)
  - Profile memory allocation during exec()
  - Optimize ELF loading (read entire file at once)
  - Optimize string copying (minimize allocations)
  - Verify no memory leaks with valgrind-style checks
  - _Requirements: Non-functional requirements_

- [ ] 19. Integration Testing
  - Test boot → shell prompt appears
  - Test "echo hello" → output appears
  - Test "ls" → directory listing appears
  - Test "ps" → process list appears
  - Test shell exit → init respawns shell
  - Test multiple exec() calls in sequence
  - Test exec() from non-init process
  - _Requirements: Success criteria_

- [ ] 20. Documentation and Cleanup
  - Document exec() syscall in `docs/architecture/syscalls.md`
  - Document ELF loading process
  - Document stack layout for new processes
  - Add code comments to exec.rs
  - Update kernel documentation
  - Create troubleshooting guide for exec failures
  - _Requirements: R10.1_

---

## Testing Strategy

### Unit Tests
- ELF parsing with valid/invalid binaries
- Pointer validation edge cases
- String copying with various lengths
- Stack setup with different argv/envp sizes

### Integration Tests
- exec() simple hello world program
- exec() with multiple arguments
- exec() with environment variables
- exec() error cases (file not found, invalid ELF)
- File descriptor inheritance

### System Tests
- Boot to shell prompt
- Execute commands in shell
- Shell respawn on exit
- Multiple programs exec in sequence

---

## Implementation Order

**Phase 1: Core Infrastructure (Tasks 1-3)** - 1 hour
- Set up exec module
- Implement validation and string copying

**Phase 2: ELF Loading (Tasks 4-5)** - 1-2 hours
- Load ELF from filesystem
- Parse and validate ELF format

**Phase 3: Memory Management (Tasks 6-7)** - 2-3 hours
- Clear old process image
- Load new segments with proper permissions

**Phase 4: Stack and Arguments (Tasks 8-9)** - 1-2 hours
- Set up stack with argc/argv/envp
- Handle file descriptors

**Phase 5: Execution (Tasks 10-12)** - 1 hour
- Jump to userspace
- Wire up syscall handler

**Phase 6: Shell Integration (Tasks 13-15)** - 1 hour
- Extract binaries to filesystem
- Update init to launch shell
- Verify shell works

**Phase 7: Testing and Polish (Tasks 16-20)** - 1-2 hours
- Error handling tests
- Security validation
- Performance tuning
- Documentation

**Total Estimated Time:** 8-12 hours

---

## Success Criteria

Implementation is complete when:

1. ✅ Boot MelloOS → See "mello$ " prompt
2. ✅ Type "echo hello" → See "hello" output
3. ✅ Type "ls" → See directory listing
4. ✅ Type "ps" → See process list
5. ✅ Press Ctrl+C → Command cancels
6. ✅ Press Ctrl+D → Shell exits and respawns
7. ✅ exec() works for any valid ELF binary
8. ✅ Error handling works correctly
9. ✅ No memory leaks
10. ✅ Security validation passes

---

## Dependencies

- ✅ Phase 6: Userland Foundation (fork, wait syscalls)
- ✅ Phase 8: Filesystem & Storage (VFS, file I/O)
- ✅ Existing: mello-sh, mello-term, mellobox binaries
- ✅ Existing: ELF loader infrastructure (kernel/src/user/elf.rs)
- ✅ Existing: Process management (kernel/src/user/process.rs)

---

## Notes

- Tasks should be completed in order as they build on each other
- Run `cargo check` after each task to catch errors early
- Test incrementally - don't wait until the end
- Use `getDiagnostics` tool to verify code correctness
- Commit after each major milestone

---

**Document Version:** 1.0  
**Status:** Ready for Implementation  
**Priority:** HIGH - Required for usable system
    