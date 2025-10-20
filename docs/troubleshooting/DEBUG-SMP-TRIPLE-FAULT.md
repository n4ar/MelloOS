# Debugging SMP Triple Fault - Practical Guide

## Quick Start

สำหรับการ debug triple fault ใน SMP boot มี 3 วิธีหลัก:

### วิธีที่ 1: QEMU Monitor (ง่ายที่สุด)

```bash
# รัน QEMU ในโหมด debug
./tools/qemu-debug-smp.sh

# เมื่อเกิด triple fault ให้เชื่อมต่อกับ monitor
telnet 127.0.0.1 55555

# ใน monitor พิมพ์:
info registers -a       # แสดง registers ของทุก CPU
info mem                # แสดง page table mappings
x/10i $rip              # disassemble ที่ตำแหน่ง RIP
```

Log จะถูกบันทึกใน `qemu-debug.log` และสามารถวิเคราะห์ด้วย:
```bash
./tools/analyze-triple-fault.sh
```

### วิธีที่ 2: GDB (ละเอียดที่สุด)

Terminal 1 - รัน QEMU กับ GDB stub:
```bash
qemu-system-x86_64 -s -S -smp 2 -m 2G -cdrom mellos.iso -serial stdio -no-reboot
```

Terminal 2 - เชื่อมต่อ GDB:
```bash
gdb -x tools/gdb-smp.gdb
```

GDB จะหยุดที่:
- 0x8000 - Trampoline start (16-bit real mode)
- 0x8050 - Protected mode entry (32-bit)
- 0x80a0 - Long mode entry (64-bit)

คำสั่งพิเศษใน GDB:
- `show-paging` - ดูโครงสร้าง page tables
- `show-trampoline-data` - ดูค่าที่ trampoline ต้องใช้
- `show-ap-state` - ดูสถานะ AP CPU
- `info threads` - ดู CPU ทั้งหมด
- `thread 2` - สลับไป AP CPU

### วิธีที่ 3: Serial Debugging (สำหรับ real hardware)

แก้ไข `boot_ap.S` เพื่อเพิ่ม serial output:

```asm
# เพิ่มหลัง protected_mode_entry:
protected_mode_entry:
    # Debug: Write 'P' to serial port (0x3F8)
    movb    $'P', %al
    movw    $0x3F8, %dx
    outb    %al, %dx
    
    # ... rest of code
```

เพิ่ม debug markers ที่:
- หลัง real mode → protected mode ('P')
- หลัง PAE enable ('A')  
- หลัง long mode enable ('L')
- ก่อน jump ไป Rust ('R')

## สาเหตุที่พบบ่อยของ Triple Fault

### 1. Page Table ไม่ถูกต้อง (พบบ่อยที่สุด)

**อาการ:** Triple fault ทันทีหลัง `mov cr3, eax`

**การตรวจสอบ:**
```bash
# ใน QEMU monitor:
info mem
gva2gpa 0x8000          # ควร map ไป 0x8000
gva2gpa <stack_addr>    # ควร map ได้
gva2gpa <entry_addr>    # ควร map ได้
```

**แก้ไข:**
- ตรวจสอบว่า identity mapping สำหรับ 0x0-0x1FFFFF มีจริง
- ตรวจสอบว่า entry point และ stack ถูก map
- ลอง print CR3 ค่าที่ BSP ส่งให้ AP:
  ```rust
  serial_println!("[SMP] CR3 = 0x{:X}", cr3);
  ```

### 2. Stack ไม่ถูก Map

**อาการ:** Triple fault หลัง jump ไป long mode หรือตอน call Rust function

**การตรวจสอบ:**
```rust
// ใน init_smp() เพิ่ม debug:
serial_println!("[SMP] AP#{} stack @ 0x{:X}", cpu_id, stack_top);

// ตรวจสอบว่า stack อยู่ใน valid range
let virt_addr = stack_top;
let phys_addr = virt_to_phys(virt_addr);
serial_println!("[SMP] Stack: virt=0x{:X} phys=0x{:X}", virt_addr, phys_addr);
```

**แก้ไข:**
- ใช้ stack ใน lower-half (< 2GB) ถ้าเป็นไปได้
- หรือต้อง map stack address ใน page table ให้ถูกต้อง

### 3. Entry Point Address ไม่ถูกต้อง

**อาการ:** Triple fault ทันทีหลัง `jmp *%rax` ใน long_mode_entry

**การตรวจสอบ:**
```rust
// Debug entry point address:
serial_println!("[SMP] ap_entry64 @ 0x{:X}", ap_entry64 as u64);

// ตรวจสอบว่า address นี้ถูก map
```

**แก้ไข:**
- ถ้า ap_entry64 อยู่ใน higher-half (0xFFFF...) ต้องแน่ใจว่า higher-half mappings ถูก copy
- อาจต้องสร้าง trampoline function ใน lower-half แทน

### 4. GDT Descriptors ไม่ถูกต้อง

**อาการ:** Triple fault หลัง `lgdt` หรือ `ljmp`

**การตรวจสอบ:**
ใน GDB ที่ breakpoint ก่อน lgdt:
```gdb
x/3gx 0x8100  # ดู GDT entries
```

**ตรวจสอบว่า:**
- Code segment (offset 0x08): bit 53 (L) = 1 สำหรับ 64-bit
- Data segment (offset 0x10): present bit = 1
- GDT limit ถูกต้อง (23 bytes = 0x17)

### 5. CR0/CR4 Bits ไม่ถูกต้อง

**อาการ:** General Protection Fault (GP) เมื่อเปิด paging

**การตรวจสอบ:**
ใน GDB หรือ QEMU monitor:
```
info registers cr0 cr4
```

**ต้องเป็น:**
- CR4.PAE = 1 (bit 5) ก่อนเปิด long mode
- CR0.PE = 1 (bit 0) protected mode
- CR0.PG = 1 (bit 31) paging
- EFER.LME = 1 (bit 8) long mode enable

## Debugging Workflow แนะนำ

### ขั้นตอนที่ 1: หา Fault Location

```bash
# รัน debug script
./tools/qemu-debug-smp.sh

# วิเคราะห์ log
./tools/analyze-triple-fault.sh
```

ดูที่ RIP value - จะบอกว่า crash ที่ไหน:
- 0x8000-0x8050: Real mode / Protected mode transition
- 0x8050-0x80a0: Protected mode / Long mode transition  
- 0x80a0-0x8100: Long mode entry
- Higher address: ใน Rust code แล้ว

### ขั้นตอนที่ 2: ตรวจสอบ Page Tables

```rust
// เพิ่มใน init_smp() ก่อน start APs:
unsafe {
    // Test identity mapping
    let test_addr = 0x8000usize;
    let test_val = *(test_addr as *const u32);
    serial_println!("[SMP] Identity map test: 0x8000 = 0x{:X}", test_val);
    
    // Verify page table structure
    verify_page_tables(cr3);
}
```

### ขั้นตอนที่ 3: แก้ไขและทดสอบ

เมื่อพบปัญหาแล้ว:

1. แก้ไขโค้ด
2. Rebuild: `make clean && make iso`
3. Test ใหม่: `./tools/qemu-debug-smp.sh`
4. ตรวจสอบ log

## Common Fixes

### Fix 1: Force Lower-Half Stack

```rust
// ใน init_smp()
const AP_STACK_PHYS_BASE: usize = 0x200000; // 2MB mark
static mut NEXT_STACK_OFFSET: usize = 0;

let stack_top = unsafe {
    let offset = NEXT_STACK_OFFSET;
    NEXT_STACK_OFFSET += AP_STACK_SIZE;
    
    // Map stack in identity region
    let stack_phys = AP_STACK_PHYS_BASE + offset;
    identity_map_range(stack_phys, AP_STACK_SIZE)?;
    
    stack_phys + AP_STACK_SIZE
};
```

### Fix 2: Verify Entry Point Mapping

```rust
// ก่อน start AP ให้ verify ว่า entry point ถูก map
let entry_virt = ap_entry64 as usize;
let entry_phys = virt_to_phys(entry_virt);

serial_println!("[SMP] Entry: virt=0x{:X} phys=0x{:X}", entry_virt, entry_phys);

// ถ้าใช้ higher-half ให้แน่ใจว่ามี mapping
if entry_virt >= 0xFFFF800000000000 {
    // Verify higher-half mapping exists
    verify_higher_half_mapping(entry_virt)?;
}
```

### Fix 3: Add Serial Debug to Trampoline

แก้ไข `boot_ap.S`:

```asm
# Macro for serial debug
.macro SERIAL_DEBUG char
    movb    $\char, %al
    movw    $0x3F8, %dx
    outb    %al, %dx
.endm

protected_mode_entry:
    SERIAL_DEBUG 'P'    # Protected mode OK
    # ... setup ...
    
long_mode_entry:
    SERIAL_DEBUG 'L'    # Long mode OK
    # ... setup ...
    
    SERIAL_DEBUG 'R'    # Ready to call Rust
    jmp     *%rax
```

## ต่อไปต้องทำอะไร?

เมื่อ AP boot สำเร็จแล้ว (เห็น "[SMP] AP#1 online"):

1. ✅ Test กับ multiple APs (2, 4, 8 cores)
2. ✅ Test scheduler บน multiple cores
3. ✅ Test IPI (Inter-Processor Interrupts)
4. ✅ Test per-CPU data structures
5. ✅ Stress test กับ concurrent workloads

## References

- Intel SDM Vol 3, Ch 8: Multiple-Processor Management
- Intel SDM Vol 3, Ch 9: Advanced Programmable Interrupt Controller (APIC)
- OSDev Wiki: SMP - https://wiki.osdev.org/SMP
- OSDev Wiki: Trampoline - https://wiki.osdev.org/Trampoline
