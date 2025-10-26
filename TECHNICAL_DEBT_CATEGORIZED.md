# MelloOS Technical Debt - Categorized

Generated: $(date)

## Summary

This document categorizes all TODO, FIXME, and HACK markers found in the MelloOS codebase.

---

## Totals

- **TODO**: 149
- **FIXME**: 0  
- **HACK**: 0
- **TOTAL**: 149

---

## By Subsystem

### Memory Management (kernel/src/mm/) - 12 items

- kernel/src/mm/security.rs:109:            // TODO: Check if page has USER flag set
- kernel/src/mm/security.rs:149:            // TODO: Check if page has USER and WRITABLE flags set
- kernel/src/mm/security.rs:205:                // TODO: Verify USER flag is set
- kernel/src/mm/security.rs:267:                // TODO: Verify USER and WRITABLE flags are set
- kernel/src/mm/security.rs:595:            // TODO: Get actual page flags from page table entry
- kernel/src/mm/log.rs:41:        // TODO: Implement actual serial output
- kernel/src/mm/mod.rs:457:    // TODO: Replace with proper logging once available
- kernel/src/mm/allocator.rs:285:            // TODO: Add logging when logging infrastructure is available
- kernel/src/mm/allocator.rs:289:            // TODO: Add logging when logging infrastructure is available
- kernel/src/mm/allocator.rs:312:        // TODO: Add logging when logging infrastructure is available
- kernel/src/mm/pmm.rs:119:        // TODO: Log memory information once logging is available
- kernel/src/mm/pmm.rs:178:            // TODO: Log error once logging is available

### Filesystem (kernel/src/fs/) - 21 items

- kernel/src/fs/proc/mod.rs:963:    proc_info.tpgid = None; // TODO: Get from TTY when available
- kernel/src/fs/proc/mod.rs:976:    // TODO: Set cmdline from task when available
- kernel/src/fs/proc/mod.rs:980:    // TODO: Get actual timing and memory info
- kernel/src/fs/proc/mod.rs:1000:            buffers: 0,              // TODO: Track buffer cache
- kernel/src/fs/proc/mod.rs:1001:            cached: 0,               // TODO: Track page cache
- kernel/src/fs/proc/mod.rs:1056:    // TODO: Get actual CPU MHz (requires TSC calibration)
- kernel/src/fs/proc/mod.rs:1072:    // TODO: Calculate actual idle time
- kernel/src/fs/proc/mod.rs:1113:    // TODO: Update when task table supports lock-free access
- kernel/src/fs/vfs/path.rs:100:            // TODO: Implement proper parent tracking
- kernel/src/fs/cache/writeback.rs:140:        // TODO: Spawn background flusher thread when task scheduler supports it
- kernel/src/fs/cache/writeback.rs:292:    // TODO: Implement actual flushing when filesystem is ready
- kernel/src/fs/cache/writeback.rs:306:    // TODO: Implement when filesystem is ready
- kernel/src/fs/cache/throttle.rs:312:    // TODO: Implement proper yielding when scheduler supports it
- kernel/src/fs/mfs/disk/super.rs:40:    // TODO: Calibrate TSC frequency on boot
- kernel/src/fs/mfs/disk/superblock_impl.rs:64:        // TODO: Load root inode from disk
- kernel/src/fs/mfs/disk/replay.rs:156:            // TODO: Implement full tree walk when child pointers are available
- kernel/src/fs/mfs/disk/replay.rs:188:        // TODO: Scan extent tree and mark allocated extents
- kernel/src/fs/mfs/ram/inode.rs:162:        // TODO: Implement proper time tracking
- kernel/src/fs/mfs/ram/inode.rs:199:            st_dev: 0, // TODO: Device ID
- kernel/src/fs/mfs/ram/inode.rs:219:        // TODO: Implement attribute setting
- kernel/src/fs/syscalls.rs:6:// TODO: Implement filesystem syscalls

### User/Process Management (kernel/src/user/) - 14 items

- kernel/src/user/exec.rs:297:        // TODO: Implement heap tracking in Task structure
- kernel/src/user/exec.rs:335:        // TODO: Implement heap pointer reset when we have heap tracking
- kernel/src/user/exec.rs:392:            // TODO: Implement full page remapping when we have proper page table isolation
- kernel/src/user/exec.rs:407:            // TODO: Restore heap pointer when we have heap tracking
- kernel/src/user/exec.rs:922:    // TODO: Check if pages are actually mapped
- kernel/src/user/elf.rs:439:                // TODO: IPI TLB shootdown for SMP when implementing full page table separation
- kernel/src/user/elf.rs:507:        // TODO: When process has its own page table, use process.page_table
- kernel/src/user/elf.rs:603:        // TODO: When process has its own page table, use process.page_table
- kernel/src/user/process.rs:215:            creation_time: 0, // TODO: Get current tick count
- kernel/src/user/process.rs:818:        // TODO: Unmap the temporary kernel mapping
- kernel/src/user/process.rs:859:    // TODO: When implementing full page table separation, use kmap_user_page:
- kernel/src/user/process.rs:899:    // TODO: When implementing full page table separation, use kmap_user_page:
- kernel/src/user/process.rs:1168:    // TODO: Switch page tables when we have per-process page tables
- kernel/src/user/process.rs:1171:    // TODO: Flush TLB if switching between different address spaces

### Syscalls (kernel/src/sys/ & kernel/src/arch/x86_64/syscall/) - 12 items

- kernel/src/arch/x86_64/syscall/mod.rs:148:    // TODO: When process management is implemented, terminate the current process
- kernel/src/arch/x86_64/syscall/mod.rs:206:    // TODO: When implementing full page table separation, replace direct pointer
- kernel/src/arch/x86_64/syscall/mod.rs:235:    // TODO: When implementing full page table separation, replace direct pointer
- kernel/src/arch/x86_64/syscall/mod.rs:377:    // TODO: Implement when process management is available
- kernel/src/arch/x86_64/syscall/mod.rs:497:    // TODO: Implement proper buffered I/O for large writes
- kernel/src/arch/x86_64/syscall/mod.rs:559:            // TODO: Wake up parent process if it's waiting
- kernel/src/arch/x86_64/syscall/mod.rs:569:                // TODO: Implement parent wakeup logic
- kernel/src/arch/x86_64/syscall/mod.rs:704:    // TODO: Implement copy-on-write optimization in the future
- kernel/src/arch/x86_64/syscall/mod.rs:722:    // TODO: Copy parent's page table (mark as TODO for copy-on-write)
- kernel/src/arch/x86_64/syscall/mod.rs:919:    // TODO: In a full implementation, we would:
- kernel/src/arch/x86_64/syscall/mod.rs:1134:    // TODO: Implement proper blocking mechanism
- kernel/src/arch/x86_64/syscall/mod.rs:1142:    let has_children = false; // TODO: Implement proper child tracking

### Drivers (kernel/src/drivers/) - 2 items

- kernel/src/drivers/block/virtio_blk.rs:132:        // TODO: Implement actual virtqueue submission
- kernel/src/drivers/block/virtio_blk.rs:152:        // TODO: Implement actual virtqueue submission

### Signals (kernel/src/signal/) - 5 items

- kernel/src/signal/security.rs:50:/// 3. TODO: Add UID-based permission checks when user management is implemented
- kernel/src/signal/security.rs:77:    // TODO: Add UID-based permission checks when user management is implemented
- kernel/src/signal/security.rs:122:    // TODO: Add protection for kernel threads
- kernel/src/signal/security.rs:158:            // TODO: Check if address is in executable code pages
- kernel/src/signal/mod.rs:409:    // TODO: Implement signal frame setup

### Devices (kernel/src/dev/) - 5 items

- kernel/src/dev/pty/mod.rs:494:        // TODO: Send signal to all processes in the process group
- kernel/src/dev/pty/mod.rs:524:    // TODO: Send signal to all processes in the process group
- kernel/src/dev/pty/mod.rs:551:        // TODO: Get actual PGID from task
- kernel/src/dev/pty/mod.rs:771:                // TODO: Get actual PGID from task
- kernel/src/dev/pty/mod.rs:837:                // TODO: Get actual PGID from task

### Userspace Programs (kernel/userspace/) - 6 items

- kernel/userspace/mello-term/src/ansi.rs:180:                // TODO: Implement clear line modes
- kernel/userspace/mello-term/src/main.rs:166:        // TODO: Implement main event loop in later subtasks
- kernel/userspace/mello-term/src/input.rs:29:        // TODO: Implement in subtask 7.6
- kernel/userspace/mello-term/src/input.rs:54:    // TODO: Implement actual keyboard reading
- kernel/userspace/mello-term/src/pty.rs:106:        // TODO: Set up non-blocking I/O with fcntl (not yet implemented)
- kernel/userspace/mellobox/src/commands/df.rs:20:    // TODO: Implement actual filesystem querying

### Tests (tests/) - 68 items

- tests/fs_stat_compat.rs:12:    // TODO: Implement when VFS is ready
- tests/fs_stat_compat.rs:20:    // TODO: Implement when VFS is ready
- tests/fs_stat_compat.rs:27:    // TODO: Implement when VFS is ready
- tests/mfs_ram_perf.rs:10:// TODO: Implement performance benchmarks once kernel test infrastructure is set up
- tests/fs_syscalls_api.rs:11:    // TODO: Implement when VFS is ready
- tests/fs_syscalls_api.rs:18:    // TODO: Implement when VFS is ready
- tests/fs_syscalls_api.rs:25:    // TODO: Implement when VFS is ready
- tests/fs_syscalls_api.rs:32:    // TODO: Implement when VFS is ready
- tests/fs_syscalls_api.rs:39:    // TODO: Implement when VFS is ready
- tests/fs_syscalls_api.rs:47:    // TODO: Implement when VFS is ready
- tests/fs_syscalls_api.rs:54:    // TODO: Implement when VFS is ready
- tests/fs_syscalls_api.rs:62:    // TODO: Implement when VFS is ready
- tests/fs_fd_ops.rs:12://! TODO: Implement tests when test infrastructure is available
- tests/fs_fd_ops.rs:16:    // TODO: Add test cases
- tests/fs_vfs_correctness.rs:13://! TODO: Implement tests when test infrastructure is available
- tests/fs_vfs_correctness.rs:17:    // TODO: Add test cases
- tests/fs_cache_behavior.rs:10:    // TODO: Implement when page cache is integrated with filesystem
- tests/fs_cache_behavior.rs:20:    // TODO: Implement when page cache is integrated
- tests/fs_cache_behavior.rs:30:    // TODO: Implement when buffer cache is integrated
- tests/fs_cache_behavior.rs:39:    // TODO: Implement when buffer cache is integrated
- tests/fs_cache_behavior.rs:47:    // TODO: Implement when page cache is integrated
- tests/fs_cache_perf.rs:7:    // TODO: Implement when page cache is integrated
- tests/fs_cache_perf.rs:19:    // TODO: Implement when page cache is integrated
- tests/fs_cache_perf.rs:30:    // TODO: Implement when writeback is integrated
- tests/fs_cache_perf.rs:40:    // TODO: Implement when writeback is integrated
- tests/fs_cache_perf.rs:50:    // TODO: Implement when writeback is integrated
- tests/fs_cache_perf.rs:60:    // TODO: Implement when writeback is integrated
- tests/mfs_ram_correctness.rs:10:// TODO: Implement tests once kernel test infrastructure is set up
- tests/userland_smoke.rs:11:    // TODO: Implement when VFS is ready
- tests/userland_smoke.rs:18:    // TODO: Implement when VFS is ready
- tests/userland_smoke.rs:25:    // TODO: Implement when VFS is ready
- tests/userland_smoke.rs:32:    // TODO: Implement when VFS is ready
- tests/userland_smoke.rs:39:    // TODO: Implement when VFS is ready
- tests/userland_smoke.rs:46:    // TODO: Implement when VFS is ready
- tests/userland_smoke.rs:53:    // TODO: Implement when VFS is ready
- tests/userland_smoke.rs:60:    // TODO: Implement when VFS is ready
- tests/userland_smoke.rs:67:    // TODO: Implement when VFS is ready
- tests/userland_smoke.rs:74:    // TODO: Implement when VFS is ready
- tests/userland_smoke.rs:81:    // TODO: Implement when VFS is ready
- tests/userland_smoke.rs:88:    // TODO: Implement when VFS is ready
- tests/userland_smoke.rs:95:    // TODO: Implement when VFS is ready
- tests/fs_special_nodes.rs:12:    // TODO: Implement when VFS is ready
- tests/fs_special_nodes.rs:20:    // TODO: Implement when VFS is ready
- tests/fs_special_nodes.rs:27:    // TODO: Implement when VFS is ready
- tests/fs_special_nodes.rs:34:    // TODO: Implement when VFS is ready
- tests/fs_special_nodes.rs:42:    // TODO: Implement when VFS is ready
- tests/fs_dir_ops.rs:12://! TODO: Implement tests when test infrastructure is available
- tests/fs_dir_ops.rs:16:    // TODO: Add test cases
- tests/fs_faults.rs:29:    // TODO: Implement when mfs_disk is integrated
- tests/fs_faults.rs:56:    // TODO: Implement when mfs_disk is integrated
- tests/fs_faults.rs:82:    // TODO: Implement when block device fault injection is available
- tests/fs_faults.rs:109:    // TODO: Implement when block device fault injection is available
- tests/fs_faults.rs:135:    // TODO: Implement when mfs_disk TxG is integrated
- tests/fs_faults.rs:161:    // TODO: Implement when memory allocation tracking is available
- tests/fs_faults.rs:187:    // TODO: Implement when page cache is integrated
- tests/fs_faults.rs:211:    // TODO: Implement when mfs_disk checksums are integrated
- tests/fs_faults.rs:237:    // TODO: Implement when mfs_disk is integrated
- tests/fs_faults.rs:264:    // TODO: Implement when mfs_disk is integrated
- tests/fs_faults.rs:288:    // TODO: Implement when mfs_disk is integrated
- tests/fs_faults.rs:312:    // TODO: Implement when mfs_disk is integrated
- tests/fs_faults.rs:337:    // TODO: Implement when mfs_disk is integrated
- tests/fs_faults.rs:366:    // TODO: Implement when mfs_disk is integrated
- tests/fs_faults.rs:390:    // TODO: Implement when mfs_disk TxG is integrated
- tests/fs_xattr.rs:11:    // TODO: Implement when VFS is ready
- tests/fs_xattr.rs:18:    // TODO: Implement when VFS is ready
- tests/fs_xattr.rs:25:    // TODO: Implement when VFS is ready
- tests/fs_xattr.rs:32:    // TODO: Implement when VFS is ready
- tests/fs_xattr.rs:39:    // TODO: Implement when VFS is ready

---

## By Priority

### 🔴 High Priority (Security, Correctness, Core Functionality)


### 🟡 Medium Priority (Features, Optimizations)

- kernel/userspace/mello-term/src/ansi.rs:180:                // TODO: Implement clear line modes
- kernel/userspace/mello-term/src/main.rs:166:        // TODO: Implement main event loop in later subtasks
- kernel/userspace/mello-term/src/input.rs:29:        // TODO: Implement in subtask 7.6
- kernel/userspace/mello-term/src/input.rs:54:    // TODO: Implement actual keyboard reading
- kernel/userspace/mello-term/src/pty.rs:106:        // TODO: Set up non-blocking I/O with fcntl (not yet implemented)
- kernel/userspace/mellobox/src/commands/df.rs:20:    // TODO: Implement actual filesystem querying
- kernel/src/drivers/block/virtio_blk.rs:132:        // TODO: Implement actual virtqueue submission
- kernel/src/drivers/block/virtio_blk.rs:152:        // TODO: Implement actual virtqueue submission
- kernel/src/user/exec.rs:297:        // TODO: Implement heap tracking in Task structure
- kernel/src/user/exec.rs:335:        // TODO: Implement heap pointer reset when we have heap tracking
- kernel/src/user/exec.rs:392:            // TODO: Implement full page remapping when we have proper page table isolation
- kernel/src/user/elf.rs:439:                // TODO: IPI TLB shootdown for SMP when implementing full page table separation
- kernel/src/user/process.rs:859:    // TODO: When implementing full page table separation, use kmap_user_page:
- kernel/src/user/process.rs:899:    // TODO: When implementing full page table separation, use kmap_user_page:
- kernel/src/arch/x86_64/gdt.rs:436:    // TODO: Implement proper page mapping when paging system is integrated
- kernel/src/arch/x86_64/syscall/mod.rs:148:    // TODO: When process management is implemented, terminate the current process
- kernel/src/arch/x86_64/syscall/mod.rs:206:    // TODO: When implementing full page table separation, replace direct pointer
- kernel/src/arch/x86_64/syscall/mod.rs:235:    // TODO: When implementing full page table separation, replace direct pointer
- kernel/src/arch/x86_64/syscall/mod.rs:377:    // TODO: Implement when process management is available
- kernel/src/arch/x86_64/syscall/mod.rs:497:    // TODO: Implement proper buffered I/O for large writes
- kernel/src/arch/x86_64/syscall/mod.rs:569:                // TODO: Implement parent wakeup logic
- kernel/src/arch/x86_64/syscall/mod.rs:704:    // TODO: Implement copy-on-write optimization in the future
- kernel/src/arch/x86_64/syscall/mod.rs:919:    // TODO: In a full implementation, we would:
- kernel/src/arch/x86_64/syscall/mod.rs:1134:    // TODO: Implement proper blocking mechanism
- kernel/src/arch/x86_64/syscall/mod.rs:1142:    let has_children = false; // TODO: Implement proper child tracking
- kernel/src/mm/log.rs:41:        // TODO: Implement actual serial output
- kernel/src/fs/vfs/path.rs:100:            // TODO: Implement proper parent tracking
- kernel/src/fs/cache/writeback.rs:292:    // TODO: Implement actual flushing when filesystem is ready
- kernel/src/fs/cache/writeback.rs:306:    // TODO: Implement when filesystem is ready
- kernel/src/fs/cache/throttle.rs:312:    // TODO: Implement proper yielding when scheduler supports it
- kernel/src/fs/mfs/disk/replay.rs:156:            // TODO: Implement full tree walk when child pointers are available
- kernel/src/fs/mfs/ram/inode.rs:162:        // TODO: Implement proper time tracking
- kernel/src/fs/mfs/ram/inode.rs:219:        // TODO: Implement attribute setting
- kernel/src/fs/syscalls.rs:6:// TODO: Implement filesystem syscalls
- kernel/src/signal/security.rs:50:/// 3. TODO: Add UID-based permission checks when user management is implemented
- kernel/src/signal/security.rs:77:    // TODO: Add UID-based permission checks when user management is implemented
- kernel/src/signal/mod.rs:409:    // TODO: Implement signal frame setup

### 🟢 Low Priority (Documentation, Logging, Nice-to-have)

- kernel/src/arch/x86_64/syscall/mod.rs:569:                // TODO: Implement parent wakeup logic
- kernel/src/mm/mod.rs:457:    // TODO: Replace with proper logging once available
- kernel/src/mm/allocator.rs:285:            // TODO: Add logging when logging infrastructure is available
- kernel/src/mm/allocator.rs:289:            // TODO: Add logging when logging infrastructure is available
- kernel/src/mm/allocator.rs:312:        // TODO: Add logging when logging infrastructure is available
- kernel/src/mm/pmm.rs:119:        // TODO: Log memory information once logging is available
- kernel/src/mm/pmm.rs:178:            // TODO: Log error once logging is available

---

## Recommendations

### Immediate Actions (High Priority)
1. Review all security-related TODOs
2. Fix any unsafe code without proper documentation
3. Implement critical missing functionality

### Short-term (Medium Priority)
1. Complete core feature implementations
2. Add missing error handling
3. Implement performance optimizations

### Long-term (Low Priority)
1. Improve documentation
2. Add comprehensive logging
3. Code cleanup and refactoring

### Process Improvements
1. Create GitHub issues for each TODO
2. Add TODOs to .kiro/specs/*/tasks.md
3. Set up automated TODO tracking in CI
4. Regular technical debt review sessions

---

## Next Steps

1. Review this categorized list
2. Prioritize items based on current phase (see roadmap.md)
3. Create tasks in appropriate spec files
4. Assign owners and deadlines
5. Track progress and remove completed TODOs

