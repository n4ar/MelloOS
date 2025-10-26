# MelloOS Technical Debt - Categorized

Generated: $(date)

## Summary

This document categorizes all TODO, FIXME, and HACK markers found in the MelloOS codebase.

---

## Totals

- **TODO**: 129
- **FIXME**: 0  
- **HACK**: 0
- **TOTAL**: 129

---

## By Subsystem

### Memory Management (kernel/src/mm/) - 7 items

- kernel/src/mm/pmm.rs:119:        // TODO: Log memory information once logging is available
- kernel/src/mm/pmm.rs:178:            // TODO: Log error once logging is available
- kernel/src/mm/allocator.rs:285:            // TODO: Add logging when logging infrastructure is available
- kernel/src/mm/allocator.rs:289:            // TODO: Add logging when logging infrastructure is available
- kernel/src/mm/allocator.rs:312:        // TODO: Add logging when logging infrastructure is available
- kernel/src/mm/mod.rs:457:    // TODO: Replace with proper logging once available
- kernel/src/mm/log.rs:41:        // TODO: Implement actual serial output

### Filesystem (kernel/src/fs/) - 18 items

- kernel/src/fs/syscalls.rs:6:// TODO: Implement filesystem syscalls
- kernel/src/fs/mfs/ram/inode.rs:162:        // TODO: Implement proper time tracking
- kernel/src/fs/mfs/ram/inode.rs:199:            st_dev: 0, // TODO: Device ID
- kernel/src/fs/mfs/ram/inode.rs:219:        // TODO: Implement attribute setting
- kernel/src/fs/mfs/disk/replay.rs:156:            // TODO: Implement full tree walk when child pointers are available
- kernel/src/fs/mfs/disk/replay.rs:188:        // TODO: Scan extent tree and mark allocated extents
- kernel/src/fs/mfs/disk/superblock_impl.rs:64:        // TODO: Load root inode from disk
- kernel/src/fs/cache/throttle.rs:312:    // TODO: Implement proper yielding when scheduler supports it
- kernel/src/fs/cache/writeback.rs:140:        // TODO: Spawn background flusher thread when task scheduler supports it
- kernel/src/fs/mfs/disk/super.rs:40:    // TODO: Calibrate TSC frequency on boot
- kernel/src/fs/proc/mod.rs:963:    proc_info.tpgid = None; // TODO: Get from TTY when available
- kernel/src/fs/proc/mod.rs:976:    // TODO: Set cmdline from task when available
- kernel/src/fs/proc/mod.rs:980:    // TODO: Get actual timing and memory info
- kernel/src/fs/proc/mod.rs:1000:            buffers: 0,              // TODO: Track buffer cache
- kernel/src/fs/proc/mod.rs:1001:            cached: 0,               // TODO: Track page cache
- kernel/src/fs/proc/mod.rs:1056:    // TODO: Get actual CPU MHz (requires TSC calibration)
- kernel/src/fs/proc/mod.rs:1072:    // TODO: Calculate actual idle time
- kernel/src/fs/proc/mod.rs:1113:    // TODO: Update when task table supports lock-free access

### User/Process Management (kernel/src/user/) - 9 items

- kernel/src/user/process.rs:215:            creation_time: 0, // TODO: Get current tick count
- kernel/src/user/process.rs:818:        // TODO: Unmap the temporary kernel mapping
- kernel/src/user/process.rs:859:    // TODO: When implementing full page table separation, use kmap_user_page:
- kernel/src/user/process.rs:899:    // TODO: When implementing full page table separation, use kmap_user_page:
- kernel/src/user/process.rs:1168:    // TODO: Switch page tables when we have per-process page tables
- kernel/src/user/process.rs:1171:    // TODO: Flush TLB if switching between different address spaces
- kernel/src/user/elf.rs:460:                // TODO: IPI TLB shootdown for SMP when implementing full page table separation
- kernel/src/user/elf.rs:528:        // TODO: When process has its own page table, use process.page_table
- kernel/src/user/elf.rs:624:        // TODO: When process has its own page table, use process.page_table

### Syscalls (kernel/src/sys/ & kernel/src/arch/x86_64/syscall/) - 10 items

- kernel/src/arch/x86_64/syscall/mod.rs:149:    // TODO: When process management is implemented, terminate the current process
- kernel/src/arch/x86_64/syscall/mod.rs:207:    // TODO: When implementing full page table separation, replace direct pointer
- kernel/src/arch/x86_64/syscall/mod.rs:236:    // TODO: When implementing full page table separation, replace direct pointer
- kernel/src/arch/x86_64/syscall/mod.rs:378:    // TODO: Implement when process management is available
- kernel/src/arch/x86_64/syscall/mod.rs:498:    // TODO: Implement proper buffered I/O for large writes
- kernel/src/arch/x86_64/syscall/mod.rs:769:    // TODO: Implement copy-on-write optimization in the future
- kernel/src/arch/x86_64/syscall/mod.rs:787:    // TODO: Copy parent's page table (mark as TODO for copy-on-write)
- kernel/src/arch/x86_64/syscall/mod.rs:982:    // TODO: In a full implementation, we would:
- kernel/src/arch/x86_64/syscall/mod.rs:1197:    // TODO: Implement proper blocking mechanism
- kernel/src/arch/x86_64/syscall/mod.rs:1205:    let has_children = false; // TODO: Implement proper child tracking

### Userspace Programs (kernel/userspace/) - 6 items

- kernel/userspace/mellobox/src/commands/df.rs:20:    // TODO: Implement actual filesystem querying
- kernel/userspace/mello-term/src/pty.rs:106:        // TODO: Set up non-blocking I/O with fcntl (not yet implemented)
- kernel/userspace/mello-term/src/input.rs:29:        // TODO: Implement in subtask 7.6
- kernel/userspace/mello-term/src/input.rs:54:    // TODO: Implement actual keyboard reading
- kernel/userspace/mello-term/src/main.rs:166:        // TODO: Implement main event loop in later subtasks
- kernel/userspace/mello-term/src/ansi.rs:180:                // TODO: Implement clear line modes

### Tests (tests/) - 68 items

- tests/fs_xattr.rs:11:    // TODO: Implement when VFS is ready
- tests/fs_xattr.rs:18:    // TODO: Implement when VFS is ready
- tests/fs_xattr.rs:25:    // TODO: Implement when VFS is ready
- tests/fs_xattr.rs:32:    // TODO: Implement when VFS is ready
- tests/fs_xattr.rs:39:    // TODO: Implement when VFS is ready
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
- tests/fs_dir_ops.rs:12://! TODO: Implement tests when test infrastructure is available
- tests/fs_dir_ops.rs:16:    // TODO: Add test cases
- tests/fs_special_nodes.rs:12:    // TODO: Implement when VFS is ready
- tests/fs_special_nodes.rs:20:    // TODO: Implement when VFS is ready
- tests/fs_special_nodes.rs:27:    // TODO: Implement when VFS is ready
- tests/fs_special_nodes.rs:34:    // TODO: Implement when VFS is ready
- tests/fs_special_nodes.rs:42:    // TODO: Implement when VFS is ready
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
- tests/mfs_ram_correctness.rs:10:// TODO: Implement tests once kernel test infrastructure is set up
- tests/fs_cache_perf.rs:7:    // TODO: Implement when page cache is integrated
- tests/fs_cache_perf.rs:19:    // TODO: Implement when page cache is integrated
- tests/fs_cache_perf.rs:30:    // TODO: Implement when writeback is integrated
- tests/fs_cache_perf.rs:40:    // TODO: Implement when writeback is integrated
- tests/fs_cache_perf.rs:50:    // TODO: Implement when writeback is integrated
- tests/fs_cache_perf.rs:60:    // TODO: Implement when writeback is integrated
- tests/fs_cache_behavior.rs:10:    // TODO: Implement when page cache is integrated with filesystem
- tests/fs_cache_behavior.rs:20:    // TODO: Implement when page cache is integrated
- tests/fs_cache_behavior.rs:30:    // TODO: Implement when buffer cache is integrated
- tests/fs_cache_behavior.rs:39:    // TODO: Implement when buffer cache is integrated
- tests/fs_cache_behavior.rs:47:    // TODO: Implement when page cache is integrated
- tests/fs_syscalls_api.rs:11:    // TODO: Implement when VFS is ready
- tests/fs_syscalls_api.rs:18:    // TODO: Implement when VFS is ready
- tests/fs_syscalls_api.rs:25:    // TODO: Implement when VFS is ready
- tests/fs_syscalls_api.rs:32:    // TODO: Implement when VFS is ready
- tests/fs_syscalls_api.rs:39:    // TODO: Implement when VFS is ready
- tests/fs_syscalls_api.rs:47:    // TODO: Implement when VFS is ready
- tests/fs_syscalls_api.rs:54:    // TODO: Implement when VFS is ready
- tests/fs_syscalls_api.rs:62:    // TODO: Implement when VFS is ready
- tests/fs_vfs_correctness.rs:13://! TODO: Implement tests when test infrastructure is available
- tests/fs_vfs_correctness.rs:17:    // TODO: Add test cases
- tests/fs_fd_ops.rs:12://! TODO: Implement tests when test infrastructure is available
- tests/fs_fd_ops.rs:16:    // TODO: Add test cases
- tests/mfs_ram_perf.rs:10:// TODO: Implement performance benchmarks once kernel test infrastructure is set up
- tests/fs_stat_compat.rs:12:    // TODO: Implement when VFS is ready
- tests/fs_stat_compat.rs:20:    // TODO: Implement when VFS is ready
- tests/fs_stat_compat.rs:27:    // TODO: Implement when VFS is ready

---

## By Priority

### 🔴 High Priority (Security, Correctness, Core Functionality)

- kernel/src/fs/syscalls.rs:6:// TODO: Implement filesystem syscalls
- kernel/src/user/process.rs:859:    // TODO: When implementing full page table separation, use kmap_user_page:
- kernel/src/user/process.rs:899:    // TODO: When implementing full page table separation, use kmap_user_page:
- kernel/src/user/elf.rs:460:                // TODO: IPI TLB shootdown for SMP when implementing full page table separation
- kernel/src/arch_x86_64/syscall/mod.rs:149:    // TODO: When process management is implemented, terminate the current process
- kernel/src/arch_x86_64/syscall/mod.rs:207:    // TODO: When implementing full page table separation, replace direct pointer
- kernel/src/arch_x86_64/syscall/mod.rs:236:    // TODO: When implementing full page table separation, replace direct pointer
- kernel/src/arch_x86_64/syscall/mod.rs:378:    // TODO: Implement when process management is available
- kernel/src/arch_x86_64/syscall/mod.rs:498:    // TODO: Implement proper buffered I/O for large writes
- kernel/src/arch_x86_64/syscall/mod.rs:769:    // TODO: Implement copy-on-write optimization in the future
- kernel/src/arch_x86_64/syscall/mod.rs:982:    // TODO: In a full implementation, we would:
- kernel/src/arch_x86_64/syscall/mod.rs:1197:    // TODO: Implement proper blocking mechanism
- kernel/src/arch_x86_64/syscall/mod.rs:1205:    // TODO: Implement proper child tracking

### 🟡 Medium Priority (Features, Optimizations)

- kernel/src/fs/mfs/ram/inode.rs:162:        // TODO: Implement proper time tracking
- kernel/src/fs/mfs/ram/inode.rs:219:        // TODO: Implement attribute setting
- kernel/src/fs/mfs/disk/replay.rs:156:            // TODO: Implement full tree walk when child pointers are available
- kernel/src/fs/cache/throttle.rs:312:    // TODO: Implement proper yielding when scheduler supports it
- kernel/src/mm/log.rs:41:        // TODO: Implement actual serial output
- kernel/src/arch_x86_64/gdt.rs:436:    // TODO: Implement proper page mapping when paging system is integrated
- kernel/userspace/mellobox/src/commands/df.rs:20:    // TODO: Implement actual filesystem querying
- kernel/userspace/mello-term/src/pty.rs:106:        // TODO: Set up non-blocking I/O with fcntl (not yet implemented)
- kernel/userspace/mello-term/src/input.rs:29:        // TODO: Implement in subtask 7.6
- kernel/userspace/mello-term/src/input.rs:54:    // TODO: Implement actual keyboard reading
- kernel/userspace/mello-term/src/main.rs:166:        // TODO: Implement main event loop in later subtasks
- kernel/userspace/mello-term/src/ansi.rs:180:                // TODO: Implement clear line modes

### 🟢 Low Priority (Documentation, Logging, Nice-to-have)

- kernel/src/mm/pmm.rs:119:        // TODO: Log memory information once logging is available
- kernel/src/mm/pmm.rs:178:            // TODO: Log error once logging is available
- kernel/src/mm/allocator.rs:285:            // TODO: Add logging when logging infrastructure is available
- kernel/src/mm/allocator.rs:289:            // TODO: Add logging when logging infrastructure is available
- kernel/src/mm/allocator.rs:312:        // TODO: Add logging when logging infrastructure is available
- kernel/src/mm/mod.rs:457:    // TODO: Replace with proper logging once available

---

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
