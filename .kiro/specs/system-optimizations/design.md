# System Optimizations & Advanced Features - Design

**Phase:** 9+ (Post Phase 8 Completion)  
**Related:** requirements.md, tasks.md

---

## Architecture Overview

This design document details the technical approach for implementing all deferred optimization features. The implementation is divided into 4 sub-phases (9A-9D) to manage complexity and risk.

---

## 1. Per-Process Page Tables (Phase 9A)

### Current Architecture
```
┌─────────────────────────────────────┐
│   All Processes Share One CR3       │
│                                     │
│  ┌──────────┐  ┌──────────┐       │
│  │Process 1 │  │Process 2 │       │
│  └──────────┘  └──────────┘       │
│         │            │             │
│         └────────────┘             │
│                │                   │
│         ┌──────▼──────┐           │
│         │  Single PT  │           │
│         └─────────────┘           │
└─────────────────────────────────────┘
```

### New Architecture
```
┌─────────────────────────────────────┐
│   Each Process Has Own CR3          │
│                                     │
│  ┌──────────┐  ┌──────────┐       │
│  │Process 1 │  │Process 2 │       │
│  │  CR3=A   │  │  CR3=B   │       │
│  └────┬─────┘  └────┬──────┘      │
│       │             │              │
│  ┌────▼─────┐  ┌───▼──────┐      │
│  │   PT A   │  │   PT B   │      │
│  │(User+Krn)│  │(User+Krn)│      │
│  └──────────┘  └──────────┘      │
└─────────────────────────────────────┘
```

### Implementation Details

#### Data Structures
```rust
pub struct Process {
    pid: usize,
    cr3: PhysAddr,  // Physical address of page table root
    // ... existing fields
}

pub struct PageTable {
    root: PhysAddr,
    refcount: AtomicUsize,  // For COW
}
```

#### Context Switch Changes
```rust
fn switch_to_process(new_pid: usize) {
    let new_process = get_process(new_pid);
    
    // Save current CR3
    let old_cr3 = read_cr3();
    save_cr3(current_pid, old_cr3);
    
    // Load new CR3
    let new_cr3 = new_process.cr3;
    write_cr3(new_cr3);
    
    // TLB is automatically flushed by CR3 write
}
```

#### Memory Layout
```
User Space:   0x0000_0000_0000_0000 - 0x0000_7FFF_FFFF_FFFF
Kernel Space: 0xFFFF_8000_0000_0000 - 0xFFFF_FFFF_FFFF_FFFF

Kernel mappings are shared (same physical pages in all page tables)
User mappings are unique per process
```

---

## 2. Copy-on-Write (Phase 9B)

### COW Page Lifecycle

```
1. Fork:
   Parent: [RW Page A] ──┐
                         ├──> [RO Page A] (shared, refcount=2)
   Child:  [RO Page A] ──┘

2. Write by Child:
   Parent: [RO Page A] (refcount=1)
   Child:  [RW Page B] (new copy)

3. Write by Parent:
   Parent: [RW Page A] (original, refcount=0)
   Child:  [RW Page B] (copy)
```

### Page Table Entry Format
```
Bit 63: COW flag (custom)
Bit 0:  Present
Bit 1:  Writable (cleared for COW pages)
Bit 2:  User
```

### Page Fault Handler
```rust
fn handle_cow_fault(fault_addr: VirtAddr) -> Result<(), FaultError> {
    let pte = get_pte(fault_addr)?;
    
    if !pte.is_cow() {
        return Err(FaultError::NotCOW);
    }
    
    let old_page = pte.phys_addr();
    let refcount = get_refcount(old_page);
    
    if refcount == 1 {
        // Last reference, just make writable
        pte.set_writable(true);
        pte.clear_cow();
    } else {
        // Multiple references, need to copy
        let new_page = alloc_page()?;
        copy_page(old_page, new_page);
        pte.set_phys_addr(new_page);
        pte.set_writable(true);
        pte.clear_cow();
        dec_refcount(old_page);
    }
    
    flush_tlb(fault_addr);
    Ok(())
}
```

### Reference Counting
```rust
struct PageRefcount {
    counts: HashMap<PhysAddr, AtomicUsize>,
    lock: SpinLock<()>,
}

impl PageRefcount {
    fn inc(&self, page: PhysAddr) {
        let count = self.counts.entry(page)
            .or_insert(AtomicUsize::new(0));
        count.fetch_add(1, Ordering::SeqCst);
    }
    
    fn dec(&self, page: PhysAddr) -> usize {
        if let Some(count) = self.counts.get(&page) {
            let old = count.fetch_sub(1, Ordering::SeqCst);
            if old == 1 {
                // Last reference, free page
                self.counts.remove(&page);
                free_page(page);
            }
            old - 1
        } else {
            0
        }
    }
}
```

---

## 3. Process Blocking & Wakeup (Phase 9C)

### Task States
```rust
pub enum TaskState {
    Ready,      // In runqueue, can be scheduled
    Running,    // Currently executing
    Blocked,    // Waiting for event
    Zombie,     // Exited, waiting for parent
}

pub enum BlockReason {
    WaitChild(usize),  // Waiting for specific child
    WaitAnyChild,      // Waiting for any child
    WaitIO(usize),     // Waiting for I/O on FD
    WaitSignal,        // Waiting for signal
    Sleep(u64),        // Sleeping until tick
}
```

### Wait Queue
```rust
struct WaitQueue {
    tasks: Vec<(usize, BlockReason)>,  // (task_id, reason)
    lock: SpinLock<()>,
}

impl WaitQueue {
    fn add(&mut self, task_id: usize, reason: BlockReason) {
        self.tasks.push((task_id, reason));
        set_task_state(task_id, TaskState::Blocked);
        remove_from_runqueue(task_id);
    }
    
    fn wake(&mut self, condition: WakeCondition) {
        let mut to_wake = Vec::new();
        
        self.tasks.retain(|(task_id, reason)| {
            if condition.matches(reason) {
                to_wake.push(*task_id);
                false  // Remove from wait queue
            } else {
                true   // Keep in wait queue
            }
        });
        
        for task_id in to_wake {
            set_task_state(task_id, TaskState::Ready);
            add_to_runqueue(task_id);
        }
    }
}
```

### sys_wait Implementation
```rust
fn sys_wait_blocking(child_pid: usize) -> isize {
    // Check for zombie children first
    if let Some((pid, exit_code)) = find_zombie_child(child_pid) {
        cleanup_zombie(pid);
        return encode_wait_result(pid, exit_code);
    }
    
    // No zombie found, block the process
    let current_pid = get_current_pid();
    let reason = if child_pid == 0 {
        BlockReason::WaitAnyChild
    } else {
        BlockReason::WaitChild(child_pid)
    };
    
    WAIT_QUEUE.lock().add(current_pid, reason);
    
    // Yield to scheduler (will not return until woken)
    yield_now();
    
    // When we get here, we've been woken up
    // Check again for zombie (it should be there now)
    if let Some((pid, exit_code)) = find_zombie_child(child_pid) {
        cleanup_zombie(pid);
        return encode_wait_result(pid, exit_code);
    }
    
    // Shouldn't happen, but handle gracefully
    return -ECHILD;
}
```

### Wakeup on Exit
```rust
fn sys_exit(exit_code: i32) {
    let current_pid = get_current_pid();
    let parent_pid = get_parent_pid(current_pid);
    
    // Mark as zombie
    set_task_state(current_pid, TaskState::Zombie);
    set_exit_code(current_pid, exit_code);
    
    // Wake up parent if waiting
    WAIT_QUEUE.lock().wake(WakeCondition::ChildExited {
        parent: parent_pid,
        child: current_pid,
    });
    
    // Never return
    yield_now();
    unreachable!();
}
```

---

## 4. Memory Mapping Improvements (Phase 9B)

### mprotect Implementation
```rust
fn sys_mprotect(addr: VirtAddr, len: usize, prot: ProtFlags) -> Result<(), Error> {
    let page_start = addr.align_down(PAGE_SIZE);
    let page_end = (addr + len).align_up(PAGE_SIZE);
    
    for page in (page_start..page_end).step_by(PAGE_SIZE) {
        let pte = get_pte_mut(page)?;
        
        // Update permissions
        pte.set_writable(prot.is_writable());
        pte.set_executable(prot.is_executable());
        pte.set_user(prot.is_user());
        
        // Flush TLB on this CPU
        flush_tlb_page(page);
    }
    
    // Send IPI to other CPUs to flush their TLBs
    tlb_shootdown(page_start, page_end);
    
    Ok(())
}
```

### msync Implementation
```rust
fn sys_msync(addr: VirtAddr, len: usize, flags: MsyncFlags) -> Result<(), Error> {
    let mapping = find_mapping(addr)?;
    
    if !mapping.is_file_backed() {
        return Ok(());  // Nothing to sync for anonymous mappings
    }
    
    let file = mapping.file()?;
    let offset = mapping.offset_of(addr);
    
    // Get dirty pages in range
    let dirty_pages = get_dirty_pages(addr, len);
    
    for page in dirty_pages {
        let data = read_page(page);
        file.write_at(offset + page.offset(), data)?;
        mark_clean(page);
    }
    
    if flags.is_sync() {
        // MS_SYNC: wait for writes to complete
        file.sync()?;
    }
    // MS_ASYNC: return immediately, writes happen in background
    
    Ok(())
}
```

### File-Backed Mappings with Demand Paging

**Design Rationale:** Demand paging reduces memory usage by loading file pages only when accessed, improving performance for large files.

```rust
struct FileMapping {
    file: Arc<dyn File>,
    offset: u64,
    length: usize,
    flags: MapFlags,  // MAP_SHARED or MAP_PRIVATE
}

fn handle_file_mapping_fault(addr: VirtAddr, mapping: &FileMapping) -> Result<(), Error> {
    let page_offset = (addr - mapping.start).as_u64();
    let file_offset = mapping.offset + page_offset;
    
    // Allocate physical page
    let page = alloc_page()?;
    
    // Read file data into page
    let data = mapping.file.read_at(file_offset, PAGE_SIZE)?;
    copy_to_page(page, &data);
    
    // Map page with appropriate permissions
    let pte = get_pte_mut(addr)?;
    pte.set_phys_addr(page);
    pte.set_present(true);
    pte.set_user(true);
    
    if mapping.flags.contains(MapFlags::MAP_SHARED) {
        // Shared mapping: writes go to file
        pte.set_writable(mapping.is_writable());
        register_shared_mapping(mapping.file.inode_id(), page_offset, page);
    } else {
        // Private mapping: use COW
        pte.set_writable(false);
        pte.set_cow(true);
        inc_refcount(page);
    }
    
    flush_tlb_page(addr);
    Ok(())
}
```

### MAP_GROWSDOWN for Stack Expansion

**Design Rationale:** Automatic stack growth simplifies userspace programming and matches POSIX behavior.

```rust
fn handle_growsdown_fault(addr: VirtAddr, mapping: &mut MemoryMapping) -> Result<(), Error> {
    // Check if fault is within guard page distance
    if addr < mapping.start && addr >= mapping.start - STACK_GUARD_SIZE {
        return Err(Error::StackOverflow);
    }
    
    // Extend mapping downward
    let new_start = addr.align_down(PAGE_SIZE);
    let extension_size = mapping.start - new_start;
    
    // Allocate and map new pages
    for page_addr in (new_start..mapping.start).step_by(PAGE_SIZE) {
        let page = alloc_page()?;
        zero_page(page);
        map_page(page_addr, page, PageFlags::USER | PageFlags::WRITABLE)?;
    }
    
    mapping.start = new_start;
    mapping.length += extension_size;
    
    Ok(())
}
```

---

## 5. Background Writeback (Phase 9C)

### Kernel Threading

**Design Rationale:** Kernel threads enable background tasks without userspace overhead, essential for async I/O operations.

```rust
struct KernelThread {
    id: usize,
    name: &'static str,
    entry: fn() -> !,
    stack: *mut u8,
    cpu_affinity: Option<usize>,
    is_kernel_thread: bool,  // Flag for signal security
}

fn spawn_kernel_thread(name: &'static str, entry: fn() -> !) -> usize {
    let thread = KernelThread {
        id: alloc_thread_id(),
        name,
        entry,
        stack: alloc_kernel_stack(),
        cpu_affinity: None,
        is_kernel_thread: true,
    };
    
    // Create task for scheduler
    let task = Task::new_kernel_thread(thread);
    add_to_scheduler(task);
    
    thread.id
}
```

### Flusher Thread

**Design Rationale:** 30-second interval balances data safety with I/O overhead, matching Linux pdflush behavior.

```rust
fn writeback_flusher_thread() -> ! {
    loop {
        // Sleep for 30 seconds
        sleep_ms(30_000);
        
        // Get dirty pages older than threshold
        let dirty_pages = get_old_dirty_pages(30_000);  // 30 seconds
        
        if dirty_pages.is_empty() {
            continue;
        }
        
        // Coalesce into batches
        let batches = coalesce_pages(dirty_pages);
        
        // Write each batch
        for batch in batches {
            write_batch(batch);
        }
    }
}
```

### Dirty Page Tracking

**Design Rationale:** Timestamp-based tracking enables age-based writeback policies for optimal I/O scheduling.

```rust
struct DirtyPageTracker {
    pages: HashMap<(InodeId, PageNum), DirtyPage>,
    lock: RwLock<()>,
}

struct DirtyPage {
    inode: InodeId,
    page_num: PageNum,
    dirty_time: u64,  // Tick when page became dirty
    data: *mut u8,
}

impl DirtyPageTracker {
    fn mark_dirty(&mut self, inode: InodeId, page_num: PageNum) {
        let key = (inode, page_num);
        self.pages.entry(key).or_insert(DirtyPage {
            inode,
            page_num,
            dirty_time: current_tick(),
            data: get_page_data(inode, page_num),
        });
    }
    
    fn get_old_pages(&self, age_ms: u64) -> Vec<DirtyPage> {
        let threshold = current_tick() - (age_ms * TICKS_PER_MS);
        self.pages.values()
            .filter(|p| p.dirty_time < threshold)
            .cloned()
            .collect()
    }
}
```

### Sync System Calls

**Design Rationale:** Multiple sync variants provide flexibility for different durability requirements.

```rust
fn sys_sync() -> Result<(), Error> {
    // Flush all dirty pages system-wide
    let all_dirty = DIRTY_PAGE_TRACKER.lock().get_all_pages();
    
    for page in all_dirty {
        write_page_to_storage(page)?;
    }
    
    // Wait for all I/O to complete
    wait_for_io_completion()?;
    
    Ok(())
}

fn sys_fsync(fd: usize) -> Result<(), Error> {
    let file = get_file(fd)?;
    let inode_id = file.inode_id();
    
    // Flush all pages for this file (data + metadata)
    let file_pages = DIRTY_PAGE_TRACKER.lock().get_pages_for_inode(inode_id);
    
    for page in file_pages {
        write_page_to_storage(page)?;
    }
    
    // Flush file metadata
    file.sync_metadata()?;
    
    // Wait for completion
    wait_for_file_io(inode_id)?;
    
    Ok(())
}

fn sys_fdatasync(fd: usize) -> Result<(), Error> {
    let file = get_file(fd)?;
    let inode_id = file.inode_id();
    
    // Flush only data pages (not metadata)
    let data_pages = DIRTY_PAGE_TRACKER.lock().get_data_pages_for_inode(inode_id);
    
    for page in data_pages {
        write_page_to_storage(page)?;
    }
    
    // Wait for completion
    wait_for_file_io(inode_id)?;
    
    Ok(())
}
```

---

## 6. Signal Security Enhancements (Phase 9D)

### UID System

**Design Rationale:** User ID-based permissions prevent privilege escalation and process interference across user boundaries.

```rust
struct User {
    uid: u32,
    gid: u32,
    euid: u32,  // Effective UID
    egid: u32,  // Effective GID
}

struct Process {
    pid: usize,
    user: User,
    is_kernel_thread: bool,
    // ... existing fields
}

impl Process {
    fn can_send_signal_to(&self, target: &Process, signal: Signal) -> bool {
        // Root can signal anyone
        if self.user.euid == 0 {
            return true;
        }
        
        // Cannot signal kernel threads
        if target.is_kernel_thread {
            return false;
        }
        
        // Must match effective UID
        self.user.euid == target.user.uid
    }
}
```

### Signal Permission Checks

**Design Rationale:** Centralized permission checking ensures consistent security policy enforcement.

```rust
fn sys_kill(target_pid: usize, signal: Signal) -> Result<(), Error> {
    let current = current_process();
    let target = get_process(target_pid)?;
    
    // Check permissions
    if !current.can_send_signal_to(&target, signal) {
        return Err(Error::PermissionDenied);
    }
    
    // Deliver signal
    deliver_signal(target, signal)?;
    
    Ok(())
}
```

### Signal Handler Validation

**Design Rationale:** Preventing execution of non-executable memory mitigates code injection attacks.

```rust
fn sys_sigaction(signal: Signal, handler: usize) -> Result<(), Error> {
    let current = current_process();
    
    // Validate handler address is in executable memory
    if handler != SIG_DFL && handler != SIG_IGN {
        let handler_addr = VirtAddr::new(handler as u64);
        
        // Check if address is mapped
        let pte = get_pte(handler_addr)?;
        if !pte.is_present() {
            return Err(Error::InvalidAddress);
        }
        
        // Check if page is executable
        if !pte.is_executable() {
            return Err(Error::PermissionDenied);
        }
        
        // Check if in user space
        if !pte.is_user() {
            return Err(Error::PermissionDenied);
        }
    }
    
    // Register handler
    current.signal_handlers[signal as usize] = handler;
    
    Ok(())
}
```

### UID System Calls

```rust
fn sys_getuid() -> u32 {
    current_process().user.uid
}

fn sys_geteuid() -> u32 {
    current_process().user.euid
}

fn sys_setuid(uid: u32) -> Result<(), Error> {
    let current = current_process();
    
    // Only root can change UID
    if current.user.euid != 0 {
        return Err(Error::PermissionDenied);
    }
    
    current.user.uid = uid;
    current.user.euid = uid;
    
    Ok(())
}

fn sys_seteuid(euid: u32) -> Result<(), Error> {
    let current = current_process();
    
    // Can set to real UID or if root
    if current.user.euid != 0 && euid != current.user.uid {
        return Err(Error::PermissionDenied);
    }
    
    current.user.euid = euid;
    
    Ok(())
}
```

---

## 7. TLB Shootdown (Phase 9A)

### IPI-Based Shootdown
```rust
fn tlb_shootdown(start: VirtAddr, end: VirtAddr) {
    let current_cpu = current_cpu_id();
    let target_cpus = get_other_cpus();
    
    // Prepare shootdown request
    let request = TlbShootdownRequest {
        start,
        end,
        ack_count: AtomicUsize::new(0),
    };
    
    // Send IPI to all other CPUs
    for cpu in target_cpus {
        send_tlb_shootdown_ipi(cpu, &request);
    }
    
    // Wait for acknowledgments
    while request.ack_count.load(Ordering::Acquire) < target_cpus.len() {
        cpu_relax();
    }
}

// IPI handler on receiving CPU
fn handle_tlb_shootdown_ipi(request: &TlbShootdownRequest) {
    // Flush TLB for requested range
    for addr in (request.start..request.end).step_by(PAGE_SIZE) {
        flush_tlb_page(addr);
    }
    
    // Acknowledge
    request.ack_count.fetch_add(1, Ordering::Release);
}
```

---

## Testing Strategy

### Unit Tests
- Test each component in isolation
- Mock dependencies
- Edge case coverage

### Integration Tests
- Test feature interactions
- Real-world scenarios
- Performance benchmarks

### Stress Tests
- High load
- SMP stress
- Memory pressure

---

## Performance Targets

| Feature | Current | Target | Improvement |
|---------|---------|--------|-------------|
| fork() time | 10ms | 1ms | 10x |
| Context switch | 5000 cycles | 1000 cycles | 5x |
| I/O throughput | 100 MB/s | 150 MB/s | 1.5x |
| Memory usage (after fork) | 100% | 20% | 5x |

---

## Risk Mitigation

### High-Risk Items
- Implement behind feature flags
- Extensive testing before merge
- Rollback plan ready

### Medium-Risk Items
- Incremental implementation
- Regular testing
- Code review required

### Low-Risk Items
- Standard development process
- Basic testing sufficient

---

## Migration Path

### Phase 9A: Foundation
- Can be enabled/disabled with feature flag
- Fallback to shared address space if issues

### Phase 9B: Optimizations
- COW optional initially
- Can fall back to copy if COW fails

### Phase 9C: Background Services
- Flusher thread optional
- Synchronous sync still works

### Phase 9D: Polish
- All features optional
- Graceful degradation

---

**Status:** Design Complete, Ready for Implementation
