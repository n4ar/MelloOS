# Implementation Plan: Advanced Scheduling, System Calls, and IPC

This implementation plan breaks down Phase 4 into discrete, incremental coding tasks. Each task builds on previous tasks and references specific requirements from the requirements document.

## Task List

- [x] 1. Set up project structure and core interfaces
  - Create directory structure for new modules: `kernel/src/sys/` and `kernel/src/sched/priority.rs`
  - Define core enums and types: `TaskPriority`, `IpcError`, `KernelMetrics`
  - Add dependencies to `kernel/Cargo.toml` if needed
  - _Requirements: 1.1, 3.1, 4.1_

- [x] 2. Implement priority scheduler
  - [x] 2.1 Create `TaskPriority` enum and extend `Task` structure
    - Add `TaskPriority` enum with Low/Normal/High variants
    - Extend `Task` struct with `priority`, `wake_tick`, and `blocked_on_port` fields
    - Update `Task::new()` to accept priority parameter
    - _Requirements: 1.1, 1.4_
  
  - [x] 2.2 Implement `PriorityScheduler` with three ready queues
    - Create `PriorityScheduler` struct with three `TaskQueue` instances
    - Implement `enqueue_task()` to add tasks to appropriate priority queue
    - Implement `select_next()` to choose highest priority ready task
    - Add bitmap optimization for O(1) queue selection
    - _Requirements: 1.2, 1.3_
  
  - [x] 2.3 Implement sleep/wake mechanism
    - Add `sleeping_tasks` Vec to `PriorityScheduler`
    - Implement `sleep_task()` to remove task from ready queue and add to sleeping list
    - Implement `wake_sleeping_tasks()` to check wake times and move tasks back to ready queues
    - Integrate `wake_sleeping_tasks()` into scheduler tick
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_
  
  - [x] 2.4 Add preemption control
    - Add `preempt_disable_count` to `PriorityScheduler`
    - Implement `preempt_disable()` and `preempt_enable()` functions
    - Implement `can_preempt()` check
    - Update scheduler tick to respect preemption disable
    - _Requirements: 1.2_
  
  - [x] 2.5 Integrate priority scheduler into existing scheduler
    - Replace single `runqueue` in `SchedState` with `PriorityScheduler`
    - Update `schedule_next()` to use priority-based selection
    - Update `spawn_task()` to accept priority parameter
    - Modify existing demo tasks to use Normal priority
    - _Requirements: 1.1, 1.2, 1.3, 1.5_

- [x] 3. Implement system call interface
  - [x] 3.1 Create syscall module structure
    - Create `kernel/src/sys/mod.rs` and `kernel/src/sys/syscall.rs`
    - Define syscall number constants (SYS_WRITE, SYS_EXIT, SYS_SLEEP, SYS_IPC_SEND, SYS_IPC_RECV)
    - Create `KernelMetrics` struct with atomic counters
    - _Requirements: 3.1, 3.2_
  
  - [x] 3.2 Implement syscall entry point and dispatcher
    - Write `syscall_entry` naked function in assembly to save/restore registers
    - Implement `syscall_dispatcher` to route syscall ID to appropriate handler
    - Add syscall ID validation (return -1 for invalid IDs)
    - Increment metrics counters for each syscall
    - _Requirements: 3.1, 3.3, 3.4, 3.6_
  
  - [x] 3.3 Configure IDT for int 0x80
    - Extend `init_idt()` in `timer.rs` to register syscall handler at vector 0x80
    - Set IDT gate type to Interrupt Gate (0xE), DPL=3, Present=1
    - Verify syscall handler address is valid
    - _Requirements: 3.1_
  
  - [x] 3.4 Implement sys_write handler
    - Create `sys_write()` function to write data to serial output
    - Validate file descriptor (only fd=0 for stdout supported)
    - Validate buffer pointer and length (Phase 4: no validation, just use directly)
    - Return bytes written or -1 on error
    - _Requirements: 3.2, 3.4, 3.5_
  
  - [x] 3.5 Implement sys_exit handler
    - Create `sys_exit()` function to terminate current task
    - Mark task as terminated and remove from all queues
    - Trigger scheduler to select next task
    - Never return to caller
    - _Requirements: 3.2_
  
  - [x] 3.6 Implement sys_sleep handler
    - Create `sys_sleep()` function to put task to sleep
    - Validate tick count (return -1 if invalid)
    - Call `PriorityScheduler::sleep_task()` with current task ID and tick count
    - Trigger scheduler to select next task
    - Return 0 on success
    - _Requirements: 2.1, 2.2, 3.2, 3.4, 3.5_

- [x] 4. Implement IPC subsystem
  - [x] 4.1 Create IPC module structure
    - Create `kernel/src/sys/ipc.rs` and `kernel/src/sys/port.rs`
    - Define `Message`, `Port`, and `PortManager` structs
    - Define `IpcError` enum with all error variants
    - _Requirements: 4.1, 4.2_
  
  - [x] 4.2 Implement Port structure
    - Create `Port` struct with `id`, `queue` (VecDeque), `blocked_tasks` (VecDeque), and `lock` (Spinlock)
    - Implement port creation with ID validation
    - Add queue size limit check (max 16 messages)
    - _Requirements: 4.1, 4.2, 4.3_
  
  - [x] 4.3 Implement PortManager
    - Create `PortManager` with array of 256 optional ports
    - Implement `create_port()` to initialize ports at boot
    - Add `table_lock` for port creation/deletion
    - Create global `PORT_MANAGER` static with Mutex
    - _Requirements: 4.1, 4.2_
  
  - [x] 4.4 Implement message sending
    - Implement `send_message()` in `PortManager`
    - Validate port ID (return InvalidPort error)
    - Validate message size <= 4096 bytes (return MessageTooLarge error)
    - Check queue capacity (return QueueFull if full)
    - Acquire port lock with preempt_disable()
    - Enqueue message to port queue
    - Wake one blocked task (FIFO) if any
    - Release port lock with preempt_enable()
    - Increment `ipc_sends` metric
    - _Requirements: 4.4, 4.5, 4.6, 4.7, 4.12_
  
  - [x] 4.5 Implement message receiving
    - Implement `recv_message()` in `PortManager`
    - Validate port ID and buffer (return errors if invalid)
    - Acquire port lock with preempt_disable()
    - If message available: dequeue, copy to buffer, return bytes received
    - If no message: add task to blocked_tasks queue, mark task as Blocked, trigger scheduler
    - Release port lock with preempt_enable()
    - Increment `ipc_recvs` metric
    - _Requirements: 4.8, 4.9, 4.10, 4.11_
  
  - [x] 4.6 Implement sys_ipc_send and sys_ipc_recv handlers
    - Create `sys_ipc_send()` syscall handler that calls `PortManager::send_message()`
    - Create `sys_ipc_recv()` syscall handler that calls `PortManager::recv_message()`
    - Add error handling and return value conversion
    - Update syscall dispatcher to route IPC syscalls
    - _Requirements: 3.2, 4.4, 4.8_

- [x] 5. Create userland init process
  - [x] 5.1 Set up userspace project structure
    - Create `kernel/userspace/init/` directory
    - Create `Cargo.toml` with `no_std` configuration
    - Create `main.rs` with `#![no_std]` and `#![no_main]`
    - Add linker script for userspace binary
    - _Requirements: 5.1, 5.2_
  
  - [x] 5.2 Implement syscall wrappers in userspace
    - Write `syscall()` inline assembly function for int 0x80
    - Implement `sys_write()` wrapper
    - Implement `sys_sleep()` wrapper
    - Implement `sys_ipc_send()` wrapper
    - Implement `sys_ipc_recv()` wrapper
    - _Requirements: 5.3_
  
  - [x] 5.3 Write init process main function
    - Implement `_start()` entry point
    - Call `sys_write()` to print "Hello from userland! ✨"
    - Demonstrate IPC by sending "ping" to port 2
    - Demonstrate IPC by receiving from port 1
    - Call `sys_write()` to print received message
    - Call `sys_sleep()` to sleep for 100 ticks
    - Call `sys_write()` to print "Woke up!"
    - Enter infinite loop with periodic sleep
    - _Requirements: 5.4_
  
  - [x] 5.4 Build and embed init binary
    - Add build target for userspace init in Makefile
    - Compile init as separate binary
    - Embed init binary into kernel image (or load from memory)
    - _Requirements: 5.1_
  
  - [x] 5.5 Implement init process loader in kernel
    - Create function to load init binary into memory
    - Map init binary pages with appropriate permissions
    - Create init task with entry point at init binary start
    - Spawn init task with Normal priority
    - _Requirements: 5.1, 5.2_

- [x] 6. Add logging and metrics
  - [x] 6.1 Implement kernel metrics collection
    - Initialize `METRICS` global static
    - Add metric increments to scheduler (ctx_switches, preemptions)
    - Add metric increments to syscall dispatcher
    - Add metric increments to IPC operations
    - _Requirements: 6.4_
  
  - [x] 6.2 Add logging for scheduler operations
    - Log task priority changes
    - Log sleep/wake operations with tick counts
    - Log preemption disable/enable
    - Throttle logs to avoid spam (log every 100th operation)
    - _Requirements: 6.5_
  
  - [x] 6.3 Add logging for syscall operations
    - Log syscall invocations with task ID and syscall name
    - Log syscall arguments (at TRACE level)
    - Log syscall return values
    - Log syscall errors
    - _Requirements: 5.5_
  
  - [x] 6.4 Add logging for IPC operations
    - Log IPC send with port ID and message size
    - Log IPC receive with port ID and bytes received
    - Log task blocking on empty port
    - Log task waking on message arrival
    - Log queue full errors
    - _Requirements: 5.5_

- [x] 7. Integration and testing
  - [x] 7.1 Create priority scheduling test
    - Spawn three tasks with High, Normal, and Low priorities
    - Verify execution order (High → Normal → Low)
    - Log task execution to verify priority ordering
    - _Requirements: 1.2, 1.3, 6.1_
  
  - [x] 7.2 Create sleep/wake test
    - Spawn task that sleeps for 50 ticks
    - Log sleep and wake events
    - Verify task wakes after correct number of ticks
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 6.2_
  
  - [x] 7.3 Create syscall integration test
    - Spawn task that invokes sys_write
    - Verify output appears on serial
    - Spawn task that invokes sys_sleep
    - Verify task sleeps and wakes
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 6.3_
  
  - [x] 7.4 Create IPC integration test
    - Spawn two tasks: sender and receiver
    - Sender sends "ping" to port 1
    - Receiver receives from port 1
    - Verify message content matches
    - Test blocking when no messages available
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9, 4.10, 6.4_
  
  - [x] 7.5 Create IPC stress test
    - Spawn two tasks that ping-pong 1000 messages
    - Add random sleep/jitter between sends
    - Verify no message loss or corruption
    - Test queue-full condition by sending 17 messages
    - _Requirements: 4.6, 4.12_
  
  - [x] 7.6 Run end-to-end system test
    - Boot kernel with all Phase 4 features enabled
    - Launch init process
    - Verify init prints "Hello from userland! ✨"
    - Verify init sends and receives IPC messages
    - Verify init sleeps and wakes
    - Verify all operations logged correctly
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 8. Documentation and cleanup
  - [x] 8.1 Update kernel documentation
    - Update `docs/architecture.md` with Phase 4 components
    - Document syscall interface and ABI
    - Document IPC message passing protocol
    - Document priority scheduling algorithm
    - _Requirements: All_
  
  - [x] 8.2 Update CHANGELOG.md
    - Add Phase 4 entry with feature list
    - Document breaking changes (if any)
    - Add migration notes for existing code
    - _Requirements: All_
  
  - [x] 8.3 Add inline documentation
    - Add rustdoc comments to all public functions
    - Add module-level documentation
    - Add examples for syscall and IPC usage
    - _Requirements: All_

## Notes
- Each task should be completed and tested before moving to the next
- Integration tests (Task 7) should be run after completing each major component
- The implementation follows the lock hierarchy: PortManager::table_lock → Port::lock → Scheduler lock
- All IPC operations must call preempt_disable() before acquiring Port lock
- Metrics should be incremented atomically using AtomicUsize::fetch_add()
