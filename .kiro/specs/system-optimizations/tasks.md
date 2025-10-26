# System Optimizations & Advanced Features - Tasks

**Phase:** 9+ (Post Phase 8 Completion)  
**Status:** Planning  
**Related:** requirements.md, design.md

---

## Phase 9A: Foundation (Weeks 1-3)

- [x] **9A.1: Per-Process Page Tables - Data Structures**
  - Create `Process` struct in `kernel/src/user/process.rs` with fields: pid, cr3, parent_pid, state
  - Add `cr3: PhysAddr` field to Process struct for page table root
  - Implement `PageTableRef` struct wrapper with atomic refcount in `kernel/src/mm/paging.rs`
  - Implement `alloc_page_table()` function that allocates and zeros a new page table
  - Implement `free_page_table()` function with refcount checking
  - Implement `clone_page_table()` and `clone_page_table_hierarchy()` functions for fork support
  - Add process table in `kernel/src/user/process.rs` to track all processes
  - _Requirements: 2.2, 1.4, 2.1, 2.7_

---

- [x] **9A.2: Per-Process Page Tables - Kernel Mappings**
  - Define kernel address space constants in `kernel/src/mm/mod.rs`: KERNEL_BASE = 0xFFFF800000000000
  - Create `KERNEL_PAGE_TABLE_TEMPLATE` static in `kernel/src/mm/paging.rs`
  - Implement `init_kernel_template()` to map kernel code/data/heap with GLOBAL flag
  - Map direct physical memory region (HHDM) in template
  - Implement `copy_kernel_mappings(dest_pml4, src_pml4)` to copy upper half entries
  - Implement `alloc_page_table_with_kernel_mappings()` convenience function
  - Add `PageMapper::new_for_process()` to create process-specific page mappers
  - _Requirements: 2.3_

---

- [x] **9A.3: Per-Process Page Tables - User Mappings**
  - Define user address space constants: USER_BASE = 0x0, USER_LIMIT = 0x00007FFFFFFFFFFF
  - Update `load_elf()` in `kernel/src/user/elf.rs` to accept Process parameter
  - Modify ELF loader to map segments into process-specific page table instead of global
  - Update `Task::add_memory_region()` to map pages into process page table
  - Update user stack allocation in `kernel/src/user/launch.rs` to use process page table
  - Ensure all user mappings have USER flag set in page table entries
  - _Requirements: 2.4, 2.8_

---

- [x] **9A.4: Per-Process Page Tables - Context Switch**
  - Add CR3 save/restore to `CpuContext` struct in `kernel/src/sched/context.rs`
  - Update `context_switch` assembly to save/restore CR3
  - Add optimization to skip CR3 write if unchanged
  - Verify TLB is automatically flushed on CR3 write (no explicit invlpg needed)
  - Test process isolation by attempting cross-process memory access
  - _Requirements: 2.5, 2.6, 2.9, PERF-2, SEC-1_

---

- [x] **9A.5: Per-Process Page Tables - Fork Integration**
  - Update `sys_fork()` in `kernel/src/sys/syscall.rs` to allocate new page table for child
  - Call `clone_page_table_hierarchy()` to copy parent's full page table structure
  - Copy user space mappings (lower half) to child page table
  - Share kernel space mappings (upper half) between parent and child
  - Set child Process cr3 field to new page table physical address
  - Test fork creates isolated address spaces
  - _Requirements: 2.7, 2.3_

---

- [x] **9A.6: TLB Shootdown - IPI Infrastructure**
  - Create `TlbShootdownRequest` struct in `kernel/src/mm/tlb.rs` with fields: start_addr, end_addr, ack_count
  - Implement `send_tlb_shootdown_ipi(cpu_id, request)` using APIC IPI mechanism
  - Define TLB_SHOOTDOWN_VECTOR (e.g., 0xF0) in `kernel/src/arch/x86_64/apic/mod.rs`
  - Implement `handle_tlb_shootdown_ipi()` interrupt handler
  - Handler should: flush TLB range, increment ack_count atomically
  - Register TLB shootdown handler in IDT
  - Add `wait_for_acks(request, target_count)` to wait for all CPUs to acknowledge
  - _Requirements: 7.1, 7.2, 7.3_

---

- [x] **9A.7: TLB Shootdown - Integration**
  - Implement `tlb_shootdown(start_addr, end_addr)` in `kernel/src/mm/tlb.rs`
  - Function should: flush local TLB, send IPI to other CPUs, wait for acks
  - Add per-CPU tracking: `CPU_PROCESS_MAP` to track which CPUs accessed which processes
  - Optimize to only send IPI to CPUs that accessed the process (Requirement 7.6)
  - Call `tlb_shootdown()` after page table modifications in `PageMapper`
  - Add batch optimization: coalesce multiple flushes within 1ms into single IPI (Requirement 7.4)
  - _Requirements: 7.1, 7.2, 7.4, 7.6, 7.7_

---

## Phase 9B: Memory Optimizations (Weeks 4-6)

- [ ] Phase 9B: Memory Optimizations (Weeks 4-6)
- [x] **9B.1: Copy-on-Write - Page Refcounting**
  - Create `PageRefcount` struct in `kernel/src/mm/refcount.rs` with HashMap<PhysAddr, AtomicUsize>
  - Implement `inc_refcount(page: PhysAddr)` using atomic fetch_add
  - Implement `dec_refcount(page: PhysAddr) -> usize` using atomic fetch_sub
  - Implement `get_refcount(page: PhysAddr) -> usize` using atomic load
  - Free page when refcount reaches zero in dec_refcount (Requirement 1.6)
  - Use SpinLock for HashMap access, AtomicUsize for count operations
  - Create global `PAGE_REFCOUNT` static instance
  - _Requirements: 1.4, 1.6_

---

- [x] **9B.2: Copy-on-Write - Page Table Marking**
  - Define `COW` flag as bit 9 (available bit) in `PageTableFlags` in `kernel/src/mm/paging.rs`
  - Add `set_cow(&mut self)` method to `PageTableEntry` to set COW bit and clear WRITABLE
  - Add `clear_cow(&mut self)` method to `PageTableEntry` to clear COW bit
  - Add `is_cow(&self) -> bool` method to `PageTableEntry` to check COW bit
  - Update `sys_fork()` to mark all writable user pages as COW (Requirement 1.1)
  - Increment refcount for each COW page in fork
  - _Requirements: 1.1_

---

- [x] **9B.3: Copy-on-Write - Page Fault Handler**
  - Update `page_fault_handler()` in `kernel/src/arch/x86_64/fault.rs` to detect COW faults
  - Check if fault is write to COW page: error_code has WRITE bit and PTE has COW bit
  - Implement `handle_cow_fault(fault_addr: VirtAddr) -> Result<(), FaultError>`
  - Get refcount for faulting page
  - If refcount == 1: clear COW, set WRITABLE, flush TLB (Requirement 1.5)
  - If refcount > 1: alloc new page, copy data, update PTE, dec old refcount (Requirement 1.3)
  - Handle allocation failure gracefully (kill process with SIGSEGV)
  - _Requirements: 1.2, 1.3, 1.4, 1.5_

---

- [x] **9B.4: Copy-on-Write - Fork Integration**
  - Remove any existing page copying code from `sys_fork()`
  - Verify fork marks all writable user pages as COW (already done in 9B.2)
  - Ensure both parent and child pages marked read-only with COW bit (Requirement 1.1)
  - Test fork performance: should be <1ms for 10MB process (Requirement 1.7, PERF-1)
  - Verify memory usage: child should share pages until write
  - Run fork stress test: 100 forks should complete quickly
  - _Requirements: 1.1, 1.7, PERF-1_

---

- [x] **9B.5: Memory Mapping - mprotect Implementation**
  - Update `sys_mprotect()` in `kernel/src/mm/mmap.rs` to modify actual page table entries
  - Validate address range is page-aligned and within valid mapping
  - Walk page tables and update PTE flags for each page in range (Requirement 4.1)
  - Set WRITABLE, USER, NO_EXECUTE bits based on prot flags
  - Flush local TLB for modified range using invlpg
  - Call `tlb_shootdown(start, end)` to invalidate on other CPUs (Requirement 4.2)
  - Return EINVAL for invalid addresses, ENOMEM for unmapped regions
  - _Requirements: 4.1, 4.2_

---

- [x] **9B.6: Memory Mapping - msync Implementation**
  - Add dirty bit tracking to page cache in `kernel/src/fs/cache/page_cache.rs`
  - Implement `get_dirty_pages(inode, start_page, end_page)` in page cache
  - Update `sys_msync()` in `kernel/src/mm/mmap.rs` to actually flush pages
  - For MS_SYNC: write dirty pages and wait for I/O completion (Requirement 4.3)
  - For MS_ASYNC: schedule writes and return immediately (Requirement 4.4)
  - Mark pages clean after successful write
  - Return EINVAL for anonymous mappings (nothing to sync)
  - _Requirements: 4.3, 4.4_

---

- [x] **9B.7: Memory Mapping - File-Backed Mappings**
  - Update page fault handler to detect faults in file-backed mappings
  - Implement `handle_file_mapping_fault(addr, mapping)` in `kernel/src/mm/mmap.rs`
  - Allocate physical page on fault
  - Read file data from backing file at correct offset (Requirement 4.7)
  - For MAP_SHARED: map page writable, register in shared page cache (Requirement 4.5)
  - For MAP_PRIVATE: map page with COW semantics (Requirement 4.6)
  - Update page cache to track shared file-backed pages
  - Implement MAP_GROWSDOWN for automatic stack expansion (Requirement 4.8)
  - _Requirements: 4.5, 4.6, 4.7, 4.8_

---

## Phase 9C: Process Management (Weeks 7-9)

- [ ] Phase 9C: Process Management (Weeks 7-9)
- [ ] **Task 9C.1: Blocking - Task States**  
  Note: TaskState already has Blocked variant. This task updates blocking infrastructure.
  - Verify `TaskState::Blocked` exists in `kernel/src/sched/task.rs` (already present)
  - Create `BlockReason` enum with variants: WaitChild(Pid), WaitAnyChild, WaitIO(Fd), Sleep(u64)
  - Add `block_reason: Option<BlockReason>` field to Task struct
  - Implement `set_task_blocked(task_id, reason)` in scheduler
  - Implement `set_task_ready(task_id)` to transition from Blocked to Ready
  - Update scheduler to skip Blocked tasks in runqueue

---

- [ ] **Task 9C.2: Blocking - Wait Queues**  
  Implement wait queue data structure for blocking processes.
  - Create `WaitQueue` struct in `kernel/src/sched/wait_queue.rs` with Vec<(TaskId, BlockReason)>
  - Add SpinLock for SMP-safe access
  - Implement `add_to_wait_queue(&mut self, task_id, reason)` (Requirement 3.3)
  - Function should: set task state to Blocked, remove from runqueue
  - Implement `wake_from_wait_queue(&mut self, condition)` (Requirement 3.4)
  - Function should: find matching tasks, set to Ready, add to runqueue
  - Implement `remove_from_wait_queue(&mut self, task_id)` for signal interruption
  - Create global wait queues: CHILD_WAIT_QUEUE, IO_WAIT_QUEUE

---

- [ ] **Task 9C.3: Blocking - Scheduler Integration**  
  Integrate blocking into the task scheduler.
  - Update `schedule()` in `kernel/src/sched/mod.rs` to skip Blocked tasks
  - Implement `yield_and_block(reason: BlockReason)` function
  - Function should: set current task Blocked, add to wait queue, call schedule()
  - Update `add_to_runqueue()` to verify task is Ready before adding
  - Implement idle task behavior: halt CPU when no Ready tasks (Requirement 3.8)
  - Add `hlt` instruction in idle loop to reduce power consumption

---

- [ ] **Task 9C.4: Blocking - sys_wait Implementation**  
  Implement blocking wait syscall for process synchronization.
  - Implement `sys_wait_blocking(child_pid)` in `kernel/src/sys/syscall.rs`
  - Check for zombie children first, return immediately if found
  - If no zombie: block parent with BlockReason::WaitChild(pid) (Requirement 3.5)
  - Call `yield_and_block()` to give up CPU
  - Update `sys_exit()` to wake parent process (Requirement 3.6)
  - In exit: call `wake_from_wait_queue(WaitChild(current_pid))`
  - Handle SIGCHLD delivery to parent on child exit

---

- [ ] **Task 9C.5: Kernel Threading - Infrastructure**  
  Implement kernel threads for background tasks.
  - Create `KernelThread` struct in `kernel/src/sched/kthread.rs` with fields: id, name, entry, stack
  - Add `is_kernel_thread: bool` flag to Task struct
  - Implement `spawn_kernel_thread(name, entry: fn() -> !)` (Requirement 5.1)
  - Allocate 16KB kernel stack for thread
  - Create Task with no user address space (cr3 = kernel page table)
  - Add kernel thread to scheduler runqueue
  - Implement kernel thread termination: remove from scheduler, free stack
  - Mark kernel threads protected from user signals (Requirement 6.5)

---

- [ ] **Task 9C.6: Background Writeback - Dirty Tracking**  
  Implement dirty page tracking for background writeback.
  - Update `DirtyPageTracker` in `kernel/src/fs/cache/writeback.rs` (skeleton exists)
  - Add HashMap<(InodeId, PageNum), DirtyPage> with RwLock
  - Implement `mark_dirty(inode, page_num)` to record page and timestamp (Requirement 5.5)
  - Implement `get_old_dirty_pages(age_ms) -> Vec<DirtyPage>` (Requirement 5.4)
  - Function should: filter pages older than threshold, return sorted by inode
  - Integrate with page cache: call mark_dirty when page modified
  - Add `get_all_dirty_pages()` for sync syscall

---

- [ ] **Task 9C.7: Background Writeback - Flusher Thread**  
  Create background flusher kernel thread for writeback.
  - Implement `writeback_flusher_thread()` in `kernel/src/fs/cache/writeback.rs`
  - Thread loop: sleep 30 seconds, get old dirty pages, write batches (Requirement 5.3, 5.4)
  - Spawn flusher thread during filesystem initialization (Requirement 5.2)
  - Implement `coalesce_pages(dirty_pages)` to group adjacent pages (Requirement 5.6)
  - Implement `write_batch(batch)` to write contiguous pages in single I/O
  - Use existing `coalesce_dirty_pages()` function from writeback.rs
  - Handle force_flush flag for immediate sync

---

- [ ] **Task 9C.8: Background Writeback - Sync Integration**  
  Integrate sync syscalls with background writeback.
  - Implement `sys_sync()` in `kernel/src/fs/syscalls.rs` (Requirement 5.7)
  - Function should: get all dirty pages, write immediately, wait for completion
  - Implement `sys_fsync(fd)` to flush specific file (Requirement 5.8)
  - Function should: get dirty pages for file's inode, write and wait
  - Implement `sys_fdatasync(fd)` to flush data only (Requirement 5.9)
  - Function should: skip metadata pages, only flush data pages
  - Set force_flush flag to trigger immediate flusher wakeup

---

## Phase 9D: Polish (Weeks 10-12)

- [ ] **Task 9D.1: Signal Security - UID System**  
  Implement user ID system for process permissions.
  - Create `User` struct in `kernel/src/user/mod.rs` with fields: uid, gid, euid, egid
  - Add `user: User` field to Process struct (Requirement 6.1)
  - Implement `sys_getuid()` to return current process UID
  - Implement `sys_geteuid()` to return effective UID
  - Implement `sys_setuid(uid)` - only root (UID 0) can change (Requirement 6.3)
  - Implement `sys_seteuid(euid)` - can set to real UID or if root
  - Initialize init process (PID 1) with UID 0 (root)
  - Initialize other processes with UID 1000 (default user)

---

- [ ] **Task 9D.2: Signal Security - Permission Checks**  
  Integrate UID-based permission checks into signal delivery.
  - Update `check_signal_permission()` in `kernel/src/signal/security.rs` to use UIDs
  - Check sender.user.euid == target.user.uid for permission (Requirement 6.2)
  - Allow root (UID 0) to signal any process (Requirement 6.3)
  - Keep existing session-based checks for job control signals
  - Update `sys_kill()` to call updated permission check
  - Add audit logging for denied signals
  - Test: non-root cannot signal different UID, root can signal anyone

---

- [ ] **Task 9D.3: Signal Security - Kernel Thread Protection**  
  Protect kernel threads from user-space signals.
  - Verify `is_kernel_thread` flag exists in Task struct (added in 9C.5)
  - Update `check_protected_process()` in `kernel/src/signal/security.rs`
  - Reject signals to kernel threads from user processes (Requirement 6.5)
  - Allow kernel to signal kernel threads (for internal coordination)
  - Return EPERM when user tries to signal kernel thread
  - Test: user process cannot kill flusher thread

---

- [ ] **Task 9D.4: Signal Security - Handler Validation**  
  Validate signal handler addresses are in executable memory.
  - Update `validate_signal_handler()` in `kernel/src/signal/security.rs`
  - Add page table lookup to check handler address (Requirement 6.6)
  - Verify handler address is in user space (< USER_LIMIT)
  - Walk page tables to get PTE for handler address
  - Check PTE has USER flag and does NOT have NO_EXECUTE flag
  - Reject handler if not in executable page (Requirement 6.7)
  - Return EFAULT for invalid handler addresses
  - Test: cannot register handler in data or stack pages

---

- [ ] **Task 9D.5: /proc Improvements - Timing**  
  Add CPU time tracking to processes for /proc.
  - Add `utime: AtomicU64` field to Task struct for user CPU time (Requirement 9.1)
  - Add `stime: AtomicU64` field to Task struct for kernel CPU time (Requirement 9.1)
  - Add `start_time: u64` field to Task struct (Requirement 9.3)
  - Update context switch code to accumulate CPU time (Requirement 9.2)
  - Track time in user mode vs kernel mode using privilege level
  - Update `/proc/[pid]/stat` in `kernel/src/fs/proc/mod.rs` to expose utime/stime
  - Format: "pid (name) state ppid pgrp session tty utime stime"

---

- [ ] **Task 9D.6: /proc Improvements - Memory**  
  Add memory statistics to /proc filesystem.
  - Implement `calculate_rss(process) -> usize` to count mapped physical pages (Requirement 9.4)
  - Walk process page tables, count present pages
  - Implement `calculate_vsz(process) -> usize` to sum memory regions (Requirement 9.5)
  - Use existing `Task::total_memory_usage()` method
  - Create `/proc/[pid]/status` file with memory info
  - Format: "VmSize: X kB\nVmRSS: Y kB\nVmShared: Z kB"
  - Track shared pages (COW pages with refcount > 1)

---

- [ ] **Task 9D.7: /proc Improvements - CPU Info**  
  Add CPU information to /proc/cpuinfo.
  - Implement TSC calibration in `kernel/src/arch/x86_64/mod.rs` (Requirement 9.6)
  - Use PIT (Programmable Interval Timer) to measure TSC frequency
  - Calibrate during early boot, store in global variable
  - Implement `detect_cpu_features()` using CPUID instruction (Requirement 9.7)
  - Check for: SSE, SSE2, AVX, AVX2, NX, RDRAND, etc.
  - Create `/proc/cpuinfo` file in `kernel/src/fs/proc/mod.rs`
  - Format: "processor: 0\nvendor_id: X\nmodel name: Y\nflags: sse sse2 ..."

---

- [ ] **Task 9D.8: /proc Improvements - System Stats**  
  Add system-wide statistics to /proc.
  - Track buffer cache usage in `kernel/src/fs/cache/buffer_cache.rs` (Requirement 9.8)
  - Add `get_buffer_cache_usage() -> (used, total)` function
  - Track page cache usage in `kernel/src/fs/cache/page_cache.rs` (Requirement 9.9)
  - Add `get_page_cache_usage() -> (used, total)` function
  - Calculate idle time by tracking time in idle task (Requirement 9.10)
  - Create `/proc/meminfo` with: MemTotal, MemFree, Buffers, Cached
  - Create `/proc/stat` with: cpu times (user, system, idle)
  - Update `/proc/[pid]/stat` to include tty and pgrp (Requirement 9.11, 9.12)

---

- [ ] **Task 9D.9: Compression - LZ4**  
  Replace placeholder compression with LZ4 algorithm.
  - Add lz4 crate to kernel/Cargo.toml or implement LZ4 algorithm
  - Update `compress()` in `kernel/src/fs/mfs/disk/compress.rs` to use LZ4 (Requirement 8.1)
  - Implement `decompress_lz4(data) -> Result<Vec<u8>, Error>`
  - Replace simple_rle_compress with LZ4 compression
  - Verify output compatible with standard lz4 tools (Requirement 8.5)
  - Benchmark: should decompress >500 MB/s (Requirement 8.3)
  - Add compression ratio tests: should achieve 2-3x on text data

---

- [ ] **Task 9D.10: Compression - Zstd**  
  Add Zstd compression as alternative to LZ4.
  - Add zstd crate to kernel/Cargo.toml or implement Zstd algorithm
  - Add `CompressionAlgorithm` enum: None, LZ4, Zstd (Requirement 8.2)
  - Implement `compress_zstd(data, level) -> Result<Vec<u8>, Error>`
  - Implement `decompress_zstd(data) -> Result<Vec<u8>, Error>`
  - Add compression level parameter (1-22) (Requirement 8.4)
  - Add per-file compression algorithm selection (Requirement 8.6)
  - Add system-wide default compression config (Requirement 8.7)
  - Verify compatibility with standard zstd tools (Requirement 8.5)

---

- [ ] **Task 9D.11: Path Resolution - Parent Tracking**  
  Add parent directory tracking for proper ".." resolution.
  - Add `parent: Option<Arc<Dentry>>` field to Dentry struct in `kernel/src/fs/vfs/dentry.rs`
  - Update `Dentry::new()` to accept parent parameter (Requirement 10.1)
  - Update all dentry creation sites to set parent
  - Implement ".." resolution in `resolve_path()` (Requirement 10.2)
  - Handle mount points: ".." at mount point goes to parent in parent filesystem (Requirement 10.3)
  - Add `get_parent()` method to Dentry
  - Test: cd into deep directory, cd .. repeatedly should work

---

- [ ] **Task 9D.12: Path Resolution - Symlinks**  
  Implement symbolic link support in VFS.
  - Add `Symlink` inode type to `InodeType` enum (Requirement 10.4)
  - Implement `read_link(inode) -> Result<String, Error>` in VFS
  - Update `resolve_path()` to detect and follow symlinks (Requirement 10.5)
  - Add symlink depth counter, limit to 40 (Requirement 10.6)
  - Return ELOOP if depth exceeds 40 (Requirement 10.7)
  - Implement `sys_symlink(target, linkpath)` syscall
  - Implement `sys_readlink(path, buf, size)` syscall
  - Test: create symlink, read through it, detect loops

---

## Testing & Integration

- [ ] **Task 9E.1: Integration Testing**  
  Write end-to-end tests for all features.
  - Write end-to-end tests for all features
  - Test feature interactions
  - Run full test suite
  - Fix any integration issues

---

- [ ] **Task 9E.2: Performance Testing**  
  Run performance benchmarks and verify targets.
  - Run performance benchmarks
  - Verify performance targets met
  - Profile and optimize hot paths
  - Document performance results

---

- [ ] **Task 9E.3: Stress Testing**  
  Run high-load and SMP stress tests.
  - Run high-load tests
  - Run SMP stress tests
  - Run memory pressure tests
  - Fix any stability issues

---

- [ ] **Task 9E.4: Documentation**  
  Update architecture documentation and guides.
  - Update architecture documentation
  - Document new APIs
  - Update developer guide
  - Write migration guide

---

- [ ] **Task 9E.5: Code Review & Cleanup**  
  Code review and cleanup all changes.
  - Code review all changes
  - Remove debug code
  - Clean up comments
  - Run rustfmt

---

## Summary

**Total Tasks:** 52  
**Estimated Time:** 12 weeks  
**Critical Path:** 9A → 9B → 9C → 9E

**Phase Breakdown:**
- Phase 9A (Foundation): 7 tasks, 3 weeks
- Phase 9B (Memory): 7 tasks, 3 weeks
- Phase 9C (Process): 8 tasks, 3 weeks
- Phase 9D (Polish): 12 tasks, 3 weeks
- Phase 9E (Testing): 5 tasks, 2 weeks (overlaps with 9D)

**Risk Mitigation:**
- Each task has clear acceptance criteria
- Dependencies clearly marked
- Can pause/rollback at phase boundaries
- Feature flags for risky changes

---

**Status:** Ready for Implementation  
**Next Step:** Complete Phase 8, then begin Phase 9A
