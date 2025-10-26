# Design Document: Technical Debt Resolution

## Overview

This design outlines a systematic approach to resolving 149 TODO items across the MelloOS codebase. The strategy prioritizes items by impact (security, correctness, features, documentation), organizes work by subsystem dependencies, and ensures each resolution is tested and verified. The design follows a phased approach aligned with the current roadmap (Phase 8: Filesystem & Storage completed, preparing for Phase 9).

## Architecture

### Resolution Strategy

The technical debt resolution follows a **dependency-aware, priority-based approach**:

```
┌─────────────────────────────────────────────────────────┐
│              Technical Debt Resolution Flow              │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
                  ┌──────────────────┐
                  │  Categorization  │
                  │  & Prioritization│
                  └──────────────────┘
                            │
                ┌───────────┴───────────┐
                ▼                       ▼
        ┌──────────────┐        ┌──────────────┐
        │ High Priority│        │Med Priority  │
        │ (Security,   │        │ (Features,   │
        │  Correctness)│        │  Optimization)│
        └──────────────┘        └──────────────┘
                │                       │
                └───────────┬───────────┘
                            ▼
                  ┌──────────────────┐
                  │ Dependency       │
                  │ Analysis         │
                  └──────────────────┘
                            │
                            ▼
                  ┌──────────────────┐
                  │ Implementation   │
                  │ (One TODO at     │
                  │  a time)         │
                  └──────────────────┘
                            │
                            ▼
                  ┌──────────────────┐
                  │ Verification     │
                  │ (cargo check +   │
                  │  tests)          │
                  └──────────────────┘
                            │
                            ▼
                  ┌──────────────────┐
                  │ Documentation    │
                  │ Update           │
                  └──────────────────┘
```

### Priority Classification

**High Priority (🔴):**
- Security vulnerabilities or missing security checks
- Correctness issues that could cause data corruption
- Core functionality required for current phase
- Items blocking other work

**Medium Priority (🟡):**
- Feature completeness (non-critical features)
- Performance optimizations
- User experience improvements
- Integration improvements

**Low Priority (🟢):**
- Documentation improvements
- Logging enhancements
- Code cleanup and refactoring
- Nice-to-have features

### Subsystem Dependencies

```
Memory Management (Foundation)
    │
    ├─→ Process Management
    │       │
    │       ├─→ Syscalls
    │       │       │
    │       │       └─→ Userspace Programs
    │       │
    │       └─→ Signals
    │               │
    │               └─→ Devices (PTY)
    │
    └─→ Filesystem
            │
            ├─→ VFS
            ├─→ MFS
            └─→ Cache
                    │
                    └─→ Tests
```

## Components and Interfaces

### 1. TODO Resolution Tracker

**Purpose:** Track progress and ensure no TODO is missed

**Interface:**
```rust
struct TodoItem {
    id: usize,
    file: String,
    line: usize,
    category: TodoCategory,
    priority: Priority,
    description: String,
    dependencies: Vec<usize>,
    status: TodoStatus,
}

enum TodoCategory {
    MemoryManagement,
    Filesystem,
    ProcessManagement,
    Syscalls,
    Drivers,
    Signals,
    Devices,
    Userspace,
    Tests,
}

enum Priority {
    High,    // Security, correctness
    Medium,  // Features, optimization
    Low,     // Documentation, logging
}

enum TodoStatus {
    NotStarted,
    InProgress,
    Testing,
    Completed,
    Deferred(String), // Reason for deferral
}
```

**Implementation:** Use TECHNICAL_DEBT_CATEGORIZED.md as source of truth, update after each resolution.

### 2. Memory Management Resolution Component

**TODOs to Address (12 items):**

1. **Security Flag Verification (High Priority)**
   - `kernel/src/mm/security.rs:109, 149, 205, 267, 595`
   - Implement actual page flag checks (USER, WRITABLE)
   - Use page table entry inspection

2. **Logging Infrastructure (Low Priority)**
   - `kernel/src/mm/log.rs:41`
   - `kernel/src/mm/mod.rs:457`
   - `kernel/src/mm/allocator.rs:285, 289, 312`
   - `kernel/src/mm/pmm.rs:119, 178`
   - Connect to serial output infrastructure

**Design Approach:**
- Extract page flags from page table entries using existing paging infrastructure
- Add helper functions: `get_page_flags(vaddr: VirtAddr) -> PageFlags`
- Integrate with existing `serial_println!` macro for logging
- Ensure SMP-safe access to page tables

### 3. Filesystem Resolution Component

**TODOs to Address (21 items):**

1. **Process Information Integration (Medium Priority)**
   - `/proc` filesystem TODOs (8 items)
   - Connect to actual task structures for TPGID, cmdline, timing

2. **VFS Parent Tracking (High Priority)**
   - `kernel/src/fs/vfs/path.rs:100`
   - Implement proper dentry parent pointers

3. **Cache Writeback (High Priority)**
   - `kernel/src/fs/cache/writeback.rs:292, 306`
   - Implement actual flushing to block devices

4. **Time Tracking (Medium Priority)**
   - `kernel/src/fs/mfs/ram/inode.rs:162, 219`
   - Use kernel timer infrastructure for timestamps

5. **Filesystem Syscalls (High Priority)**
   - `kernel/src/fs/syscalls.rs:6`
   - Implement deferred syscall interfaces

**Design Approach:**
- Add `parent: Option<Arc<Dentry>>` to Dentry structure
- Implement `flush_to_device()` using BlockDevice trait
- Add `get_kernel_time()` helper using timer subsystem
- Complete syscall implementations with proper error handling

### 4. Process Management Resolution Component

**TODOs to Address (14 items):**

1. **Heap Tracking (High Priority)**
   - `kernel/src/user/exec.rs:297, 335, 407`
   - Add `heap_start` and `heap_end` fields to Task structure

2. **Page Table Separation (High Priority)**
   - `kernel/src/user/exec.rs:392`
   - `kernel/src/user/elf.rs:439, 507, 603`
   - `kernel/src/user/process.rs:859, 899, 1168, 1171`
   - Implement per-process page tables with proper isolation

3. **Temporary Kernel Mappings (Medium Priority)**
   - `kernel/src/user/process.rs:818`
   - Implement `kmap_user_page()` for safe user memory access

**Design Approach:**
```rust
// Add to Task structure
struct Task {
    // ... existing fields ...
    heap_start: VirtAddr,
    heap_end: VirtAddr,
    page_table: Option<Arc<PageTable>>, // Per-process page table
}

// Kernel mapping helper
fn kmap_user_page(user_addr: VirtAddr, process: &Task) -> Result<*mut u8, Error> {
    // Temporarily map user page into kernel address space
    // Return kernel virtual address
    // Ensure proper cleanup
}
```

### 5. Syscall Resolution Component

**TODOs to Address (12 items):**

1. **Process Termination (High Priority)**
   - `kernel/src/arch/x86_64/syscall/mod.rs:148`
   - Implement proper exit handling

2. **Safe User Memory Access (High Priority)**
   - Lines 206, 235 - Use kmap_user_page when page tables are separated

3. **Buffered I/O (Medium Priority)**
   - Line 497 - Implement chunked writes for large data

4. **Parent Wakeup (High Priority)**
   - Lines 559, 569 - Wake waiting parents on child exit

5. **Copy-on-Write (Medium Priority)**
   - Lines 704, 722 - Implement COW for fork optimization

6. **Child Tracking (High Priority)**
   - Lines 1134, 1142 - Proper child process management

**Design Approach:**
- Add `children: Vec<Pid>` to Task structure
- Implement `wake_parent()` using scheduler wakeup mechanism
- Add COW page fault handler in fault.rs
- Implement proper waitpid blocking/wakeup

### 6. Driver Resolution Component

**TODOs to Address (2 items):**

1. **virtio-blk Virtqueue Submission (High Priority)**
   - `kernel/src/drivers/block/virtio_blk.rs:132, 152`
   - Implement actual virtqueue descriptor chain setup
   - Handle completion interrupts

**Design Approach:**
```rust
impl VirtioBlkDevice {
    fn submit_request(&mut self, req: &BlockRequest) -> Result<(), Error> {
        // 1. Allocate descriptor chain
        // 2. Fill request header
        // 3. Add data buffers
        // 4. Add status buffer
        // 5. Ring doorbell
        // 6. Wait for completion interrupt
    }
}
```

### 7. Signal Resolution Component

**TODOs to Address (5 items):**

1. **UID Permission Checks (Medium Priority)**
   - `kernel/src/signal/security.rs:50, 77`
   - Defer until user management (Phase 10+)

2. **Kernel Thread Protection (High Priority)**
   - Line 122 - Prevent signals to kernel threads

3. **Executable Page Verification (Medium Priority)**
   - Line 158 - Verify signal handler in executable memory

4. **Signal Frame Setup (High Priority)**
   - `kernel/src/signal/mod.rs:409`
   - Implement user stack frame for signal delivery

**Design Approach:**
```rust
fn setup_signal_frame(task: &mut Task, sig: Signal) -> Result<(), Error> {
    // 1. Save current context to user stack
    // 2. Set up return trampoline
    // 3. Modify user RIP to signal handler
    // 4. Modify user RSP to signal frame
}
```

### 8. Device Resolution Component

**TODOs to Address (5 items):**

1. **Process Group Signal Delivery (High Priority)**
   - `kernel/src/dev/pty/mod.rs:494, 524`
   - Send signals to all processes in foreground group

2. **PGID Retrieval (High Priority)**
   - Lines 551, 771, 837
   - Get actual PGID from task structure

**Design Approach:**
- Add `pgid: Pid` field to Task structure
- Implement `send_signal_to_group(pgid: Pid, sig: Signal)`
- Iterate task table and deliver to matching PGID

### 9. Userspace Program Resolution Component

**TODOs to Address (6 items):**

1. **mello-term ANSI Support (Medium Priority)**
   - `kernel/userspace/mello-term/src/ansi.rs:180`
   - Implement clear line modes (0, 1, 2)

2. **mello-term Event Loop (High Priority)**
   - `kernel/userspace/mello-term/src/main.rs:166`
   - Implement main event loop

3. **Keyboard Input (High Priority)**
   - `kernel/userspace/mello-term/src/input.rs:29, 54`
   - Read from PTY master

4. **Non-blocking I/O (Medium Priority)**
   - `kernel/userspace/mello-term/src/pty.rs:106`
   - Use fcntl when available

5. **Filesystem Querying (Medium Priority)**
   - `kernel/userspace/mellobox/src/commands/df.rs:20`
   - Query mounted filesystems

**Design Approach:**
- Implement ANSI escape sequence handlers
- Event loop: poll PTY, keyboard, handle input/output
- Use read() syscall for keyboard input
- Implement statfs syscall for df command

### 10. Test Resolution Component

**TODOs to Address (68 items):**

**Categories:**
1. **VFS Integration Tests** (13 items) - Defer until VFS fully operational
2. **Cache Tests** (11 items) - Implement when cache integrated
3. **MFS Disk Tests** (14 items) - Implement when disk backend complete
4. **Userland Smoke Tests** (13 items) - Implement when userspace stable
5. **Fault Injection Tests** (14 items) - Implement for robustness
6. **Performance Tests** (3 items) - Implement for benchmarking

**Design Approach:**
- Create test infrastructure in `tests/` directory
- Use integration test framework
- Each test should be self-contained
- Tests should clean up after themselves
- Use QEMU for end-to-end testing

## Data Models

### TODO Tracking Database

```markdown
# TECHNICAL_DEBT_TRACKING.md

## In Progress
- [ ] MM-001: Implement page flag verification (security.rs:109)
  - Assigned: [Developer]
  - Started: [Date]
  - Blocked by: None

## Completed
- [x] MM-002: Add serial logging (mm/log.rs:41)
  - Completed: [Date]
  - Verified: cargo check + boot test

## Deferred
- [ ] SIG-001: UID permission checks (signal/security.rs:77)
  - Reason: Waiting for Phase 10 user management
  - Revisit: Phase 10
```

### Task Structure Extensions

```rust
// kernel/src/sched/task.rs
pub struct Task {
    // ... existing fields ...
    
    // New fields for TODO resolution
    pub heap_start: VirtAddr,
    pub heap_end: VirtAddr,
    pub page_table: Option<Arc<PageTable>>,
    pub pgid: Pid,
    pub children: Vec<Pid>,
}
```

## Error Handling

### Resolution Verification

Each TODO resolution must pass:

1. **Compilation Check**
   ```bash
   cd kernel && cargo check
   ```

2. **Subsystem Tests**
   ```bash
   cargo test --test <subsystem>_*
   ```

3. **Integration Test**
   ```bash
   make clean && make build && make iso
   ./tools/qemu.sh
   # Verify functionality
   ```

4. **Regression Check**
   ```bash
   ./tools/testing/test_user_mode_integration.sh
   ./tools/testing/run_filesystem_tests.sh
   ```

### Rollback Strategy

If a TODO resolution causes regressions:

1. Revert the change
2. Analyze the failure
3. Create a more targeted fix
4. Re-test thoroughly
5. Document the issue in troubleshooting guide

## Testing Strategy

### Per-TODO Testing

**For each TODO resolution:**

1. **Unit Test** (if applicable)
   - Test the specific function/module
   - Verify edge cases

2. **Integration Test**
   - Test interaction with other subsystems
   - Verify no regressions

3. **System Test**
   - Boot in QEMU
   - Run relevant userspace programs
   - Verify expected behavior

### Subsystem Testing

**After completing all TODOs in a subsystem:**

1. Run full subsystem test suite
2. Run cross-subsystem integration tests
3. Performance benchmarks (if applicable)
4. Update architecture documentation

### Final Verification

**After all TODOs resolved:**

1. Full test suite execution
2. Performance regression testing
3. Security audit of changes
4. Documentation review and update
5. Update roadmap.md with completion status

## Implementation Phases

### Phase 1: High Priority Security & Correctness (Weeks 1-3)

**Focus:** Items that affect security, data integrity, or core functionality

**Subsystems:**
1. Memory Management security checks (5 items)
2. Filesystem writeback and VFS parent tracking (3 items)
3. Process management heap tracking (3 items)
4. Syscall process termination and parent wakeup (4 items)
5. Driver virtqueue implementation (2 items)
6. Signal kernel thread protection and frame setup (2 items)
7. Device process group handling (5 items)
8. Userspace event loop and keyboard input (3 items)

**Total: ~27 high-priority items**

### Phase 2: Medium Priority Features & Optimization (Weeks 4-6)

**Focus:** Feature completeness and performance improvements

**Subsystems:**
1. Memory Management logging (7 items)
2. Filesystem time tracking and /proc integration (10 items)
3. Process management page table separation (8 items)
4. Syscall buffered I/O and COW (4 items)
5. Signal UID checks and handler verification (3 items)
6. Userspace ANSI support and filesystem querying (3 items)

**Total: ~35 medium-priority items**

### Phase 3: Low Priority & Test Implementation (Weeks 7-10)

**Focus:** Documentation, logging, and comprehensive test coverage

**Subsystems:**
1. Test infrastructure setup
2. VFS integration tests (13 items)
3. Cache behavior tests (11 items)
4. MFS disk tests (14 items)
5. Userland smoke tests (13 items)
6. Fault injection tests (14 items)
7. Performance benchmarks (3 items)

**Total: ~68 test items**

### Phase 4: Documentation & Cleanup (Week 11)

**Focus:** Final documentation and verification

**Tasks:**
1. Update all architecture docs
2. Update TECHNICAL_DEBT_CATEGORIZED.md
3. Remove all completed TODO comments
4. Update roadmap.md
5. Create completion summary
6. Final regression testing

## Dependencies and Prerequisites

### External Dependencies

- **Current Phase:** Phase 8 (Filesystem & Storage) completed
- **Kernel Infrastructure:** Serial output, timer, scheduler operational
- **Test Infrastructure:** QEMU, integration test scripts available

### Internal Dependencies

```
Memory Management TODOs
    ↓
Process Management TODOs
    ↓
Syscall TODOs
    ↓
Userspace Program TODOs
    ↓
Test TODOs

Filesystem TODOs (parallel track)
    ↓
Cache TODOs
    ↓
Test TODOs
```

### Blocking Issues

None identified. All TODOs can be resolved with existing infrastructure.

## Performance Considerations

### Resolution Impact

- **Memory Management:** Minimal impact, mostly adding checks
- **Filesystem:** Writeback may affect I/O performance (needs benchmarking)
- **Process Management:** Page table separation will improve security, may slightly impact context switch time
- **Syscalls:** Buffered I/O will improve large write performance

### Optimization Opportunities

1. **COW Implementation:** Significant memory savings for fork-heavy workloads
2. **Cache Writeback:** Batch writes for better I/O performance
3. **Per-Process Page Tables:** Better isolation, potential TLB optimization

## Security Implications

### Security Improvements

1. **Memory Security:** Proper page flag verification prevents privilege escalation
2. **Process Isolation:** Per-process page tables improve isolation
3. **Signal Security:** Kernel thread protection prevents signal-based attacks
4. **User Memory Access:** Safe mapping prevents kernel memory corruption

### Security Testing

After resolution:
1. Verify all user pointer validation
2. Test privilege escalation scenarios
3. Verify signal permission checks
4. Test page table isolation

## Monitoring and Metrics

### Progress Tracking

**Metrics to track:**
- TODOs resolved per week
- Test coverage percentage
- Regression count
- Build time impact
- Boot time impact

**Reporting:**
- Weekly progress updates
- Blocker identification
- Risk assessment

### Success Criteria

**Phase 1 Complete:**
- All high-priority TODOs resolved
- No regressions in existing tests
- System boots and runs userspace programs

**Phase 2 Complete:**
- All medium-priority TODOs resolved
- Feature completeness verified
- Performance benchmarks pass

**Phase 3 Complete:**
- All test TODOs implemented
- Test coverage > 80%
- All tests passing

**Phase 4 Complete:**
- All documentation updated
- Zero TODO comments remaining (except legitimately deferred)
- Final regression testing passed

## Rollout Strategy

### Incremental Approach

1. **One TODO at a time:** Never resolve multiple TODOs in a single commit
2. **Immediate verification:** Run cargo check after each change
3. **Frequent testing:** Boot test after every 3-5 TODO resolutions
4. **Documentation updates:** Update tracking doc after each completion

### Risk Mitigation

1. **Backup before major changes:** Git commits before risky changes
2. **Incremental testing:** Test after each subsystem completion
3. **Rollback plan:** Keep previous working version available
4. **Peer review:** Review complex changes before integration

## Conclusion

This design provides a systematic, dependency-aware approach to resolving all 149 TODO items in MelloOS. By prioritizing security and correctness, organizing work by subsystem dependencies, and ensuring thorough testing at each step, we can eliminate technical debt while maintaining system stability and improving overall code quality.

The phased approach allows for incremental progress with regular verification points, minimizing risk while maximizing the value delivered at each phase completion.
