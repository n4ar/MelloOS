# MelloOS Troubleshooting Guide

This document provides solutions to common issues when building and running MelloOS.

## Build Errors

### Problem: `error: target 'x86_64-unknown-none' not found`

**Solution:** ติดตั้ง Rust target:
```bash
rustup target add x86_64-unknown-none
```

---

### Problem: `cargo: command not found`

**Solution:** ติดตั้ง Rust toolchain และเพิ่ม Cargo ใน PATH:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

---

### Problem: Linker errors เกี่ยวกับ `_start` symbol

**Solution:** ตรวจสอบว่า `linker.ld` ถูกกำหนดใน `.cargo/config.toml` และมี `#[no_mangle]` บน `_start` function

## ISO Creation Errors

### Problem: `xorriso: command not found`

**Solution:** ติดตั้ง xorriso:
- macOS: `brew install xorriso`
- Ubuntu/Debian: `sudo apt install xorriso`
- Arch: `sudo pacman -S xorriso`

---

### Problem: `limine: command not found` หรือ Limine files ไม่พบ

**Solution:** Makefile จะดาวน์โหลด Limine อัตโนมัติ แต่ถ้ามีปัญหา ให้ลอง clone manually:
```bash
git clone https://github.com/limine-bootloader/limine.git --branch=v8.x-binary --depth=1
cd limine
make
```

## QEMU Errors

### Problem: `qemu-system-x86_64: command not found`

**Solution:** ติดตั้ง QEMU:
- macOS: `brew install qemu`
- Ubuntu/Debian: `sudo apt install qemu-system-x86`
- Arch: `sudo pacman -S qemu-full`

---

### Problem: `Could not open '/usr/share/ovmf/OVMF.fd'`

**Solution:** OVMF firmware path อาจแตกต่างกันในแต่ละระบบ แก้ไข `tools/qemu.sh`:

- macOS (Homebrew): `/opt/homebrew/share/edk2-ovmf/x64/OVMF.fd`
- Ubuntu/Debian: `/usr/share/OVMF/OVMF_CODE.fd`
- Arch: `/usr/share/edk2-ovmf/x64/OVMF.fd`

หรือรัน QEMU โดยไม่ใช้ UEFI (legacy BIOS mode):
```bash
qemu-system-x86_64 -M q35 -m 2G -cdrom mellos.iso -boot d
```

---

### Problem: QEMU เปิดแต่หน้าจอดำ

**Solution:** 
1. ตรวจสอบว่า ISO ถูกสร้างสำเร็จ: `ls -lh mellos.iso`
2. ตรวจสอบ serial output ใน terminal
3. ลอง rebuild: `make clean && make run`

---

### Problem: ข้อความไม่แสดงบนหน้าจอ QEMU

**Solution:**
1. ตรวจสอบว่า framebuffer request ใน `main.rs` ถูกต้อง
2. ตรวจสอบว่า Limine configuration มี `PROTOCOL=limine`
3. ลอง rebuild kernel: `make clean && make build && make iso && make run`

## Runtime Errors

### Problem: Kernel panic ทันทีหลังบูต

**Solution:**
1. ตรวจสอบ panic message ใน serial output
2. ตรวจสอบว่า framebuffer response จาก Limine ไม่เป็น null
3. เพิ่ม debug output ใน panic handler

---

### Problem: Kernel หยุดทำงานโดยไม่แสดง error

**Solution:**
1. เพิ่ม serial port debugging
2. ใช้ QEMU monitor: กด `Ctrl+Alt+2` เพื่อเข้า monitor mode
3. ตรวจสอบ CPU state ด้วย `info registers` ใน QEMU monitor

---

### Problem: Kernel Hangs After `sti`

**Cause:** IDT not properly initialized or timer not configured

**Solution:**
```rust
// Ensure proper initialization order
init_idt();           // First
remap_pic();          // Second
init_pit_timer(100);  // Third
core::arch::asm!("sti");  // Finally
```

---

### Problem: Triple Fault / Reboot Loop

**Cause:** Stack overflow or invalid memory access

**Solution:**
```rust
// Add stack validation
if task.context.rsp == 0 {
    panic!("Task has null RSP!");
}

// Check stack bounds
let stack_bottom = task.stack as u64;
let stack_top = stack_bottom + 8192;
if task.context.rsp < stack_bottom || task.context.rsp >= stack_top {
    panic!("RSP outside stack bounds!");
}
```

---

### Problem: Out of Memory

**Cause:** Too many allocations or memory leak

**Solution:**
```rust
// Check available memory
let free_mb = pmm.free_memory_mb();
if free_mb < 10 {
    serial_println!("WARNING: Low memory! {} MB free", free_mb);
}

// Always free allocated memory
let ptr = kmalloc(1024);
// ... use ptr ...
kfree(ptr, 1024);  // Don't forget!
```

---

### Problem: Tasks Not Switching

**Cause:** Timer not firing or runqueue empty

**Solution:**
```rust
// Check timer ticks
let ticks = get_tick_count();
serial_println!("Timer ticks: {}", ticks);

// Check runqueue
let sched = SCHED.lock();
if sched.runqueue.is_empty() {
    serial_println!("WARNING: Runqueue is empty!");
}
```

## Debugging Tips

### Enable Serial Debugging

Add debug output to track execution:

```rust
serial_println!("[DEBUG] Entering function: {}", function_name);
serial_println!("[DEBUG] Variable value: {}", value);
serial_println!("[DEBUG] Address: 0x{:x}", addr);
```

### Use QEMU Monitor

Start QEMU with monitor:
```bash
qemu-system-x86_64 -monitor stdio -cdrom mellos.iso
```

Useful commands:
```
info registers    # Show CPU state
info mem          # Show memory mappings
info tlb          # Show TLB entries
info pic          # Show PIC state
x /10x 0x1000     # Examine memory
```

### Check Build Artifacts

```bash
# Verify kernel binary
file kernel/target/x86_64-unknown-none/release/mellos-kernel

# Check ISO contents
xorriso -indev mellos.iso -find

# Disassemble kernel
objdump -d kernel/target/x86_64-unknown-none/release/mellos-kernel | less
```

## Getting Help

If you can't resolve an issue:

1. Check the [documentation](../docs/)
2. Review [specifications](.kiro/specs/)
3. Search existing [GitHub issues](https://github.com/<your-repo>/issues)
4. Create a new issue with:
   - Detailed problem description
   - Steps to reproduce
   - Error messages and logs
   - System information (OS, versions)
   - What you've already tried
