# MelloOS Debug Tools

เครื่องมือสำหรับ debug MelloOS kernel

## ไฟล์ในโฟลเดอร์นี้

### Scripts

- **start_qemu_debug.sh** - เริ่ม QEMU ในโหมด debug พร้อม GDB server
- **quick_debug.sh** - Debug แบบรวดเร็วด้วย GDB command line
- **gdb_commands.txt** - รายการคำสั่ง GDB ที่มีประโยชน์

## การใช้งาน

### 1. Debug ด้วย VS Code (แนะนำสำหรับผู้เริ่มต้น)

```bash
# Terminal 1: เริ่ม QEMU
./tools/debug/start_qemu_debug.sh

# VS Code: กด F5 เพื่อเริ่ม debug
```

### 2. Quick Debug ด้วย GDB

```bash
# Debug และหยุดที่ kernel_main
./tools/debug/quick_debug.sh

# Debug และหยุดที่ function อื่น
./tools/debug/quick_debug.sh page_fault_handler
./tools/debug/quick_debug.sh schedule
```

### 3. Manual Debug

```bash
# Terminal 1: เริ่ม QEMU
./tools/debug/start_qemu_debug.sh

# Terminal 2: เชื่อมต่อ GDB
gdb kernel/target/x86_64-unknown-none/debug/kernel
(gdb) target remote localhost:1234
(gdb) break kernel_main
(gdb) continue
```

## คำสั่ง GDB ที่ใช้บ่อย

ดูรายละเอียดใน `gdb_commands.txt`

### พื้นฐาน

```gdb
break kernel_main        # ตั้ง breakpoint
continue                 # รันต่อ
step                     # รันทีละบรรทัด (เข้า function)
next                     # รันทีละบรรทัด (ข้าม function)
backtrace                # แสดง call stack
info registers           # แสดง registers
```

### ดู Memory

```gdb
x/10x 0x100000          # แสดง 10 bytes
x/10gx $rsp             # แสดง stack
x/10i $rip              # แสดง instructions
```

### Watchpoints

```gdb
watch variable_name      # หยุดเมื่อตัวแปรเปลี่ยน
watch *0x100000         # หยุดเมื่อ memory เปลี่ยน
```

## Tips

1. **ใช้ VS Code** สำหรับ UI ที่ใช้งานง่าย
2. **ใช้ GDB command line** สำหรับ automation และ scripting
3. **ดู serial output** ใน terminal ที่รัน QEMU
4. **ใช้ QEMU log** (`qemu.log`) เมื่อเกิดปัญหา
5. **Build ใหม่** ทุกครั้งก่อน debug เพื่อให้แน่ใจว่าใช้โค้ดล่าสุด

## Troubleshooting

### QEMU ไม่เริ่ม

```bash
# ตรวจสอบว่า port 1234 ว่าง
lsof -i :1234

# ถ้ามีอะไรใช้อยู่ ให้ kill
pkill -f qemu-system-x86_64
```

### GDB ไม่เชื่อมต่อ

```bash
# ตรวจสอบว่า QEMU รันด้วย -s -S
ps aux | grep qemu

# ลอง connect ใหม่
(gdb) disconnect
(gdb) target remote localhost:1234
```

### ไม่เห็น source code

```bash
# ตรวจสอบว่า build ด้วย debug mode
cd kernel
cargo build  # ไม่ใช่ --release

# ใน GDB ตั้ง source path
(gdb) directory kernel/src
```

## เอกสารเพิ่มเติม

- [Debugging Guide](../../docs/development/DEBUGGING_GUIDE.md) - คู่มือการ debug แบบละเอียด
- [GDB Documentation](https://sourceware.org/gdb/documentation/)
- [QEMU Debugging](https://qemu.readthedocs.io/en/latest/system/gdb.html)

## ตัวอย่างการใช้งาน

### Debug Boot Process

```bash
./tools/debug/quick_debug.sh _start
```

### Debug Page Fault

```bash
./tools/debug/quick_debug.sh page_fault_handler
```

### Debug Scheduler

```bash
./tools/debug/quick_debug.sh schedule
```

### Debug Memory Allocation

```bash
./tools/debug/quick_debug.sh kmalloc
```

Happy debugging! 🐛🔍
