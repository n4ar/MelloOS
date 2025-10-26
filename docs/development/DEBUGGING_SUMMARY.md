# MelloOS Debugging Setup Summary

## 📁 ไฟล์ที่สร้างขึ้น

### VS Code Configuration

1. **`.vscode/launch.json`** - Debug configurations
   - "Debug MelloOS Kernel (GDB)" - Launch และ debug
   - "Attach to QEMU (GDB)" - Attach ไปยัง QEMU ที่รันอยู่

2. **`.vscode/tasks.json`** - Build และ debug tasks
   - `build-kernel` - Build kernel
   - `start-qemu-debug` - เริ่ม QEMU ในโหมด debug
   - `stop-qemu` - หยุด QEMU
   - `cargo-check-kernel` - ตรวจสอบ syntax
   - `run-kernel` - Build และรัน
   - `quick-debug` - Debug แบบรวดเร็ว

3. **`.vscode/settings.json`** - VS Code settings
   - Rust analyzer configuration
   - Editor settings
   - Debug settings
   - File associations

4. **`.vscode/extensions.json`** - แนะนำ extensions
   - rust-analyzer
   - CodeLLDB / C/C++ Tools
   - Assembly support
   - และอื่นๆ

### Debug Tools

5. **`tools/debug/start_qemu_debug.sh`** - เริ่ม QEMU พร้อม GDB server
6. **`tools/debug/quick_debug.sh`** - Debug แบบรวดเร็วด้วย GDB
7. **`tools/debug/gdb_commands.txt`** - รายการคำสั่ง GDB
8. **`tools/debug/README.md`** - คู่มือเครื่องมือ debug
9. **`tools/debug/example_session.md`** - ตัวอย่างการใช้งาน

### GDB Configuration

10. **`.gdbinit`** - GDB initialization file
    - Architecture settings
    - Pretty printing
    - Custom commands

### Documentation

11. **`docs/development/DEBUGGING_GUIDE.md`** - คู่มือ debug แบบละเอียด
12. **`docs/development/DEBUGGING_QUICKSTART.md`** - Quick start guide
13. **`docs/development/DEBUGGING_SUMMARY.md`** - เอกสารนี้

### Updated Files

14. **`README.md`** - เพิ่มลิงก์ไปยังคู่มือ debugging

## 🚀 วิธีใช้งาน

### แบบง่าย (VS Code)

```bash
# 1. เปิด VS Code
code .

# 2. กด F5
# เสร็จแล้ว! 🎉
```

### แบบ Command Line

```bash
# Terminal 1
./tools/debug/start_qemu_debug.sh

# Terminal 2
gdb kernel/target/x86_64-unknown-none/debug/kernel
(gdb) target remote localhost:1234
(gdb) break kernel_main
(gdb) continue
```

### แบบ Quick Debug

```bash
./tools/debug/quick_debug.sh kernel_main
```

## 📚 เอกสารที่ควรอ่าน

1. **เริ่มต้น:** [DEBUGGING_QUICKSTART.md](DEBUGGING_QUICKSTART.md)
2. **ละเอียด:** [DEBUGGING_GUIDE.md](DEBUGGING_GUIDE.md)
3. **ตัวอย่าง:** [example_session.md](../../tools/debug/example_session.md)
4. **เครื่องมือ:** [tools/debug/README.md](../../tools/debug/README.md)

## 🎯 Features

### VS Code Integration
- ✅ One-click debugging (F5)
- ✅ Breakpoints ด้วย UI
- ✅ Variables และ Watch panels
- ✅ Call stack visualization
- ✅ Debug console
- ✅ Integrated terminal

### GDB Support
- ✅ Remote debugging ผ่าน QEMU
- ✅ Breakpoints และ watchpoints
- ✅ Memory inspection
- ✅ Register viewing
- ✅ Assembly debugging
- ✅ Custom commands

### QEMU Integration
- ✅ GDB server (-s flag)
- ✅ Freeze on start (-S flag)
- ✅ Serial output
- ✅ Logging support
- ✅ Multi-core debugging

## 🔧 Keyboard Shortcuts

### VS Code
- `F5` - Start/Continue
- `F9` - Toggle Breakpoint
- `F10` - Step Over
- `F11` - Step Into
- `Shift+F11` - Step Out
- `Shift+F5` - Stop

### GDB
- `c` - Continue
- `s` - Step Into
- `n` - Step Over
- `fin` - Step Out
- `bt` - Backtrace
- `i r` - Info Registers

## 💡 Tips

1. **Build ก่อนทุกครั้ง** - `make clean && make build && make iso`
2. **ใช้ VS Code** สำหรับ UI ที่ใช้งานง่าย
3. **ใช้ GDB** สำหรับ automation
4. **ดู serial output** สำหรับ kernel logs
5. **ใช้ conditional breakpoints** สำหรับ debug ที่ซับซ้อน

## 🐛 Common Issues

### Debugger ไม่เชื่อมต่อ
```bash
lsof -i :1234
pkill -f qemu-system-x86_64
./tools/debug/start_qemu_debug.sh
```

### Breakpoint ไม่หยุด
```bash
make clean && make build && make iso
```

### ไม่เห็น source code
```gdb
(gdb) directory kernel/src
```

## 📖 ตัวอย่างการใช้งาน

### Debug Boot
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

## 🎓 Learning Resources

- [GDB Documentation](https://sourceware.org/gdb/documentation/)
- [QEMU Debugging](https://qemu.readthedocs.io/en/latest/system/gdb.html)
- [VS Code Debugging](https://code.visualstudio.com/docs/editor/debugging)
- [Rust Debugging](https://doc.rust-lang.org/book/appendix-04-useful-development-tools.html)

## ✅ Checklist

เมื่อเริ่มต้น debug ให้ตรวจสอบ:

- [ ] ติดตั้ง VS Code extensions แล้ว
- [ ] Build kernel สำเร็จ
- [ ] QEMU รันได้
- [ ] GDB ติดตั้งแล้ว
- [ ] Port 1234 ว่าง
- [ ] อ่านคู่มือแล้ว

## 🎉 สรุป

ตอนนี้คุณมีเครื่องมือครบสำหรับ debug MelloOS:

1. ✅ VS Code integration
2. ✅ GDB configuration
3. ✅ QEMU scripts
4. ✅ Documentation
5. ✅ Examples

Happy debugging! 🐛🔍
