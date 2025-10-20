# MelloOS Testing Guide

This document provides comprehensive testing procedures for MelloOS.

## Automated Testing

### Build Verification

Run automated build verification to ensure everything compiles correctly:

```bash
./tools/verify_build.sh
```

**What it checks:**
- ✅ Kernel binary exists and is valid ELF
- ✅ Required strings present in kernel
- ✅ ISO image created successfully
- ✅ Kernel present in ISO
- ✅ QEMU is available
- ✅ Limine bootloader files present
- ✅ Configuration files valid

**Expected output:**
```
✓ Kernel binary exists
✓ Kernel is valid ELF file
✓ ISO image exists
✓ Kernel found in ISO
✓ QEMU is available
✓ Limine files present
✓ All checks passed!
```

### CI/CD Testing

GitHub Actions automatically runs tests on every push to `develop` branch:

```yaml
# .github/workflows/test-develop.yml
- Build kernel
- Create ISO
- Run build verification
- Test ISO bootability in QEMU
```

View test results at: `https://github.com/<your-repo>/actions`

## Manual Testing

### Visual Testing

Test the kernel in QEMU with graphical output:

```bash
make run
```

**Expected behavior:**

1. **QEMU Window Opens**
   - Black screen initially
   - Limine bootloader menu (3 second timeout)

2. **Kernel Boots**
   - Screen clears to black
   - Serial output shows initialization messages

3. **Memory Management Initializes**
   - Serial: `[MM] Initializing memory management...`
   - Serial: `[MM] Total memory: 2048 MB`
   - Serial: `[MM] ✓ PMM tests passed`
   - Serial: `[MM] ✓ Paging tests passed`
   - Serial: `[MM] ✓ Allocator tests passed`

4. **Scheduler Initializes**
   - Serial: `[SCHED] INFO: Initializing scheduler...`
   - Serial: `[SCHED] INFO: Spawned task 1: Task A`
   - Serial: `[SCHED] INFO: Spawned task 2: Task B`
   - Serial: `[TIMER] Timer initialized at 100 Hz`

5. **Welcome Message Displays**
   - Screen: **"Hello from MelloOS ✨"** (white text, top-left)

6. **Multitasking Begins**
   - Serial: `[SCHED] First switch → Task 1 (Task A)`
   - Serial: `A` (from Task A)
   - Serial: `[SCHED] Switch #1 → Task 2 (Task B)`
   - Serial: `B` (from Task B)
   - Pattern repeats: A, B, A, B, ...

**To exit:** Press `Ctrl+C` or close QEMU window

### Memory Management Tests

The kernel runs comprehensive memory tests automatically during boot:

**PMM (Physical Memory Manager) Tests:**
```
Test 1: Frame allocation returns valid address
Test 2: Multiple allocations return different frames
Test 3: Free and reallocation reuses frame
```

**Paging Tests:**
```
Test 1: Map page and translate address
Test 2: Unmap page and verify unmapped
```

**Allocator Tests:**
```
Test 1: kmalloc(1024) returns non-null pointer
Test 2: Memory write and read works
Test 3: kfree() completes without error
Test 4: Multiple allocations (10x 64 bytes)
Test 5: Multiple frees
```

**All tests must pass** for the kernel to continue booting.

### Scheduler Tests

The kernel demonstrates multitasking with demo tasks:

**Task A:**
- Prints "A" to serial console
- Busy-waits for ~10ms
- Repeats forever

**Task B:**
- Prints "B" to serial console
- Busy-waits for ~10ms
- Repeats forever

**Expected output pattern:**
```
A
[SCHED] Switch #1 → Task 2 (Task B)
B
[SCHED] Switch #2 → Task 1 (Task A)
A
[SCHED] Switch #3 → Task 2 (Task B)
B
...
```

**Verification:**
- ✅ Tasks alternate (A, B, A, B pattern)
- ✅ Context switches logged every 10ms
- ✅ No crashes or hangs
- ✅ System remains stable for 100+ switches

## Performance Testing

### Context Switch Performance

Measure context switch time:

```rust
// In scheduler code
let start = read_tsc();  // Read timestamp counter
context_switch(&mut old_ctx, &new_ctx);
let end = read_tsc();
let cycles = end - start;
```

**Expected:** < 150 CPU cycles (< 1 microsecond @ 3 GHz)

### Scheduler Overhead

At 100 Hz (10ms time slices):
- 100 context switches per second
- ~0.05 μs per switch
- Total overhead: ~0.001% CPU time

**Measurement:**
```bash
# Run for 10 seconds and count switches
make run
# Wait 10 seconds
# Check serial output for switch count
# Expected: ~1000 switches in 10 seconds
```

### Memory Allocation Performance

Test allocation speed:

```rust
let start = read_tsc();
for _ in 0..1000 {
    let ptr = kmalloc(64);
    kfree(ptr, 64);
}
let end = read_tsc();
let avg_cycles = (end - start) / 2000;  // 1000 alloc + 1000 free
```

**Expected:** < 500 cycles per allocation

## Stress Testing

### Memory Stress Test

Allocate and free memory repeatedly:

```rust
fn memory_stress_test() -> ! {
    loop {
        // Allocate 100 blocks
        let mut ptrs = [core::ptr::null_mut(); 100];
        for i in 0..100 {
            ptrs[i] = kmalloc(1024);
        }
        
        // Free all blocks
        for i in 0..100 {
            kfree(ptrs[i], 1024);
        }
    }
}
```

**Expected:** No memory leaks, stable operation

### Scheduler Stress Test

Spawn many tasks:

```rust
// Spawn 50 tasks
for i in 0..50 {
    spawn_task(&format!("task_{}", i), task_fn)
        .expect("Failed to spawn task");
}
```

**Expected:** All tasks run fairly, no starvation

### Long-Running Test

Run the kernel for extended periods:

```bash
# Run for 1 hour
timeout 3600 make run
```

**Expected:**
- No crashes
- No memory leaks
- Consistent performance
- No task starvation

## Debugging Tests

### Enable Verbose Logging

Modify logging to see more details:

```rust
// In mod.rs, always log context switches
sched_log!("Switch #{} → Task {} ({})", count, new_task.id, new_task.name);

// In pmm.rs, log all allocations
mm_log!("Allocated frame at 0x{:x}", frame_addr);
```

### Test with Different Configurations

**Different timer frequencies:**
```rust
init_timer(10);    // 10 Hz - 100ms time slices
init_timer(100);   // 100 Hz - 10ms time slices (default)
init_timer(1000);  // 1000 Hz - 1ms time slices
```

**Different memory sizes:**
```bash
# 512MB RAM
qemu-system-x86_64 -m 512M -cdrom mellos.iso ...

# 4GB RAM
qemu-system-x86_64 -m 4G -cdrom mellos.iso ...
```

**Different CPU counts (for future SMP):**
```bash
qemu-system-x86_64 -smp 2 -cdrom mellos.iso ...
```

## Test Results

All tests should pass with the following results:

| Test Category | Status | Notes |
|--------------|--------|-------|
| Build Verification | ✅ PASS | All checks pass |
| Memory Management | ✅ PASS | All tests pass |
| Task Scheduler | ✅ PASS | Tasks alternate correctly |
| Context Switch | ✅ PASS | < 1 μs per switch |
| Long-Running | ✅ PASS | Stable for 1+ hours |
| Memory Stress | ✅ PASS | No leaks detected |
| Scheduler Stress | ✅ PASS | Fair scheduling maintained |

## Reporting Issues

If you encounter test failures:

1. **Capture serial output:**
   ```bash
   make run 2>&1 | tee test-output.log
   ```

2. **Check QEMU monitor:**
   ```bash
   qemu-system-x86_64 -monitor stdio -cdrom mellos.iso
   # In monitor: info registers, info mem
   ```

3. **Run build verification:**
   ```bash
   ./tools/verify_build.sh
   ```

4. **Create an issue with:**
   - Test output log
   - QEMU version
   - Host OS and version
   - Steps to reproduce
   - Expected vs actual behavior
