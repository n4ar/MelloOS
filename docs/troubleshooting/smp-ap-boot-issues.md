# SMP AP Boot Issues and Solutions

This document describes the critical issues encountered during SMP (Symmetric Multi-Processing) 
implementation in MelloOS and their solutions.

## Overview

During the implementation of multi-core support, we encountered several critical bugs that 
prevented Application Processors (APs) from coming online. This document details each issue, 
the debugging process, and the final solution.

## Issue 1: LAPIC Address Corruption in Trampoline

### Symptoms
- AP reached the Rust entry point (`ap_entry64`)
- GDT/TSS initialization succeeded
- LAPIC address was corrupted: `0xFEE003F8` instead of `0xFEE00000`
- The value `0x3F8` is the serial port I/O address

### Root Cause
The AP trampoline code (`boot_ap.S`) was loading the LAPIC address into `%rdx` register,
but then immediately overwrote the lower 16 bits (`%dx`) with the serial port address 
for debug output:

```asm
/* Load arguments for Rust function */
movq    (TRAMPOLINE_LAPIC_ADDR), %rdx /* Third arg: lapic_address */

/* Debug: Write 'R' to serial (about to jump) */
movb    $'R', %al
movw    $0x3F8, %dx              /* BUG: Overwrites lower 16 bits of %rdx! */
outb    %al, %dx
```

This corrupted `0xFEE00000` → `0xFEE003F8`.

### Solution
Save and restore `%rdx` around all debug output operations:

```asm
/* Debug: Write 'R' to serial (about to jump) */
pushq   %rax
pushq   %rdx                     /* Save %rdx */
movb    $'R', %al
movw    $0x3F8, %dx
outb    %al, %dx
popq    %rdx                     /* Restore %rdx */
popq    %rax
```


### Files Modified
- `kernel/src/arch/x86_64/smp/boot_ap.S`: Added `pushq %rdx` / `popq %rdx` around all 
  serial debug output operations

### Verification
After the fix, the debug output showed:
```
[SMP] AP entry: cpu_id=1, apic_id=1, lapic=0xFEE00000
```

The LAPIC address is now correct!

---

## Issue 2: CPU ID Corruption in Syscall Initialization

### Symptoms
- Syscall MSR initialization showed garbage CPU ID: `17294103305076670291`
- This occurred even though the correct CPU ID (1) was passed to `ap_entry64`
- GDT/TSS initialization worked correctly with CPU ID 1

### Root Cause
The `init_syscall_msrs()` function was trying to read the CPU ID from per-CPU data 
using `percpu_current().id`, but this reads from the GS.BASE MSR. At the point where 
syscall initialization was called (during GDT setup), the GS.BASE might not have been 
properly configured yet, resulting in reading garbage memory.

Original code in `kernel/src/arch/x86_64/syscall/mod.rs`:
```rust
pub unsafe fn init_syscall_msrs() {
    let cpu_id = crate::arch::x86_64::smp::percpu::percpu_current().id;  // BUG!
    serial_println!("[SYSCALL] Initializing syscall MSRs for CPU {}", cpu_id);
    // ...
}
```

### Solution
Pass the CPU ID as a parameter instead of reading it from per-CPU data:

```rust
pub unsafe fn init_syscall_msrs(cpu_id: usize) {
    serial_println!("[SYSCALL] Initializing syscall MSRs for CPU {}", cpu_id);
    // ...
}
```

Update the call site in `kernel/src/arch/x86_64/gdt.rs`:
```rust
// Initialize syscall MSRs for fast syscall support
crate::arch::x86_64::syscall::init_syscall_msrs(cpu_id);
```


### Files Modified
- `kernel/src/arch/x86_64/syscall/mod.rs`: Changed `init_syscall_msrs()` to accept `cpu_id` parameter
- `kernel/src/arch/x86_64/gdt.rs`: Updated call to pass `cpu_id`

### Verification
After the fix:
```
[SYSCALL] Initializing syscall MSRs for CPU 1
[SYSCALL] CPU 1 EFER.SCE enabled: 0xd01
[SYSCALL] CPU 1 STAR configured: 0x38002800000000
```

CPU ID is now correct!

---

## Issue 3: CPU_COUNT Synchronization Between Modules

### Symptoms
- AP came online successfully: `[SMP] AP#1 came online successfully`
- SMP reported: `[SMP] SMP initialization complete: 2 CPUs online`
- But scheduler still enqueued all tasks to CPU 0:
  ```
  [SCHED] Enqueued task 12 to CPU 0 (runqueue size: 12)
  [SCHED] Enqueued task 13 to CPU 0 (runqueue size: 13)
  ```
- Kernel reported: `[KERNEL] CPU count: 1` (should be 2)

### Root Cause
There were **two separate** `CPU_COUNT` static variables:

1. In `kernel/src/arch/x86_64/smp/mod.rs`:
   ```rust
   static CPU_COUNT: AtomicUsize = AtomicUsize::new(0);
   ```
   This was incremented when APs came online.

2. In `kernel/src/sched/mod.rs`:
   ```rust
   static CPU_COUNT: AtomicUsize = AtomicUsize::new(1);
   ```
   This was never updated!

The scheduler was using its own `CPU_COUNT` which remained at 1, so it thought there 
was only one CPU available.


### Solution
Remove the duplicate `CPU_COUNT` from the scheduler and use the SMP module's version:

1. In `kernel/src/sched/mod.rs`, replace the static with a function:
   ```rust
   /// Get the number of online CPUs from SMP module
   fn get_cpu_count() -> usize {
       crate::arch::x86_64::smp::get_cpu_count()
   }
   ```

2. Replace all `CPU_COUNT.load(Ordering::Relaxed)` calls with `get_cpu_count()`

3. Remove the `set_cpu_count()` function (no longer needed)

### Additional Fix: BSP Not Counted
The SMP module's `CPU_COUNT` started at 0 and only counted APs, not the BSP:
```rust
static CPU_COUNT: AtomicUsize = AtomicUsize::new(0);  // BUG: BSP not counted!
```

Fixed by starting at 1 (for BSP):
```rust
static CPU_COUNT: AtomicUsize = AtomicUsize::new(1);  // BSP is CPU 0
```

### Files Modified
- `kernel/src/sched/mod.rs`: Removed duplicate `CPU_COUNT`, added `get_cpu_count()` wrapper
- `kernel/src/arch/x86_64/smp/mod.rs`: Changed initial value from 0 to 1

### Verification
After the fix:
```
[SMP] SMP initialization complete: 2 CPUs online
[KERNEL] CPU count: 2
[SCHED] Enqueued task 2 to CPU 1 (runqueue size: 1)
[SCHED] send RESCHED IPI → core1
[SCHED] Enqueued task 3 to CPU 0 (runqueue size: 2)
[SCHED] Enqueued task 4 to CPU 1 (runqueue size: 2)
```

Tasks are now distributed across both CPUs! 🎉


---

## Debugging Techniques Used

### 1. Serial Port Debug Output
Added debug characters at critical points in the trampoline:
```asm
movb    $'R', %al
movw    $0x3F8, %dx
outb    %al, %dx
```

This helped identify exactly where the AP was in the boot process.

### 2. Debug Logging with Values
Added detailed logging to show actual values:
```rust
serial_println!("[SMP] AP entry: cpu_id={}, apic_id={}, lapic=0x{:X}", 
               cpu_id, apic_id, lapic_address);
```

This immediately revealed the LAPIC address corruption.

### 3. Trampoline Data Verification
Added logging to show what was written to the trampoline:
```rust
serial_println!("[SMP] DEBUG: Wrote to trampoline - cpu_id={}, apic_id={}, lapic=0x{:X}", 
               cpu_id, cpu_info.apic_id, madt_info.lapic_address);
```

This confirmed the data was correct before the AP read it.

### 4. Comparing Expected vs Actual
When we saw `0xFEE003F8` instead of `0xFEE00000`, the `0x3F8` suffix immediately 
suggested serial port interference.

### 5. Grep for Duplicate Symbols
Used `grep -r "CPU_COUNT" kernel/src/` to find all instances of the variable,
revealing the duplicate definition.


---

## Lessons Learned

### 1. Register Preservation in Assembly
**Always save and restore registers** when using them for temporary operations, 
especially in low-level boot code where registers hold critical arguments.

### 2. Avoid Reading Uninitialized State
Don't read from per-CPU data structures (via GS.BASE) until they're fully initialized.
Pass values explicitly as parameters instead.

### 3. Single Source of Truth
Avoid duplicate global state. Use a single authoritative source and provide accessor 
functions if needed. In our case, the SMP module should be the only place that tracks 
CPU count.

### 4. Initialize Counters Correctly
When counting resources, make sure to include all of them. The BSP is a CPU too!

### 5. Debug Early, Debug Often
Add debug output at every critical step. The serial port is your best friend when 
debugging low-level code.

---

## Final Results

After fixing all three issues:

✅ **AP Boot Success:**
```
[SMP] AP#1 came online successfully
[SMP] SMP initialization complete: 2 CPUs online
```

✅ **Load Balancing Working:**
```
[SCHED] Enqueued task 2 to CPU 1 (runqueue size: 1)
[SCHED] Enqueued task 4 to CPU 1 (runqueue size: 2)
[SCHED] Enqueued task 6 to CPU 1 (runqueue size: 3)
```

✅ **Cross-CPU Process Execution:**
```
[SYSCALL][cpu0 pid=11 rip=0x400200] SYS_FORK (2)
[SYSCALL][cpu1 pid=12 rip=0x400220] SYS_GETPID (6)
```

✅ **IPI Communication:**
```
[SCHED] send RESCHED IPI → core1
```

**MelloOS now has true multi-core support!** 🚀

