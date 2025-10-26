---
inclusion: always
---

# Cargo Check Policy - บังคับใช้ทุกครั้ง

## หลักการสำคัญ: ตรวจสอบทันทีหลังแก้ไข

**MANDATORY:** ทุกครั้งที่แก้ไขโค้ด Rust ต้องรัน `cargo check` ทันทีเพื่อตรวจสอบข้อผิดพลาด

### เหตุผล:
- จับข้อผิดพลาดได้เร็วขึ้น (catch errors early)
- ประหยัดเวลาในการ debug
- ป้องกันข้อผิดพลาดสะสม
- รักษาคุณภาพโค้ดตลอดเวลา

## กฎบังคับ - REQUIRED BEHAVIOR

### 1. หลังแก้ไขไฟล์ Rust ทุกครั้ง

❌ **ห้ามทำ:**
- แก้หลายไฟล์แล้วค่อยเช็คทีเดียว
- สมมติว่าโค้ดถูกต้องโดยไม่เช็ค
- ข้ามการเช็คเพราะคิดว่าเป็นการแก้เล็กน้อย
- รอจนเขียนโค้ดเสร็จหมดแล้วค่อยเช็ค

✅ **ต้องทำ:**
- รัน `cargo check` ทันทีหลังแก้ไขไฟล์
- แก้ error ที่เจอให้หมดก่อนไปต่อ
- ใช้ `getDiagnostics` tool เพื่อดู errors/warnings
- รายงานผลการเช็คให้ user ทราบ

### 2. Workflow ที่ถูกต้อง

```
1. แก้ไขโค้ด Rust
   ↓
2. รัน cargo check ทันที
   ↓
3. ถ้ามี errors → แก้ให้หมด → กลับไปข้อ 2
   ↓
4. ถ้าไม่มี errors → ดำเนินการต่อได้
```

### 3. คำสั่งที่ใช้

**สำหรับ kernel:**
```bash
cd kernel && cargo check
```

**สำหรับ userspace programs:**
```bash
cd kernel/userspace/mello-term && cargo check
cd kernel/userspace/mello-sh && cargo check
cd kernel/userspace/mellobox && cargo check
```

**สำหรับทั้งหมด (ถ้ามี workspace):**
```bash
cargo check --workspace
```

### 4. ใช้ getDiagnostics Tool

**PREFERRED METHOD:** ใช้ `getDiagnostics` tool แทนการรัน cargo check ด้วย bash

```
getDiagnostics(paths=["kernel/src/dev/pty/mod.rs"])
```

**ข้อดี:**
- เห็น errors และ warnings แบบ structured
- ไม่ต้องรอ compile
- ได้ข้อมูลละเอียดกว่า
- รวดเร็วกว่า

### 5. เมื่อไหร่ต้องเช็ค

**ต้องเช็คทุกครั้งที่:**
- สร้างไฟล์ใหม่
- แก้ไข struct, enum, trait
- เพิ่ม/ลบ function
- เปลี่ยน function signature
- แก้ไข imports
- เปลี่ยน dependencies
- แก้ไขโค้ดใดๆ ใน Rust files

**ไม่ต้องเช็คเมื่อ:**
- แก้ไขไฟล์ที่ไม่ใช่ Rust (markdown, shell scripts, etc.)
- แก้ไข comments เท่านั้น
- แก้ไข documentation เท่านั้น

## ตัวอย่างการใช้งาน

### ตัวอย่าง 1: แก้ไขไฟล์เดียว

```
User: แก้ไข kernel/src/dev/pty/mod.rs

Agent:
1. แก้ไขไฟล์ตามที่ต้องการ
2. รัน: getDiagnostics(paths=["kernel/src/dev/pty/mod.rs"])
3. ถ้ามี errors → แก้ไขและเช็คอีกครั้ง
4. รายงานผล: "✓ No errors found in pty/mod.rs"
```

### ตัวอย่าง 2: แก้หลายไฟล์

```
User: เพิ่ม signal infrastructure

Agent:
1. สร้าง kernel/src/signal/mod.rs
2. เช็ค: getDiagnostics(paths=["kernel/src/signal/mod.rs"])
3. แก้ kernel/src/lib.rs เพื่อ import signal module
4. เช็ค: getDiagnostics(paths=["kernel/src/lib.rs", "kernel/src/signal/mod.rs"])
5. แก้ errors ถ้ามี
6. รายงานผล: "✓ All files checked successfully"
```

### ตัวอย่าง 3: เจอ errors

```
Agent:
1. แก้ไข kernel/src/dev/pty/mod.rs
2. รัน getDiagnostics
3. เจอ errors:
   - error[E0425]: cannot find value `TIOCGWINSZ` in this scope
   - error[E0308]: mismatched types
4. แก้ไข:
   - เพิ่ม import สำหรับ TIOCGWINSZ
   - แก้ type mismatch
5. เช็คอีกครั้ง
6. ✓ No errors
```

## Integration กับ Workflow อื่น

### กับ Spec Tasks

เมื่อทำงานตาม tasks.md:
1. อ่าน task
2. เขียน/แก้โค้ด
3. **รัน cargo check ทันที** ← สำคัญ!
4. แก้ errors
5. ทดสอบ (ถ้าจำเป็น)
6. mark task เป็น complete

### กับ Testing

```
1. เขียนโค้ด
2. cargo check ← ต้องผ่านก่อน
3. cargo build
4. รัน tests
```

### กับ Debugging

เมื่อเจอปัญหา:
1. แก้โค้ด
2. cargo check ← ตรวจสอบ syntax/type errors ก่อน
3. ถ้าผ่าน → ลอง build และรัน
4. ถ้าไม่ผ่าน → แก้ errors ก่อน

## การรายงานผล

**ต้องรายงานให้ user ทราบเสมอ:**

✅ **กรณีไม่มี errors:**
```
"Checked kernel/src/dev/pty/mod.rs - no errors found"
```

❌ **กรณีมี errors:**
```
"Found 2 errors in kernel/src/dev/pty/mod.rs:
- Line 45: cannot find value `TIOCGWINSZ`
- Line 67: mismatched types

Fixing these errors..."
```

## สรุป

**จำไว้:**
- แก้โค้ด → เช็คทันที
- ใช้ getDiagnostics tool เป็นหลัก
- แก้ errors ให้หมดก่อนไปต่อ
- รายงานผลให้ user ทราบ

**ประโยชน์:**
- ประหยัดเวลา
- โค้ดมีคุณภาพ
- จับ bugs ได้เร็ว
- ลด frustration

---

**This policy is MANDATORY and must be followed for all Rust code modifications.**
