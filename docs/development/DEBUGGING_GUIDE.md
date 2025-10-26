# MelloOS Debugging Guide

คู่มือการใช้งาน Debugger สำหรับ MelloOS kernel development

## เครื่องมือที่ต้องมี

### 1. GDB (GNU Debugger)

ตรวจสอบว่ามี GDB ติดตั้งแล้ว:
```bash
gdb --version
```

ถ้ายังไม่มี ติดตั้งด้วย:
```bash
# macOS
brew install gdb

# หรือใช้ lldb ที่มาพร้อม Xcode
xcode-select --install
```

### 2. QEMU

ต้องมี QEMU สำหรับรัน kernel:
```bash
qemu-system-x86_64 --version
```

## วิธีการ Debug

### วิธีที่ 1: ใช้ VS Code Debugger (แนะนำ)

#### ขั้นตอน:

1. **เปิด VS Code ที่ workspace ของ MelloOS**

2. **เริ่ม QEMU ในโหมด debug:**
   ```bash
   ./tools/debug/start_qemu_debug.sh
   ```
   
   หรือรันด้วยตัวเอง:
   ```bash
   make clean && make build && make iso
   qemu-system-x86_64 -cdrom melloos.iso -m 512M -smp 4 -serial stdio -s -S
   ```
   
   **หมายเหตุ:**
   - `-s`: เปิด GDB server ที่ port 1234
   - `-S`: หยุด CPU รอให้ debugger เชื่อมต่อ

3. **เปิด Debug panel ใน VS Code:**
   - กด `Cmd+Shift+D` (macOS) หรือ `Ctrl+Shift+D` (Linux/Windows)
   - หรือคลิกไอคอน Debug ที่ sidebar

4. **เลือก configuration:**
   - เลือก "Debug MelloOS Kernel (GDB)" จาก dropdown
   - หรือ "Attach to QEMU (GDB)" ถ้า QEMU รันอยู่แล้ว

5. **เริ่ม debug:**
   - กด `F5` หรือคลิก "Start Debugging"
   - Debugger จะเชื่อมต่อกับ QEMU

6. **ใช้งาน debugger:**
   - **F9**: Toggle breakpoint
   - **F5**: Continue
   - **F10**: Step over
   - **F11**: Step into
   - **Shift+F11**: Step out
   - **Cmd+K Cmd+I**: Show hover info

### วิธีที่ 2: ใช้ GDB Command Line

#### ขั้นตอน:

1. **เริ่ม QEMU ในโหมด debug:**
   ```bash
   ./tools/debug/start_qemu_debug.sh
   ```

2. **เปิด terminal ใหม่และรัน GDB:**
   ```bash
   gdb kernel/target/x86_64-unknown-none/debug/kernel
   ```

3. **เชื่อมต่อกับ QEMU:**
   ```gdb
   (gdb) target remote localhost:1234
   ```

4. **ตั้ง breakpoint:**
   ```gdb
   (gdb) break kernel_main
   (gdb) break panic_handler
   ```

5. **เริ่มรัน:**
   ```gdb
   (gdb) continue
   ```

### วิธีที่ 3: ใช้ LLDB (สำหรับ macOS)

ถ้าใช้ LLDB แทน GDB:

```bash
lldb kernel/target/x86_64-unknown-none/debug/kernel

(lldb) gdb-remote localhost:1234
(lldb) breakpoint set --name kernel_main
(lldb) continue
```

## คำสั่ง GDB ที่มีประโยชน์

### การควบคุมการทำงาน

```gdb
continue (c)          # รันต่อจนถึง breakpoint ถัดไป
step (s)              # รันทีละบรรทัด (เข้าไปใน function)
next (n)              # รันทีละบรรทัด (ข้าม function)
finish                # รันจนจบ function ปัจจุบัน
until <line>          # รันจนถึงบรรทัดที่กำหนด
```

### Breakpoints

```gdb
break kernel_main                    # ตั้ง breakpoint ที่ function
break kernel/src/main.rs:42         # ตั้ง breakpoint ที่บรรทัด
break *0x100000                     # ตั้ง breakpoint ที่ address
info breakpoints                    # แสดง breakpoints ทั้งหมด
delete 1                            # ลบ breakpoint หมายเลข 1
disable 1                           # ปิดใช้งาน breakpoint
enable 1                            # เปิดใช้งาน breakpoint
```

### ดูข้อมูล

```gdb
info registers                      # แสดง CPU registers
info registers rax rbx rcx          # แสดง registers เฉพาะ
print variable_name                 # แสดงค่าตัวแปร
print/x $rax                        # แสดงค่า register ในรูป hex
x/10x 0x100000                      # แสดง memory 10 bytes ที่ address
backtrace (bt)                      # แสดง call stack
frame 0                             # เปลี่ยนไปที่ stack frame
list                                # แสดง source code
disassemble                         # แสดง assembly code
```

### Memory และ Registers

```gdb
x/10i $rip                          # แสดง 10 instructions ที่ RIP
x/10gx $rsp                         # แสดง 10 qwords ที่ stack pointer
set $rax = 0x1234                   # เปลี่ยนค่า register
set {int}0x100000 = 42              # เขียนค่าลง memory
```

### Watchpoints

```gdb
watch variable_name                 # หยุดเมื่อตัวแปรเปลี่ยนค่า
watch *0x100000                     # หยุดเมื่อ memory address เปลี่ยน
rwatch *0x100000                    # หยุดเมื่ออ่าน memory
awatch *0x100000                    # หยุดเมื่ออ่านหรือเขียน memory
```

## การ Debug สถานการณ์เฉพาะ

### Debug Boot Process

```gdb
# ตั้ง breakpoint ที่จุดเริ่มต้น
break _start
break kernel_main

# ดู boot parameters
print/x $rdi    # Limine boot info pointer
```

### Debug Page Faults

```gdb
# ตั้ง breakpoint ที่ page fault handler
break page_fault_handler

# เมื่อเกิด page fault ดูข้อมูล
info registers cr2    # Address ที่เกิด fault
info registers cr3    # Page table base
backtrace            # ดูว่าเกิดจากไหน
```

### Debug Scheduler

```gdb
# ตั้ง breakpoint ที่ scheduler
break schedule
break context_switch

# ดูข้อมูล task
print current_task
print *current_task
```

### Debug Memory Allocation

```gdb
# ตั้ง breakpoint ที่ allocator
break kmalloc
break kfree

# ดู heap state
print heap_start
print heap_end
```

### Debug Interrupts

```gdb
# ตั้ง breakpoint ที่ interrupt handlers
break timer_interrupt_handler
break keyboard_interrupt_handler

# ดู interrupt state
info registers rflags    # ดู IF flag
```

## Tips และ Tricks

### 1. สร้าง GDB Scripts

สร้างไฟล์ `debug_kernel.gdb`:
```gdb
target remote localhost:1234
break kernel_main
continue
```

รันด้วย:
```bash
gdb -x debug_kernel.gdb kernel/target/x86_64-unknown-none/debug/kernel
```

### 2. ใช้ Conditional Breakpoints

```gdb
break kernel_main if some_variable == 42
```

### 3. ดู Assembly พร้อม Source

```gdb
layout split    # แสดงทั้ง source และ assembly
layout asm      # แสดงแค่ assembly
layout src      # แสดงแค่ source
```

### 4. Save Breakpoints

```gdb
save breakpoints breakpoints.txt
source breakpoints.txt
```

### 5. Debug Symbols

ตรวจสอบว่ามี debug symbols:
```bash
file kernel/target/x86_64-unknown-none/debug/kernel
# ควรเห็น "not stripped"
```

ถ้าไม่มี symbols ให้ build ใหม่:
```bash
cd kernel
cargo build
```

## Troubleshooting

### ปัญหา: GDB ไม่เชื่อมต่อกับ QEMU

**แก้ไข:**
1. ตรวจสอบว่า QEMU รันด้วย `-s -S`
2. ตรวจสอบว่า port 1234 ไม่ถูกใช้งาน:
   ```bash
   lsof -i :1234
   ```
3. ลอง connect ใหม่:
   ```gdb
   disconnect
   target remote localhost:1234
   ```

### ปัญหา: ไม่เห็น source code

**แก้ไข:**
1. ตรวจสอบว่า build ด้วย debug mode
2. ตั้ง source path:
   ```gdb
   directory kernel/src
   ```

### ปัญหา: Breakpoint ไม่หยุด

**แก้ไข:**
1. ตรวจสอบว่า function name ถูกต้อง
2. ลองใช้ address แทน:
   ```gdb
   info symbol kernel_main
   break *<address>
   ```

### ปัญหา: QEMU หยุดทำงาน

**แก้ไข:**
1. ตรวจสอบ QEMU log:
   ```bash
   cat qemu.log
   ```
2. ลองรันโดยไม่มี debugger:
   ```bash
   make run
   ```

## การ Debug Userspace Programs

สำหรับ debug userspace programs (init, shell, etc.):

1. **Build ด้วย debug symbols:**
   ```bash
   cd kernel/userspace/mello-sh
   cargo build
   ```

2. **ใช้ GDB ดู symbols:**
   ```bash
   gdb kernel/userspace/mello-sh/target/x86_64-unknown-none/debug/mello-sh
   ```

3. **ตั้ง breakpoint ที่ userspace code:**
   ```gdb
   break main
   break execute_command
   ```

**หมายเหตุ:** การ debug userspace ใน OS kernel ซับซ้อนกว่าเพราะต้องจัดการกับ context switching

## เอกสารเพิ่มเติม

- [GDB Documentation](https://sourceware.org/gdb/documentation/)
- [QEMU Debugging](https://qemu.readthedocs.io/en/latest/system/gdb.html)
- [Rust Debugging](https://doc.rust-lang.org/book/appendix-04-useful-development-tools.html#debugging)
- [OS Dev Wiki - Debugging](https://wiki.osdev.org/Debugging)

## สรุป

การ debug OS kernel ต้องใช้เครื่องมือและเทคนิคพิเศษ:

1. **ใช้ QEMU + GDB** เป็นหลัก
2. **ตั้ง breakpoints** ที่จุดสำคัญ
3. **ดู registers และ memory** เพื่อเข้าใจสถานะ
4. **ใช้ VS Code** สำหรับ UI ที่ใช้งานง่าย
5. **อ่าน logs** จาก serial output

Happy debugging! 🐛🔍
