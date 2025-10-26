# Implementation Plan: Technical Debt Resolution

## Phase 1: High Priority Security & Correctness (Weeks 1-3)

- [x] 1. Memory Management Security Checks
  - Implement page flag verification for user memory access
  - Add helper function `get_page_flags(vaddr: VirtAddr) -> PageFlags`
  - Update security.rs lines 109, 149, 205, 267, 595 with actual flag checks
  - Verify USER and WRITABLE flags are properly set before access
  - _Requirements: 1.1, 1.5_

- [x] 2. Filesystem VFS Parent Tracking
  - Add `parent: Option<Arc<Dentry>>` field to Dentry structure
  - Update path.rs line 100 with proper parent tracking implementation
  - Ensure parent pointers are maintained during dentry operations
  - _Requirements: 2.2_

- [x] 3. Filesystem Cache Writeback Implementation
  - Implement actual flushing in writeback.rs lines 292, 306
  - Create `flush_to_device()` method using BlockDevice trait
  - Integrate with virtio-blk driver for persistent writes
  - Add error handling for flush failures
  - _Requirements: 2.3_

- [x] 4. Process Management Heap Tracking
  - Add `heap_start: VirtAddr` and `heap_end: VirtAddr` fields to Task structure
  - Update exec.rs lines 297, 335, 407 to track and reset heap pointers
  - Implement heap pointer initialization during process creation
  - Implement heap pointer restoration after exec failure
  - _Requirements: 3.1, 3.2_

- [ ] 5. Syscall Process Termination
  - Implement proper exit handling in syscall/mod.rs line 148
  - Add process cleanup: close file descriptors, free memory, remove from task table
  - Notify parent process of child termination
  - Set exit status for parent to retrieve
  - _Requirements: 4.1_

- [ ] 6. Syscall Parent Wakeup on Child Exit
  - Implement parent wakeup logic in syscall/mod.rs lines 559, 569
  - Add `wake_parent()` function using scheduler wakeup mechanism
  - Update waitpid to properly block and wake on child state changes
  - Handle zombie process cleanup after parent retrieves status
  - _Requirements: 4.4_

- [ ] 7. Syscall Child Process Tracking
  - Add `children: Vec<Pid>` field to Task structure
  - Update fork to add child PID to parent's children list
  - Update exit to remove PID from parent's children list
  - Implement proper child tracking in syscall/mod.rs lines 1134, 1142
  - _Requirements: 4.4_

- [ ] 8. Driver virtio-blk Virtqueue Submission
  - Implement actual virtqueue submission in virtio_blk.rs lines 132, 152
  - Create descriptor chain for block requests
  - Fill request header, data buffers, and status buffer
  - Ring doorbell to notify device
  - Handle completion interrupts
  - _Requirements: 5.1, 5.2, 5.3_

- [ ] 9. Signal Kernel Thread Protection
  - Implement kernel thread protection in signal/security.rs line 122
  - Add check to prevent signals from being delivered to kernel threads
  - Identify kernel threads by task flags or PID range
  - Return appropriate error when attempting to signal kernel thread
  - _Requirements: 6.2_

- [ ] 10. Signal Frame Setup
  - Implement signal frame setup in signal/mod.rs line 409
  - Save current user context to user stack
  - Set up return trampoline for signal handler return
  - Modify user RIP to point to signal handler
  - Modify user RSP to point to signal frame
  - _Requirements: 6.3_

- [ ] 11. Device PTY Process Group Signal Delivery
  - Implement process group signal delivery in pty/mod.rs lines 494, 524
  - Create `send_signal_to_group(pgid: Pid, sig: Signal)` function
  - Iterate task table and deliver signal to all processes with matching PGID
  - Handle signal delivery errors gracefully
  - _Requirements: 7.1_

- [ ] 12. Device PTY PGID Retrieval
  - Add `pgid: Pid` field to Task structure
  - Update pty/mod.rs lines 551, 771, 837 to retrieve actual PGID from task
  - Initialize PGID during process creation
  - Update PGID during setpgid syscall
  - _Requirements: 7.2_

- [ ] 13. Userspace mello-term Event Loop
  - Implement main event loop in mello-term/src/main.rs line 166
  - Poll PTY master for output from shell
  - Poll keyboard for user input
  - Handle input/output with proper buffering
  - Implement graceful shutdown on exit signal
  - _Requirements: 8.2_

- [ ] 14. Userspace mello-term Keyboard Input
  - Implement keyboard reading in mello-term/src/input.rs lines 29, 54
  - Use read() syscall to read from PTY master
  - Handle partial reads and buffering
  - Process special keys (Ctrl+C, Ctrl+D, etc.)
  - _Requirements: 8.3_

## Phase 2: Medium Priority Features & Optimization (Weeks 4-6)

- [ ] 15. Memory Management Logging Infrastructure
  - Connect logging to serial output in mm/log.rs line 41
  - Update mm/mod.rs line 457 with proper logging
  - Add logging to allocator.rs lines 285, 289, 312
  - Add logging to pmm.rs lines 119, 178
  - Use `serial_println!` macro for all memory management logs
  - _Requirements: 1.2, 1.3, 1.4_

- [ ] 16. Filesystem Process Information Integration
  - Update /proc filesystem in proc/mod.rs line 963 to get actual TPGID from TTY
  - Implement cmdline retrieval from task structure (line 976)
  - Get actual timing and memory info (line 980)
  - Track buffer cache statistics (line 1000)
  - Track page cache statistics (line 1001)
  - Calculate actual CPU MHz (line 1056)
  - Calculate actual idle time (line 1072)
  - _Requirements: 2.1_

- [ ] 17. Filesystem MFS Time Tracking
  - Implement proper time tracking in mfs/ram/inode.rs lines 162, 219
  - Use kernel timer infrastructure to get current time
  - Update atime, mtime, ctime on file operations
  - Implement attribute setting for time fields
  - _Requirements: 2.4_

- [ ] 18. Filesystem MFS Disk Replay Tree Walk
  - Implement full tree walk in mfs/disk/replay.rs line 156
  - Add child pointer traversal when available
  - Scan extent tree and mark allocated extents (line 188)
  - Ensure all allocated blocks are properly tracked
  - _Requirements: 2.3_

- [ ] 19. Filesystem Syscall Implementation
  - Implement deferred filesystem syscalls in fs/syscalls.rs line 6
  - Add any missing syscall interfaces
  - Ensure proper error handling and validation
  - Test with userspace programs
  - _Requirements: 2.5_

- [ ] 20. Filesystem Cache Background Flusher
  - Implement background flusher thread in cache/writeback.rs line 140
  - Spawn thread when task scheduler supports it
  - Periodically flush dirty pages to disk
  - Implement proper yielding in cache/throttle.rs line 312
  - _Requirements: 2.3_

- [ ] 21. Process Management Page Table Separation - Phase 1
  - Add `page_table: Option<Arc<PageTable>>` field to Task structure
  - Implement per-process page table creation during fork
  - Update process.rs line 1168 to switch page tables on context switch
  - Implement TLB flush on address space switch (line 1171)
  - _Requirements: 3.3_

- [ ] 22. Process Management Page Table Separation - Phase 2
  - Implement full page remapping in exec.rs line 392
  - Use per-process page tables in elf.rs lines 507, 603
  - Implement IPI TLB shootdown for SMP (elf.rs line 439)
  - Ensure proper synchronization across CPUs
  - _Requirements: 3.3, 3.4_

- [ ] 23. Process Management Kernel User Page Mapping
  - Implement `kmap_user_page()` helper function
  - Update process.rs lines 818, 859, 899 to use safe mapping
  - Temporarily map user pages into kernel address space
  - Ensure proper cleanup of temporary mappings
  - _Requirements: 3.4_

- [ ] 24. Syscall Safe User Memory Access
  - Update syscall/mod.rs lines 206, 235 to use kmap_user_page
  - Replace direct pointer access with safe mapping
  - Ensure proper error handling for invalid user addresses
  - Test with various user memory scenarios
  - _Requirements: 4.2_

- [ ] 25. Syscall Buffered I/O for Large Writes
  - Implement proper buffered I/O in syscall/mod.rs line 497
  - Chunk large writes to avoid kernel stack overflow
  - Use temporary kernel buffer for data transfer
  - Optimize for common write sizes
  - _Requirements: 4.3_

- [ ] 26. Syscall Copy-on-Write Optimization
  - Implement COW for fork in syscall/mod.rs lines 704, 722
  - Mark parent and child pages as read-only
  - Add page fault handler for COW in fault.rs
  - Copy page only when write occurs
  - Update page table entries appropriately
  - _Requirements: 4.5_

- [ ] 27. Signal UID Permission Checks
  - Document deferral of UID checks in signal/security.rs lines 50, 77
  - Add TODO comments referencing Phase 10 user management
  - Ensure current permission checks are sufficient for now
  - Plan integration with future user management system
  - _Requirements: 6.1_

- [ ] 28. Signal Handler Address Verification
  - Implement executable page verification in signal/security.rs line 158
  - Check if signal handler address is in executable code pages
  - Verify page has EXECUTE permission
  - Return error if handler address is invalid
  - _Requirements: 6.4_

- [ ] 29. Userspace mello-term ANSI Clear Line Modes
  - Implement clear line modes in mello-term/src/ansi.rs line 180
  - Add support for mode 0 (clear from cursor to end)
  - Add support for mode 1 (clear from start to cursor)
  - Add support for mode 2 (clear entire line)
  - _Requirements: 8.1_

- [ ] 30. Userspace mello-term Non-blocking I/O
  - Implement non-blocking I/O setup in mello-term/src/pty.rs line 106
  - Use fcntl syscall when available
  - Handle EAGAIN/EWOULDBLOCK errors appropriately
  - Fall back to blocking I/O if fcntl not available
  - _Requirements: 8.5_

- [ ] 31. Userspace mellobox df Command
  - Implement filesystem querying in mellobox/src/commands/df.rs line 20
  - Use statfs syscall to query mounted filesystems
  - Display filesystem size, used, available space
  - Format output in human-readable format
  - _Requirements: 8.4_

## Phase 3: Test Implementation (Weeks 7-10)

- [ ]* 32. Test Infrastructure Setup
  - Review existing test infrastructure in tests/ directory
  - Ensure integration test framework is operational
  - Set up test utilities and helpers
  - Document test writing guidelines
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_

- [ ]* 33. VFS Integration Tests - Basic Operations
  - Implement tests in fs_syscalls_api.rs (lines 11, 18, 25, 32, 39, 47, 54, 62)
  - Test open, read, write, close operations
  - Test file creation and deletion
  - Test directory operations
  - _Requirements: 9.1_

- [ ]* 34. VFS Integration Tests - Special Nodes
  - Implement tests in fs_special_nodes.rs (lines 12, 20, 27, 34, 42)
  - Test device nodes
  - Test symbolic links
  - Test FIFOs
  - _Requirements: 9.1_

- [ ]* 35. VFS Integration Tests - Extended Attributes
  - Implement tests in fs_xattr.rs (lines 11, 18, 25, 32, 39)
  - Test xattr get/set operations
  - Test xattr list operation
  - Test xattr remove operation
  - _Requirements: 9.1_

- [ ]* 36. VFS Integration Tests - Stat Compatibility
  - Implement tests in fs_stat_compat.rs (lines 12, 20, 27)
  - Test stat syscall
  - Test fstat syscall
  - Test lstat syscall
  - _Requirements: 9.1_

- [ ]* 37. VFS Correctness Tests
  - Implement tests in fs_vfs_correctness.rs (lines 17)
  - Test path resolution
  - Test mount point handling
  - Test dentry cache correctness
  - _Requirements: 9.1_

- [ ]* 38. File Descriptor Operations Tests
  - Implement tests in fs_fd_ops.rs (lines 16)
  - Test dup/dup2 operations
  - Test fcntl operations
  - Test file descriptor inheritance
  - _Requirements: 9.1_

- [ ]* 39. Directory Operations Tests
  - Implement tests in fs_dir_ops.rs (lines 16)
  - Test readdir operation
  - Test directory creation/deletion
  - Test directory traversal
  - _Requirements: 9.1_

- [ ]* 40. Cache Behavior Tests - Page Cache
  - Implement tests in fs_cache_behavior.rs (lines 10, 20, 47)
  - Test page cache hit/miss behavior
  - Test page cache eviction
  - Test page cache coherency
  - _Requirements: 9.2_

- [ ]* 41. Cache Behavior Tests - Buffer Cache
  - Implement tests in fs_cache_behavior.rs (lines 30, 39)
  - Test buffer cache hit/miss behavior
  - Test buffer cache write-through
  - Test buffer cache coherency
  - _Requirements: 9.2_

- [ ]* 42. Cache Performance Tests - Read Performance
  - Implement tests in fs_cache_perf.rs (lines 7, 19)
  - Benchmark cached vs uncached reads
  - Measure cache hit rate
  - Verify performance targets
  - _Requirements: 9.2_

- [ ]* 43. Cache Performance Tests - Write Performance
  - Implement tests in fs_cache_perf.rs (lines 30, 40, 50, 60)
  - Benchmark writeback performance
  - Test write coalescing
  - Measure write throughput
  - _Requirements: 9.2_

- [ ]* 44. MFS RAM Correctness Tests
  - Implement tests in mfs_ram_correctness.rs (line 10)
  - Test basic file operations
  - Test directory operations
  - Test extended attributes
  - _Requirements: 9.3_

- [ ]* 45. MFS RAM Performance Tests
  - Implement tests in mfs_ram_perf.rs (line 10)
  - Benchmark file creation/deletion
  - Benchmark read/write performance
  - Compare with expected performance targets
  - _Requirements: 9.3_

- [ ]* 46. MFS Disk Fault Tests - Corruption Handling
  - Implement tests in fs_faults.rs (lines 29, 56, 211, 237, 264)
  - Test handling of corrupted metadata
  - Test checksum verification
  - Test recovery from corruption
  - _Requirements: 9.3_

- [ ]* 47. MFS Disk Fault Tests - I/O Errors
  - Implement tests in fs_faults.rs (lines 82, 109)
  - Test handling of block device read errors
  - Test handling of block device write errors
  - Verify proper error propagation
  - _Requirements: 9.3_

- [ ]* 48. MFS Disk Fault Tests - Resource Exhaustion
  - Implement tests in fs_faults.rs (lines 135, 161, 187, 288, 312, 337, 366, 390)
  - Test out-of-space conditions
  - Test memory allocation failures
  - Test transaction group limits
  - _Requirements: 9.3_

- [ ]* 49. Userland Smoke Tests - Basic Commands
  - Implement tests in userland_smoke.rs (lines 11, 18, 25, 32, 39)
  - Test echo command
  - Test ls command
  - Test cat command
  - Test pwd command
  - _Requirements: 9.4_

- [ ]* 50. Userland Smoke Tests - File Operations
  - Implement tests in userland_smoke.rs (lines 46, 53, 60, 67)
  - Test touch command
  - Test rm command
  - Test mkdir command
  - Test cp command
  - _Requirements: 9.4_

- [ ]* 51. Userland Smoke Tests - Process Operations
  - Implement tests in userland_smoke.rs (lines 74, 81, 88, 95)
  - Test ps command
  - Test kill command
  - Test process creation
  - Test process termination
  - _Requirements: 9.4_

- [ ]* 52. Memory Mapping Coherence Tests
  - Implement tests in fs_mmap_coherence.rs
  - Test mmap read/write coherency with file I/O
  - Test shared memory mappings
  - Test private memory mappings
  - _Requirements: 9.5_

## Phase 4: Documentation & Cleanup (Week 11)

- [ ] 53. Update Architecture Documentation
  - Update docs/architecture/ with completed implementations
  - Document new memory management features
  - Document filesystem improvements
  - Document process management enhancements
  - _Requirements: 10.3_

- [ ] 54. Update Technical Debt Tracking
  - Update TECHNICAL_DEBT_CATEGORIZED.md with completion status
  - Mark all resolved TODOs as complete
  - Document any legitimately deferred items
  - Update priority classifications
  - _Requirements: 10.2_

- [ ] 55. Remove Completed TODO Comments
  - Scan codebase for resolved TODO comments
  - Remove TODO comments for completed work
  - Ensure no stale TODO comments remain
  - Verify with grep/search tools
  - _Requirements: 10.1_

- [ ] 56. Update Roadmap
  - Update roadmap.md with technical debt resolution completion
  - Document any deferred items and reasons
  - Update phase completion status
  - Plan next development phase
  - _Requirements: 10.5_

- [ ] 57. Final Regression Testing
  - Run full test suite: `cargo test --workspace`
  - Run all integration tests in tools/testing/
  - Boot test in QEMU with all features
  - Verify no regressions introduced
  - _Requirements: 12.3_

- [ ] 58. Performance Verification
  - Run performance benchmarks
  - Compare with baseline performance
  - Verify no performance regressions
  - Document any performance improvements
  - _Requirements: 12.4_

- [ ] 59. Security Audit
  - Review all security-related changes
  - Verify user pointer validation
  - Test privilege escalation scenarios
  - Verify signal permission checks
  - Test page table isolation
  - _Requirements: 12.5_

- [ ] 60. Create Completion Summary
  - Summarize all resolved TODOs
  - Document key improvements
  - List any remaining deferred items
  - Provide recommendations for future work
  - _Requirements: 10.4_
