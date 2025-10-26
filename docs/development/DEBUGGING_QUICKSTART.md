# Debugging Quick Start

เริ่มต้น debug MelloOS ใน 5 นาที! 🚀

## ขั้นตอนที่ 1: ติดตั้ง Extensions

เปิด VS Code และติดตั้ง extensions ที่แนะนำ:

1. กด `Cmd+Shift+P` (macOS) หรือ `Ctrl+Shift+P` (Linux/Windows)
2. พิมพ์ "Extensions: Show Recommended Extensions"
3. คลิก "Install All"

หรือติดตั้งด้วยตัวเอง:
- **rust-analyzer** - Rust language support
- **CodeLLDB** หรือ **C/C++** - Debugging support

## ขั้นตอนที่ 2: Build Kernel

```bash
make clean
make build
make iso
```

## ขั้นตอนที่ 3: เริ่ม Debug

### วิธีที่ 1: ใช้ VS Code (แนะนำ)

1. **เปิด Debug panel:**
   - กด `Cmd+Shift+D` (macOS) หรือ `Ctrl+Shift+D`
   - หรือคลิกไอคอน Debug ที่ sidebar

2. **เลือก configuration:**
   - เลือก "Debug MelloOS Kernel (GDB)" จาก dropdown

3. **เริ่ม debug:**
   - กด `F5`
   - QEMU จะเริ่มและ debugger จะเชื่อมต่อโดยอัตโนมัติ

4. **ตั้ง breakpoint:**
   - เปิดไฟล์ `kernel/src/main.rs`
   - คลิกที่ซ้ายสุดของบรรทัดที่ต้องการหยุด (จะเห็นจุดสีแดง)
   - หรือกด `F9` ที่บรรทัดที่ต้องการ

5. **ควบคุมการทำงาน:**
   - `F5` - Continue
   - `F10` - Step Over
   - `F11` - Step Into
   - `Shift+F11` - Step Out
   - `Shift+F5` - Stop

### วิธีที่ 2: ใช้ Command Line

```bash
# Terminal 1: เริ่ม QEMU
./tools/debug/start_qemu_debug.sh

# Terminal 2: เชื่อมต่อ GDB
gdb kernel/target/x86_64-unknown-none/debug/kernel
(gdb) target remote localhost:1234
(gdb) break kernel_main
(gdb) continue
```

### วิธีที่ 3: Quick Debug Script

```bash
# Debug และหยุดที่ kernel_main
./tools/debug/quick_debug.sh

# Debug และหยุดที่ function อื่น
./tools/debug/quick_debug.sh page_fault_handler
```

## ขั้นตอนที่ 4: ดูข้อมูล

### ใน VS Code:

- **Variables panel** - ดูตัวแปรทั้งหมด
- **Watch panel** - เพิ่มตัวแปรที่ต้องการติดตาม
- **Call Stack panel** - ดู function call stack
- **Debug Console** - รันคำสั่ง GDB

### ใน GDB:

```gdb
# ดู registers
info registers

# ดูตัวแปร
print variable_name

# ดู memory
x/10x 0x100000

# ดู call stack
backtrace

# ดู source code
list
```

## ตัวอย่างการใช้งาน

### ตัวอย่าง 1: Debug Boot Process

1. ตั้ง breakpoint ที่ `kernel_main`
2. กด F5 เพื่อเริ่ม debug
3. เมื่อหยุดที่ `kernel_main` ดูตัวแปร `boot_info`
4. กด F10 เพื่อ step through code

### ตัวอย่าง 2: Debug Page Fault

1. ตั้ง breakpoint ที่ `page_fault_handler`
2. กด F5 เพื่อเริ่ม debug
3. เมื่อเกิด page fault จะหยุดที่ breakpoint
4. ดู register CR2 เพื่อดู faulting address:
   - ใน Debug Console: `-exec info registers cr2`
5. ดู call stack เพื่อหาสาเหตุ

### ตัวอย่าง 3: Debug Scheduler

1. ตั้ง breakpoint ที่ `schedule`
2. กด F5 เพื่อเริ่ม debug
3. เมื่อหยุดที่ `schedule` ดูตัวแปร `current_task`
4. กด F5 เพื่อ continue และดู task switching

## Keyboard Shortcuts

### VS Code Debug Shortcuts:

| Shortcut | Action |
|----------|--------|
| `F5` | Start/Continue |
| `F9` | Toggle Breakpoint |
| `F10` | Step Over |
| `F11` | Step Into |
| `Shift+F11` | Step Out |
| `Shift+F5` | Stop |
| `Cmd+K Cmd+I` | Show Hover |

### GDB Shortcuts:

| Command | Shortcut | Action |
|---------|----------|--------|
| `continue` | `c` | Continue |
| `step` | `s` | Step Into |
| `next` | `n` | Step Over |
| `finish` | `fin` | Step Out |
| `backtrace` | `bt` | Call Stack |
| `print` | `p` | Print Variable |
| `info registers` | `i r` | Show Registers |

## Tips

1. **Build ก่อนทุกครั้ง** - ให้แน่ใจว่า build ใหม่ก่อน debug
2. **ใช้ Conditional Breakpoints** - คลิกขวาที่ breakpoint → Edit Breakpoint
3. **ดู Serial Output** - QEMU จะแสดง kernel logs ใน terminal
4. **ใช้ Watch** - เพิ่มตัวแปรที่ต้องการติดตามใน Watch panel
5. **Save Breakpoints** - VS Code จะจำ breakpoints ไว้ให้

## Troubleshooting

### ปัญหา: Debugger ไม่เชื่อมต่อ

**แก้ไข:**
```bash
# ตรวจสอบว่า QEMU รันอยู่
lsof -i :1234

# ถ้าไม่มี ให้เริ่ม QEMU ใหม่
./tools/debug/start_qemu_debug.sh
```

### ปัญหา: Breakpoint ไม่หยุด

**แก้ไข:**
1. ตรวจสอบว่า build ด้วย debug mode (ไม่ใช่ --release)
2. ลอง rebuild: `make clean && make build && make iso`
3. ตรวจสอบว่า function name ถูกต้อง

### ปัญหา: ไม่เห็น source code

**แก้ไข:**
1. ตรวจสอบว่าอยู่ใน workspace ที่ถูกต้อง
2. ตรวจสอบว่าไฟล์ source ยังอยู่
3. ลอง rebuild

## เอกสารเพิ่มเติม

- [Debugging Guide](DEBUGGING_GUIDE.md) - คู่มือแบบละเอียด
- [Debug Tools](../../tools/debug/README.md) - เครื่องมือ debug
- [Example Sessions](../../tools/debug/example_session.md) - ตัวอย่างการใช้งาน

## สรุป

การ debug MelloOS ง่ายๆ แค่:

1. ✅ ติดตั้ง extensions
2. ✅ Build kernel
3. ✅ กด F5
4. ✅ ตั้ง breakpoints
5. ✅ Debug!

Happy debugging! 🐛🔍
