---
inclusion: always
---

# MelloOS Debugging Guide - MANDATORY

## Overview

This guide defines the debugging workflow, tools, and practices for MelloOS development. Follow these guidelines when investigating issues, crashes, or unexpected behavior.

## Debugging Tools Available

### 1. Serial Output (Primary Debug Method)

**Default behavior:** All kernel logs output to COM1 (serial port)

```bash
# QEMU automatically captures serial output to terminal
./tools/qemu.sh
```

**Key points:**
- Use `serial_println!()` macro for kernel debug output
- Serial output persists even during crashes
- Available before and after system initialization
- Not affected by framebuffer issues

### 2. QEMU Monitor

**Access:** Press `Ctrl+Alt+2` in QEMU window (or `Ctrl+Alt+1` to return to console)

**Useful commands:**
- `info registers` - View CPU register state
- `info mem` - Display memory mappings
- `info tlb` - Show TLB entries
- `x/10i $rip` - Disassemble at instruction pointer
- `x/10gx $rsp` - Examine stack
- `system_reset` - Restart VM
- `quit` - Exit QEMU

### 3. GDB Debugging

**Setup:** Use provided debugging scripts in `tools/debug/`

```bash
# Terminal 1: Start QEMU with GDB server
./tools/debug/start_qemu_debug.sh

# Terminal 2: Connect GDB
gdb kernel/target/x86_64-mellos/debug/mellos_kernel
(gdb) target remote :1234
(gdb) source tools/debug/gdb_commands.txt
```

**Key GDB commands:**
- `break function_name` - Set breakpoint
- `continue` - Resume execution
- `step` / `next` - Step through code
- `backtrace` - Show call stack
- `info threads` - List all CPUs/threads
- `thread N` - Switch to CPU N
- `print variable` - Inspect variable

**SMP debugging:**
- Use `tools/debug/gdb-smp.gdb` for multi-core debugging
- Each CPU appears as a separate thread in GDB
- Use `info threads` to see all CPUs
- Use `thread N` to switch between CPUs

### 4. Userspace Utilities

**dmesg** - View kernel log buffer:
```bash
# In MelloOS shell
dmesg
```

**lsdev** - List devices:
```bash
lsdev
```

**diskinfo** - Display block device information:
```bash
diskinfo /dev/vda
```

**irq_test** - Test interrupt distribution:
```bash
irq_test
```

## Common Debugging Scenarios

### Scenario 1: Kernel Panic or Triple Fault

**Symptoms:** System crashes, reboots, or hangs

**Debug steps:**
1. Check serial output for panic message
2. Note the instruction pointer (RIP) and fault address
3. Use GDB to set breakpoint before crash location
4. Examine register state and stack trace
5. Check for:
   - Invalid memory access
   - Stack overflow
   - Uninitialized data
   - Lock ordering violations

**Tools:**
- Serial output (primary)
- GDB with breakpoints
- `tools/debug/analyze-triple-fault.sh` for automated analysis

### Scenario 2: SMP Issues (Multi-core)

**Symptoms:** Deadlocks, race conditions, inconsistent state

**Debug steps:**
1. Enable SMP-specific logging in affected subsystem
2. Use `info threads` in GDB to check all CPU states
3. Check lock ordering (see `kernel/src/sync/lock_ordering.rs`)
4. Verify per-CPU data isolation
5. Look for missing memory barriers or atomic operations

**Tools:**
- GDB multi-threaded debugging
- `tools/debug/gdb-smp.gdb`
- Serial output with CPU ID in logs

**Common issues:**
- Deadlock from incorrect lock ordering
- Race conditions in shared data structures
- TLB coherency issues (missing shootdowns)
- Interrupt handling on wrong CPU

### Scenario 3: Memory Corruption

**Symptoms:** Random crashes, data corruption, use-after-free

**Debug steps:**
1. Enable memory allocator debugging (if available)
2. Check for buffer overflows
3. Verify page table mappings
4. Look for double-free or use-after-free
5. Check alignment requirements

**Tools:**
- GDB memory examination (`x/` commands)
- QEMU monitor (`info mem`, `info tlb`)
- Memory security checks in `kernel/src/mm/security.rs`

### Scenario 4: Filesystem Issues

**Symptoms:** File operations fail, data loss, corruption

**Debug steps:**
1. Run filesystem tests: `./tools/testing/run_filesystem_tests.sh`
2. Check VFS layer logs
3. Verify block device operations
4. Test with `fs_test` userspace utility
5. Check cache coherency (buffer cache, page cache)

**Tools:**
- `fs_test` userspace program
- `diskinfo` for device status
- Test suite in `tests/fs_*.rs`
- `tools/testing/test_mfs_disk.sh`

### Scenario 5: Userspace Program Crashes

**Symptoms:** User program exits unexpectedly, segfault

**Debug steps:**
1. Check syscall return values
2. Verify ELF loading was successful
3. Check user memory mappings
4. Look for invalid syscall arguments
5. Verify signal handling

**Tools:**
- Serial output for syscall tracing
- GDB (can debug userspace with symbols)
- `ps` command to check process state

### Scenario 6: Device Driver Issues

**Symptoms:** Device not detected, I/O failures, interrupts not firing

**Debug steps:**
1. Check device enumeration: `lsdev`
2. Verify IRQ routing with `irq_test`
3. Check driver registration in device tree
4. Test with device-specific utilities (`kbd_test`, `serial_test`, `disk_bench`)
5. Verify MMIO/port I/O access

**Tools:**
- `lsdev` - Device enumeration
- `irq_test` - Interrupt testing
- Device-specific test programs
- `tools/testing/test_drivers.sh`

## Debugging Workflow

### Standard Debug Cycle

1. **Reproduce the issue**
   - Use consistent test case
   - Note exact steps to trigger
   - Check if issue is deterministic or intermittent

2. **Gather information**
   - Capture serial output
   - Note error messages and addresses
   - Check system state before crash

3. **Form hypothesis**
   - What subsystem is involved?
   - What could cause this behavior?
   - Is it SMP-related?

4. **Test hypothesis**
   - Add targeted logging
   - Use GDB breakpoints
   - Modify code to isolate issue

5. **Fix and verify**
   - Implement fix
   - Run `cargo check` immediately
   - Test with full build cycle
   - Run relevant test suite

### Adding Debug Output

**Kernel code:**
```rust
use crate::serial_println;

serial_println!("Debug: value = {}, state = {:?}", value, state);
```

**Conditional debugging:**
```rust
#[cfg(feature = "debug-subsystem")]
serial_println!("Detailed debug info: {:?}", data);
```

**Userspace code:**
```rust
// Use syscall wrapper for write to stderr
syscall::write(2, b"Debug message\n");
```

## Performance Debugging

### Identifying Bottlenecks

1. **Use kernel metrics system** (`kernel/src/metrics.rs`)
2. **Run benchmarks:**
   - `./tools/testing/benchmark_mellos.sh`
   - `./tools/testing/benchmark_performance.sh`
   - `benches/fork_exec_p95.rs`
   - `benches/fs_seq_rand.rs`

3. **Profile with QEMU:**
   - Use `-d` flags for instruction tracing
   - Monitor interrupt frequency
   - Check lock contention

### Performance Issues

**Symptoms:** Slow operations, high latency, poor throughput

**Common causes:**
- Excessive locking (check lock contention)
- Cache misses (check data locality)
- Unnecessary syscalls (batch operations)
- Inefficient algorithms (profile hot paths)

## Integration with Development Workflow

### After Code Changes

1. **Run cargo check** (see cargo-check-policy.md)
2. **Build and test:**
   ```bash
   make clean && make build && make iso
   ./tools/qemu.sh
   ```
3. **Monitor serial output** for warnings/errors
4. **Run relevant tests** from `tools/testing/`

### Before Committing

1. All tests pass
2. No new warnings in serial output
3. System boots successfully
4. Relevant integration tests pass

## Debugging Best Practices

### DO:
- ✅ Add serial output liberally during development
- ✅ Use GDB for complex issues
- ✅ Test on clean build (`make clean`)
- ✅ Check lock ordering in SMP code
- ✅ Verify assumptions with assertions
- ✅ Document workarounds and known issues
- ✅ Use provided test utilities
- ✅ Check both kernel and userspace logs

### DON'T:
- ❌ Assume single-core behavior
- ❌ Skip testing after "small" changes
- ❌ Ignore warnings in serial output
- ❌ Use `unwrap()` or `expect()` without justification
- ❌ Debug on stale builds
- ❌ Forget to check TLB invalidation
- ❌ Ignore intermittent issues (often SMP races)

## Quick Reference

### Fast Debug Commands

```bash
# Quick boot test
make clean && make build && make iso && ./tools/qemu.sh

# Run with GDB
./tools/debug/start_qemu_debug.sh

# Run specific test suite
./tools/testing/test_drivers.sh
./tools/testing/run_filesystem_tests.sh
./tools/testing/test_user_mode_integration.sh

# Check for errors after code change
cd kernel && cargo check
```

### Common Error Patterns

**Page fault in kernel:**
- Check pointer validity
- Verify page table mappings
- Look for uninitialized data

**Deadlock:**
- Check lock ordering
- Look for missing unlock
- Verify interrupt state

**Triple fault:**
- Stack overflow (check stack size)
- Invalid GDT/IDT
- Recursive fault handler

**Userspace crash:**
- Invalid syscall arguments
- Memory access violation
- Signal handling issue

## Documentation References

- **Architecture:** `docs/architecture/`
- **Troubleshooting:** `docs/troubleshooting/`
- **Debug tools:** `tools/debug/README.md`
- **Testing:** `tools/testing/INTEGRATION_TESTS.md`
- **SMP issues:** `docs/troubleshooting/smp-ap-boot-issues.md`

## Summary

When debugging MelloOS:
1. **Serial output is your friend** - Use it liberally
2. **GDB for complex issues** - Don't hesitate to use it
3. **Test incrementally** - Small changes, frequent testing
4. **Think SMP** - Consider multi-core implications
5. **Use provided tools** - Scripts and utilities are there to help
6. **Document findings** - Help future debugging efforts

บอก AI ด้วยว่าเรามีตัว Debugger พร้อมแล้วสามารถใช้ได้