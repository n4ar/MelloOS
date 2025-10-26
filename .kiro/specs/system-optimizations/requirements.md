# System Optimizations & Advanced Features - Requirements Document

**Phase:** 9+ (Post Phase 8 Completion)  
**Status:** Planning  
**Priority:** P0-P2 (Mixed)  
**Estimated Timeline:** 8-12 weeks

---

## Introduction

This requirements document specifies the deferred optimization features and advanced capabilities for MelloOS. These features require major architectural changes and will significantly improve system performance, memory efficiency, security, and POSIX compliance. The requirements are organized into ten major feature areas, each with specific user stories and EARS-compliant acceptance criteria.

## Glossary

- **System**: The MelloOS kernel and its subsystems
- **Process**: A user-space or kernel-space execution context with its own address space
- **Page Table**: A hierarchical data structure mapping virtual addresses to physical addresses
- **CR3**: The x86_64 control register containing the physical address of the page table root
- **COW (Copy-on-Write)**: A memory optimization technique where pages are shared until written
- **TLB (Translation Lookaside Buffer)**: A CPU cache for virtual-to-physical address translations
- **IPI (Inter-Processor Interrupt)**: A mechanism for one CPU to interrupt another CPU
- **Page Fault**: A hardware exception triggered when accessing an unmapped or protected page
- **Refcount**: A reference counter tracking how many entities share a resource
- **Wait Queue**: A data structure holding processes blocked waiting for an event
- **Kernel Thread**: A thread executing kernel code without a user-space context
- **Dirty Page**: A memory page that has been modified but not yet written to disk
- **Flusher Thread**: A kernel thread responsible for writing dirty pages to storage
- **UID (User ID)**: A numeric identifier for a user account
- **Signal Handler**: A function executed when a process receives a signal
- **Symlink (Symbolic Link)**: A file that contains a reference to another file or directory
- **Dentry**: A directory entry in the VFS layer
- **LZ4**: A fast compression algorithm
- **Zstd**: A compression algorithm with configurable compression levels

---

## Requirements

### Requirement 1: Copy-on-Write (COW) for Fork

**User Story:** As a system developer, I want fork() to use copy-on-write semantics, so that process creation is faster and uses less memory.

#### Acceptance Criteria

1.1. WHEN a Process invokes fork(), THE System SHALL mark all writable pages in both parent and child page tables as copy-on-write and read-only.

1.2. WHEN a Process writes to a copy-on-write page, THE System SHALL trigger a page fault.

1.3. WHEN the System handles a copy-on-write page fault, THE System SHALL allocate a new physical page, copy the original page contents to the new page, update the faulting process page table entry to reference the new page with writable permissions, and clear the copy-on-write flag.

1.4. THE System SHALL maintain an atomic reference count for each physical page shared via copy-on-write.

1.5. WHEN a Process writes to a copy-on-write page with reference count equal to one, THE System SHALL mark the page as writable and clear the copy-on-write flag without allocating a new page.

1.6. WHEN the reference count for a physical page reaches zero, THE System SHALL deallocate the physical page.

1.7. THE System SHALL complete fork() operations in less than one tenth the time required for full memory copying for processes with virtual memory size greater than ten megabytes.

### Requirement 2: Per-Process Page Tables

**User Story:** As a system developer, I want each process to have its own page table, so that processes are isolated from each other and memory security is enforced.

#### Acceptance Criteria

2.1. THE System SHALL allocate a unique page table root for each Process.

2.2. THE System SHALL store the physical address of each Process page table root in the Process control block.

2.3. THE System SHALL map kernel address space from 0xFFFF800000000000 to 0xFFFFFFFFFFFFFFFF identically in all Process page tables.

2.4. THE System SHALL map user address space from 0x0000000000000000 to 0x00007FFFFFFFFFFF uniquely per Process.

2.5. WHEN the System performs a context switch to a Process, THE System SHALL load the Process page table root physical address into the CR3 register.

2.6. WHEN the System writes to the CR3 register, THE System SHALL invalidate all non-global TLB entries.

2.7. WHEN a Process invokes fork(), THE System SHALL clone the parent Process page table structure for the child Process.

2.8. THE System SHALL prevent any Process from accessing memory mapped in another Process user address space.

2.9. THE System SHALL complete context switch operations including CR3 update in less than one thousand CPU cycles.

### Requirement 3: Process Blocking and Wakeup Mechanism

**User Story:** As a system developer, I want processes to block when waiting for events, so that CPU resources are not wasted on busy-waiting.

#### Acceptance Criteria

3.1. THE System SHALL support a BLOCKED task state in addition to READY, RUNNING, and ZOMBIE states.

3.2. WHEN a Process blocks waiting for an event, THE System SHALL transition the Process to BLOCKED state and remove it from the scheduler run queue.

3.3. THE System SHALL maintain a wait queue data structure associating blocked processes with their blocking conditions.

3.4. WHEN a blocking condition is satisfied, THE System SHALL transition all processes waiting on that condition from BLOCKED to READY state and add them to the scheduler run queue.

3.5. WHEN a Process invokes sys_wait for a child Process that has not exited, THE System SHALL block the parent Process until the child Process exits.

3.6. WHEN a child Process exits, THE System SHALL wake all processes blocked waiting for that child Process.

3.7. WHEN a Process receives a signal while in BLOCKED state, THE System SHALL transition the Process to READY state.

3.8. WHEN no Process is in READY or RUNNING state, THE System SHALL execute the CPU halt instruction to reduce power consumption.

### Requirement 4: Memory Mapping Improvements

**User Story:** As a system developer, I want mprotect and msync to work correctly, so that processes can change memory permissions and synchronize file-backed mappings.

#### Acceptance Criteria

4.1. WHEN a Process invokes mprotect with a virtual address range and permission flags, THE System SHALL update all page table entries in that range to reflect the new permissions.

4.2. WHEN the System updates page table entries via mprotect, THE System SHALL invalidate TLB entries for the affected virtual address range on all CPUs.

4.3. WHEN a Process invokes msync with the MS_SYNC flag on a file-backed memory mapping, THE System SHALL write all dirty pages in the specified range to the backing file and wait for write completion before returning.

4.4. WHEN a Process invokes msync with the MS_ASYNC flag on a file-backed memory mapping, THE System SHALL schedule all dirty pages in the specified range for writing to the backing file and return immediately.

4.5. WHEN a Process creates a file-backed memory mapping with MAP_SHARED flag, THE System SHALL ensure that modifications to the mapped memory are visible to all processes mapping the same file region.

4.6. WHEN a Process creates a file-backed memory mapping with MAP_PRIVATE flag, THE System SHALL use copy-on-write semantics for the mapping.

4.7. WHEN a Process accesses an unmapped page within a file-backed memory mapping, THE System SHALL load the corresponding file data into a physical page and update the page table entry.

4.8. WHEN a Process creates an anonymous memory mapping with MAP_GROWSDOWN flag, THE System SHALL automatically extend the mapping downward when the Process accesses addresses below the current mapping boundary.

### Requirement 5: Background Writeback System

**User Story:** As a system developer, I want dirty pages to be written to disk automatically in the background, so that data is not lost and I/O performance is optimized.

#### Acceptance Criteria

5.1. THE System SHALL support creation of kernel threads that execute kernel code without user address space.

5.2. THE System SHALL create a background flusher kernel thread during system initialization.

5.3. THE flusher kernel thread SHALL wake every thirty seconds and scan for dirty pages.

5.4. WHEN the flusher kernel thread finds a dirty page with age greater than thirty seconds, THE System SHALL write the page to its backing storage device.

5.5. THE System SHALL record the timestamp when each page becomes dirty.

5.6. WHEN the System writes multiple dirty pages from the same file with contiguous file offsets, THE System SHALL coalesce the writes into a single I/O operation.

5.7. WHEN a Process invokes sys_sync, THE System SHALL immediately write all dirty pages to their backing storage devices and wait for completion.

5.8. WHEN a Process invokes sys_fsync with a file descriptor, THE System SHALL write all dirty pages associated with that file to storage and wait for completion.

5.9. WHEN a Process invokes sys_fdatasync with a file descriptor, THE System SHALL write all dirty data pages associated with that file to storage without writing metadata pages.

### Requirement 6: Signal Security Enhancements

**User Story:** As a system developer, I want signal delivery to be restricted by user permissions, so that processes cannot interfere with other users' processes.

#### Acceptance Criteria

6.1. THE System SHALL assign a numeric user identifier to each Process.

6.2. WHEN a Process attempts to send a signal to another Process, THE System SHALL verify that the sender Process user identifier matches the target Process user identifier.

6.3. WHEN a Process with user identifier zero attempts to send a signal to any Process, THE System SHALL allow the signal delivery.

6.4. THE System SHALL mark kernel threads with a kernel thread flag in the Process control block.

6.5. WHEN a user-space Process attempts to send a signal to a kernel thread, THE System SHALL reject the signal and return an error code.

6.6. WHEN a Process registers a signal handler, THE System SHALL verify that the handler address is within a memory region mapped with execute permission.

6.7. IF a Process attempts to register a signal handler at an address without execute permission, THEN THE System SHALL reject the registration and return an error code.

### Requirement 7: TLB Shootdown for SMP

**User Story:** As a system developer, I want TLB entries to be invalidated on all CPUs when page tables change, so that no CPU uses stale address translations.

#### Acceptance Criteria

7.1. WHEN the System modifies a page table entry, THE System SHALL send an inter-processor interrupt to all other CPUs.

7.2. WHEN a CPU receives a TLB shootdown inter-processor interrupt, THE System SHALL invalidate the specified TLB entries on that CPU.

7.3. WHEN the System sends TLB shootdown inter-processor interrupts, THE System SHALL wait for acknowledgment from all target CPUs before returning.

7.4. WHEN the System must invalidate multiple TLB entries within a short time period, THE System SHALL batch the invalidations into a single inter-processor interrupt.

7.5. THE System SHALL track which CPUs have accessed each Process address space.

7.6. WHEN the System sends TLB shootdown inter-processor interrupts for a Process, THE System SHALL send interrupts only to CPUs that have accessed that Process address space.

7.7. THE System SHALL complete TLB shootdown operations with overhead less than five percent of total system execution time under normal workload.

### Requirement 8: Real Compression Algorithms

**User Story:** As a system developer, I want to use industry-standard compression algorithms, so that compressed data is compatible with external tools and achieves good compression ratios.

#### Acceptance Criteria

8.1. THE System SHALL support LZ4 compression algorithm for data compression.

8.2. THE System SHALL support Zstd compression algorithm for data compression.

8.3. THE System SHALL decompress LZ4-compressed data at a rate greater than five hundred megabytes per second on reference hardware.

8.4. THE System SHALL support configurable compression levels for Zstd compression ranging from one to twenty-two.

8.5. WHEN the System compresses data with LZ4 or Zstd, THE System SHALL produce output compatible with standard LZ4 and Zstd decompression tools.

8.6. THE System SHALL allow per-file configuration of compression algorithm selection.

8.7. THE System SHALL provide a default compression algorithm configuration that can be set system-wide.

### Requirement 9: /proc Filesystem Improvements

**User Story:** As a system developer, I want /proc filesystem to provide accurate system information, so that monitoring tools can display correct process and system statistics.

#### Acceptance Criteria

9.1. THE System SHALL track user-mode CPU time and kernel-mode CPU time for each Process.

9.2. THE System SHALL update Process CPU time counters on every context switch.

9.3. THE System SHALL record the creation timestamp for each Process.

9.4. THE System SHALL calculate resident set size for each Process by counting mapped physical pages.

9.5. THE System SHALL calculate virtual memory size for each Process by summing all virtual memory region sizes.

9.6. THE System SHALL calibrate the timestamp counter frequency during system initialization using a reference timer.

9.7. THE System SHALL detect CPU features using the CPUID instruction and expose them via /proc/cpuinfo.

9.8. THE System SHALL track buffer cache memory usage and expose it via /proc/meminfo.

9.9. THE System SHALL track page cache memory usage and expose it via /proc/meminfo.

9.10. THE System SHALL calculate CPU idle time by measuring time spent in the idle task.

9.11. THE System SHALL expose the controlling terminal device for each Process via /proc/[pid]/stat.

9.12. THE System SHALL expose the foreground process group identifier for each Process via /proc/[pid]/stat.

### Requirement 10: Path Resolution Improvements

**User Story:** As a system developer, I want path resolution to handle parent directories and symbolic links correctly, so that file operations work as expected in all cases.

#### Acceptance Criteria

10.1. THE System SHALL store a reference to the parent directory entry in each directory entry structure.

10.2. WHEN the System resolves the path component "..", THE System SHALL return the parent directory entry.

10.3. WHEN the System resolves ".." at a mount point, THE System SHALL return the parent directory of the mount point in the parent filesystem.

10.4. THE System SHALL support symbolic link file types.

10.5. WHEN the System resolves a path containing a symbolic link, THE System SHALL read the symbolic link target and continue resolution from the target path.

10.6. THE System SHALL detect symbolic link loops by limiting symbolic link traversal depth to forty levels.

10.7. IF the System encounters more than forty symbolic links during path resolution, THEN THE System SHALL return an error code.

10.8. THE System SHALL canonicalize paths by removing redundant slash characters.

10.9. THE System SHALL canonicalize paths by resolving "." components to the current directory.

10.10. THE System SHALL canonicalize paths by resolving ".." components to parent directories.

---

## Implementation Priority

The requirements are organized into four implementation phases:

**Phase 9A (Foundation):** Requirements 2, 7 - Per-process page tables and TLB shootdown  
**Phase 9B (Memory Optimizations):** Requirements 1, 4 - Copy-on-write and memory mapping improvements  
**Phase 9C (Process Management):** Requirements 3, 5 - Blocking/wakeup and background writeback  
**Phase 9D (Polish):** Requirements 6, 8, 9, 10 - Security, compression, /proc, and path resolution

---

## Cross-Cutting Requirements

### Performance Requirements

PERF-1. THE System SHALL complete fork() operations in less than one millisecond for processes with ten megabyte virtual memory size.

PERF-2. THE System SHALL complete context switch operations in less than one thousand CPU cycles.

PERF-3. THE System SHALL achieve I/O throughput at least fifty percent greater than baseline after implementing background writeback.

### Security Requirements

SEC-1. THE System SHALL prevent any Process from reading or writing memory belonging to another Process with different user identifier.

SEC-2. THE System SHALL validate all user-provided pointers before dereferencing them in kernel code.

SEC-3. THE System SHALL prevent execution of code in memory pages not marked as executable.

### Reliability Requirements

REL-1. THE System SHALL persist all dirty pages to storage within sixty seconds of modification under normal operation.

REL-2. THE System SHALL maintain data consistency across system crashes by using atomic operations for critical metadata updates.

REL-3. THE System SHALL detect and report memory corruption via checksums or other integrity mechanisms.

---

## Acceptance Criteria

The System Optimizations & Advanced Features specification is complete when:

- [ ] All requirements 1 through 10 are implemented and verified
- [ ] All performance requirements (PERF-1 through PERF-3) are met
- [ ] All security requirements (SEC-1 through SEC-3) are verified
- [ ] All reliability requirements (REL-1 through REL-3) are tested
- [ ] Integration test suite passes with zero failures
- [ ] Performance benchmarks meet or exceed targets
- [ ] Security audit identifies no critical vulnerabilities
- [ ] Documentation is complete and accurate
- [ ] Code review is approved by maintainers

---

**Status:** Requirements complete, ready for design phase
