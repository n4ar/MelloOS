# Implementation Plan

- [x] 1. ตั้งค่าโครงสร้างโปรเจกต์และ Cargo configuration
  - สร้างโครงสร้างไดเรกทอรี `kernel/src/`, `boot/`, `tools/`
  - สร้างไฟล์ `kernel/Cargo.toml` พร้อม dependencies (`limine` crate)
  - สร้างไฟล์ `.cargo/config.toml` กำหนด target เป็น `x86_64-unknown-none` และ rustflags ที่จำเป็น
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [x] 2. สร้าง kernel entry point และ panic handler
  - สร้างไฟล์ `kernel/src/main.rs` พร้อม `#![no_std]` และ `#![no_main]` attributes
  - เขียน `_start` function เป็น entry point ที่ไม่ return (`-> !`)
  - สร้างไฟล์ `kernel/src/panic.rs` พร้อม `#[panic_handler]` function
  - เพิ่ม infinite loop (`loop {}`) ใน panic handler เพื่อ halt CPU
  - _Requirements: 3.1, 3.5_

- [x] 3. Implement Limine protocol integration
  - เพิ่ม `limine` crate ใน `Cargo.toml`
  - สร้าง static `FRAMEBUFFER_REQUEST` ใน `main.rs` ด้วย `#[used]` และ `#[link_section = ".requests"]`
  - เขียนโค้ดเพื่อดึง framebuffer response จาก Limine
  - จัดการกรณีที่ framebuffer ไม่พร้อมใช้งาน (panic)
  - _Requirements: 2.4, 3.2_

- [x] 4. สร้าง framebuffer driver
  - สร้างไฟล์ `kernel/src/framebuffer.rs`
  - สร้าง `Framebuffer` struct เก็บ address, width, height, pitch, bpp
  - Implement `new()` method รับ Limine framebuffer info
  - Implement `put_pixel()` method เขียน pixel ไปยัง framebuffer memory
  - Implement `clear()` method เคลียร์หน้าจอด้วยสีที่กำหนด
  - _Requirements: 3.2, 3.3, 3.4_

- [x] 5. Implement text rendering
  - สร้าง simple bitmap font หรือใช้ built-in font data (8x8 หรือ 8x16)
  - Implement `draw_char()` method วาดตัวอักษรหนึ่งตัวบน framebuffer
  - Implement `write_string()` method เขียน string โดยวนลูปเรียก `draw_char()`
  - จัดการ newline และ wrapping (optional)
  - _Requirements: 3.3, 3.4_

- [x] 6. เชื่อมต่อ framebuffer กับ kernel entry point
  - ใน `_start` function เรียกใช้ framebuffer request
  - สร้าง `Framebuffer` instance จาก Limine response
  - เรียก `clear()` เพื่อเคลียร์หน้าจอ
  - เรียก `write_string("Hello from my kernel ✨", x, y)` เพื่อแสดงข้อความ
  - เพิ่ม infinite loop (`loop {}`) เพื่อป้องกัน kernel จาก return
  - _Requirements: 3.1, 3.3, 3.5_

- [x] 7. สร้าง linker script (ถ้าจำเป็น)
  - สร้างไฟล์ `kernel/linker.ld` กำหนด memory layout
  - กำหนด `.text`, `.data`, `.bss`, `.requests` sections
  - เพิ่ม linker script path ใน `.cargo/config.toml`
  - _Requirements: 1.5_

- [x] 8. สร้าง Makefile สำหรับ build automation
  - สร้างไฟล์ `Makefile` ใน root directory
  - เพิ่ม `build` target เรียก `cargo build --release`
  - เพิ่ม `clean` target เรียก `cargo clean` และลบ ISO files
  - เพิ่มตัวแปรสำหรับ paths และ configuration
  - _Requirements: 4.1, 4.4, 4.5_

- [x] 9. สร้าง ISO image creation process
  - เพิ่ม `iso` target ใน Makefile
  - สร้าง directory structure สำหรับ ISO (`iso_root/boot/`)
  - ดาวน์โหลด Limine bootloader binaries (หรือใช้ git submodule)
  - คัดลอก kernel binary ไปยัง `iso_root/boot/kernel.elf`
  - คัดลอก Limine files (`limine-bios.sys`, `limine-uefi.efi`, etc.)
  - ใช้ `xorriso` สร้าง ISO image
  - รัน `limine bios-install` เพื่อติดตั้ง bootloader
  - _Requirements: 4.2, 4.3_

- [x] 10. สร้าง Limine bootloader configuration
  - สร้างไฟล์ `boot/limine.cfg`
  - กำหนด `TIMEOUT` สำหรับ boot menu
  - สร้าง boot entry ชื่อ "MelloOS"
  - ตั้งค่า `PROTOCOL=limine`
  - ตั้งค่า `KERNEL_PATH=boot:///boot/kernel.elf`
  - คัดลอก `limine.cfg` ไปยัง `iso_root/boot/` ใน Makefile
  - _Requirements: 2.1, 2.2, 2.3, 2.5_

- [x] 11. สร้าง QEMU launch script
  - สร้างไฟล์ `tools/qemu.sh`
  - เพิ่ม shebang (`#!/bin/bash`) และ execute permission
  - เขียนคำสั่ง `qemu-system-x86_64` พร้อมพารามิเตอร์:
    - `-M q35` (modern chipset)
    - `-m 2G` (2GB RAM)
    - `-cdrom mellos.iso`
    - `-boot d` (boot from CD)
    - `-serial stdio` (serial output)
    - `-bios /usr/share/ovmf/OVMF.fd` (UEFI firmware)
  - _Requirements: 5.1, 5.2, 5.3_

- [x] 12. เพิ่ม `run` target ใน Makefile
  - สร้าง `run` target ที่ depend on `iso` target
  - เรียก `./tools/qemu.sh` เพื่อรัน QEMU
  - _Requirements: 5.2, 5.3, 5.4_

- [x] 13. สร้าง README documentation
  - สร้างไฟล์ `README.md` ใน root directory
  - เขียนภาพรวมของโปรเจกต์ MelloOS
  - ระบุ dependencies ที่จำเป็น (Rust, QEMU, xorriso, OVMF)
  - อธิบายขั้นตอนการติดตั้ง dependencies
  - อธิบายคำสั่ง build: `make build`
  - อธิบายคำสั่ง create ISO: `make iso`
  - อธิบายคำสั่ง run: `make run`
  - เพิ่มส่วน troubleshooting สำหรับปัญหาที่พบบ่อย
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 14. ทดสอบ build และ boot process
  - รัน `make build` ตรวจสอบว่า kernel compile สำเร็จ
  - รัน `make iso` ตรวจสอบว่า ISO image ถูกสร้าง
  - รัน `make run` ตรวจสอบว่า QEMU เริ่มต้นและบูต kernel
  - ตรวจสอบว่าข้อความ "Hello from my kernel ✨" แสดงบนหน้าจอ QEMU
  - ตรวจสอบว่า kernel ไม่ crash และรอคำสั่งต่อไป
  - _Requirements: 3.3, 3.4, 3.5, 5.4_
