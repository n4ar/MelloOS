# Requirements Document

## Introduction

ระบบ Task Scheduler สำหรับ MelloOS Kernel เป็นระบบ Multitasking เบื้องต้นที่ช่วยให้ Kernel สามารถจัดการและสลับการทำงานระหว่าง Task หลายๆ ตัวได้อย่างมีประสิทธิภาพ โดยใช้วิธี Round-Robin Scheduling ร่วมกับ Timer Interrupt เพื่อสร้างพื้นฐานสำหรับระบบ Multitasking ที่สมบูรณ์ในอนาคต

## Glossary

- **Scheduler**: ระบบที่จัดการการสลับการทำงานระหว่าง Task ต่างๆ
- **Task**: หน่วยการทำงานที่สามารถถูก schedule ได้ มีข้อมูล context และ state ของตัวเอง
- **Context Switch**: กระบวนการบันทึก CPU register ของ Task ปัจจุบันและโหลด register ของ Task ถัดไป
- **Round-Robin**: วิธีการ scheduling ที่ให้แต่ละ Task ทำงานตามลำดับวนรอบ
- **Timer Interrupt**: สัญญาณขัดจังหวะจาก hardware timer ที่เกิดขึ้นเป็นระยะเพื่อให้ Scheduler สลับ Task
- **Runqueue**: คิวที่เก็บ Task ทั้งหมดที่พร้อมทำงาน
- **Task Control Block (TCB)**: โครงสร้างข้อมูลที่เก็บข้อมูลทั้งหมดของ Task
- **CPU Context**: ชุดของ register ทั้งหมดที่ต้องบันทึกและกลับคืนเมื่อสลับ Task
- **APIC**: Advanced Programmable Interrupt Controller สำหรับจัดการ interrupt
- **PIT**: Programmable Interval Timer สำหรับสร้าง timer interrupt
- **IDT**: Interrupt Descriptor Table ตารางที่เก็บ interrupt handler

## Requirements

### Requirement 1

**User Story:** ในฐานะ Kernel Developer ฉันต้องการให้ระบบสามารถสร้าง Task ใหม่ได้ เพื่อให้สามารถรัน code หลายๆ ส่วนพร้อมกันได้

#### Acceptance Criteria

1. THE Scheduler SHALL provide a function to spawn new Task with a function pointer
2. WHEN a new Task is spawned, THE Scheduler SHALL allocate a dedicated stack for the Task
3. WHEN a new Task is spawned, THE Scheduler SHALL assign a unique identifier to the Task
4. WHEN a new Task is spawned, THE Scheduler SHALL initialize the Task state to Ready
5. WHEN a new Task is spawned, THE Scheduler SHALL add the Task to the Runqueue

### Requirement 2

**User Story:** ในฐานะ Kernel Developer ฉันต้องการให้ระบบสามารถสลับ Context ระหว่าง Task ได้ เพื่อให้หลายๆ Task สามารถทำงานสลับกันได้

#### Acceptance Criteria

1. THE Scheduler SHALL implement a Context Switch mechanism that saves all CPU registers
2. WHEN performing Context Switch, THE Scheduler SHALL save the current Task CPU Context to its Task Control Block
3. WHEN performing Context Switch, THE Scheduler SHALL restore the next Task CPU Context from its Task Control Block
4. WHEN performing Context Switch, THE Scheduler SHALL update the current Task state from Running to Ready
5. WHEN performing Context Switch, THE Scheduler SHALL update the next Task state from Ready to Running

### Requirement 3

**User Story:** ในฐานะ Kernel Developer ฉันต้องการให้ระบบใช้ Round-Robin Scheduling เพื่อให้แต่ละ Task ได้รับเวลา CPU อย่างเท่าเทียมกัน

#### Acceptance Criteria

1. THE Scheduler SHALL maintain a Runqueue containing all Ready and Running Tasks
2. WHEN selecting the next Task, THE Scheduler SHALL choose the Task at the front of the Runqueue
3. WHEN a Task completes its time slice, THE Scheduler SHALL move the Task to the back of the Runqueue
4. THE Scheduler SHALL rotate through all Tasks in the Runqueue in sequential order
5. WHEN the Runqueue contains at least one Task, THE Scheduler SHALL ensure continuous Task execution

### Requirement 4

**User Story:** ในฐานะ Kernel Developer ฉันต้องการให้ระบบมี Timer Interrupt เพื่อให้ Scheduler สามารถสลับ Task ได้อย่างอัตโนมัติ

#### Acceptance Criteria

1. THE Scheduler SHALL configure a hardware timer to generate periodic interrupts
2. THE Scheduler SHALL register a Timer Interrupt handler in the Interrupt Descriptor Table
3. WHEN a Timer Interrupt occurs, THE Scheduler SHALL invoke the scheduler tick function
4. THE Scheduler SHALL configure the timer frequency to generate interrupts at a rate between 10 Hz and 1000 Hz
5. WHEN the Timer Interrupt handler completes, THE Scheduler SHALL acknowledge the interrupt to the hardware

### Requirement 5

**User Story:** ในฐานะ Kernel Developer ฉันต้องการให้ระบบแสดง log เมื่อสลับ Task เพื่อให้สามารถ debug และตรวจสอบการทำงานของ Scheduler ได้

#### Acceptance Criteria

1. WHEN a Context Switch occurs, THE Scheduler SHALL log the Task identifier being switched from
2. WHEN a Context Switch occurs, THE Scheduler SHALL log the Task identifier being switched to
3. WHEN a Context Switch occurs, THE Scheduler SHALL log the Task name being switched to
4. THE Scheduler SHALL output all log messages to the serial port
5. THE Scheduler SHALL format log messages with a consistent prefix indicating scheduler activity

### Requirement 6

**User Story:** ในฐานะ Kernel Developer ฉันต้องการให้ระบบสามารถรัน Task ตัวอย่างหลายตัวและสลับกันได้ เพื่อพิสูจน์ว่า Scheduler ทำงานได้จริง

#### Acceptance Criteria

1. THE Scheduler SHALL support spawning at least two demonstration Tasks during kernel initialization
2. WHEN demonstration Tasks execute, THE Scheduler SHALL allow each Task to output its identifier to serial port
3. THE Scheduler SHALL demonstrate successful Context Switch by showing alternating output from different Tasks
4. THE Scheduler SHALL maintain system stability during continuous Task switching for at least 100 context switches
5. WHEN demonstration Tasks run, THE Scheduler SHALL show visible evidence of Round-Robin scheduling through log output
