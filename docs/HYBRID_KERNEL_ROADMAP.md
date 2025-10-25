# MelloOS Hybrid Kernel Roadmap

## Overview

This document outlines the roadmap for transforming MelloOS from its current monolithic architecture into a true **Hybrid Kernel** similar to macOS XNU (Mach + BSD).

**Status:** Planning Phase  
**Target:** Post Phase 8 (Filesystem completion)  
**Priority:** Medium (after core functionality is stable)

---

## Current Architecture (Monolithic)

```
┌─────────────────────────────────────────┐
│         User Applications               │
│    (mello-sh, mellobox, mello-term)    │
└─────────────────────────────────────────┘
              ↕ syscalls
┌─────────────────────────────────────────┐
│      Monolithic Kernel                  │
│  ┌─────────────────────────────────┐   │
│  │ Scheduler, MM, VFS, Drivers     │   │
│  │ IPC, Signals, Everything        │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

**Characteristics:**
- All kernel components in single address space
- Drivers compiled into kernel
- Direct function calls between subsystems
- High performance, lower isolation

---

## Target Architecture (Hybrid - XNU-like)

```
┌─────────────────────────────────────────┐
│         User Applications               │
│    (mello-sh, mellobox, mello-term)    │
└─────────────────────────────────────────┘
              ↕ POSIX syscalls
┌─────────────────────────────────────────┐
│         BSD Layer (Personality)         │
│  ┌─────────────────────────────────┐   │
│  │ VFS, Sockets, Process Mgmt      │   │
│  │ POSIX Syscalls, Signals         │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
              ↕ Internal API
┌─────────────────────────────────────────┐
│      Mach Microkernel Core              │
│  ┌─────────────────────────────────┐   │
│  │ IPC (Ports & Messages)          │   │
│  │ VM Objects & External Pagers    │   │
│  │ Task/Thread Management          │   │
│  │ Scheduler (Mach threads)        │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
       ↕                    ↕
┌──────────────┐    ┌──────────────┐
│   IOKit      │    │  KExt        │
│   Framework  │    │  Modules     │
│  (Drivers)   │    │  (Loadable)  │
└──────────────┘    └──────────────┘
```

**Characteristics:**
- Mach microkernel provides core primitives
- BSD layer provides POSIX compatibility
- IOKit for object-oriented driver framework
- KExt for loadable kernel extensions
- Message-based IPC between components

---

## Implementation Phases

### Phase 8: Filesystem & Storage ✅ (Current Priority)

**Complete this first before starting hybrid transition!**

- VFS layer
- MFS RAM filesystem
- MFS Disk filesystem
- Mount/umount operations
- File operations (read, write, seek, etc.)

**Why first?** Filesystem is fundamental infrastructure needed by all other components.

---

### Phase 8.5: Mach-like IPC Foundation

**Goal:** Replace current IPC with Mach-style ports and messages

**Components to implement:**

1. **Mach Ports** (`kernel/src/mach/port.rs`)
   - Port rights (send, receive, send-once)
   - Port sets
   - Port name space per task
   - Port death notifications

2. **Mach Messages** (`kernel/src/mach/message.rs`)
   - Message structure (header + body)
   - Out-of-line memory (OOL)
   - Message queues
   - Message send/receive primitives

3. **Mach IPC** (`kernel/src/mach/ipc.rs`)
   - `mach_msg()` system call
   - Message passing semantics
   - Port right transfer
   - Timeout handling

4. **Migration Path:**
   - Keep existing IPC for compatibility
   - Add Mach IPC alongside
   - Gradually migrate internal components
   - Deprecate old IPC

**Estimated effort:** 3-4 weeks

---

### Phase 8.6: Mach VM Objects

**Goal:** Implement Mach-style virtual memory management

**Components to implement:**

1. **VM Objects** (`kernel/src/mach/vm/object.rs`)
   - Memory object abstraction
   - Copy-on-write (COW) support
   - Shadow objects for fork()
   - Object caching

2. **External Pagers** (`kernel/src/mach/vm/pager.rs`)
   - Pager interface
   - Default pager (swap)
   - Vnode pager (file-backed)
   - Device pager

3. **VM Maps** (`kernel/src/mach/vm/map.rs`)
   - Virtual address space management
   - Map entries
   - Inheritance attributes
   - Protection attributes

4. **Integration:**
   - Refactor current MM to use VM objects
   - Implement page fault handling with pagers
   - Add mmap() support with VM objects

**Estimated effort:** 4-5 weeks

---

### Phase 8.7: Mach Tasks & Threads

**Goal:** Separate Mach tasks/threads from BSD processes

**Components to implement:**

1. **Mach Tasks** (`kernel/src/mach/task.rs`)
   - Task structure (address space container)
   - Task ports
   - Task suspend/resume
   - Task info queries

2. **Mach Threads** (`kernel/src/mach/thread.rs`)
   - Thread structure
   - Thread states (running, waiting, stopped)
   - Thread ports
   - Thread scheduling info

3. **Scheduler Integration** (`kernel/src/mach/sched.rs`)
   - Mach scheduling policies
   - Thread priorities
   - Timesharing vs fixed priority
   - Handoff scheduling

4. **BSD Process Layer** (`kernel/src/bsd/proc.rs`)
   - BSD process wraps Mach task
   - POSIX semantics on top of Mach
   - Process groups, sessions
   - Signal delivery via Mach exceptions

**Estimated effort:** 3-4 weeks

---

### Phase 9: IOKit Framework

**Goal:** Object-oriented driver framework

**Components to implement:**

1. **IOKit Core** (`kernel/src/iokit/`)
   - IOService base class
   - IORegistry (device tree)
   - IOCatalogue (driver matching)
   - IOWorkLoop (event handling)

2. **Driver Matching** (`kernel/src/iokit/matching.rs`)
   - Property matching
   - Probe scoring
   - Driver loading
   - Device attachment

3. **Power Management** (`kernel/src/iokit/power.rs`)
   - Power states
   - Power domains
   - Sleep/wake
   - Power assertions

4. **Refactor Existing Drivers:**
   - Convert to IOService subclasses
   - Use IOKit APIs
   - Implement power management
   - Add to IORegistry

**Estimated effort:** 5-6 weeks

---

### Phase 9.5: Kernel Extensions (KExt)

**Goal:** Loadable kernel modules

**Components to implement:**

1. **KExt Loader** (`kernel/src/kext/loader.rs`)
   - ELF module loading
   - Symbol resolution
   - Dependency management
   - Version checking

2. **KExt Manager** (`kernel/src/kext/manager.rs`)
   - KExt lifecycle (load, start, stop, unload)
   - KExt database
   - KExt info queries
   - Security checks

3. **KExt API** (`kernel/src/kext/api.rs`)
   - Exported kernel symbols
   - KExt entry points
   - Resource management
   - Logging/debugging

4. **Build System:**
   - KExt bundle format
   - Info.plist equivalent
   - Signing/verification
   - Installation tools

**Estimated effort:** 4-5 weeks

---

### Phase 10: BSD Layer Separation

**Goal:** Clean separation of BSD personality from Mach core

**Components to refactor:**

1. **BSD Syscalls** (`kernel/src/bsd/syscalls/`)
   - Move all POSIX syscalls to BSD layer
   - Implement via Mach primitives
   - Clear API boundary

2. **BSD VFS** (`kernel/src/bsd/vfs/`)
   - Keep VFS in BSD layer
   - Use Mach VM for file caching
   - Vnode pager integration

3. **BSD Networking** (`kernel/src/bsd/net/`)
   - BSD sockets
   - Network stack
   - Socket buffers

4. **BSD Process Model** (`kernel/src/bsd/proc/`)
   - Process groups
   - Sessions
   - Signals
   - TTY control

**Estimated effort:** 3-4 weeks

---

### Phase 10.5: Multiple Personalities (Optional)

**Goal:** Support for non-POSIX personalities

**Potential personalities:**

1. **Mach Personality**
   - Direct Mach API access
   - No BSD overhead
   - For system services

2. **Linux Personality** (Future)
   - Linux syscall compatibility
   - Run Linux binaries
   - Separate from BSD layer

3. **Custom Personality**
   - MelloOS-specific APIs
   - Optimized for performance
   - Modern design

**Estimated effort:** Variable (2-8 weeks per personality)

---

## Benefits of Hybrid Architecture

### 1. **Modularity**
- Clear separation of concerns
- Easier to maintain and debug
- Independent component evolution

### 2. **Flexibility**
- Multiple personalities support
- Loadable drivers (KExt)
- Runtime extensibility

### 3. **Stability**
- Driver isolation
- Fault containment
- Graceful degradation

### 4. **Performance**
- Mach core is highly optimized
- Message passing can be fast
- Zero-copy where possible

### 5. **Compatibility**
- POSIX via BSD layer
- Mach API for advanced features
- Future: Linux compatibility

---

## Trade-offs and Challenges

### Challenges:

1. **Complexity**
   - More layers to understand
   - More code to maintain
   - Steeper learning curve

2. **Performance Overhead**
   - Message passing vs direct calls
   - Layer transitions
   - Need careful optimization

3. **Migration Effort**
   - Large refactoring required
   - Risk of breaking existing code
   - Need comprehensive testing

4. **Debugging**
   - More complex call stacks
   - Inter-layer issues
   - Requires better tooling

### Mitigation:

- Incremental migration (keep old code working)
- Comprehensive test suite
- Performance benchmarking at each phase
- Good documentation
- Rollback capability

---

## Success Criteria

### Phase 8.5 (Mach IPC):
- [ ] All internal IPC uses Mach ports
- [ ] Message passing works reliably
- [ ] Performance within 10% of direct calls
- [ ] Port rights work correctly

### Phase 8.6 (Mach VM):
- [ ] All memory management uses VM objects
- [ ] External pagers work
- [ ] COW fork() works
- [ ] mmap() fully functional

### Phase 8.7 (Mach Tasks):
- [ ] BSD processes wrap Mach tasks
- [ ] Thread management works
- [ ] Scheduler uses Mach threads
- [ ] Signals via Mach exceptions

### Phase 9 (IOKit):
- [ ] All drivers use IOKit
- [ ] Device matching works
- [ ] Power management functional
- [ ] IORegistry complete

### Phase 9.5 (KExt):
- [ ] Drivers loadable at runtime
- [ ] KExt dependencies work
- [ ] Unloading is safe
- [ ] Security checks pass

### Phase 10 (BSD Layer):
- [ ] Clear API boundary
- [ ] All POSIX syscalls work
- [ ] VFS integrated with Mach VM
- [ ] Networking functional

---

## Timeline Estimate

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| 8.5 - Mach IPC | 3-4 weeks | Phase 8 complete |
| 8.6 - Mach VM | 4-5 weeks | Phase 8.5 |
| 8.7 - Mach Tasks | 3-4 weeks | Phase 8.6 |
| 9 - IOKit | 5-6 weeks | Phase 8.7 |
| 9.5 - KExt | 4-5 weeks | Phase 9 |
| 10 - BSD Layer | 3-4 weeks | Phase 9.5 |

**Total estimated time:** 22-28 weeks (5.5-7 months)

**Note:** This is aggressive. Real-world development with testing, debugging, and iteration will likely take 8-12 months.

---

## References

### XNU (macOS Kernel):
- https://github.com/apple/darwin-xnu
- XNU source code (Mach + BSD)
- IOKit framework

### Mach Documentation:
- "The Mach System" by Avadis Tevanian
- CMU Mach papers
- OSF/1 documentation

### Similar Projects:
- **Redox OS** - Microkernel in Rust
- **seL4** - Verified microkernel
- **Fuchsia** - Zircon microkernel

---

## Decision Points

### Before Starting Phase 8.5:

**Questions to answer:**
1. Is Phase 8 (Filesystem) complete and stable?
2. Do we have comprehensive tests for current functionality?
3. Are we ready for a major refactoring?
4. Do we have time/resources for 6+ months of work?

**If YES to all:** Proceed with hybrid transition  
**If NO to any:** Continue with current architecture, add features

### Alternative: Incremental Hybrid

Instead of full XNU-style hybrid, consider:
- Keep monolithic core
- Add KExt support only
- Add better IPC
- Improve modularity without full Mach

**Pros:** Less work, lower risk  
**Cons:** Not true hybrid, limited benefits

---

## Conclusion

Transforming MelloOS into a hybrid kernel is a significant undertaking that will take 6-12 months of focused development. The benefits include better modularity, flexibility, and long-term maintainability.

**Recommendation:** Complete Phase 8 (Filesystem) first, then reassess based on project goals and available resources.

**Next Steps:**
1. ✅ Complete Phase 8 (Filesystem)
2. Review this roadmap
3. Decide: Full hybrid vs incremental improvements
4. If proceeding: Start with Phase 8.5 (Mach IPC)

---

**Document Version:** 1.0  
**Last Updated:** 2025-01-XX  
**Status:** Planning / Not Started
