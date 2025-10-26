# Requirements Document

## Introduction

MelloOS เป็นระบบปฏิบัติการที่สร้างขึ้นตั้งแต่ศูนย์ด้วยภาษา Rust โดยเริ่มจากการพัฒนา Kernel ที่สามารถบูตผ่าน UEFI bootloader และแสดงข้อความบนหน้าจอได้ โปรเจกต์นี้มุ่งเน้นการสร้างพื้นฐานของระบบปฏิบัติการที่ทันสมัย ปลอดภัย และสามารถขยายต่อได้ในอนาคต

## Requirements

### Requirement 1: Kernel Development Environment

**User Story:** ในฐานะนักพัฒนา ฉันต้องการตั้งค่าสภาพแวดล้อมการพัฒนา Kernel ด้วย Rust เพื่อให้สามารถคอมไพล์และสร้าง bare-metal kernel ได้

#### Acceptance Criteria

1. WHEN โปรเจกต์ถูกสร้างขึ้น THEN ระบบ SHALL มีโครงสร้างไดเรกทอรี `/kernel/src/` สำหรับ source code ของ kernel
2. WHEN การคอมไพล์เริ่มต้น THEN ระบบ SHALL ใช้ Rust toolchain ที่รองรับ `no_std` environment
3. WHEN target architecture ถูกกำหนด THEN ระบบ SHALL ตั้งค่า target เป็น `x86_64-unknown-none`
4. WHEN Cargo configuration ถูกอ่าน THEN ระบบ SHALL มีไฟล์ `.cargo/config.toml` ที่กำหนด build target และ compiler flags ที่เหมาะสม
5. WHEN kernel ถูกคอมไพล์ THEN ระบบ SHALL สร้าง ELF binary ที่สามารถบูตได้ผ่าน UEFI bootloader

### Requirement 2: Bootloader Configuration

**User Story:** ในฐานะนักพัฒนา ฉันต้องการกำหนดค่า Limine bootloader เพื่อให้สามารถโหลดและเริ่มต้น kernel บนสถาปัตยกรรม x86_64 ผ่าน UEFI ได้

#### Acceptance Criteria

1. WHEN bootloader configuration ถูกสร้าง THEN ระบบ SHALL มีไฟล์ `/boot/limine.cfg` ที่กำหนดค่าการบูต
2. WHEN limine.cfg ถูกอ่าน THEN ระบบ SHALL ระบุ path ไปยัง kernel binary
3. WHEN bootloader เริ่มทำงาน THEN ระบบ SHALL โหลด kernel ในโหมด x86_64
4. WHEN UEFI boot process เริ่มต้น THEN ระบบ SHALL ใช้ Limine protocol สำหรับการสื่อสารระหว่าง bootloader และ kernel
5. WHEN boot configuration มีการเปลี่ยนแปลง THEN ระบบ SHALL สามารถปรับแต่งพารามิเตอร์การบูตได้ง่าย

### Requirement 3: Kernel Entry Point และ Display Output

**User Story:** ในฐานะนักพัฒนา ฉันต้องการให้ kernel มี entry point ที่ถูกต้องและสามารถแสดงข้อความบนหน้าจอได้ เพื่อยืนยันว่าการบูตสำเร็จ

#### Acceptance Criteria

1. WHEN kernel ถูกโหลดเข้าหน่วยความจำ THEN ระบบ SHALL เรียก entry point function ที่กำหนดไว้
2. WHEN entry point ถูกเรียก THEN ระบบ SHALL เริ่มต้น framebuffer หรือ text mode สำหรับการแสดงผล
3. WHEN framebuffer พร้อมใช้งาน THEN ระบบ SHALL แสดงข้อความ "Hello from my kernel ✨" บนหน้าจอ
4. WHEN ข้อความถูกแสดง THEN ระบบ SHALL ใช้ font rendering หรือ text mode ที่อ่านได้ชัดเจน
5. WHEN kernel ทำงานสำเร็จ THEN ระบบ SHALL ไม่ crash และรอคำสั่งต่อไป (infinite loop)

### Requirement 4: Build System

**User Story:** ในฐานะนักพัฒนา ฉันต้องการระบบ build ที่อัตโนมัติ เพื่อให้สามารถคอมไพล์ kernel และสร้าง bootable ISO image ได้ง่าย

#### Acceptance Criteria

1. WHEN Makefile ถูกสร้าง THEN ระบบ SHALL มีคำสั่ง `make build` สำหรับคอมไพล์ kernel
2. WHEN kernel binary ถูกสร้าง THEN ระบบ SHALL รวม kernel เข้ากับ Limine bootloader เป็น ISO image
3. WHEN ISO image ถูกสร้าง THEN ระบบ SHALL สามารถบูตได้บน QEMU หรือ hardware จริง
4. WHEN Makefile ถูกเรียกใช้ THEN ระบบ SHALL มีคำสั่ง `make clean` สำหรับลบไฟล์ที่สร้างขึ้น
5. WHEN build process เสร็จสิ้น THEN ระบบ SHALL แสดงสถานะความสำเร็จหรือข้อผิดพลาดที่ชัดเจน

### Requirement 5: QEMU Testing Environment

**User Story:** ในฐานะนักพัฒนา ฉันต้องการสคริปต์สำหรับรัน kernel บน QEMU emulator เพื่อให้สามารถทดสอบได้อย่างรวดเร็วโดยไม่ต้องใช้ hardware จริง

#### Acceptance Criteria

1. WHEN QEMU script ถูกสร้าง THEN ระบบ SHALL มีไฟล์ `tools/qemu.sh` ที่สามารถ execute ได้
2. WHEN qemu.sh ถูกเรียกใช้ THEN ระบบ SHALL เริ่มต้น QEMU ด้วยพารามิเตอร์ที่เหมาะสมสำหรับ UEFI boot
3. WHEN QEMU เริ่มทำงาน THEN ระบบ SHALL โหลด ISO image และบูต kernel
4. WHEN kernel ทำงานใน QEMU THEN ระบบ SHALL แสดงข้อความ "Hello from my kernel ✨" ในหน้าต่าง QEMU
5. WHEN ผู้ใช้ต้องการหยุด QEMU THEN ระบบ SHALL สามารถปิดได้ด้วย Ctrl+C หรือคำสั่งที่กำหนด

### Requirement 6: Documentation และ Build Instructions

**User Story:** ในฐานะนักพัฒนา ฉันต้องการเอกสารที่อธิบายขั้นตอนการ build และ run kernel เพื่อให้ผู้อื่นสามารถทำตามได้

#### Acceptance Criteria

1. WHEN โปรเจกต์ถูกสร้าง THEN ระบบ SHALL มีไฟล์ README.md ที่อธิบายภาพรวมของโปรเจกต์
2. WHEN README ถูกอ่าน THEN ระบบ SHALL ระบุ dependencies ที่จำเป็น (Rust, QEMU, xorriso, etc.)
3. WHEN ผู้ใช้ต้องการ build THEN เอกสาร SHALL แสดงคำสั่ง `make build` พร้อมคำอธิบาย
4. WHEN ผู้ใช้ต้องการ run THEN เอกสาร SHALL แสดงคำสั่ง `make run` หรือ `./tools/qemu.sh` พร้อมคำอธิบาย
5. WHEN มีปัญหาเกิดขึ้น THEN เอกสาร SHALL มีส่วน troubleshooting สำหรับปัญหาที่พบบ่อย
