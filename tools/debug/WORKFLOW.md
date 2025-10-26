# MelloOS Debugging Workflow

## 🔄 Workflow Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    MelloOS Debug Workflow                    │
└─────────────────────────────────────────────────────────────┘

1. Code Change
   ↓
2. Build (make clean && make build && make iso)
   ↓
3. Start QEMU with GDB server (-s -S)
   ↓
4. Connect Debugger (VS Code or GDB)
   ↓
5. Set Breakpoints
   ↓
6. Debug & Analyze
   ↓
7. Fix Issues
   ↓
8. Repeat
```

## 📋 Detailed Workflow

### Phase 1: Preparation

#### 1.1 Setup Environment
```bash
# ติดตั้ง tools
brew install gdb qemu

# ติดตั้ง VS Code extensions
code --install-extension rust-lang.rust-analyzer
code --install-extension vadimcn.vscode-lldb
```

#### 1.2 Verify Setup
```bash
# ตรวจสอบ tools
gdb --version
qemu-system-x86_64 --version
rustc --version

# ตรวจสอบ build
make clean
make build
make iso
```

### Phase 2: Development

#### 2.1 Write Code
```rust
// kernel/src/main.rs
pub fn kernel_main() {
    // Your code here
}
```

#### 2.2 Check Syntax
```bash
cd kernel && cargo check
```

#### 2.3 Build
```bash
make clean
make build
make iso
```

### Phase 3: Debugging

#### 3.1 Start Debug Session

**Option A: VS Code (Recommended)**
```
1. Open VS Code
2. Press F5
3. Select "Debug MelloOS Kernel (GDB)"
4. Wait for debugger to connect
```

**Option B: Command Line**
```bash
# Terminal 1
./tools/debug/start_qemu_debug.sh

# Terminal 2
gdb kernel/target/x86_64-unknown-none/debug/kernel
(gdb) target remote localhost:1234
```

**Option C: Quick Debug**
```bash
./tools/debug/quick_debug.sh kernel_main
```

#### 3.2 Set Breakpoints

**VS Code:**
- Click left of line number
- Or press F9

**GDB:**
```gdb
break kernel_main
break page_fault_handler
break schedule
```

#### 3.3 Control Execution

**VS Code:**
- F5: Continue
- F10: Step Over
- F11: Step Into
- Shift+F11: Step Out

**GDB:**
```gdb
continue (c)
step (s)
next (n)
finish (fin)
```

#### 3.4 Inspect State

**VS Code:**
- Variables panel
- Watch panel
- Call Stack panel
- Debug Console

**GDB:**
```gdb
info registers
print variable_name
x/10x 0x100000
backtrace
```

### Phase 4: Analysis

#### 4.1 Identify Issue

**Common Issues:**
- Page fault (check CR2)
- Null pointer dereference
- Stack overflow
- Memory corruption
- Race condition

**Analysis Tools:**
```gdb
# Page fault
info registers cr2
info registers cr3
backtrace

# Memory
x/10gx $rsp
x/10i $rip

# Registers
info registers
info registers rax rbx rcx
```

#### 4.2 Understand Root Cause

**Questions to Ask:**
1. What was the program doing?
2. What was the expected behavior?
3. What actually happened?
4. Why did it happen?
5. How can we fix it?

**Investigation Steps:**
1. Check call stack
2. Examine variables
3. Review memory
4. Check registers
5. Read logs

### Phase 5: Fix

#### 5.1 Make Changes
```rust
// Fix the issue
pub fn fixed_function() {
    // Corrected code
}
```

#### 5.2 Verify Fix
```bash
# Rebuild
make clean
make build
make iso

# Test
./tools/debug/quick_debug.sh
```

#### 5.3 Test Thoroughly
```bash
# Run test suite
./tools/testing/test_drivers.sh
./tools/testing/test_user_mode_integration.sh

# Manual testing
make run
```

### Phase 6: Cleanup

#### 6.1 Stop Debugging
```bash
# Stop QEMU
pkill -f qemu-system-x86_64

# Or in VS Code: Shift+F5
```

#### 6.2 Document
```markdown
# Document the issue and fix
- What was the problem?
- How was it fixed?
- What was learned?
```

## 🎯 Debugging Strategies

### Strategy 1: Binary Search

```
1. Set breakpoint at middle of suspected code
2. Check if issue occurs before or after
3. Repeat until issue is isolated
```

### Strategy 2: Watchpoints

```gdb
# Watch variable changes
watch global_variable

# Watch memory changes
watch *0x100000

# Continue and wait for change
continue
```

### Strategy 3: Conditional Breakpoints

```gdb
# Break only when condition is true
break kmalloc if size > 4096
break schedule if current_task->pid == 42
```

### Strategy 4: Logging

```rust
// Add debug logging
log::debug!("Variable value: {}", value);
log::info!("Entering function");
log::error!("Error occurred: {}", error);
```

### Strategy 5: Assertions

```rust
// Add assertions
assert!(pointer.is_not_null());
assert_eq!(expected, actual);
debug_assert!(expensive_check());
```

## 🔍 Common Debugging Scenarios

### Scenario 1: Kernel Panic

```
1. Note panic message
2. Check backtrace
3. Identify panic location
4. Set breakpoint before panic
5. Debug to find root cause
```

### Scenario 2: Page Fault

```
1. Break at page_fault_handler
2. Check CR2 (faulting address)
3. Check error code
4. Examine page tables
5. Find why access failed
```

### Scenario 3: Hang/Deadlock

```
1. Ctrl+C in GDB to break
2. Check backtrace
3. Examine locks held
4. Check for circular wait
5. Review lock ordering
```

### Scenario 4: Memory Corruption

```
1. Set watchpoint on corrupted memory
2. Continue until watchpoint hits
3. Examine who wrote to memory
4. Check if write was valid
5. Fix bounds checking
```

### Scenario 5: Race Condition

```
1. Reproduce consistently
2. Add logging around suspected code
3. Check SMP safety
4. Review synchronization
5. Add proper locking
```

## 📊 Debug Checklist

### Before Debugging
- [ ] Code compiles without errors
- [ ] cargo check passes
- [ ] Build is up to date
- [ ] QEMU can start
- [ ] GDB is installed

### During Debugging
- [ ] Breakpoints are set
- [ ] Debugger is connected
- [ ] Serial output is visible
- [ ] Logs are being captured
- [ ] State is being examined

### After Debugging
- [ ] Issue is understood
- [ ] Fix is implemented
- [ ] Fix is tested
- [ ] Tests pass
- [ ] Documentation updated

## 🎓 Best Practices

### 1. Start Simple
- Debug one issue at a time
- Use simple test cases
- Isolate the problem

### 2. Use Tools Effectively
- Learn GDB commands
- Use VS Code features
- Read documentation

### 3. Document Everything
- Note what you tried
- Record findings
- Document solutions

### 4. Test Thoroughly
- Test the fix
- Test edge cases
- Run regression tests

### 5. Learn from Issues
- Understand root cause
- Prevent similar issues
- Share knowledge

## 🚀 Advanced Techniques

### 1. Python Scripting in GDB

```python
# .gdbinit
python
def print_task_list():
    # Custom Python code
    pass
end
```

### 2. GDB Scripts

```bash
# debug_script.gdb
target remote localhost:1234
break kernel_main
commands
  info registers
  backtrace
  continue
end
continue
```

### 3. Automated Testing

```bash
# test_debug.sh
#!/bin/bash
./tools/debug/start_qemu_debug.sh &
sleep 2
gdb -batch -x debug_script.gdb kernel/target/.../kernel
pkill qemu
```

### 4. Custom GDB Commands

```gdb
# .gdbinit
define klog
  # Print kernel log
  x/100s kernel_log_buffer
end

define task
  # Print current task
  print *current_task
end
```

## 📚 Resources

### Documentation
- [Debugging Guide](../../docs/development/DEBUGGING_GUIDE.md)
- [Quick Start](../../docs/development/DEBUGGING_QUICKSTART.md)
- [Examples](example_session.md)

### External Resources
- [GDB Manual](https://sourceware.org/gdb/documentation/)
- [QEMU Debugging](https://qemu.readthedocs.io/en/latest/system/gdb.html)
- [OS Dev Wiki](https://wiki.osdev.org/Debugging)

## 🎉 Summary

Effective debugging workflow:

1. ✅ Prepare environment
2. ✅ Write and build code
3. ✅ Start debug session
4. ✅ Set breakpoints
5. ✅ Analyze state
6. ✅ Identify issue
7. ✅ Fix and verify
8. ✅ Test thoroughly
9. ✅ Document findings
10. ✅ Learn and improve

Happy debugging! 🐛🔍
