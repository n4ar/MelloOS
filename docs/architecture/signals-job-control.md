# Signals and Job Control Architecture

## Overview

The signal and job control subsystem enables process management, inter-process communication, and terminal control in MelloOS. It implements POSIX-compatible signal handling and job control semantics for interactive shell sessions.

## Architecture

### Component Hierarchy

```
┌─────────────────────────────────────────────────┐
│              Shell (mello-sh)                   │
│  - Job table management                         │
│  - Foreground/background control                │
│  - Signal handler installation                  │
└────────────────┬────────────────────────────────┘
                 │ System calls
                 ↓
┌─────────────────────────────────────────────────┐
│         Kernel Signal Subsystem                 │
│  ┌──────────────────────────────────────────┐  │
│  │      Signal Delivery Engine              │  │
│  │  - Pending signal tracking               │  │
│  │  - Signal handler dispatch               │  │
│  │  - Default action handling               │  │
│  └──────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────┐  │
│  │    Process Group Management              │  │
│  │  - PGID tracking                         │  │
│  │  - Session management                    │  │
│  │  - Foreground group control              │  │
│  └──────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────┐  │
│  │      Terminal Integration                │  │
│  │  - Controlling terminal tracking         │  │
│  │  - Keyboard signal generation            │  │
│  │  - Background process control            │  │
│  └──────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

## Data Structures

### Task Extensions

```rust
pub struct Task {
    // Process identification
    pub pid: Pid,
    pub ppid: Pid,
    
    // Job control
    pub pgid: Pid,              // Process group ID
    pub sid: Sid,               // Session ID
    pub tty: Option<DeviceId>,  // Controlling terminal
    
    // Signal handling
    pub signal_handlers: [SigAction; 64],
    pub pending_signals: AtomicU64,
    pub signal_mask: u64,
}
```

### Signal Action

```rust
pub struct SigAction {
    pub handler: SigHandler,
    pub mask: u64,        // Signals to block during handler
    pub flags: u32,       // SA_RESTART, SA_SIGINFO, etc.
}

pub enum SigHandler {
    Default,              // Default action (term/stop/ignore)
    Ignore,               // Explicitly ignore
    Custom(usize),        // User-space handler address
}
```

### Process Group

```rust
pub struct ProcessGroup {
    pub pgid: Pid,
    pub session: Sid,
    pub processes: Vec<Pid>,
    pub lock: SpinLock<()>,
}
```

### Session

```rust
pub struct Session {
    pub sid: Sid,
    pub leader: Pid,
    pub controlling_tty: Option<DeviceId>,
    pub foreground_pgid: Option<Pid>,
    pub process_groups: Vec<Pid>,
}
```

## Signal Types

### Standard Signals

| Signal   | Number | Default Action | Description |
|----------|--------|----------------|-------------|
| SIGHUP   | 1      | Terminate      | Hangup (controlling terminal closed) |
| SIGINT   | 2      | Terminate      | Interrupt (Ctrl-C) |
| SIGQUIT  | 3      | Core dump      | Quit (Ctrl-\) |
| SIGKILL  | 9      | Terminate      | Kill (cannot be caught) |
| SIGTERM  | 15     | Terminate      | Termination request |
| SIGCHLD  | 17     | Ignore         | Child status changed |
| SIGCONT  | 18     | Continue       | Continue if stopped |
| SIGSTOP  | 19     | Stop           | Stop (cannot be caught) |
| SIGTSTP  | 20     | Stop           | Terminal stop (Ctrl-Z) |
| SIGTTIN  | 21     | Stop           | Background read from TTY |
| SIGTTOU  | 22     | Stop           | Background write to TTY |
| SIGWINCH | 28     | Ignore         | Window size change |

### Signal Actions

1. **Terminate**: Kill the process
2. **Stop**: Suspend process execution
3. **Continue**: Resume stopped process
4. **Ignore**: Do nothing
5. **Core dump**: Terminate and dump core (future)

## Signal Delivery

### Delivery Flow

```
1. Signal Generation
   ├─ Keyboard (Ctrl-C, Ctrl-Z)
   ├─ System call (kill, raise)
   ├─ Kernel event (SIGCHLD, SIGWINCH)
   └─ Exception (SIGSEGV, SIGFPE)
   
2. Signal Pending
   └─ Set bit in task->pending_signals
   
3. Signal Delivery (on return to userspace)
   ├─ Check pending_signals & ~signal_mask
   ├─ Select highest priority signal
   └─ Deliver signal
   
4. Signal Handling
   ├─ Default action
   │  ├─ Terminate
   │  ├─ Stop
   │  ├─ Continue
   │  └─ Ignore
   └─ Custom handler
      ├─ Save context
      ├─ Setup signal frame
      ├─ Jump to handler
      └─ Return via sigreturn
```

### Signal Delivery Rules

**To Process:**
```rust
fn send_signal(pid: Pid, sig: Signal) {
    let task = find_task(pid)?;
    task.pending_signals.fetch_or(1 << sig, Ordering::SeqCst);
    // Wake task if sleeping
}
```

**To Process Group:**
```rust
fn send_signal_to_group(pgid: Pid, sig: Signal) {
    let pg = find_process_group(pgid)?;
    for pid in &pg.processes {
        send_signal(*pid, sig);
    }
}
```

### Signal Masking

- Signals can be blocked via `signal_mask`
- Blocked signals remain pending
- SIGKILL and SIGSTOP cannot be blocked
- Signal handlers can specify additional masks

## Job Control

### Process Groups

**Creation:**
```rust
// Create new process group
setpgid(0, 0);  // Put self in new group with PGID = PID

// Join existing group
setpgid(child_pid, leader_pid);
```

**Rules:**
- Process can only set PGID for self or children
- Must be in same session
- Cannot move process to different session

### Sessions

**Creation:**
```rust
// Create new session
let sid = setsid();  // Returns new SID = PID
// Detaches from controlling terminal
// Becomes session leader
```

**Rules:**
- Only non-group-leaders can create sessions
- Session leader cannot create new session
- New session has no controlling terminal

### Controlling Terminal

**Acquisition:**
```rust
// Open terminal
let tty_fd = open("/dev/pts/0", O_RDWR);

// Make it controlling terminal (session leader only)
ioctl(tty_fd, TIOCSCTTY, 0);
```

**Properties:**
- Each session has at most one controlling terminal
- Only session leader can acquire controlling terminal
- Signals (SIGHUP, SIGINT, etc.) sent via controlling terminal

### Foreground Process Group

**Setting:**
```rust
// Set foreground group
tcsetpgrp(tty_fd, pgid);

// Get foreground group
let fg_pgid = tcgetpgrp(tty_fd);
```

**Behavior:**
- Only foreground group receives keyboard signals
- Background groups get SIGTTIN/SIGTTOU on TTY access
- Foreground group must be in same session as terminal

## Terminal Signal Generation

### Keyboard Signals

```
User Input → PTY → Check ISIG flag → Generate Signal → Foreground PGID
```

| Input  | Character | Signal   | Condition |
|--------|-----------|----------|-----------|
| Ctrl-C | VINTR     | SIGINT   | ISIG set  |
| Ctrl-Z | VSUSP     | SIGTSTP  | ISIG set  |
| Ctrl-\ | VQUIT     | SIGQUIT  | ISIG set  |

### Background Process Signals

```rust
// Background process tries to read from TTY
if current_pgid != foreground_pgid {
    send_signal_to_group(current_pgid, SIGTTIN);
    return Err(EIO);
}

// Background process tries to write to TTY
if current_pgid != foreground_pgid && TOSTOP_set {
    send_signal_to_group(current_pgid, SIGTTOU);
    return Err(EIO);
}
```

### Terminal Hangup

```rust
// PTY master closes
if pty.slave_open {
    if let Some(sid) = pty.session {
        let leader = find_session_leader(sid);
        send_signal(leader, SIGHUP);
    }
}
```

## Shell Integration

### Job Table

```rust
pub struct Job {
    pub id: JobId,           // Job number (1, 2, 3, ...)
    pub pgid: Pid,           // Process group ID
    pub command: String,     // Command line
    pub state: JobState,     // Running, Stopped, Done
    pub background: bool,    // Started with &
}

pub enum JobState {
    Running,
    Stopped,
    Done(i32),  // Exit status
}
```

### Job Control Commands

**Foreground (fg):**
```rust
fn fg(job_id: JobId) {
    let job = find_job(job_id)?;
    tcsetpgrp(tty_fd, job.pgid);      // Make foreground
    kill(-job.pgid, SIGCONT);         // Resume if stopped
    wait_for_job(job);                // Wait for completion
    tcsetpgrp(tty_fd, shell_pgid);    // Restore shell
}
```

**Background (bg):**
```rust
fn bg(job_id: JobId) {
    let job = find_job(job_id)?;
    kill(-job.pgid, SIGCONT);  // Resume in background
    job.state = JobState::Running;
}
```

**Jobs List:**
```rust
fn jobs() {
    for job in &job_table {
        println!("[{}]{} {}    {}",
            job.id,
            if job.id == current_job { "+" } else { "-" },
            job.state,
            job.command);
    }
}
```

### SIGCHLD Handling

```rust
fn sigchld_handler() {
    loop {
        match wait4(-1, WNOHANG) {
            Ok((pid, status)) => {
                update_job_status(pid, status);
                if status.exited() {
                    print_job_done(pid, status.exit_code());
                } else if status.stopped() {
                    print_job_stopped(pid);
                }
            }
            Err(ECHILD) => break,  // No more children
            _ => break,
        }
    }
}
```

## System Calls

### Signal Management

```rust
// Install signal handler
fn sys_sigaction(sig: Signal, act: &SigAction, oldact: &mut SigAction) -> Result<()>;

// Send signal
fn sys_kill(pid: Pid, sig: Signal) -> Result<()>;

// Wait for signal
fn sys_sigsuspend(mask: u64) -> Result<()>;

// Return from signal handler
fn sys_sigreturn() -> !;
```

### Process Group Management

```rust
// Set process group
fn sys_setpgid(pid: Pid, pgid: Pid) -> Result<()>;

// Get process group
fn sys_getpgrp() -> Pid;

// Create new session
fn sys_setsid() -> Result<Sid>;

// Get session ID
fn sys_getsid(pid: Pid) -> Result<Sid>;
```

### Terminal Control

```rust
// Set foreground process group
fn sys_tcsetpgrp(fd: Fd, pgid: Pid) -> Result<()>;

// Get foreground process group
fn sys_tcgetpgrp(fd: Fd) -> Result<Pid>;
```

## Security

### Permission Checks

**Signal Sending:**
```rust
fn check_signal_permission(sender: &Task, target: &Task, sig: Signal) -> Result<()> {
    // Root can send to anyone
    if sender.uid == 0 {
        return Ok(());
    }
    
    // Must have same UID
    if sender.uid != target.uid {
        return Err(EPERM);
    }
    
    // Must be in same session (for terminal signals)
    if sender.sid != target.sid {
        return Err(EPERM);
    }
    
    // Cannot kill init
    if target.pid == 1 && (sig == SIGKILL || sig == SIGSTOP) {
        return Err(EPERM);
    }
    
    Ok(())
}
```

**Process Group Operations:**
```rust
fn check_setpgid_permission(caller: &Task, target_pid: Pid, pgid: Pid) -> Result<()> {
    // Can only set for self or children
    if target_pid != caller.pid {
        let target = find_task(target_pid)?;
        if target.ppid != caller.pid {
            return Err(ESRCH);
        }
    }
    
    // Must be in same session
    let target = find_task(target_pid)?;
    if target.sid != caller.sid {
        return Err(EPERM);
    }
    
    Ok(())
}
```

## Synchronization

### Lock Hierarchy

```
1. Global session table lock
2. Session lock
3. Process group lock
4. Task lock
```

### Atomic Operations

- `pending_signals`: Atomic bitset operations
- Signal delivery: Lock task during handler setup
- Process group membership: RCU for read-mostly access

## State Machines

### Process State Transitions

```
[Running] --SIGSTOP--> [Stopped]
[Stopped] --SIGCONT--> [Running]
[Running] --SIGTERM--> [Zombie]
[Zombie]  --wait()---> [Reaped]
```

### Job State Transitions

```
[Running] --Ctrl-Z--> [Stopped]
[Stopped] --fg----> [Running (foreground)]
[Stopped] --bg----> [Running (background)]
[Running] --exit--> [Done]
```

## Performance Characteristics

- **Signal delivery latency**: < 5 µs
- **Process group lookup**: O(1) with hash table
- **Signal handler invocation**: < 10 µs
- **Job control operations**: < 100 µs

## Debugging

### Debug Information

```
/proc/<pid>/stat:
- PGID, SID, TTY number
- Pending signals (bitmask)
- Blocked signals (bitmask)

/proc/debug/sessions:
- Session tree
- Process groups per session
- Foreground PGID per terminal
```

### Logging

```
[cpu0][pid=123][signal] Delivered SIGINT to PGID 125
[cpu1][pid=124][pgid] Created process group 124
[cpu0][pid=123][session] Created session 123
[cpu2][pid=125][tty] Set foreground PGID to 126
```

## Common Patterns

### Pipeline Execution

```rust
// Create pipeline: cmd1 | cmd2 | cmd3
let pgid = fork();  // First child becomes group leader
setpgid(0, pgid);   // Join group

for cmd in pipeline {
    let pid = fork();
    if pid == 0 {
        setpgid(0, pgid);  // All children in same group
        setup_pipes();
        exec(cmd);
    }
}

tcsetpgrp(tty, pgid);  // Make pipeline foreground
wait_for_group(pgid);
tcsetpgrp(tty, shell_pgid);  // Restore shell
```

### Background Job

```rust
let pid = fork();
if pid == 0 {
    setpgid(0, 0);  // New process group
    exec(cmd);
}
setpgid(pid, pid);  // Parent's view
add_to_job_table(pid, cmd, background=true);
// Don't wait, don't make foreground
```

## Future Enhancements

- **Real-time signals**: SIGRTMIN-SIGRTMAX
- **Signal queuing**: Multiple pending signals
- **Signal info**: siginfo_t structure
- **Advanced job control**: Job control in subshells
- **Namespace isolation**: PID namespaces
