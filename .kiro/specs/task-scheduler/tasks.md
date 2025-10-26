# Implementation Plan

- [x] 1. Set up project structure and core interfaces
  - Create `kernel/src/sched/` directory structure
  - Define module declarations in `kernel/src/main.rs`
  - Create empty module files: `mod.rs`, `task.rs`, `context.rs`, `timer.rs`
  - _Requirements: 1.1, 1.2, 1.3_

- [x] 2. Implement Task structure and management
- [x] 2.1 Define Task Control Block and TaskState enum
  - Write `Task` struct with id, name, stack, state, context fields
  - Write `TaskState` enum with Ready, Running, Sleeping variants
  - Write `TaskId` type alias
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

- [x] 2.2 Implement Task::new() with stack allocation
  - Allocate 8KB stack using `kmalloc()`
  - Calculate stack top address
  - Prepare initial stack frame with entry_trampoline and callee-saved registers
  - Initialize CpuContext with prepared RSP
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [x] 2.3 Implement entry_trampoline function
  - Write entry_trampoline that extracts entry_point from R12
  - Call entry_point function
  - Add panic handler if entry_point returns
  - _Requirements: 1.1_

- [x] 2.4 Write unit tests for Task creation
  - Test Task::new() allocates stack correctly
  - Test initial context setup
  - Test stack frame layout
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [x] 3. Implement CPU context and context switching
- [x] 3.1 Define CpuContext structure
  - Write `CpuContext` struct with callee-saved registers (r15-r12, rbp, rbx, rsp)
  - Mark struct as `#[repr(C)]` for stable layout
  - _Requirements: 2.1, 2.2, 2.3_

- [x] 3.2 Implement context_switch in assembly
  - Write inline assembly for context_switch function
  - Save current task's callee-saved registers to stack
  - Save current RSP to current.rsp
  - Load next RSP from next.rsp
  - Restore next task's callee-saved registers from stack
  - Return to next task (ret instruction)
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [x] 3.3 Write unit tests for context switching
  - Test register save/restore
  - Test RSP switching
  - Test return address handling
  - _Requirements: 2.1, 2.2, 2.3_

- [x] 4. Implement scheduler core with Round-Robin algorithm
- [x] 4.1 Define global scheduler state
  - Create `SchedState` struct with runqueue, current, next_tid fields
  - Create `SCHED` static with Mutex<SchedState>
  - Create `TASK_TABLE` static with Mutex<Vec<Option<Box<Task>>>>
  - _Requirements: 3.1, 3.2_

- [x] 4.2 Implement spawn_task function
  - Generate unique TaskId
  - Create new Task with Task::new()
  - Box the Task and add to TASK_TABLE
  - Add TaskId to runqueue
  - Log task spawn with [SCHED] prefix
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 4.3 Implement schedule_next function
  - Lock SCHED state
  - Move current TaskId to back of runqueue if exists
  - Pop front TaskId from runqueue
  - Update current TaskId
  - Unlock SCHED state
  - Return Task references for context switch
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 4.4 Implement tick function with context switching
  - Call schedule_next() to get old and new tasks
  - Log context switch with task IDs and names
  - Call context_switch(&mut old_ctx, &new_ctx)
  - Implement switch counter and log throttling
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 4.5 Implement init_scheduler function
  - Initialize SCHED and TASK_TABLE
  - Create idle task (task id 0)
  - Log scheduler initialization
  - _Requirements: 3.1, 3.5_

- [x] 4.6 Write unit tests for scheduler
  - Test spawn_task adds to runqueue
  - Test Round-Robin task selection
  - Test multiple task switching
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 5. Implement timer interrupt handling
- [x] 5.1 Set up Interrupt Descriptor Table (IDT)
  - Create IDT using x86_64 crate
  - Register timer interrupt handler at vector 32
  - Load IDT
  - _Requirements: 4.1, 4.2_

- [x] 5.2 Implement PIC remapping
  - Remap master PIC to vectors 32-39
  - Remap slave PIC to vectors 40-47
  - Mask all IRQs except timer (IRQ0)
  - _Requirements: 4.1, 4.2_

- [x] 5.3 Implement PIT timer configuration
  - Calculate divisor for desired frequency (100 Hz)
  - Configure PIT mode 3 (square wave)
  - Write divisor to PIT channel 0
  - _Requirements: 4.1, 4.4_

- [x] 5.4 Implement timer interrupt handler
  - Send EOI to PIC master
  - Call sched::tick()
  - Add interrupt handler attribute
  - _Requirements: 4.1, 4.2, 4.3, 4.5_

- [x] 5.5 Implement send_eoi function
  - Write 0x20 to PIC master command port
  - Add comment about slave PIC EOI for future
  - _Requirements: 4.5_

- [x] 5.6 Write integration tests for timer
  - Test timer interrupt fires
  - Test interrupt handler is called
  - Test tick counter increments
  - _Requirements: 4.1, 4.2, 4.3_

- [x] 6. Integrate scheduler with kernel main
- [x] 6.1 Add sched module to main.rs
  - Add `mod sched;` declaration
  - Import necessary types and functions
  - _Requirements: 6.1_

- [x] 6.2 Create demonstration tasks
  - Write task_a function that prints "A" in loop
  - Write task_b function that prints "B" in loop
  - Add busy-wait delays to make output visible
  - _Requirements: 6.1, 6.2_

- [x] 6.3 Initialize scheduler in kernel boot
  - Call sched::init_scheduler() after memory management
  - Spawn task_a and task_b
  - Enable interrupts with `sti` instruction
  - Log scheduler initialization complete
  - _Requirements: 6.1, 6.2, 6.3_

- [x] 6.4 Update kernel idle loop
  - Replace simple hlt loop with scheduler-aware idle
  - Ensure interrupts are enabled
  - _Requirements: 6.1, 6.5_

- [x] 6.5 Write end-to-end integration test
  - Test two tasks switching successfully
  - Verify alternating output (A B A B...)
  - Test system stability for 100+ context switches
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 7. Add comprehensive logging and error handling
- [x] 7.1 Implement scheduler logging macros
  - Create consistent [SCHED] prefix for all logs
  - Implement log throttling for context switches
  - Add log levels (INFO, WARNING, ERROR)
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 7.2 Add error handling for out of memory
  - Handle kmalloc failure in Task::new()
  - Return error from spawn_task if allocation fails
  - Log error with [SCHED] ERROR prefix
  - _Requirements: 1.1, 1.2_

- [x] 7.3 Add error handling for empty runqueue
  - Check for empty runqueue in schedule_next()
  - Fall back to idle task if no tasks available
  - Log warning with [SCHED] WARNING prefix
  - _Requirements: 3.5_

- [x] 7.4 Add panic handlers for critical errors
  - Panic if context switch fails
  - Panic if IDT setup fails
  - Add descriptive panic messages
  - _Requirements: 2.1, 2.2, 2.3_

- [x] 8. Testing and validation
- [x] 8.1 Build and run kernel with QEMU
  - Build kernel with `make build`
  - Run with `make run` or `tools/qemu.sh`
  - Verify kernel boots successfully
  - _Requirements: 6.1, 6.2, 6.3_

- [x] 8.2 Verify task switching output
  - Check serial output for [SCHED] messages
  - Verify alternating A and B output
  - Verify context switch logs show correct task IDs
  - _Requirements: 6.2, 6.3, 5.1, 5.2, 5.3_

- [x] 8.3 Stress test with multiple tasks
  - Spawn 5 tasks with different output
  - Run for 1000+ context switches
  - Verify no crashes or hangs
  - _Requirements: 6.4_

- [x] 8.4 Test different timer frequencies
  - Test with 10 Hz, 100 Hz, 1000 Hz
  - Verify context switches match frequency
  - Verify system stability at all frequencies
  - _Requirements: 4.4_

- [x] 8.5 Verify memory management integration
  - Check that task stacks are allocated correctly
  - Verify no memory leaks after task destruction
  - Test with memory management logging enabled
  - _Requirements: 1.2_

- [x] 9. Documentation and cleanup
- [x] 9.1 Add inline documentation to all functions
  - Document all public functions with /// comments
  - Explain assembly code with inline comments
  - Document safety requirements for unsafe code
  - _Requirements: All_

- [x] 9.2 Update README.md with scheduler information
  - Add Phase 3: Task Scheduler section
  - Explain Round-Robin scheduling
  - Describe context switching mechanism
  - Document timer interrupt configuration
  - _Requirements: All_

- [x] 9.3 Update CHANGELOG.md
  - Add entry for Phase 3 completion
  - List all new features
  - Document any breaking changes
  - _Requirements: All_

- [x] 9.4 Create scheduler documentation
  - Write detailed scheduler architecture document
  - Add diagrams for context switch flow
  - Document performance characteristics
  - Add troubleshooting guide
  - _Requirements: All_
