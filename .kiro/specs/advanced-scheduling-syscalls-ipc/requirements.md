# Requirements Document

## Introduction

This document specifies the requirements for Phase 4 of MelloOS kernel development, which adds advanced scheduling capabilities, system call interface, and inter-process communication (IPC) mechanisms. This phase builds upon the existing round-robin task scheduler to provide priority-based scheduling, task sleep/wake functionality, a syscall interface for userland-kernel communication, and message-passing IPC between tasks.

### Architecture Overview

The Phase 4 components interact as follows:

- **Timer ISR Flow**: Timer interrupt → Scheduler tick update → Check sleeping tasks → Wake eligible tasks → Preemption decision
- **Syscall Flow**: Userland invokes syscall instruction → Kernel mode entry → Syscall dispatcher validates ID → Handler execution → Return to userland
- **IPC Flow**: Sender task → ipc_send syscall → Message enqueued to port → Receiver task blocked on ipc_recv → Message dequeued → Receiver wakes and continues

### System Call Table

| Syscall ID | Name | Arguments | Description | Return Type | Error Return |
|------------|------|-----------|-------------|-------------|--------------|
| 0 | SYS_WRITE | (fd, buf, len) | Write data to output device | isize | -1 on invalid fd/buffer |
| 1 | SYS_EXIT | (code) | Terminate current task | ! | Does not return |
| 2 | SYS_SLEEP | (ticks) | Sleep for given duration | isize | -1 on invalid ticks |
| 3 | SYS_IPC_SEND | (port_id, buf, len) | Send message to port | isize | -1 on invalid port/queue full |
| 4 | SYS_IPC_RECV | (port_id, buf, len) | Receive message from port (blocks) | isize | -1 on invalid port/buffer, or bytes received |

### Scheduler Algorithm Overview

The Phase 4 scheduler implements a priority-based preemptive scheduling algorithm:

- **Three Priority Levels**: High, Normal, Low - tasks are organized into separate ready queues per priority
- **Priority Selection**: Always select from the highest priority non-empty queue
- **Round-Robin within Priority**: Tasks at the same priority level are scheduled in round-robin fashion
- **Preemption**: Timer interrupts trigger scheduler decisions; higher priority tasks preempt lower priority ones
- **Sleep/Wake Management**: Sleeping tasks are removed from ready queues and tracked separately with wake-up times
- **Future Enhancement**: Priority aging or Multi-Level Feedback Queue (MLFQ) can be added in future phases to prevent starvation

## Glossary

- **Scheduler**: The kernel component responsible for selecting which task to execute next
- **Task**: An independent execution context with its own stack and registers
- **Priority**: A classification (High, Normal, Low) that determines task execution preference
- **System Call (Syscall)**: A mechanism for userland code to request kernel services
- **IPC**: Inter-Process Communication - mechanism for tasks to exchange messages
- **Port**: A communication endpoint for IPC message passing
- **Userland**: Code running in user mode with restricted privileges
- **Kernel Mode**: Privileged execution mode with full hardware access
- **Message Queue**: A FIFO buffer storing messages for a port
- **PID**: Process Identifier - unique numeric identifier for each task

## Requirements

### Requirement 1: Priority-Based Task Scheduling

**User Story:** As a kernel developer, I want tasks to have different priority levels, so that critical tasks can execute before less important ones.

#### Acceptance Criteria

1. THE Scheduler SHALL support three priority levels: High, Normal, and Low
2. WHEN selecting the next task to run, THE Scheduler SHALL choose the highest priority ready task
3. WHEN multiple tasks have the same priority, THE Scheduler SHALL use round-robin scheduling among them
4. THE Scheduler SHALL allow tasks to be created with a specified priority level
5. THE Scheduler SHALL maintain separate ready queues for each priority level

### Requirement 2: Task Sleep and Wake Functionality

**User Story:** As a task, I want to sleep for a specified duration, so that I can yield CPU time and wait for events without busy-waiting.

#### Acceptance Criteria

1. THE Scheduler SHALL provide a sleep function that accepts a duration in timer ticks
2. WHEN a task calls sleep, THE Scheduler SHALL mark the task as sleeping and remove it from ready queues
3. THE Scheduler SHALL store the wake-up time for each sleeping task
4. WHEN a timer interrupt occurs, THE Scheduler SHALL check all sleeping tasks and wake those whose sleep duration has elapsed
5. WHEN a sleeping task is woken, THE Scheduler SHALL add it back to the appropriate priority ready queue

### Requirement 3: System Call Interface

**User Story:** As a userland developer, I want to invoke kernel services through system calls, so that my code can perform privileged operations safely.

#### Acceptance Criteria

1. THE Kernel SHALL provide a syscall dispatch mechanism that handles syscall requests from userland
2. THE Kernel SHALL support at least five system calls: SYS_WRITE, SYS_EXIT, SYS_SLEEP, SYS_IPC_SEND, and SYS_IPC_RECV
3. WHEN a syscall is invoked, THE Kernel SHALL validate the syscall number and arguments
4. WHEN a syscall number is invalid, THE Kernel SHALL return -1 to userland
5. WHEN syscall arguments are invalid, THE Kernel SHALL return -1 to userland without executing the operation
6. THE Kernel SHALL execute the requested syscall handler and return the result to userland
7. THE Kernel SHALL preserve task state across syscall invocations

### Requirement 4: Inter-Process Communication via Message Passing

**User Story:** As a task developer, I want to send and receive messages to other tasks, so that tasks can coordinate and share data safely.

#### Acceptance Criteria

1. THE IPC System SHALL provide ports as communication endpoints identified by numeric IDs
2. THE IPC System SHALL support a maximum of 256 ports
3. THE IPC System SHALL limit each port's message queue to 16 messages maximum
4. THE IPC System SHALL provide an ipc_send function that accepts a port ID and message data
5. WHEN ipc_send is called with a valid port, THE IPC System SHALL enqueue the message to the target port's message queue
6. WHEN ipc_send is called and the target port's queue is full, THE IPC System SHALL return -1 without blocking
7. WHEN ipc_send is called with an invalid port ID, THE IPC System SHALL return -1
8. THE IPC System SHALL provide an ipc_recv function that accepts a port ID and buffer
9. WHEN ipc_recv is called and messages are available, THE IPC System SHALL dequeue the oldest message and copy it to the buffer
10. WHEN ipc_recv is called and no messages are available, THE IPC System SHALL block the calling task until a message arrives
11. WHEN ipc_recv is called with an invalid port ID or buffer, THE IPC System SHALL return -1
12. THE IPC System SHALL use spinlocks to protect message queue operations from race conditions

### Requirement 5: Userland Init Process

**User Story:** As a kernel developer, I want to launch an initial userland process after boot, so that the system can run user-mode applications.

#### Acceptance Criteria

1. THE Kernel SHALL create and launch an init task (PID 1) after completing boot initialization
2. THE Init Task SHALL execute in user mode with restricted privileges
3. THE Init Task SHALL be able to invoke system calls using the syscall interface
4. THE Init Task SHALL demonstrate IPC functionality by sending and receiving messages
5. THE Kernel SHALL log syscall invocations and IPC operations for debugging purposes

### Requirement 6: Verification and Testing

**User Story:** As a kernel developer, I want to verify that all Phase 4 features work correctly, so that I can ensure system stability and correctness.

#### Acceptance Criteria

1. THE Kernel SHALL demonstrate priority scheduling by spawning three tasks with High, Normal, and Low priorities and logging execution order
2. THE Kernel SHALL demonstrate sleep functionality by having a task sleep for a specified duration and logging wake time
3. THE Kernel SHALL demonstrate IPC by having two tasks exchange ping-pong messages and logging the message flow
4. THE Kernel SHALL log all syscall invocations with syscall ID and arguments for debugging
5. THE Kernel SHALL log scheduler decisions including task selection and priority queue states

## Resource Limits

- Maximum ports: 256
- Maximum messages per port queue: 16
- Maximum message size: 4096 bytes
- Maximum sleeping tasks: Limited by total task count

## Security and Isolation Notes

Phase 4 implements a simplified userland model without full memory isolation. Tasks run with kernel privileges but use the syscall interface for service requests. Full user/kernel mode separation with paging-based isolation is planned for Phase 5. The current design prepares the syscall and IPC infrastructure to support future isolation mechanisms.
