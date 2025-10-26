# ตัวอย่าง Debug Session

## Scenario 1: Debug Kernel Boot

### เป้าหมาย
ต้องการดูว่า kernel boot อย่างไรและ parameters อะไรที่ได้รับจาก bootloader

### ขั้นตอน

1. **เริ่ม QEMU:**
```bash
./tools/debug/start_qemu_debug.sh
```

2. **เชื่อมต่อ GDB:**
```bash
gdb kernel/target/x86_64-unknown-none/debug/kernel
```

3. **ตั้ง breakpoints:**
```gdb
(gdb) target remote localhost:1234
(gdb) break _start
(gdb) break kernel_main
(gdb) continue
```

4. **ดู boot parameters:**
```gdb
# เมื่อหยุดที่ _start
(gdb) info registers rdi    # Limine boot info pointer
(gdb) x/10gx $rdi          # ดู boot info structure

# รันต่อไป kernel_main
(gdb) continue

# ดู registers ที่ kernel_main
(gdb) info registers
(gdb) backtrace
```

### ผลลัพธ์ที่คาดหวัง
- เห็น boot info pointer
- เห็น memory map
- เห็น kernel entry point

---

## Scenario 2: Debug Page Fault

### เป้าหมาย
เกิด page fault และต้องการหาสาเหตุ

### ขั้นตอน

1. **ตั้ง breakpoint ที่ page fault handler:**
```gdb
(gdb) target remote localhost:1234
(gdb) break page_fault_handler
(gdb) continue
```

2. **เมื่อเกิด page fault:**
```gdb
# ดู faulting address
(gdb) info registers cr2

# ดู error code
(gdb) info registers

# ดู call stack
(gdb) backtrace

# ดู page table
(gdb) info registers cr3
(gdb) x/10gx $cr3
```

3. **วิเคราะห์:**
```gdb
# ดู instruction ที่เกิด fault
(gdb) x/10i $rip

# ดู memory รอบๆ faulting address
(gdb) x/10gx $cr2-0x20

# ดู stack
(gdb) x/20gx $rsp
```

### สาเหตุที่พบบ่อย
- NULL pointer dereference (CR2 = 0x0)
- Stack overflow (CR2 ใกล้ stack boundary)
- Invalid page table entry
- Permission violation (user accessing kernel memory)

---

## Scenario 3: Debug Scheduler

### เป้าหมาย
ต้องการดูว่า scheduler ทำงานอย่างไรและ task switching เป็นอย่างไร

### ขั้นตอน

1. **ตั้ง breakpoints:**
```gdb
(gdb) target remote localhost:1234
(gdb) break schedule
(gdb) break context_switch
(gdb) continue
```

2. **ดู current task:**
```gdb
# เมื่อหยุดที่ schedule
(gdb) print current_task
(gdb) print *current_task

# ดู task state
(gdb) print current_task->state
(gdb) print current_task->pid
(gdb) print current_task->priority
```

3. **ดู task switching:**
```gdb
# ตั้ง watchpoint ที่ current_task
(gdb) watch current_task

# รันต่อและดูว่า task เปลี่ยนเมื่อไหร่
(gdb) continue

# เมื่อ task เปลี่ยน
(gdb) print current_task
(gdb) backtrace
```

### ข้อมูลที่น่าสนใจ
- Task state transitions
- Context switch frequency
- CPU time per task
- Priority scheduling

---

## Scenario 4: Debug Memory Allocation

### เป้าหมาย
ต้องการดูว่า memory allocation ทำงานถูกต้องหรือไม่

### ขั้นตอน

1. **ตั้ง breakpoints:**
```gdb
(gdb) target remote localhost:1234
(gdb) break kmalloc
(gdb) break kfree
(gdb) continue
```

2. **ดู allocation:**
```gdb
# เมื่อหยุดที่ kmalloc
(gdb) print size          # ขนาดที่ขอ
(gdb) finish              # รันจนจบ function
(gdb) print $rax          # address ที่ได้

# ดู memory ที่ allocate
(gdb) x/10gx $rax
```

3. **ตรวจสอบ memory leak:**
```gdb
# ตั้ง conditional breakpoint
(gdb) break kmalloc if size > 1024

# นับจำนวน allocations
(gdb) set $alloc_count = 0
(gdb) commands
> set $alloc_count = $alloc_count + 1
> print $alloc_count
> continue
> end
```

### ปัญหาที่พบบ่อย
- Memory leak (alloc มากกว่า free)
- Double free
- Use after free
- Heap corruption

---

## Scenario 5: Debug Interrupt Handler

### เป้าหมาย
ต้องการดูว่า interrupt handler ทำงานถูกต้องหรือไม่

### ขั้นตอน

1. **ตั้ง breakpoints:**
```gdb
(gdb) target remote localhost:1234
(gdb) break timer_interrupt_handler
(gdb) break keyboard_interrupt_handler
(gdb) continue
```

2. **ดู interrupt state:**
```gdb
# เมื่อหยุดที่ interrupt handler
(gdb) info registers rflags    # ดู IF flag
(gdb) backtrace

# ดู interrupt frame
(gdb) x/10gx $rsp
```

3. **ดู interrupt frequency:**
```gdb
# นับจำนวน interrupts
(gdb) set $int_count = 0
(gdb) commands
> set $int_count = $int_count + 1
> if $int_count % 100 == 0
>   print $int_count
> end
> continue
> end
```

### ข้อมูลที่น่าสนใจ
- Interrupt frequency
- Handler execution time
- Nested interrupts
- Interrupt latency

---

## Scenario 6: Debug Userspace Program

### เป้าหมาย
ต้องการ debug userspace program (เช่น shell)

### ขั้นตอน

1. **Build userspace program ด้วย debug symbols:**
```bash
cd kernel/userspace/mello-sh
cargo build
```

2. **ตั้ง breakpoints:**
```gdb
(gdb) target remote localhost:1234

# ตั้ง breakpoint ที่ syscall entry
(gdb) break syscall_entry

# ตั้ง breakpoint ที่ userspace function (ถ้ารู้ address)
(gdb) break *0x400000

(gdb) continue
```

3. **ดู userspace state:**
```gdb
# ดู user registers
(gdb) info registers

# ดู user stack
(gdb) x/20gx $rsp

# ดู user code
(gdb) x/10i $rip
```

### ความท้าทาย
- Context switching ระหว่าง kernel และ user mode
- Symbol resolution สำหรับ userspace code
- Multiple processes

---

## Tips สำหรับ Debug ที่มีประสิทธิภาพ

### 1. ใช้ Conditional Breakpoints
```gdb
break kmalloc if size > 4096
break schedule if current_task->pid == 42
```

### 2. ใช้ Commands
```gdb
break page_fault_handler
commands
  info registers cr2
  backtrace
  continue
end
```

### 3. ใช้ Watchpoints
```gdb
watch some_global_variable
watch *0x100000
```

### 4. Save และ Restore Session
```gdb
# Save breakpoints
save breakpoints my_breakpoints.txt

# Restore
source my_breakpoints.txt
```

### 5. ใช้ Python Scripting
```gdb
python
def print_task_info():
    # Custom Python code to print task info
    pass
end
```

---

## Common Issues และวิธีแก้

### Issue: Breakpoint ไม่หยุด

**สาเหตุ:**
- Function ถูก inline
- Symbol ไม่ถูกต้อง
- Code ไม่ถูกรัน

**วิธีแก้:**
```gdb
# ใช้ address แทน symbol
info symbol kernel_main
break *<address>

# ตรวจสอบว่า code ถูกรัน
x/10i <address>
```

### Issue: Source code ไม่แสดง

**สาเหตุ:**
- Build ไม่มี debug symbols
- Source path ไม่ถูกต้อง

**วิธีแก้:**
```gdb
# ตั้ง source path
directory kernel/src

# ตรวจสอบ debug symbols
info sources
```

### Issue: QEMU crash

**สาเหตุ:**
- Kernel panic
- Invalid instruction
- Triple fault

**วิธีแก้:**
```bash
# ดู QEMU log
cat qemu.log

# รัน QEMU ด้วย verbose logging
qemu-system-x86_64 ... -d int,cpu_reset,guest_errors
```

---

## สรุป

การ debug OS kernel ต้องใช้:
1. **Patience** - ใช้เวลานานกว่า debug แอปพลิเคชันทั่วไป
2. **Understanding** - เข้าใจ hardware และ low-level concepts
3. **Tools** - ใช้เครื่องมือที่เหมาะสม (GDB, QEMU)
4. **Methodology** - มีแนวทางที่เป็นระบบ

Happy debugging! 🐛🔍
