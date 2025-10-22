# MelloOS Troubleshooting Guide

## Introduction

This guide helps diagnose and resolve common issues with MelloOS, particularly related to PTY, signals, and job control functionality.

## General Troubleshooting Steps

### 1. Check System Logs

View kernel logs via serial output:
```bash
# In QEMU, logs appear in terminal
# Look for error messages, warnings, or panics
```

### 2. Check Process State

```bash
# List all processes
ps aux

# Check specific process
cat /proc/<pid>/stat
cat /proc/<pid>/status
```

### 3. Check System Resources

```bash
# Memory usage
cat /proc/meminfo

# CPU information
cat /proc/cpuinfo

# System uptime
cat /proc/uptime
```

### 4. Check Debug Information

```bash
# PTY state
cat /proc/debug/pty

# Session tree
cat /proc/debug/sessions

# Lock statistics
cat /proc/debug/locks
```

## PTY Issues

### Issue: Cannot Open /dev/ptmx

**Symptoms:**
```bash
$ cat /dev/ptmx
Error: No such device
```

**Possible Causes:**
1. PTY subsystem not initialized
2. Device node not created
3. Permissions issue

**Solutions:**

1. **Check if PTY subsystem is loaded:**
   ```bash
   # Look for PTY initialization in boot logs
   # Should see: [pty] Initialized PTY subsystem
   ```

2. **Verify device node exists:**
   ```bash
   ls -l /dev/ptmx
   # Should show: crw-rw-rw- 1 root root 5, 2 ... /dev/ptmx
   ```

3. **Check kernel configuration:**
   - Ensure PTY driver is compiled in
   - Check `kernel/src/dev/pty/mod.rs` is included in build

### Issue: PTY Allocation Fails

**Symptoms:**
```bash
$ open /dev/ptmx
Error: No device available (ENODEV)
```

**Possible Causes:**
1. All PTY pairs allocated (max 256)
2. Memory allocation failure
3. PTY table corruption

**Solutions:**

1. **Check PTY allocation table:**
   ```bash
   cat /proc/debug/pty
   # Look for available PTY numbers
   ```

2. **Check for leaked PTYs:**
   ```bash
   # Look for PTYs with master closed but slave still open
   cat /proc/debug/pty | grep "closed.*open"
   ```

3. **Restart system:**
   - PTY leaks are cleared on reboot
   - Fix underlying leak in code

**Debug Steps:**
```rust
// In kernel/src/dev/pty/mod.rs
// Add logging to allocation:
log::debug!("PTY allocation: {} pairs in use", count_allocated());
```

### Issue: Terminal Echo Not Working

**Symptoms:**
- Typing doesn't show on screen
- Characters appear but are wrong

**Possible Causes:**
1. ECHO flag disabled in termios
2. PTY in raw mode
3. Line discipline issue

**Solutions:**

1. **Check termios settings:**
   ```bash
   # In userspace, check c_lflag
   # ECHO should be set (0x0008)
   ```

2. **Reset terminal:**
   ```bash
   # Exit and restart shell
   # Or set termios explicitly
   ```

3. **Verify line discipline:**
   ```rust
   // Check that echo is processed in line discipline
   if termios.c_lflag & ECHO != 0 {
       // Echo should happen here
   }
   ```

**Debug Steps:**
```rust
// Add logging to PTY write:
log::debug!("PTY write: {} bytes, echo={}", 
    len, termios.c_lflag & ECHO != 0);
```

### Issue: Window Resize Not Working

**Symptoms:**
- Terminal resize doesn't update shell
- SIGWINCH not delivered
- Programs don't redraw

**Possible Causes:**
1. TIOCSWINSZ ioctl not called
2. SIGWINCH not generated
3. Foreground process group not set

**Solutions:**

1. **Verify ioctl is called:**
   ```rust
   // In terminal emulator
   ioctl(pty_master, TIOCSWINSZ, &winsize)?;
   ```

2. **Check SIGWINCH delivery:**
   ```bash
   cat /proc/<pid>/status
   # Look for pending signals
   ```

3. **Verify foreground group:**
   ```bash
   cat /proc/debug/sessions
   # Check fg_pgid matches shell PGID
   ```

**Debug Steps:**
```rust
// Add logging to resize handler:
log::debug!("Window resize: {}x{}, fg_pgid={}", 
    rows, cols, pty.foreground_pgid);
log::debug!("Sending SIGWINCH to PGID {}", pgid);
```

### Issue: PTY Deadlock

**Symptoms:**
- System hangs when using terminal
- Both master and slave blocked
- Cannot type or see output

**Possible Causes:**
1. Both buffers full
2. Lock ordering violation
3. Circular wait

**Solutions:**

1. **Use non-blocking I/O:**
   ```rust
   // Set O_NONBLOCK on PTY master
   fcntl(fd, F_SETFL, O_NONBLOCK)?;
   ```

2. **Check buffer sizes:**
   ```bash
   cat /proc/debug/pty
   # Look for full buffers (4096/4096)
   ```

3. **Verify lock ordering:**
   ```rust
   // Always acquire in same order:
   // 1. PTY table lock
   // 2. PTY pair lock
   // 3. Buffer locks
   ```

**Debug Steps:**
```rust
// Add timeout to locks:
if let Some(guard) = pty.lock.try_lock_timeout(Duration::from_millis(100)) {
    // Success
} else {
    log::error!("PTY lock timeout - possible deadlock");
}
```

## Signal Issues

### Issue: Ctrl-C Doesn't Work

**Symptoms:**
- Pressing Ctrl-C doesn't stop program
- No SIGINT delivered
- Program continues running

**Possible Causes:**
1. ISIG flag disabled
2. Not in foreground process group
3. Signal handler ignoring SIGINT
4. VINTR character not set correctly

**Solutions:**

1. **Check termios ISIG flag:**
   ```bash
   # c_lflag should have ISIG set (0x0001)
   ```

2. **Verify foreground group:**
   ```bash
   cat /proc/debug/sessions
   # Ensure program is in foreground PGID
   ```

3. **Check signal handler:**
   ```bash
   # Program may have installed custom handler
   # Try SIGKILL instead: kill -9 <pid>
   ```

4. **Verify VINTR setting:**
   ```rust
   // termios.c_cc[VINTR] should be 3 (Ctrl-C)
   ```

**Debug Steps:**
```rust
// Add logging to signal generation:
log::debug!("Special char: {}, ISIG={}, fg_pgid={}", 
    ch, termios.c_lflag & ISIG != 0, foreground_pgid);
log::debug!("Sending SIGINT to PGID {}", pgid);
```

### Issue: Background Job Won't Stop

**Symptoms:**
- Background job continues after Ctrl-Z
- Job shows as Running instead of Stopped
- Cannot bring to foreground

**Possible Causes:**
1. Job not in foreground when Ctrl-Z pressed
2. SIGTSTP ignored by program
3. Job control not working

**Solutions:**

1. **Verify job is foreground:**
   ```bash
   jobs
   # Current job should not have & at end
   ```

2. **Try SIGSTOP instead:**
   ```bash
   kill -STOP <pid>
   # SIGSTOP cannot be caught
   ```

3. **Check process group:**
   ```bash
   cat /proc/<pid>/stat
   # Verify PGID matches foreground PGID
   ```

**Debug Steps:**
```rust
// Log signal delivery:
log::debug!("Delivering SIGTSTP to PID {}, PGID {}", 
    pid, pgid);
log::debug!("Task state before: {:?}, after: {:?}", 
    old_state, new_state);
```

### Issue: SIGCHLD Not Received

**Symptoms:**
- Zombie processes accumulate
- Shell doesn't detect child exit
- Jobs list shows wrong state

**Possible Causes:**
1. SIGCHLD handler not installed
2. Signal masked
3. Handler not calling wait()

**Solutions:**

1. **Install SIGCHLD handler:**
   ```rust
   let action = SigAction {
       handler: SigHandler::Custom(sigchld_handler as usize),
       mask: 0,
       flags: 0,
   };
   sigaction(SIGCHLD, &action)?;
   ```

2. **Check signal mask:**
   ```bash
   cat /proc/self/status
   # SigBlk should not include SIGCHLD (bit 17)
   ```

3. **Call wait() in handler:**
   ```rust
   fn sigchld_handler() {
       while let Ok((pid, status)) = wait4(-1, WNOHANG) {
           update_job_status(pid, status);
       }
   }
   ```

**Debug Steps:**
```rust
// Log SIGCHLD delivery:
log::debug!("Child {} exited with status {}", pid, status);
log::debug!("Sending SIGCHLD to parent {}", ppid);
```

### Issue: Signal Delivered to Wrong Process

**Symptoms:**
- Signal affects unrelated process
- Wrong process terminates
- Security violation

**Possible Causes:**
1. PID reuse race
2. Incorrect PGID
3. Permission check failure

**Solutions:**

1. **Check PID validity:**
   ```rust
   // Always verify PID exists before sending signal
   let task = find_task(pid).ok_or(ESRCH)?;
   ```

2. **Verify PGID:**
   ```bash
   cat /proc/<pid>/stat
   # Check PGID field (5th field)
   ```

3. **Check permissions:**
   ```rust
   // Verify sender has permission
   if sender.uid != target.uid && sender.uid != 0 {
       return Err(EPERM);
   }
   ```

**Debug Steps:**
```rust
// Log signal delivery:
log::debug!("Signal {} from PID {} (UID {}) to PID {} (UID {})", 
    sig, sender.pid, sender.uid, target.pid, target.uid);
```

## Job Control Issues

### Issue: Cannot Set Process Group

**Symptoms:**
```bash
$ setpgid(pid, pgid)
Error: Operation not permitted (EPERM)
```

**Possible Causes:**
1. Process not child of caller
2. Different session
3. Process already exec'd

**Solutions:**

1. **Only set for self or children:**
   ```rust
   // Can set for self
   setpgid(0, 0)?;
   
   // Can set for child immediately after fork
   let pid = fork()?;
   if pid == 0 {
       setpgid(0, 0)?;  // Child sets own
   } else {
       setpgid(pid, pid)?;  // Parent sets child's
   }
   ```

2. **Check session:**
   ```bash
   cat /proc/<pid>/stat
   # Verify SID matches (6th field)
   ```

3. **Set before exec:**
   ```rust
   let pid = fork()?;
   if pid == 0 {
       setpgid(0, pgid)?;  // Set BEFORE exec
       exec(cmd)?;
   }
   ```

**Debug Steps:**
```rust
// Log setpgid attempts:
log::debug!("setpgid({}, {}) from PID {} (SID {})", 
    pid, pgid, caller.pid, caller.sid);
```

### Issue: Foreground Group Not Working

**Symptoms:**
- Background process receives keyboard input
- Foreground process doesn't get signals
- Terminal control confused

**Possible Causes:**
1. tcsetpgrp not called
2. Wrong PGID passed
3. Not controlling terminal

**Solutions:**

1. **Set foreground group:**
   ```rust
   // After forking child
   tcsetpgrp(tty_fd, child_pgid)?;
   
   // After child exits
   tcsetpgrp(tty_fd, shell_pgid)?;
   ```

2. **Verify PGID exists:**
   ```bash
   ps aux | grep <pgid>
   # Ensure process group exists
   ```

3. **Check controlling terminal:**
   ```bash
   cat /proc/self/stat
   # TTY field (7th) should not be 0
   ```

**Debug Steps:**
```rust
// Log foreground changes:
log::debug!("Setting foreground PGID to {} on TTY {:?}", 
    pgid, tty);
```

### Issue: Session Creation Fails

**Symptoms:**
```bash
$ setsid()
Error: Operation not permitted (EPERM)
```

**Possible Causes:**
1. Already a process group leader
2. Already a session leader

**Solutions:**

1. **Fork before setsid:**
   ```rust
   let pid = fork()?;
   if pid == 0 {
       // Child is not group leader
       setsid()?;
       // Now session leader
   }
   ```

2. **Check current state:**
   ```bash
   cat /proc/self/stat
   # If PID == PGID, you're a group leader
   # If PID == SID, you're a session leader
   ```

**Debug Steps:**
```rust
// Log setsid attempts:
log::debug!("setsid() from PID {} (PGID {}, SID {})", 
    pid, pgid, sid);
```

### Issue: Zombie Processes

**Symptoms:**
```bash
$ ps aux
# Shows processes with state Z (zombie)
```

**Possible Causes:**
1. Parent not calling wait()
2. SIGCHLD handler missing
3. Parent died before reaping

**Solutions:**

1. **Install SIGCHLD handler:**
   ```rust
   fn sigchld_handler() {
       while let Ok(_) = wait4(-1, WNOHANG) {
           // Reap zombies
       }
   }
   ```

2. **Call wait() explicitly:**
   ```rust
   // After child exits
   wait4(child_pid, 0)?;
   ```

3. **Init reaps orphans:**
   - Kernel should reparent to init (PID 1)
   - Init should periodically call wait()

**Debug Steps:**
```rust
// Log zombie creation:
log::debug!("Process {} became zombie, parent {}", 
    pid, ppid);
```

## Performance Issues

### Issue: Slow Terminal Response

**Symptoms:**
- Typing has noticeable lag
- Screen updates slowly
- Commands take long to start

**Possible Causes:**
1. Buffer full, blocking writes
2. Lock contention
3. Inefficient rendering

**Solutions:**

1. **Use non-blocking I/O:**
   ```rust
   fcntl(fd, F_SETFL, O_NONBLOCK)?;
   ```

2. **Check lock statistics:**
   ```bash
   cat /proc/debug/locks
   # Look for high contention
   ```

3. **Optimize rendering:**
   - Batch screen updates
   - Use dirty regions
   - Minimize redraws

**Debug Steps:**
```rust
// Measure latency:
let start = rdtsc();
write(fd, buf)?;
let end = rdtsc();
log::debug!("Write latency: {} cycles", end - start);
```

### Issue: High CPU Usage

**Symptoms:**
- System feels sluggish
- CPU at 100%
- Excessive context switches

**Possible Causes:**
1. Busy-wait loops
2. Excessive polling
3. Lock spinning

**Solutions:**

1. **Use blocking I/O:**
   ```rust
   // Instead of polling
   loop {
       if let Ok(n) = read(fd, buf) {
           break;
       }
   }
   
   // Use blocking read
   let n = read(fd, buf)?;
   ```

2. **Check context switches:**
   ```bash
   cat /proc/stat
   # Look at ctxt line
   ```

3. **Use sleep/yield:**
   ```rust
   // Instead of spinning
   while !condition {
       // Spin
   }
   
   // Yield CPU
   while !condition {
       sched_yield()?;
   }
   ```

**Debug Steps:**
```rust
// Profile hot paths:
static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
CALL_COUNT.fetch_add(1, Ordering::Relaxed);
```

### Issue: Memory Leak

**Symptoms:**
- Available memory decreases over time
- System eventually runs out of memory
- OOM errors

**Possible Causes:**
1. PTY pairs not freed
2. Process structures leaked
3. Buffer allocations not freed

**Solutions:**

1. **Check memory usage:**
   ```bash
   cat /proc/meminfo
   # Monitor MemFree over time
   ```

2. **Verify cleanup:**
   ```rust
   // Ensure Drop is implemented
   impl Drop for PtyPair {
       fn drop(&mut self) {
           // Free resources
       }
   }
   ```

3. **Use RAII patterns:**
   ```rust
   // Resources freed automatically
   {
       let _guard = acquire_resource();
       // Use resource
   }  // Freed here
   ```

**Debug Steps:**
```rust
// Track allocations:
log::debug!("Allocated PTY {}", num);
log::debug!("Freed PTY {}", num);
```

## System Stability Issues

### Issue: Kernel Panic

**Symptoms:**
- System crashes with panic message
- Shows register dump and stack trace

**Common Causes:**
1. Null pointer dereference
2. Assertion failure
3. Stack overflow
4. Deadlock detection

**Solutions:**

1. **Read panic message:**
   ```
   KERNEL PANIC on CPU 0
   Message: assertion failed: ptr != null
   ```

2. **Check stack trace:**
   ```
   Stack trace:
     0xffffffff80001234  <- Look up in kernel binary
     0xffffffff80002345
   ```

3. **Use GDB:**
   ```bash
   # Get address from panic
   addr2line -e kernel.elf 0xffffffff80001234
   ```

**Debug Steps:**
```rust
// Add assertions:
assert!(ptr != null, "Pointer must not be null");
assert!(size > 0, "Size must be positive");

// Add validation:
if !is_valid(ptr) {
    log::error!("Invalid pointer: {:p}", ptr);
    return Err(EINVAL);
}
```

### Issue: System Hang

**Symptoms:**
- System stops responding
- No output on serial
- Cannot interact

**Common Causes:**
1. Deadlock
2. Infinite loop
3. Interrupt disabled too long

**Solutions:**

1. **Use GDB to break:**
   ```bash
   # In GDB
   Ctrl-C
   backtrace
   info threads
   ```

2. **Check for deadlock:**
   ```bash
   cat /proc/debug/locks
   # Look for high wait times
   ```

3. **Add timeouts:**
   ```rust
   if let Some(guard) = lock.try_lock_timeout(timeout) {
       // Success
   } else {
       panic!("Lock timeout - deadlock?");
   }
   ```

**Debug Steps:**
```rust
// Add watchdog:
static LAST_HEARTBEAT: AtomicU64 = AtomicU64::new(0);

fn heartbeat() {
    LAST_HEARTBEAT.store(current_time(), Ordering::Relaxed);
}

fn check_watchdog() {
    let last = LAST_HEARTBEAT.load(Ordering::Relaxed);
    if current_time() - last > TIMEOUT {
        panic!("Watchdog timeout");
    }
}
```

### Issue: Triple Fault

**Symptoms:**
- System reboots immediately
- No panic message
- Happens during exception handling

**Common Causes:**
1. Stack overflow in exception handler
2. Invalid page table
3. Recursive exception

**Solutions:**

1. **Check stack size:**
   ```rust
   // Ensure adequate stack for exceptions
   const EXCEPTION_STACK_SIZE: usize = 8192;
   ```

2. **Validate page tables:**
   ```rust
   // Before switching
   assert!(page_table.is_valid());
   ```

3. **Use separate exception stack:**
   ```rust
   // IST (Interrupt Stack Table) for critical exceptions
   ```

**Debug Steps:**
```rust
// Log before dangerous operations:
log::debug!("About to switch page tables");
switch_page_table(new_pt);
log::debug!("Page table switch successful");
```

## Diagnostic Tools

### Built-in Tools

1. **ps**: List processes
   ```bash
   ps aux
   ```

2. **/proc files**: System information
   ```bash
   cat /proc/meminfo
   cat /proc/cpuinfo
   cat /proc/<pid>/stat
   ```

3. **Debug /proc files**: Internal state
   ```bash
   cat /proc/debug/pty
   cat /proc/debug/sessions
   cat /proc/debug/locks
   ```

### External Tools

1. **GDB**: Kernel debugging
   ```bash
   gdb kernel.elf
   target remote :1234
   ```

2. **QEMU monitor**: VM control
   ```
   Ctrl-A C  # Enter monitor
   info registers
   info mem
   ```

3. **Serial console**: Kernel logs
   ```bash
   # Logs appear in QEMU terminal
   ```

## Getting Help

### Information to Provide

When reporting issues, include:

1. **System state:**
   ```bash
   cat /proc/meminfo
   cat /proc/cpuinfo
   ps aux
   ```

2. **Error messages:**
   - Kernel panic messages
   - Error codes
   - Serial log output

3. **Reproduction steps:**
   - Exact commands run
   - Expected vs actual behavior
   - Frequency (always, sometimes, rare)

4. **Environment:**
   - QEMU version
   - Host OS
   - Build configuration

### Resources

- **Architecture docs**: `docs/architecture/`
- **User guide**: `docs/USER_GUIDE.md`
- **Developer guide**: `docs/DEVELOPER_GUIDE.md`
- **Test results**: `tools/testing/`

## Appendix: Error Code Reference

| Code   | Name    | Description |
|--------|---------|-------------|
| 1      | EPERM   | Operation not permitted |
| 2      | ENOENT  | No such file or directory |
| 3      | ESRCH   | No such process |
| 5      | EIO     | I/O error |
| 9      | EBADF   | Bad file descriptor |
| 11     | EAGAIN  | Try again |
| 12     | ENOMEM  | Out of memory |
| 13     | EACCES  | Permission denied |
| 14     | EFAULT  | Bad address |
| 16     | EBUSY   | Device or resource busy |
| 17     | EEXIST  | File exists |
| 19     | ENODEV  | No such device |
| 22     | EINVAL  | Invalid argument |
| 25     | ENOTTY  | Not a typewriter |
| 38     | ENOSYS  | Function not implemented |

## Appendix: Signal Reference

| Signal  | Number | Default | Description |
|---------|--------|---------|-------------|
| SIGHUP  | 1      | Term    | Hangup |
| SIGINT  | 2      | Term    | Interrupt |
| SIGQUIT | 3      | Core    | Quit |
| SIGKILL | 9      | Term    | Kill (uncatchable) |
| SIGTERM | 15     | Term    | Terminate |
| SIGCHLD | 17     | Ignore  | Child status changed |
| SIGCONT | 18     | Cont    | Continue |
| SIGSTOP | 19     | Stop    | Stop (uncatchable) |
| SIGTSTP | 20     | Stop    | Terminal stop |
| SIGTTIN | 21     | Stop    | Background read |
| SIGTTOU | 22     | Stop    | Background write |
| SIGWINCH| 28     | Ignore  | Window size change |
