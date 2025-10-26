# Requirements Document

## Introduction

This document specifies the requirements for implementing Symmetric Multi-Processing (SMP) support in MelloOS kernel. The SMP feature enables the kernel to utilize multiple CPU cores simultaneously, distributing tasks across cores for improved performance and responsiveness. This phase builds upon the existing scheduler, syscall, and IPC infrastructure from Phase 4.

## Glossary

- **BSP (Bootstrap Processor)**: The primary CPU core that boots first and initializes the system
- **AP (Application Processor)**: Secondary CPU cores that are brought online after BSP initialization
- **LAPIC (Local APIC)**: Local Advanced Programmable Interrupt Controller, one per CPU core
- **IOAPIC**: I/O Advanced Programmable Interrupt Controller for routing external interrupts
- **MADT (Multiple APIC Description Table)**: ACPI table containing information about system processors and interrupt controllers
- **SIPI (Startup Inter-Processor Interrupt)**: Special interrupt used to wake up Application Processors
- **IPI (Inter-Processor Interrupt)**: Interrupt sent from one CPU core to another
- **xAPIC**: Extended APIC mode using memory-mapped I/O
- **x2APIC**: Extended xAPIC mode using MSR-based I/O
- **ICR (Interrupt Command Register)**: APIC register used to send IPIs
- **Per-CPU Data**: Data structures unique to each CPU core
- **Runqueue**: Queue of tasks ready to execute on a specific CPU core
- **TLB (Translation Lookaside Buffer)**: CPU cache for virtual-to-physical address translations
- **Spinlock**: Synchronization primitive for protecting shared data in multi-processor systems
- **MelloOS Kernel**: The operating system kernel being developed
- **Task**: An executable unit of work scheduled by the kernel

## Requirements

### Requirement 1

**User Story:** As a kernel developer, I want the system to detect all available CPU cores during boot, so that the kernel can utilize all processing resources.

#### Acceptance Criteria

1. WHEN the MelloOS Kernel boots, THE MelloOS Kernel SHALL parse the ACPI MADT table to identify all CPU cores
2. THE MelloOS Kernel SHALL extract APIC ID and enabled status for each detected CPU core
3. THE MelloOS Kernel SHALL create a list of CpuInfo structures containing APIC ID and enabled flag for each core
4. THE MelloOS Kernel SHALL log the total number of detected CPUs and their APIC IDs with format "[SMP] CPUs detected: N (apic_ids=[...])"
5. WHERE at least 2 CPU cores are detected, THE MelloOS Kernel SHALL proceed with multi-core initialization

### Requirement 2

**User Story:** As a kernel developer, I want the BSP to initialize the Local APIC subsystem, so that interrupt handling and inter-processor communication can function correctly.

#### Acceptance Criteria

1. THE MelloOS Kernel SHALL initialize the Local APIC on the BSP during early boot
2. THE MelloOS Kernel SHALL configure the spurious interrupt vector register with a valid vector number
3. THE MelloOS Kernel SHALL enable xAPIC mode using memory-mapped I/O registers
4. THE MelloOS Kernel SHALL verify LAPIC functionality by reading the LAPIC ID register
5. THE MelloOS Kernel SHALL log BSP online status with format "[SMP] BSP online (apic_id=N)"

### Requirement 3

**User Story:** As a kernel developer, I want the BSP to bring Application Processors online, so that all CPU cores can execute tasks.

#### Acceptance Criteria

1. THE MelloOS Kernel SHALL prepare an AP trampoline code section in identity-mapped memory at address range 0x8000 to 0x9000
2. THE MelloOS Kernel SHALL store a pointer to the trampoline address in BSP-accessible memory for SIPI delivery
3. WHEN bringing up each AP, THE MelloOS Kernel SHALL send an INIT IPI via the ICR register
4. THE MelloOS Kernel SHALL wait 10 milliseconds after sending INIT IPI
5. THE MelloOS Kernel SHALL send two SIPI IPIs with the trampoline address, waiting 200 microseconds between each
6. WHEN an AP starts execution, THE MelloOS Kernel SHALL transition the AP from real mode to long mode
7. THE MelloOS Kernel SHALL configure GDT, IDT, and CR3 registers for each AP
8. WHEN an AP completes initialization, THE MelloOS Kernel SHALL log with format "[SMP] AP#N online"

### Requirement 4

**User Story:** As a kernel developer, I want each CPU core to maintain its own data structures, so that cores can operate independently without contention.

#### Acceptance Criteria

1. THE MelloOS Kernel SHALL define a PerCpu structure containing core ID, runqueue, current task pointer, and LAPIC timer frequency
2. THE MelloOS Kernel SHALL allocate one PerCpu structure instance for each detected CPU core
3. THE MelloOS Kernel SHALL configure the GS base MSR using wrmsr(MSR_GS_BASE, &percpu[core_id]) for the BSP during early initialization
4. WHEN an AP initializes, THE MelloOS Kernel SHALL configure the GS base MSR in the ap_init() function for that AP
5. THE MelloOS Kernel SHALL provide a percpu_current() function that returns a reference to the current core's PerCpu structure
6. THE MelloOS Kernel SHALL ensure each core can access its PerCpu data without requiring locks

### Requirement 5

**User Story:** As a kernel developer, I want each CPU core to have its own task runqueue, so that task scheduling can occur independently per core.

#### Acceptance Criteria

1. THE MelloOS Kernel SHALL maintain a separate runqueue within each PerCpu structure
2. WHEN a new task is created, THE MelloOS Kernel SHALL assign the task to the CPU core with the smallest runqueue size
3. THE MelloOS Kernel SHALL protect runqueue modifications with spinlocks to prevent race conditions
4. THE MelloOS Kernel SHALL perform periodic load rebalancing by sending rebalance IPIs every 100 milliseconds
5. THE MelloOS Kernel SHALL allow tasks to be migrated between core runqueues when load balancing is required
6. WHEN scheduling occurs on a core, THE MelloOS Kernel SHALL select tasks only from that core's runqueue

### Requirement 6

**User Story:** As a kernel developer, I want each CPU core to have its own timer interrupt, so that time-slicing and scheduling can occur independently per core.

#### Acceptance Criteria

1. THE MelloOS Kernel SHALL configure the APIC timer on each CPU core during core initialization
2. THE MelloOS Kernel SHALL calibrate the APIC timer frequency using the PIT or TSC as a reference clock source
3. THE MelloOS Kernel SHALL set the APIC timer to generate periodic interrupts at the calibrated frequency
4. WHEN an APIC timer interrupt fires on a core, THE MelloOS Kernel SHALL invoke the scheduler tick function for that core
5. THE MelloOS Kernel SHALL update time slice counters and wake sleeping tasks during each timer tick
6. THE MelloOS Kernel SHALL log timer initialization with format "[APIC] coreN timer @XHz"

### Requirement 7

**User Story:** As a kernel developer, I want to send Inter-Processor Interrupts between cores, so that cores can coordinate operations like rescheduling.

#### Acceptance Criteria

1. THE MelloOS Kernel SHALL provide a send_ipi(apic_id, vector) function to send an IPI to a specific core
2. THE MelloOS Kernel SHALL provide a broadcast_ipi(vector, exclude_self) function to send IPIs to all cores
3. THE MelloOS Kernel SHALL define a RESCHEDULE_IPI vector for triggering scheduler preemption on remote cores
4. WHEN a RESCHEDULE_IPI is received, THE MelloOS Kernel SHALL invoke the scheduler on the receiving core
5. THE MelloOS Kernel SHALL log IPI transmission with format "[SCHED] send RESCHED IPI → coreN"

### Requirement 8

**User Story:** As a kernel developer, I want proper synchronization primitives, so that shared data structures remain consistent across multiple cores.

#### Acceptance Criteria

1. THE MelloOS Kernel SHALL implement a SpinLock type that uses atomic compare-and-swap operations
2. THE MelloOS Kernel SHALL use pause-loop instructions with exponential backoff to reduce bus contention during spinlock acquisition
3. THE MelloOS Kernel SHALL provide irqsave variants of spinlocks that disable interrupts while holding the lock
4. THE MelloOS Kernel SHALL use appropriate memory ordering (Acquire/Release) for atomic operations
5. THE MelloOS Kernel SHALL protect all shared data structures (runqueues, IPC queues) with spinlocks
6. THE MelloOS Kernel SHALL ensure spinlock implementations prevent deadlocks through proper lock ordering

### Requirement 9

**User Story:** As a kernel developer, I want to verify that tasks execute on multiple cores, so that I can confirm SMP functionality is working correctly.

#### Acceptance Criteria

1. THE MelloOS Kernel SHALL create 3 to 4 test tasks with different priorities during initialization
2. THE MelloOS Kernel SHALL distribute test tasks across available CPU cores
3. WHEN a task executes, THE MelloOS Kernel SHALL log with format "[SCHED][coreN] run TASKNAME"
4. THE MelloOS Kernel SHALL demonstrate tasks executing on at least 2 different cores
5. THE MelloOS Kernel SHALL maintain compatibility with Phase 4 syscall and IPC functionality during multi-core operation

### Requirement 10

**User Story:** As a kernel developer, I want the system to remain stable under multi-core operation, so that the kernel does not crash or deadlock.

#### Acceptance Criteria

1. THE MelloOS Kernel SHALL complete boot sequence without panics when multiple cores are active
2. THE MelloOS Kernel SHALL handle concurrent syscalls from multiple cores without data corruption
3. THE MelloOS Kernel SHALL prevent deadlocks through proper lock ordering and timeout mechanisms
4. THE MelloOS Kernel SHALL handle IPC operations correctly when sender and receiver are on different cores
5. WHEN running on QEMU with parameters "-smp 4 -enable-kvm", THE MelloOS Kernel SHALL execute for at least 30 seconds without errors
6. THE MelloOS Kernel SHALL be testable on QEMU with 2 to 4 cores using the command line parameter "-smp N" where N is between 2 and 4
