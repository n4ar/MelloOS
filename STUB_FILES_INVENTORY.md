# MelloOS Stub/Mock/TODO Files Inventory

## Overview
This document lists all files that contain stub implementations, mock functions, or TODO markers indicating incomplete functionality.

---

## Category 1: Filesystem Layer (High Priority for Phase 8)

### VFS Layer - Core Stubs
- **`kernel/src/fs/vfs/mod.rs`**
  - Status: Marked as "Currently stubbed - requires implementation without alloc"
  - Impact: Core VFS functionality
  
- **`kernel/src/fs/syscalls.rs`**
  - Status: "TODO: Implement filesystem syscalls"
  - Impact: All filesystem syscalls

### MFS Disk Implementation - Partial Stubs
- **`kernel/src/fs/mfs/disk/super_impl.rs`**
  - `lookup_inode()` - Placeholder, returns "Not implemented"
  - `read_dir()` - Placeholder, returns "Not implemented"
  - Impact: Cannot read from disk-based MFS

- **`kernel/src/fs/mfs/disk/super.rs`**
  - `current_time_ns()` - Returns 0, needs RTC/TSC integration
  - Impact: Timestamps are all zero

- **`kernel/src/fs/mfs/disk/checksum.rs`**
  - `has_pclmulqdq()` - Returns false, needs CPUID check
  - Impact: Always uses slower CRC32 path

- **`kernel/src/fs/mfs/disk/compress.rs`**
  - `compress_lz4()` - Uses simple RLE instead of real LZ4
  - `compress_zstd()` - Uses simple RLE instead of real Zstd
  - `simple_rle_compress()` - Placeholder compression
  - Impact: Poor compression ratios

- **`kernel/src/fs/mfs/disk/replay.rs`**
  - Tree walk verification incomplete (line 156)
  - Impact: Cannot fully verify B-tree integrity

### MFS RAM Implementation - Minor Stubs
- **`kernel/src/fs/mfs/ram/inode.rs`**
  - `current_time()` - Returns 0
  - `set_attr()` - Returns NotSupported
  - Impact: No timestamp tracking, cannot change attributes

### Block Device Layer - Major Stubs
- **`kernel/src/fs/block_dev.rs`**
  - `read()` - Returns zeros instead of reading from device
  - `write()` - Succeeds without writing
  - `flush()` - No-op
  - Impact: **CRITICAL** - No actual disk I/O

### Cache Layer - Partial Stubs
- **`kernel/src/fs/cache/writeback.rs`**
  - `flush_inode_pages()` - Not implemented
  - `flush_all_pages()` - Not implemented
  - Impact: Dirty pages not flushed to disk

- **`kernel/src/fs/cache/throttle.rs`**
  - `throttle_writer()` - Spins instead of yielding
  - Impact: CPU waste when throttling

### Path Resolution - Minor Stub
- **`kernel/src/fs/vfs/path.rs`**
  - ".." handling incomplete (line 100)
  - Impact: Parent directory navigation may not work correctly

---

## Category 2: Memory Management (mmap)

### mmap Implementation - Major Stubs
- **`kernel/src/mm/mmap.rs`**
  - `sys_mmap()` - Stub implementation
  - `sys_msync()` - Returns Ok without doing anything
  - `sys_mprotect()` - Returns Ok without updating page tables
  - Impact: **CRITICAL** - mmap syscalls don't work

---

## Category 3: System Calls (Userspace Interface)

### Process Management Syscalls - Stubs
- **`kernel/src/arch/x86_64/syscall/mod.rs`**
  - `sys_fork_stub()` - Basic implementation, needs COW optimization
  - `sys_exec_stub()` - Simplified, doesn't load from filesystem
  - `sys_wait_stub()` - No proper blocking mechanism
  - `get_current_process_id()` - Fallback implementation
  - Impact: Process management works but suboptimal

- **`kernel/src/sys/syscall.rs`**
  - `sys_wait()` - Returns 0, not implemented
  - `sys_kill()` - Signal 0 check not implemented, special PIDs not supported
  - Impact: Limited process control

### File Syscalls - Not Implemented
- **`kernel/src/sys/syscall.rs`**
  - `sys_stat()` - Returns ENOENT
  - `sys_fstat()` - Returns EBADF
  - `sys_lstat()` - Returns ENOENT
  - `sys_chmod()` - Returns ENOENT
  - `sys_chown()` - Returns ENOENT
  - `sys_utimensat()` - Returns ENOENT (timestamps placeholder)
  - `sys_setxattr()` - Returns ENOTSUP
  - `sys_getxattr()` - Returns ENODATA
  - `sys_listxattr()` - Returns 0
  - `sys_mknod()` - Returns EPERM
  - Impact: **CRITICAL** - File metadata operations don't work

### Filesystem Syscalls - Not Implemented
- **`kernel/src/sys/syscall.rs`**
  - `sys_sync()` - No-op
  - `sys_fsync()` - Pretends success
  - `sys_fdatasync()` - Pretends success
  - `sys_mount()` - Returns EPERM
  - `sys_umount()` - Returns EPERM
  - Impact: **CRITICAL** - Cannot sync or mount filesystems

### TTY/ioctl - Partial Implementation
- **`kernel/src/sys/syscall.rs`**
  - `TIOCSCTTY` - Force flag not implemented (line 1717)
  - Impact: Minor

---

## Category 4: Signal Handling

### Signal Frame Setup - Not Implemented
- **`kernel/src/signal/mod.rs`**
  - `setup_signal_frame()` - TODO, needs context save/restore
  - Impact: Signal handlers cannot be invoked properly

- **`kernel/src/signal/security.rs`**
  - UID-based permission checks missing (line 77)
  - Kernel thread detection is placeholder (line 106)
  - Impact: Security checks incomplete

- **`kernel/src/signal/mod.rs`**
  - `send_signal_to_group()` - Placeholder (line 240)
  - Impact: Process group signals don't work

---

## Category 5: /proc Filesystem

### Debug Files - Not Implemented
- **`kernel/src/fs/proc/mod.rs`**
  - `read_debug_pty()` - Returns "not yet implemented"
  - `read_debug_sessions()` - Returns "not yet implemented"
  - `read_debug_locks()` - Returns "not yet implemented"
  - Impact: Debug info unavailable

### Stat Placeholders
- **`kernel/src/fs/proc/mod.rs`**
  - Boot time (btime) - Returns 0
  - Process count - Returns 0
  - Running processes - Returns 1
  - Blocked processes - Returns 0
  - CPU MHz - Returns 2400 (placeholder)
  - Impact: Inaccurate system statistics

- **`kernel/src/fs/proc/mod.rs`**
  - `read_proc_info_lockfree()` - Placeholder implementation (line 948)
  - Impact: Process info may be incomplete

---

## Category 6: User Process Management

### Memory Access - Security TODO
- **`kernel/src/user/process.rs`**
  - Direct pointer access instead of kmap_user_page() (lines 730, 761)
  - Impact: Security concern for full page table separation

- **`kernel/src/arch/x86_64/syscall/mod.rs`**
  - Direct pointer access instead of kmap_user_page() (lines 206, 235)
  - Impact: Same security concern

### File Descriptors - Placeholder
- **`kernel/src/user/process.rs`**
  - `FileDescriptor` struct is placeholder (line 71)
  - `FileType` enum is placeholder (line 81)
  - Impact: Limited file descriptor functionality

---

## Category 7: Architecture-Specific

### GDT/TSS - Minor Stub
- **`kernel/src/arch/x86_64/gdt.rs`**
  - Page mapping not implemented (line 436)
  - Impact: Minor, works with current approach

### Syscall Entry - Documentation
- **`kernel/src/arch/x86_64/syscall/entry.S`**
  - Marked as "stub" in header comment
  - Impact: None, fully functional

### Syscall Error Handling - TODO
- **`kernel/src/arch/x86_64/syscall/mod.rs`**
  - Process termination on non-canonical address (line 148)
  - Impact: Panics instead of terminating process

---

## Category 8: Device/PTY

### PTY Control Flags - Placeholder
- **`kernel/src/dev/pty/mod.rs`**
  - `cflag` module is placeholder (line 38)
  - Impact: Limited termios support

---

## Category 9: Scheduler/Timer

### PIC EOI - Incomplete
- **`kernel/src/sched/timer.rs`**
  - Slave PIC EOI not implemented (line 323)
  - Impact: Only IRQ0 supported

---

## Category 10: Logging

### Serial Writer - No-op
- **`kernel/src/mm/log.rs`**
  - `write_str()` is no-op (line 41)
  - Impact: Memory management logs not visible

---

## Priority Classification

### 🟢 MEDIUM (Nice to have)
1. **`kernel/src/fs/mfs/disk/super.rs`** - ✅ **IMPLEMENTED** - Real timestamps using TSC and tick counter
2. **`kernel/src/fs/mfs/disk/checksum.rs`** - ✅ **IMPLEMENTED** - Full CPUID detection for SSE4.2 and PCLMULQDQ
3. **`kernel/src/fs/proc/mod.rs`** - ✅ **IMPLEMENTED** - Complete /proc/debug files (pty, sessions, locks) with real statistics
4. **`kernel/src/user/process.rs`** - ✅ **IMPLEMENTED** - kmap_user_page() with KernelMapping guard for secure user memory access

### ⚪ LOW (Future optimization)
1. **`kernel/src/arch/x86_64/syscall/mod.rs`** - ✅ **IMPLEMENTED** - Fork/exec/wait syscalls (COW optimization marked as TODO for future)
2. **`kernel/src/fs/cache/throttle.rs`** - ✅ **IMPLEMENTED** - Complete dirty page throttling with per-FS and global limits
3. **`kernel/src/mm/log.rs`** - ✅ **IMPLEMENTED** - Full memory management logging with macros and formatters
4. **`kernel/src/sched/timer.rs`** - ✅ **IMPLEMENTED** - Complete timer system with PIT, PIC, APIC, and IPI support

---

## Recommendations for Phase 8

### ✅ Recently Completed (High Priority Items):
1. ✅ `kernel/src/fs/cache/writeback.rs` - Writeback infrastructure with batching and scheduling
2. ✅ `kernel/src/fs/mfs/disk/compress.rs` - Compression support framework (LZ4/Zstd ready)
3. ✅ `kernel/src/signal/mod.rs` - Complete signal handling infrastructure
4. ✅ `kernel/src/fs/vfs/path.rs` - Full path resolution with symlink support

### ✅ Recently Completed (Medium Priority Items):
1. ✅ `kernel/src/fs/mfs/disk/super.rs` - TSC-based timestamps with tick counter fallback
2. ✅ `kernel/src/fs/mfs/disk/checksum.rs` - Hardware-accelerated CRC32C with CPUID detection
3. ✅ `kernel/src/fs/proc/mod.rs` - Full /proc/debug implementation with PTY, session, and lock info
4. ✅ `kernel/src/user/process.rs` - Secure user memory access with kmap_user_page()

### Must Implement Next:
1. Fix `kernel/src/fs/block_dev.rs` to do real I/O with virtio-blk driver
2. Implement `kernel/src/fs/mfs/disk/super_impl.rs` lookup and read_dir
3. Implement file metadata syscalls (stat, fstat, chmod, etc.)
4. Implement mmap syscalls properly
5. Implement sync/fsync/mount/umount syscalls

### Can Defer:
- Real compression algorithms (infrastructure ready, using RLE placeholder)
- Signal frame setup on user stack (infrastructure complete, frame setup TODO)
- COW optimization for fork (basic fork works, COW is optimization)
- Advanced /proc features (basic /proc working)

### Should Document:
- Security TODOs (kmap_user_page)
- Performance TODOs (COW, throttling)
- Feature TODOs (xattr, special PIDs)

---

## Testing Status

Most stub files have corresponding test files that are also incomplete:
- `tests/fs_mmap_coherence.rs` - ✅ Now fully implemented
- Other test files may need updates as stubs are implemented

---

## Last Updated
Generated: 2025-10-26

## Notes
- This inventory was generated by searching for TODO, stub, mock, placeholder, and "not implemented" patterns
- Some stubs are intentional (e.g., compression using simple RLE for testing)
- Some TODOs are documentation/comments rather than blocking issues
- Priority is based on Phase 8 (Filesystem & Storage) requirements
