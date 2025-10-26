# Design Document - Advanced Userland & Shell Environment

## Overview

Phase 6.6 transforms MelloOS from a basic ELF execution environment into a fully interactive terminal-based operating system. The design implements a layered architecture spanning kernel subsystems (PTY, signals, /proc) and userland applications (terminal emulator, shell, coreutils). This phase establishes the foundation for native zsh porting (Phase 7) and full terminfo support (Phase 8).

### Design Goals

1. **Interactive Terminal Experience**: Provide responsive, POSIX-like terminal interaction
2. **Job Control**: Full support for foreground/background process management
3. **UNIX Compatibility**: Implement standard utilities and shell features
4. **Performance**: Sub-10ms shell startup, 200+ MB/s pipe throughput
5. **SMP Safety**: Lock-free where possible, fine-grained locking elsewhere
6. **UTF-8 Native**: Full Unicode support throughout the stack

### System Context

```
┌─────────────────────────────────────────────────┐
│              Mello-Term (Terminal UI)           │
│         VT/ANSI Renderer · UTF-8 · Scrollback   │
└────────────────┬────────────────────────────────┘
                 │ PTY Master (/dev/ptmx)
                 ↓
┌─────────────────────────────────────────────────┐
│           Kernel PTY Subsystem                  │
│    termios · ioctl · SIGWINCH · Line Discipline │
└────────────────┬────────────────────────────────┘
                 │ PTY Slave (/dev/pts/N)
                 ↓
┌─────────────────────────────────────────────────┐
│         Mello-sh (Shell)                        │
│  Parser · Job Control · Builtins · Pipelines    │
└────────────────┬────────────────────────────────┘
                 │ fork/exec
                 ↓
┌─────────────────────────────────────────────────┐
│         Mellobox (Coreutils)                    │
│    ls · cp · mv · rm · cat · grep · ps · kill   │
└─────────────────────────────────────────────────┘
```

## Architecture

### Layer 1: Kernel PTY Subsystem


#### PTY Device Model

The PTY subsystem provides pseudo-terminal pairs for terminal emulation:

**Components:**
- `/dev/ptmx` - PTY master multiplexer (single device node)
- `/dev/pts/<n>` - PTY slave devices (dynamically allocated)
- PTY driver managing master/slave pairs
- Line discipline for canonical/raw mode processing

**Data Flow:**
```
Terminal → PTY Master → Line Discipline → PTY Slave → Shell
Shell → PTY Slave → Line Discipline → PTY Master → Terminal
```

**Key Operations:**
- `open("/dev/ptmx")` - Allocates new PTY pair, returns master FD
- `ioctl(master, TIOCGPTN, &n)` - Gets slave number
- `open("/dev/pts/N")` - Opens slave side
- `ioctl(fd, TCGETS/TCSETS)` - Get/set termios attributes
- `ioctl(fd, TIOCGWINSZ/TIOCSWINSZ)` - Get/set window size
- `ioctl(fd, TIOCSCTTY, 0)` - Make this TTY the controlling terminal
- `ioctl(fd, TIOCSPGRP/TIOCGPGRP)` - Set/get foreground process group

#### Termios Structure

```rust
pub struct Termios {
    pub c_iflag: u32,  // Input modes
    pub c_oflag: u32,  // Output modes
    pub c_cflag: u32,  // Control modes
    pub c_lflag: u32,  // Local modes
    pub c_cc: [u8; 32], // Control characters
}
```

**Critical Flags (Minimum Set):**

*Input modes (c_iflag):*
- `ICRNL` - Map CR to NL on input
- `INLCR` - Map NL to CR on input
- `IXON` - Enable software flow control (XON/XOFF)
- `IXOFF` - Enable software flow control output

*Output modes (c_oflag):*
- `OPOST` - Enable output processing
- `ONLCR` - Map NL to CRNL on output

*Local modes (c_lflag):*
- `ICANON` - Canonical mode (line buffering)
- `ECHO` - Echo input characters
- `ISIG` - Generate signals for special characters

**Control Characters (c_cc):**
- `VINTR` - Interrupt character (Ctrl-C, generates SIGINT)
- `VSUSP` - Suspend character (Ctrl-Z, generates SIGTSTP)
- `VEOF` - End-of-file character (Ctrl-D)
- `VERASE` - Erase character (Backspace)
- `VMIN` - Minimum characters for non-canonical read
- `VTIME` - Timeout for non-canonical read (deciseconds)

#### Signal Generation

The PTY subsystem generates signals based on special characters:

| Character | Signal   | Condition |
|-----------|----------|-----------|
| Ctrl-C    | SIGINT   | ISIG set  |
| Ctrl-Z    | SIGTSTP  | ISIG set  |
| Ctrl-\    | SIGQUIT  | ISIG set  |

Signals are delivered only to the **foreground process group** of the terminal.


### Layer 2: Signal & Job Control

#### Process Groups and Sessions

```rust
pub struct ProcessGroup {
    pub pgid: Pid,
    pub processes: Vec<Pid>,
    pub session: Sid,
}

pub struct Session {
    pub sid: Sid,
    pub controlling_tty: Option<DeviceId>,
    pub foreground_pgid: Option<Pid>,
}
```

**Key Concepts:**
- **Process Group (PGID)**: Collection of related processes (e.g., pipeline)
- **Session (SID)**: Collection of process groups sharing a controlling terminal
- **Foreground Group**: The PGID currently receiving terminal input
- **Background Groups**: Other PGIDs in the session

#### Signal Delivery Rules

1. **Keyboard Signals** (SIGINT, SIGTSTP, SIGQUIT):
   - Delivered only to foreground process group
   - All processes in the group receive the signal

2. **SIGCHLD**:
   - Sent to parent when child stops, continues, or exits
   - Parent uses `wait4()` to reap child status

3. **SIGWINCH**:
   - Sent to foreground group when terminal size changes
   - Applications can handle to redraw UI

4. **SIGCONT**:
   - Resumes stopped processes
   - Automatically sent by `fg` and `bg` commands

#### System Calls

```rust
// Process group and session management
fn setpgid(pid: Pid, pgid: Pid) -> Result<()>;
fn getpgrp() -> Pid;
fn setsid() -> Result<Pid>;  // Create new session
fn getsid(pid: Pid) -> Result<Sid>;

// Terminal foreground control
fn tcsetpgrp(fd: Fd, pgid: Pid) -> Result<()>;
fn tcgetpgrp(fd: Fd) -> Result<Pid>;

// Signal handling
fn sigaction(sig: Signal, act: &SigAction) -> Result<()>;
fn kill(pid: Pid, sig: Signal) -> Result<()>;
```

#### Additional TTY Signals

Beyond keyboard signals, the PTY subsystem generates:

| Signal   | Condition | Target |
|----------|-----------|--------|
| SIGHUP   | PTY master closed | Foreground PGID |
| SIGTTIN  | Background read from TTY | Background PGID attempting read |
| SIGTTOU  | Background write to TTY | Background PGID attempting write |

**Controlling TTY Semantics:**
- `setsid()` creates new session and detaches from controlling TTY
- `ioctl(fd, TIOCSCTTY, 0)` makes fd the controlling TTY (session leader only)
- When controlling TTY closes, kernel sends SIGHUP to session leader


### Layer 3: /proc Filesystem

The /proc filesystem provides a virtual interface to kernel data structures.

#### Directory Structure

```
/proc/
├── <pid>/
│   ├── stat       # Process state (PID, state, PPID, PGID, etc.)
│   ├── cmdline    # Command line (null-separated)
│   ├── status     # Human-readable status
│   └── ...
├── self -> <pid>  # Symlink to current process
├── meminfo        # Memory statistics
├── cpuinfo        # CPU information
├── uptime         # System uptime
└── ...
```

#### Key Files Format

**`/proc/<pid>/stat`** (space-separated, Linux-compatible format):
```
pid (comm) state ppid pgrp session tty_nr tpgid flags minflt cminflt majflt cmajflt utime stime cutime cstime priority nice num_threads itrealvalue starttime vsize rss rsslim ...
```

**Key fields:**
- `pid` - Process ID
- `comm` - Command name (in parentheses)
- `state` - Process state (R=running, S=sleeping, Z=zombie, T=stopped)
- `ppid` - Parent process ID
- `pgrp` - Process group ID
- `session` - Session ID
- `tty_nr` - Controlling terminal device number (0 if none)
- `tpgid` - Foreground process group of controlling terminal
- `utime` - User mode time (clock ticks)
- `stime` - Kernel mode time (clock ticks)

**`/proc/<pid>/cmdline`** (null-separated):
```
arg0\0arg1\0arg2\0
```

**`/proc/meminfo`** (key-value pairs):
```
MemTotal:     1048576 kB
MemFree:       524288 kB
MemAvailable:  786432 kB
```

**`/proc/uptime`** (seconds.fraction format):
```
12345.67 98765.43
```
First number: system uptime in seconds
Second number: idle time in seconds (sum across all CPUs)

#### Implementation Strategy

- **Read-only virtual files**: Generated on-demand from kernel structures
- **No caching**: Always return current state
- **Minimal overhead**: Simple string formatting
- **Security**: Only show processes owned by current user (or all for root)


## Components and Interfaces

### Component 1: Mello-Term (Terminal Emulator)

**Responsibilities:**
- Render VT/ANSI escape sequences
- Handle keyboard input and send to PTY master
- Manage scrollback buffer
- Handle window resize events

**Architecture:**

```rust
pub struct MelloTerm {
    pty_master: File,
    screen: ScreenBuffer,
    scrollback: VecDeque<Line>,
    parser: AnsiParser,
}

impl MelloTerm {
    pub fn new() -> Result<Self>;
    pub fn run(&mut self) -> Result<()>;
    fn handle_input(&mut self, key: KeyEvent);
    fn handle_output(&mut self, data: &[u8]);
    fn resize(&mut self, rows: u16, cols: u16);
}
```

**VT/ANSI Support:**
- Cursor movement: `ESC[<n>A/B/C/D` (up/down/right/left)
- Clear screen: `ESC[2J`
- Set color: `ESC[<n>m` (SGR sequences)
- Cursor position: `ESC[<row>;<col>H`
- UTF-8 multi-byte character handling

**UTF-8 Rendering:**
- Use wcwidth-like rules for character width (0, 1, or 2 columns)
- Never split combining character sequences
- Handle wide characters (CJK) as 2-column cells
- Scrollback buffer: limit to 10,000 lines to prevent memory exhaustion
- Evict oldest lines when limit reached

**Resize Flow:**
```
User resizes window
  → MelloTerm.resize(rows, cols)
  → ioctl(pty_master, TIOCSWINSZ, &winsize)
  → Kernel sends SIGWINCH to foreground group
  → Shell/app handles SIGWINCH and redraws
```


### Component 2: Mello-sh (Shell)

**Responsibilities:**
- Parse command lines (pipes, redirects, background)
- Execute built-in commands
- Fork/exec external commands
- Manage job control (fg/bg/jobs)
- Maintain command history

**Architecture:**

```rust
pub struct Shell {
    jobs: Vec<Job>,
    history: Vec<String>,
    env: HashMap<String, String>,
    cwd: PathBuf,
    tty_fd: Option<Fd>,
}

pub struct Job {
    id: JobId,
    pgid: Pid,
    command: String,
    state: JobState,  // Running, Stopped, Done
    processes: Vec<Pid>,
}

impl Shell {
    pub fn run(&mut self) -> Result<()>;
    fn read_line(&mut self) -> Result<String>;
    fn parse(&self, line: &str) -> Result<Command>;
    fn execute(&mut self, cmd: Command) -> Result<i32>;
}
```

**Command Parsing:**

```rust
pub enum Command {
    Simple { args: Vec<String>, background: bool },
    Pipeline { commands: Vec<Command> },
    Redirect { cmd: Box<Command>, input: Option<String>, output: Option<String> },
    Builtin { name: String, args: Vec<String> },
}
```

**Execution Flow:**

1. **Simple Command:**
   ```
   fork() → child: setpgid(0, 0) → execve()
   parent: setpgid(child, child) → tcsetpgrp(tty, child) → wait4()
   ```

2. **Pipeline:**
   ```
   For each command in pipeline:
     pipe() → fork() → child: dup2(pipe) → execve()
   All children in same PGID
   Parent waits for last command
   ```

3. **Background Job:**
   ```
   fork() → setpgid(0, 0) → execve()
   Parent does NOT wait, adds to job list
   ```

**Built-in Commands:**

| Command | Description |
|---------|-------------|
| `cd`    | Change directory |
| `pwd`   | Print working directory |
| `echo`  | Print arguments |
| `export`| Set environment variable |
| `unset` | Unset environment variable |
| `jobs`  | List background jobs |
| `fg`    | Bring job to foreground |
| `bg`    | Resume job in background |
| `exit`  | Exit shell |
| `which` | Show command path |


### Component 3: Mellobox (Coreutils)

**Responsibilities:**
- Provide common UNIX utilities in a single binary
- Support standard command-line options
- Handle UTF-8 text correctly

**Architecture:**

```rust
pub struct Mellobox {
    applets: HashMap<&'static str, AppletFn>,
}

type AppletFn = fn(&[String]) -> Result<i32>;

impl Mellobox {
    pub fn run(args: &[String]) -> Result<i32> {
        let name = args[0].split('/').last().unwrap();
        match name {
            "ls" => ls::main(&args[1..]),
            "cp" => cp::main(&args[1..]),
            // ... other applets
            _ => Err("Unknown applet"),
        }
    }
}
```

**Applet List:**

| Applet | Key Options | Description |
|--------|-------------|-------------|
| `ls`   | `-l -a -h`  | List directory contents |
| `cp`   | `-r -i -v`  | Copy files |
| `mv`   | `-i -v`     | Move files |
| `rm`   | `-r -f -i`  | Remove files |
| `cat`  | `-n`        | Concatenate files |
| `grep` | `-i -r -n`  | Search text |
| `ps`   | `-a -u -x`  | List processes |
| `kill` | `-<sig>`    | Send signal to process |
| `mkdir`| `-p`        | Create directory |
| `touch`| -           | Create empty file |
| `echo` | `-n -e`     | Print text |
| `pwd`  | -           | Print working directory |
| `true` | -           | Exit with 0 |
| `false`| -           | Exit with 1 |

**Implementation Notes:**

- **Multi-call binary**: Single executable with symlinks for each utility
- **Argument parsing**: Use simple getopt-style parser
- **Error handling**: Return appropriate exit codes (0=success, 1=error, 2=usage)
- **UTF-8**: Use Rust's native UTF-8 string handling


## Data Models

### Kernel Data Structures

#### PTY Pair

```rust
pub struct PtyPair {
    pub master: PtyMaster,
    pub slave: PtySlave,
    pub number: u32,
}

pub struct PtyMaster {
    pub buffer: RingBuffer<u8>,
    pub termios: Termios,
    pub winsize: Winsize,
    pub slave_open: bool,
}

pub struct PtySlave {
    pub buffer: RingBuffer<u8>,
    pub session: Option<Sid>,
    pub foreground_pgid: Option<Pid>,
}

pub struct Winsize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}
```

#### Process Control Block Extensions

```rust
pub struct Task {
    // Existing fields...
    pub pid: Pid,
    pub ppid: Pid,
    
    // New fields for job control
    pub pgid: Pid,
    pub sid: Sid,
    pub tty: Option<DeviceId>,
    pub signal_handlers: [SigAction; 64],
    pub pending_signals: u64,
    pub signal_mask: u64,
}

pub struct SigAction {
    pub handler: SigHandler,
    pub mask: u64,
    pub flags: u32,
}

pub enum SigHandler {
    Default,
    Ignore,
    Custom(usize),  // User-space handler address
}
```


### Userland Data Structures

#### Terminal Screen Buffer

```rust
pub struct ScreenBuffer {
    pub rows: u16,
    pub cols: u16,
    pub cells: Vec<Cell>,
    pub cursor: Cursor,
}

pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attributes,  // Bold, underline, etc.
}

pub struct Cursor {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
}
```

#### Shell Job Table

```rust
pub struct JobTable {
    jobs: Vec<Job>,
    next_id: JobId,
}

pub struct Job {
    pub id: JobId,
    pub pgid: Pid,
    pub command: String,
    pub state: JobState,
    pub background: bool,
}

pub enum JobState {
    Running,
    Stopped,
    Done(i32),  // Exit status
}
```


## Error Handling

### Kernel Error Handling

**PTY Subsystem:**
- `ENODEV` - No PTY devices available
- `EIO` - I/O error on PTY
- `EINVAL` - Invalid ioctl request
- `ENOTTY` - File descriptor is not a TTY

**Signal Handling:**
- `EINVAL` - Invalid signal number
- `ESRCH` - Process not found
- `EPERM` - Permission denied

**Job Control:**
- `EPERM` - Not session leader (for TIOCSCTTY)
- `ENOTTY` - Not a controlling terminal
- `EINVAL` - Invalid process group

### Userland Error Handling

**Shell:**
- Command not found → Print error, continue
- Syntax error → Print error, continue
- Exec failure → Print error, exit child with code 127
- Signal during wait → Check if child stopped/continued

**Mellobox:**
- File not found → Print error, exit 1
- Permission denied → Print error, exit 1
- Invalid arguments → Print usage, exit 2

**Terminal:**
- PTY open failure → Fatal error, exit
- Read/write error → Attempt recovery, log error
- Parse error → Skip invalid sequence, continue

### Recovery Strategies

1. **Zombie Prevention:**
   - Shell installs SIGCHLD handler
   - Periodically calls `wait4(-1, WNOHANG)` to reap zombies
   - Kernel reaper thread for orphaned processes

2. **Deadlock Prevention:**
   - Use trylock with timeout for PTY operations
   - Never hold multiple locks in inconsistent order
   - Lock ordering: Session → ProcessGroup → Task

3. **Resource Cleanup:**
   - Close all FDs on exec
   - Free PTY pairs when both sides closed
   - Clean up job table on shell exit


## Testing Strategy

### Unit Tests

**Kernel:**
- PTY allocation/deallocation
- Termios flag handling
- Signal generation from special characters
- Process group management
- /proc file generation

**Userland:**
- Command parsing (pipes, redirects, quotes)
- Job state transitions
- ANSI escape sequence parsing
- Mellobox applet argument parsing

### Integration Tests

**PTY + Terminal:**
1. Open PTY pair
2. Write ANSI sequences to master
3. Verify screen buffer state
4. Resize terminal
5. Verify SIGWINCH delivery

**Shell + Job Control:**
1. Start background job (`sleep 10 &`)
2. Verify job appears in `jobs` list
3. Send SIGTSTP to foreground job (Ctrl-Z)
4. Verify job state changes to Stopped
5. Resume with `fg` or `bg`
6. Verify SIGCONT delivery

**Pipeline Execution:**
1. Execute `echo hello | grep h | wc -l`
2. Verify all processes in same PGID
3. Verify correct output
4. Verify exit status propagation

### End-to-End Tests

**Scenario 1: Interactive Session**
```bash
# Boot system
# Open mello-term
# Verify prompt appears within 10ms
ls /
cd /proc
cat /proc/self/stat
ps aux
```

**Scenario 2: Job Control**
```bash
sleep 100 &
jobs
# Press Ctrl-Z on foreground job
jobs
fg %1
# Press Ctrl-C
jobs
```

**Scenario 3: Pipeline & Redirect**
```bash
echo "hello world" > test.txt
cat test.txt | tr a-z A-Z > output.txt
cat output.txt
# Verify: "HELLO WORLD"
```

### Performance Tests

**Benchmarks:**
- Shell startup time: `time mello-sh -c 'exit'`
- Process spawn: `time mello-sh -c 'for i in {1..100}; do /bin/true; done'`
- Pipe throughput: `dd if=/dev/zero bs=1M count=100 | cat > /dev/null`
- Directory listing: `time ls -la /usr/bin` (1000+ entries)
- Syscall latency: `read()/write()` on 4 KiB buffer (hot cache)

**Targets:**
- Shell startup: < 10ms
- Spawn /bin/true: < 2ms per iteration
- Pipe throughput: > 200 MB/s
- ls 1000 entries: < 80ms
- Syscall latency: < 5µs median

**Test Environment:**
- KVM virtualization
- 4 vCPUs
- CPU governor: performance
- Payload: 4 KiB hot cache for latency tests


## SMP Safety and Concurrency

### Lock Hierarchy

To prevent deadlocks, locks must be acquired in this order:

```
1. Global PTY table lock
2. Session lock
3. Process group lock
4. Task lock
5. PTY pair lock
```

**Rule:** Never acquire a lock at a higher level while holding a lock at a lower level.

### Per-CPU Data Structures

To reduce contention, use per-CPU structures where possible:

```rust
pub struct PerCpuScheduler {
    run_queue: VecDeque<Pid>,
    idle_task: Pid,
}

pub struct PerCpuStats {
    context_switches: u64,
    signals_delivered: u64,
    syscalls: u64,
}
```

### Lock-Free Operations

**Read-mostly data:**
- Use RCU (Read-Copy-Update) for process group membership
- Atomic operations for reference counts
- Seqlock for /proc reads

**Signal delivery:**
- Atomic bitset for pending signals
- Lock-free queue for signal info

### TLB Shootdown

When modifying page tables on multicore:

```rust
fn unmap_user_pages(task: &Task, vaddr: VirtAddr, len: usize) {
    // 1. Mark pages invalid in page table
    task.page_table.unmap(vaddr, len);
    
    // 2. Send IPI to all CPUs running this task
    let cpus = task.cpu_affinity_mask;
    send_tlb_shootdown_ipi(cpus, vaddr, len);
    
    // 3. Wait for acknowledgment
    wait_for_tlb_ack(cpus);
    
    // 4. Free physical pages
    pmm_free(pages);
}
```

### Race Condition Prevention

**PTY resize race:**
```rust
// WRONG: Race between resize and read
fn resize(pty: &mut Pty, size: Winsize) {
    pty.winsize = size;  // ← Another thread might read here
    send_sigwinch(pty.foreground_pgid);
}

// CORRECT: Atomic update
fn resize(pty: &mut Pty, size: Winsize) {
    let _lock = pty.lock.lock();
    pty.winsize = size;
    send_sigwinch(pty.foreground_pgid);
}
```

**Job control race:**
```rust
// WRONG: TOCTOU between check and use
if is_foreground(pgid) {
    // ← pgid might change here
    send_signal(pgid, SIGINT);
}

// CORRECT: Hold lock across check and use
let _lock = session.lock();
if session.foreground_pgid == Some(pgid) {
    send_signal(pgid, SIGINT);
}
```


## Security Considerations

### Memory Protection

**User/Kernel Separation:**
- User pages: U=1, kernel pages: U=0
- Kernel validates all user pointers before dereferencing
- Use `copy_from_user()` / `copy_to_user()` for all user data access

**W^X Enforcement:**
- Code pages: R+X, no W
- Data pages: R+W, no X
- Stack: R+W, no X (with NX bit)

**Validation Example:**
```rust
fn copy_from_user<T>(user_ptr: usize) -> Result<T> {
    // 1. Check alignment
    if user_ptr % align_of::<T>() != 0 {
        return Err(EINVAL);
    }
    
    // 2. Check bounds
    let task = current_task();
    if !task.vm.is_user_address(user_ptr, size_of::<T>()) {
        return Err(EFAULT);
    }
    
    // 3. Check permissions
    if !task.vm.is_readable(user_ptr) {
        return Err(EFAULT);
    }
    
    // 4. Safe to read
    Ok(unsafe { *(user_ptr as *const T) })
}
```

### Ioctl Validation

```rust
fn pty_ioctl(fd: &File, cmd: u32, arg: usize) -> Result<i32> {
    match cmd {
        TCGETS => {
            let termios_ptr = arg as *mut Termios;
            validate_user_write(termios_ptr)?;
            copy_to_user(termios_ptr, &fd.pty.termios)?;
            Ok(0)
        }
        TIOCSWINSZ => {
            let winsize_ptr = arg as *const Winsize;
            validate_user_read(winsize_ptr)?;
            let winsize = copy_from_user(winsize_ptr)?;
            fd.pty.set_winsize(winsize)?;
            Ok(0)
        }
        _ => Err(EINVAL),
    }
}
```

### Signal Security

**Restrictions:**
- Process can only send signals to processes in same session (unless root)
- Cannot send SIGKILL/SIGSTOP to PID 1 (init)
- Signal handlers must be in user-space code pages

**Validation:**
```rust
fn sys_kill(pid: Pid, sig: Signal) -> Result<()> {
    let sender = current_task();
    let target = find_task(pid)?;
    
    // Check permission
    if sender.uid != 0 && sender.uid != target.uid {
        return Err(EPERM);
    }
    
    // Check session
    if sender.sid != target.sid && sender.uid != 0 {
        return Err(EPERM);
    }
    
    // Send signal
    target.send_signal(sig)
}
```


## Observability and Debugging

### Logging Framework

**Structured Logging:**
```rust
log!("[cpu{}][pid={}][pty] Allocated PTY pair {}", 
     cpu_id(), current_pid(), pty_num);

log!("[cpu{}][pid={}][signal] Delivered {} to PGID {}", 
     cpu_id(), current_pid(), sig, pgid);
```

**Log Levels:**
- `ERROR`: Critical failures (panic imminent)
- `WARN`: Recoverable errors
- `INFO`: Important state changes
- `DEBUG`: Detailed execution flow
- `TRACE`: Very verbose (disabled in release)

### Metrics Collection

**Kernel Metrics:**
```rust
pub struct SystemMetrics {
    pub context_switches: AtomicU64,
    pub signals_delivered: AtomicU64,
    pub pty_bytes_in: AtomicU64,
    pub pty_bytes_out: AtomicU64,
    pub syscalls: [AtomicU64; 512],  // Per-syscall counters
}
```

**Exposed via /proc:**
- `/proc/stat` - System-wide statistics
- `/proc/<pid>/stat` - Per-process statistics
- `/proc/interrupts` - Interrupt counters

### Panic Dump

When kernel panics, dump critical state:

```rust
fn panic_handler(info: &PanicInfo) {
    let cpu = cpu_id();
    let task = current_task_or_null();
    
    serial_println!("KERNEL PANIC on CPU {}", cpu);
    serial_println!("Message: {}", info);
    
    if let Some(task) = task {
        serial_println!("Task: PID={} PGID={} SID={}", 
                       task.pid, task.pgid, task.sid);
        serial_println!("TTY: {:?}", task.tty);
        serial_println!("Last syscall: {:?}", task.last_syscall);
    }
    
    let regs = get_saved_registers();
    serial_println!("RIP: {:#x}", regs.rip);
    serial_println!("RSP: {:#x}", regs.rsp);
    serial_println!("CR2: {:#x}", read_cr2());
    
    dump_stack_trace();
    halt();
}
```

### Debug Commands

**Shell built-ins for debugging:**
- `debug-pty` - Show PTY state
- `debug-jobs` - Show job table
- `debug-signals` - Show pending signals

**Kernel debug interface:**
- `/proc/debug/pty` - PTY allocation table
- `/proc/debug/sessions` - Session/PGID tree
- `/proc/debug/locks` - Lock contention stats


## Implementation Phases

### Phase 1: PTY Subsystem (Milestone 6.6.1)

**Deliverables:**
- `/dev/ptmx` and `/dev/pts/<n>` device nodes
- PTY allocation and deallocation
- Basic termios support (ICANON, ECHO)
- ioctl: TCGETS, TCSETS, TIOCGWINSZ, TIOCSWINSZ
- SIGWINCH generation on resize

**Acceptance:**
- Can open PTY pair
- Can read/write through PTY
- Resize triggers SIGWINCH

### Phase 2: Signals & Job Control (Milestone 6.6.2)

**Deliverables:**
- Process group and session management
- System calls: setpgid, getpgrp, tcsetpgrp, tcgetpgrp
- Signal delivery to foreground group
- SIGCHLD, SIGINT, SIGTSTP, SIGCONT handling

**Acceptance:**
- Can create process groups
- Keyboard signals go to foreground group only
- Job control primitives work

### Phase 3: Shell Core (Milestone 6.6.3)

**Deliverables:**
- Command parser (pipes, redirects, background)
- Built-in commands
- Job table management
- Command history
- Prompt rendering

**Acceptance:**
- Can execute simple commands
- Pipes work: `echo hi | cat`
- Redirects work: `echo hi > file`
- Background jobs work: `sleep 10 &`
- Built-ins work: `cd`, `jobs`, `fg`, `bg`

### Phase 4: Mellobox Coreutils (Milestone 6.6.4)

**Deliverables:**
- Multi-call binary framework
- Core utilities: ls, cp, mv, rm, cat, grep, ps, kill
- Argument parsing
- UTF-8 support

**Acceptance:**
- All utilities execute correctly
- Options work as expected
- Exit codes correct
- UTF-8 text handled properly

### Phase 5: Terminal Emulator (Milestone 6.6.5)

**Deliverables:**
- VT/ANSI parser and renderer
- Screen buffer management
- Scrollback
- Resize handling
- Copy/paste

**Acceptance:**
- Can render shell prompt
- ANSI colors work
- Resize works end-to-end
- Interactive shell usable

### Phase 6: /proc Filesystem & E2E Tests (Milestone 6.6.6)

**Deliverables:**
- /proc/<pid>/* files
- /proc/meminfo, cpuinfo, uptime
- End-to-end integration tests
- Performance benchmarks
- Stability testing

**Acceptance:**
- All /proc files readable
- No kernel panics
- No zombie processes
- Performance targets met
- System stable under load


## Dependencies and Prerequisites

### Existing Infrastructure (from previous phases)

**Required:**
- ✅ SMP support (Phase 5) - Multi-core scheduling and synchronization
- ✅ User mode support (Phase 6.5) - ELF loading, syscalls, user/kernel separation
- ✅ Memory management - Virtual memory, page tables, heap allocator
- ✅ Scheduler - Task switching, priority scheduling
- ✅ Basic syscalls - fork, execve, wait4, read, write, open, close

**Assumptions:**
- Kernel can load and execute ELF binaries
- User-space memory is properly isolated
- System calls work reliably
- SMP synchronization primitives (spinlocks, atomics) are available

### New Dependencies

**Kernel:**
- Device driver framework (for PTY devices)
- Virtual filesystem layer (for /proc)
- Signal infrastructure (if not already present)
- Ioctl dispatch mechanism

**Userland:**
- Rust standard library (or no_std with alloc)
- Basic libc functions (if using C)
- UTF-8 string handling
- ANSI terminal library (or implement from scratch)

### Build System Changes

**Kernel:**
```makefile
# Add PTY subsystem
KERNEL_SOURCES += kernel/src/dev/pty.rs
KERNEL_SOURCES += kernel/src/fs/proc.rs
KERNEL_SOURCES += kernel/src/signal.rs
```

**Userland:**
```makefile
# Build userland programs
USERLAND_BINS = mello-term mello-sh mellobox

mello-term: userspace/mello-term/src/*.rs
	cargo build --release --target x86_64-unknown-none

mello-sh: userspace/mello-sh/src/*.rs
	cargo build --release --target x86_64-unknown-none

mellobox: userspace/mellobox/src/*.rs
	cargo build --release --target x86_64-unknown-none
```

### External Libraries

**Optional (for faster development):**
- `termion` or `crossterm` - Terminal handling (if porting to hosted environment)
- `nix` - POSIX bindings (for reference implementation)
- `clap` - Argument parsing for mellobox

**Not needed (implement from scratch):**
- VT/ANSI parser - Simple state machine
- Shell parser - Recursive descent parser
- Coreutils - Straightforward implementations


## Risks and Mitigations

### Risk 1: PTY Deadlock

**Description:** Master and slave sides of PTY can deadlock if both buffers fill up.

**Scenario:**
```
Master writes → Slave buffer full → Master blocks
Slave writes → Master buffer full → Slave blocks
→ Deadlock
```

**Mitigation:**
- Use non-blocking I/O with EAGAIN
- Implement flow control (IXON/IXOFF)
- Set reasonable buffer sizes (4KB default)
- Use trylock with timeout for PTY operations

### Risk 2: Signal Race Conditions

**Description:** Race between signal delivery and process state changes.

**Scenario:**
```
Thread A: Checks if process is running
Thread B: Process exits
Thread A: Sends signal to exited process
→ Signal lost or sent to wrong process
```

**Mitigation:**
- Hold process lock during signal delivery
- Use atomic operations for process state
- Validate PID before signal delivery
- Handle ESRCH gracefully

### Risk 3: Zombie Process Accumulation

**Description:** If shell doesn't reap children, zombies accumulate.

**Mitigation:**
- Install SIGCHLD handler in shell
- Periodically call `wait4(-1, WNOHANG)`
- Kernel reaper thread for orphaned processes
- Monitor zombie count in tests

### Risk 4: SIGWINCH Flood

**Description:** Rapid terminal resizing can flood system with SIGWINCH.

**Mitigation:**
- Debounce resize events (100ms window)
- Coalesce multiple resizes into one signal
- Rate-limit signal delivery
- Use signal queuing (not just pending bit)

### Risk 5: UTF-8 Corruption

**Description:** Multi-byte UTF-8 sequences can be split across buffer boundaries.

**Mitigation:**
- Never split UTF-8 sequences
- Buffer incomplete sequences at boundary
- Validate UTF-8 on input
- Use Rust's native UTF-8 handling

### Risk 6: Performance Regression

**Description:** Lock contention or inefficient algorithms slow down system.

**Mitigation:**
- Use per-CPU data structures
- Profile hot paths
- Benchmark regularly
- Set performance targets in CI

### Risk 7: Security Vulnerabilities

**Description:** Improper validation of user input can lead to exploits.

**Mitigation:**
- Validate all user pointers
- Check buffer bounds
- Use safe Rust where possible
- Fuzz test ioctl handlers
- Security review before release


## Future Enhancements (Out of Scope for Phase 6.6)

### Phase 7: Native zsh Port

- Port zsh to MelloOS
- Static linking with musl libc
- Full POSIX compatibility layer
- Advanced shell features (completion, themes)

### Phase 8: Terminfo Support

- Terminfo database
- Terminal capability detection
- Support for various terminal types
- ncurses library port

### Phase 9: Persistent Filesystem

- Disk-backed filesystem (ext2/ext4)
- Mount/unmount support
- File permissions and ownership
- Symbolic links and hard links

### Phase 10: Desktop Environment

- Window manager
- GUI toolkit
- Desktop applications
- mello-term as GUI terminal app

### Additional Features

**Shell enhancements:**
- Tab completion
- Syntax highlighting
- Command suggestions
- Persistent history (requires filesystem)
- Configuration files (.mellorc)

**Terminal enhancements:**
- Multiple tabs
- Split panes
- Themes and color schemes
- Font rendering (requires GUI)
- Ligature support

**Coreutils additions:**
- More utilities (find, sed, awk, tar, gzip)
- Advanced options
- Performance optimizations
- Compatibility with GNU coreutils

**System features:**
- User management (login, su, sudo)
- Permissions and ACLs
- Audit logging
- System monitoring tools (top, htop)


## Conclusion

Phase 6.6 represents a major milestone in MelloOS development, transforming it from a basic kernel that can execute binaries into a fully interactive operating system with a usable terminal environment. The design balances POSIX compatibility with implementation simplicity, focusing on core functionality that enables daily use.

### Key Design Decisions

1. **PTY-based architecture**: Standard UNIX approach, well-understood and portable
2. **Multi-call binary for coreutils**: Reduces binary size and simplifies distribution
3. **Native Rust implementation**: Leverages memory safety and modern language features
4. **Minimal dependencies**: Implement from scratch to maintain control and learning
5. **Performance-first**: Target sub-10ms latencies for interactive feel
6. **SMP-safe from start**: Avoid retrofitting concurrency later

### Success Criteria

The phase is successful when:
- ✅ User can boot and interact with shell through terminal
- ✅ All basic UNIX operations work (ls, cd, pipes, redirects)
- ✅ Job control is fully functional (fg, bg, Ctrl-Z, Ctrl-C)
- ✅ System is stable (no panics, no zombies, no deadlocks)
- ✅ Performance targets are met
- ✅ UTF-8 works correctly throughout

### Handoff to Phase 7

Upon completion, the system will be ready for:
- Native zsh porting (requires stable PTY and signals)
- Terminfo integration (requires working terminal emulator)
- Filesystem development (shell and coreutils provide testing tools)
- Desktop environment (mello-term becomes GUI application)

This design provides a solid foundation for future development while delivering immediate value through a usable terminal environment.

### File Descriptor Management

#### FD Flags and Non-blocking I/O

```rust
// fcntl operations
fn fcntl(fd: Fd, cmd: u32, arg: usize) -> Result<i32> {
    match cmd {
        F_GETFD => Ok(fd.flags & FD_CLOEXEC),
        F_SETFD => {
            fd.flags = (fd.flags & !FD_CLOEXEC) | (arg & FD_CLOEXEC);
            Ok(0)
        }
        F_GETFL => Ok(fd.status_flags),
        F_SETFL => {
            fd.status_flags = arg & (O_NONBLOCK | O_APPEND);
            Ok(0)
        }
        _ => Err(EINVAL),
    }
}

// pipe2 with flags
fn pipe2(flags: u32) -> Result<(Fd, Fd)> {
    let (read_fd, write_fd) = create_pipe()?;
    
    if flags & O_CLOEXEC != 0 {
        read_fd.flags |= FD_CLOEXEC;
        write_fd.flags |= FD_CLOEXEC;
    }
    
    if flags & O_NONBLOCK != 0 {
        read_fd.status_flags |= O_NONBLOCK;
        write_fd.status_flags |= O_NONBLOCK;
    }
    
    Ok((read_fd, write_fd))
}
```

**FD_CLOEXEC Behavior:**
- Set on FD → closed automatically on `execve()`
- Prevents FD leaks to child processes
- Shell should set on internal FDs (job table, history file)

**O_NONBLOCK Behavior:**
- Read returns EAGAIN if no data available
- Write returns EAGAIN if buffer full
- Prevents deadlock in PTY master/slave interaction

