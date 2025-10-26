# Requirements Document

## Introduction

Phase 2 ของ MelloOS จะเพิ่มระบบจัดการหน่วยความจำ (Memory Management System) ที่สมบูรณ์ให้กับ kernel ระบบนี้จะจัดการทั้ง physical memory และ virtual memory โดยใช้ 4-level paging ตามสถาปัตยกรรม x86_64 และมี kernel allocator สำหรับจองและคืนหน่วยความจำแบบ dynamic

ระบบ Memory Management นี้จะเป็นพื้นฐานสำคัญสำหรับการพัฒนาฟีเจอร์อื่นๆ ในอนาคต เช่น process management, file system, และ device drivers

## Requirements

### Requirement 1: Physical Memory Management

**User Story:** ในฐานะ kernel developer ผมต้องการให้ kernel สามารถตรวจจับและจัดการ physical memory ที่มีอยู่ในระบบ เพื่อให้สามารถจัดสรรหน่วยความจำให้กับส่วนต่างๆ ของ kernel ได้

#### Acceptance Criteria

1. WHEN kernel บูตขึ้น THEN ระบบ SHALL อ่านข้อมูล memory map จาก Limine bootloader
2. WHEN ระบบได้รับ memory map THEN ระบบ SHALL คำนวณและแสดงจำนวน physical memory ทั้งหมดที่ใช้งานได้ (usable memory) ใน MB
3. WHEN ระบบเริ่มต้น physical memory allocator THEN ระบบ SHALL สร้าง data structure สำหรับติดตาม physical frames ที่ว่างและที่ถูกใช้งาน
4. WHEN มีการร้องขอ physical frame THEN ระบบ SHALL จัดสรร frame ที่ว่างและทำเครื่องหมายว่าถูกใช้งาน
5. WHEN มีการคืน physical frame THEN ระบบ SHALL ทำเครื่องหมาย frame นั้นว่าว่างและพร้อมใช้งานใหม่
6. WHEN มีการจัดสรรหรือคืน physical memory THEN ระบบ SHALL แสดง log ข้อความผ่าน serial output

### Requirement 2: Virtual Memory และ Paging

**User Story:** ในฐานะ kernel developer ผมต้องการให้ kernel ใช้ virtual memory addressing ผ่าน 4-level paging เพื่อแยก address space และเพิ่มความปลอดภัย

#### Acceptance Criteria

1. WHEN ระบบเริ่มต้น paging THEN ระบบ SHALL สร้าง page table hierarchy 4 ระดับ (PML4, PDPT, PD, PT) สำหรับ kernel
2. WHEN มีการ map virtual address ไป physical address THEN ระบบ SHALL สร้างหรืออัพเดท page table entries ที่จำเป็น
3. WHEN มีการ unmap virtual address THEN ระบบ SHALL ลบ page table entry และ invalidate TLB
4. WHEN มีการแปลง virtual address เป็น physical address THEN ระบบ SHALL traverse page tables และคืนค่า physical address หรือ None ถ้าไม่มี mapping
5. WHEN kernel ใช้งาน virtual address THEN ระบบ SHALL ใช้ higher half addressing (0xFFFF_8000_0000_0000 ขึ้นไป)
6. WHEN มีการสร้าง page mapping THEN ระบบ SHALL ตั้งค่า flags ที่เหมาะสม (present, writable, etc.)

### Requirement 3: Kernel Memory Allocator

**User Story:** ในฐานะ kernel developer ผมต้องการมีฟังก์ชัน kmalloc และ kfree สำหรับจัดสรรหน่วยความจำแบบ dynamic ใน kernel space

#### Acceptance Criteria

1. WHEN เรียกใช้ kmalloc(size) THEN ระบบ SHALL คืนค่า pointer ไปยังหน่วยความจำขนาด size bytes ที่จัดสรรให้
2. WHEN เรียกใช้ kfree(ptr) THEN ระบบ SHALL คืนหน่วยความจำที่ pointer ชี้ไปกลับสู่ allocator
3. WHEN มีการจัดสรรหน่วยความจำ THEN ระบบ SHALL แสดง log ที่ระบุขนาดและ address ที่จัดสรรให้
4. WHEN มีการคืนหน่วยความจำ THEN ระบบ SHALL แสดง log ที่ระบุขนาดและ address ที่ถูกคืน
5. WHEN allocator ไม่มีหน่วยความจำเพียงพอ THEN ระบบ SHALL คืนค่า null pointer และแสดง error log
6. WHEN มีการจัดสรรหน่วยความจำหลายครั้ง THEN allocator SHALL จัดการ fragmentation ได้อย่างมีประสิทธิภาพ
7. IF allocator ใช้ buddy algorithm THEN ระบบ SHALL แบ่งและรวม blocks ตามขนาด power of 2

### Requirement 4: Memory Management Initialization

**User Story:** ในฐานะ kernel developer ผมต้องการให้ระบบ memory management เริ่มต้นทำงานอัตโนมัติตอน kernel บูต

#### Acceptance Criteria

1. WHEN kernel เริ่มต้นทำงาน THEN ระบบ SHALL เรียกฟังก์ชัน init_memory() หลังจากแสดงข้อความ boot
2. WHEN init_memory() ทำงาน THEN ระบบ SHALL เริ่มต้น physical memory allocator ก่อน
3. WHEN physical allocator พร้อม THEN ระบบ SHALL เริ่มต้น paging system
4. WHEN paging system พร้อม THEN ระบบ SHALL เริ่มต้น kernel allocator
5. WHEN memory management เริ่มต้นเสร็จ THEN ระบบ SHALL แสดง log สรุปสถานะของระบบหน่วยความจำ
6. WHEN มี error ในการเริ่มต้น THEN ระบบ SHALL แสดง error message และหยุดการทำงาน

### Requirement 5: Memory Management Testing และ Logging

**User Story:** ในฐานะ kernel developer ผมต้องการทดสอบว่าระบบ memory management ทำงานถูกต้อง และมี log ที่ชัดเจนสำหรับ debugging

#### Acceptance Criteria

1. WHEN kernel บูตเสร็จ THEN ระบบ SHALL ทดสอบการจัดสรรหน่วยความจำด้วย kmalloc(1024)
2. WHEN ทดสอบ allocation THEN ระบบ SHALL แสดง address ที่ได้รับจาก kmalloc
3. WHEN ทดสอบ free THEN ระบบ SHALL เรียก kfree() และแสดง log การคืนหน่วยความจำ
4. WHEN มีการทำงานของ memory management THEN ระบบ SHALL ใช้ prefix "[MM]" ใน log messages
5. WHEN แสดง memory addresses THEN ระบบ SHALL ใช้รูปแบบ hexadecimal (0x...)
6. WHEN ระบบทำงานเสร็จสมบูรณ์ THEN kernel SHALL ยังคงแสดงข้อความ "Hello from MelloOS ✨" ได้เหมือนเดิม
7. IF มีการใช้สีใน log (optional) THEN log ของ memory management SHALL ใช้สีที่แตกต่างจาก system log
