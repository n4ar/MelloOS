# /proc Filesystem Architecture

## Overview

The /proc filesystem is a virtual filesystem that provides an interface to kernel data structures. It exposes process information, system statistics, and debugging interfaces through a file-based API.

## Architecture

### Component Structure

```
┌─────────────────────────────────────────────────┐
│         User Space Applications                 │
│    (ps, top, shell, monitoring tools)           │
└────────────────┬────────────────────────────────┘
                 │ read(), readdir(), stat()
                 ↓
┌─────────────────────────────────────────────────┐
│           VFS Layer                             │
│    (File operations dispatch)                   │
└────────────────┬────────────────────────────────┘
                 │
                 ↓
┌─────────────────────────────────────────────────┐
│         /proc Filesystem Driver                 │
│  ┌──────────────────────────────────────────┐  │
│  │    Directory Structure Generator         │  │
│  │  - /proc root                            │  │
│  │  - /proc/<pid> directories               │  │
│  │  - /proc/sys hierarchy                   │  │
│  └──────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────┐  │
│  │    File Content Generators               │  │
│  │  - Process info formatters               │  │
│  │  - System info formatters                │  │
│  │  - Dynamic content generation            │  │
│  └──────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────┐  │
│  │    Data Source Interfaces                │  │
│  │  - Task list access                      │  │
│  │  - Memory manager queries                │  │
│  │  - CPU info queries                      │  │
│  └──────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

## Directory Structure

```
/proc/
├── <pid>/                  # Per-process directories
│   ├── stat               # Process status (Linux-compatible)
│   ├── cmdline            # Command line (null-separated)
│   ├── status             # Human-readable status
│   ├── maps               # Memory mappings (future)
│   └── fd/                # File descriptors (future)
├── self -> <pid>          # Symlink to current process
├── meminfo                # Memory statistics
├── cpuinfo                # CPU information
├── uptime                 # System uptime
├── stat                   # System statistics
├── version                # Kernel version
└── debug/                 # Debug interfaces
    ├── pty                # PTY allocation table
    ├── sessions           # Session/PGID tree
    └── locks              # Lock contention stats
```

## File Formats

### /proc/<pid>/stat

**Format:** Space-separated fields (Linux-compatible)

```
pid (comm) state ppid pgrp session tty_nr tpgid flags minflt cminflt majflt cmajflt utime stime cutime cstime priority nice num_threads itrealvalue starttime vsize rss rsslim ...
```

**Key Fields:**
- `pid`: Process ID
- `comm`: Command name (in parentheses)
- `state`: R (running), S (sleeping), Z (zombie), T (stopped)
- `ppid`: Parent process ID
- `pgrp`: Process group ID
- `session`: Session ID
- `tty_nr`: Controlling terminal device number (0 if none)
- `tpgid`: Foreground process group of controlling terminal
- `utime`: User mode time (clock ticks)
- `stime`: Kernel mode time (clock ticks)
- `priority`: Scheduling priority
- `nice`: Nice value
- `num_threads`: Number of threads
- `starttime`: Time process started (clock ticks since boot)
- `vsize`: Virtual memory size (bytes)
- `rss`: Resident set size (pages)

**Example:**
```
123 (mello-sh) S 1 123 123 34816 123 0 0 0 0 0 150 50 0 0 20 0 1 0 1234 4096000 512 18446744073709551615
```

### /proc/<pid>/cmdline

**Format:** Null-separated arguments

```
arg0\0arg1\0arg2\0
```

**Example:**
```
/bin/mello-sh\0-c\0echo hello\0
```

### /proc/<pid>/status

**Format:** Human-readable key-value pairs

```
Name:   mello-sh
State:  S (sleeping)
Pid:    123
PPid:   1
Pgid:   123
Sid:    123
TTY:    pts/0
Uid:    1000
Gid:    1000
VmSize: 4096 kB
VmRSS:  2048 kB
Threads: 1
```

### /proc/meminfo

**Format:** Key-value pairs with units

```
MemTotal:     1048576 kB
MemFree:       524288 kB
MemAvailable:  786432 kB
Buffers:        65536 kB
Cached:        131072 kB
SwapTotal:          0 kB
SwapFree:           0 kB
```

### /proc/cpuinfo

**Format:** Key-value pairs per CPU

```
processor       : 0
vendor_id       : GenuineIntel
cpu family      : 6
model           : 142
model name      : Intel(R) Core(TM) i7-8550U CPU @ 1.80GHz
stepping        : 10
cpu MHz         : 1800.000
cache size      : 8192 KB
physical id     : 0
siblings        : 4
core id         : 0
cpu cores       : 4
flags           : fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx rdtscp lm

processor       : 1
...
```

### /proc/uptime

**Format:** Two floating-point numbers

```
12345.67 98765.43
```

- First: System uptime in seconds
- Second: Idle time in seconds (sum across all CPUs)

### /proc/stat

**Format:** Various statistics

```
cpu  150 50 100 10000 0 0 0 0 0 0
cpu0 40 10 25 2500 0 0 0 0 0 0
cpu1 35 15 25 2500 0 0 0 0 0 0
cpu2 40 10 25 2500 0 0 0 0 0 0
cpu3 35 15 25 2500 0 0 0 0 0 0
intr 12345 100 200 300
ctxt 567890
btime 1234567890
processes 1234
procs_running 2
procs_blocked 0
```

## Implementation

### File Operations

```rust
pub struct ProcFile {
    pub name: &'static str,
    pub read: fn(&mut [u8]) -> Result<usize>,
    pub readdir: Option<fn() -> Vec<DirEntry>>,
}

impl ProcFile {
    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        (self.read)(buf)
    }
}
```

### Dynamic Content Generation

```rust
fn generate_stat_file(pid: Pid, buf: &mut [u8]) -> Result<usize> {
    let task = find_task(pid)?;
    
    // Use seqlock for consistent read
    let seq = task.seqlock.read_begin();
    let snapshot = TaskSnapshot {
        pid: task.pid,
        ppid: task.ppid,
        pgid: task.pgid,
        sid: task.sid,
        state: task.state,
        // ... other fields
    };
    if !task.seqlock.read_retry(seq) {
        return Err(EAGAIN);  // Retry
    }
    
    // Format output
    let output = format!(
        "{} ({}) {} {} {} {} {} {} ...",
        snapshot.pid,
        snapshot.comm,
        snapshot.state,
        snapshot.ppid,
        snapshot.pgid,
        snapshot.sid,
        snapshot.tty_nr,
        snapshot.tpgid,
    );
    
    buf[..output.len()].copy_from_slice(output.as_bytes());
    Ok(output.len())
}
```

### Directory Enumeration

```rust
fn readdir_proc_root() -> Vec<DirEntry> {
    let mut entries = vec![
        DirEntry::new(".", DT_DIR),
        DirEntry::new("..", DT_DIR),
        DirEntry::new("self", DT_LNK),
        DirEntry::new("meminfo", DT_REG),
        DirEntry::new("cpuinfo", DT_REG),
        DirEntry::new("uptime", DT_REG),
        DirEntry::new("stat", DT_REG),
        DirEntry::new("version", DT_REG),
        DirEntry::new("debug", DT_DIR),
    ];
    
    // Add per-process directories
    for task in all_tasks() {
        entries.push(DirEntry::new(
            &task.pid.to_string(),
            DT_DIR
        ));
    }
    
    entries
}
```

## Data Access Patterns

### Lock-Free Reads

Use seqlock for reading process state without holding locks:

```rust
pub struct Task {
    pub seqlock: SeqLock,
    // ... fields
}

// Reader (in /proc file generation)
loop {
    let seq = task.seqlock.read_begin();
    let data = read_task_data(task);
    if task.seqlock.read_retry(seq) {
        break data;
    }
    // Retry if data changed during read
}

// Writer (in kernel)
task.seqlock.write_begin();
modify_task_data(task);
task.seqlock.write_end();
```

### Handling Races

```rust
fn read_proc_file(pid: Pid) -> Result<String> {
    // Process might exit during read
    match find_task(pid) {
        Some(task) => {
            // Use seqlock for consistent snapshot
            generate_content(task)
        }
        None => Err(ESRCH),  // Process exited
    }
}
```

## Security

### Permission Checks

```rust
fn check_proc_access(pid: Pid, accessor: &Task) -> Result<()> {
    let target = find_task(pid)?;
    
    // Root can access everything
    if accessor.uid == 0 {
        return Ok(());
    }
    
    // Can only access own processes
    if accessor.uid != target.uid {
        return Err(EPERM);
    }
    
    Ok(())
}
```

### Sensitive Information

- Hide processes from other users (unless root)
- Sanitize kernel pointers in output
- Limit access to debug files (root only)

## Performance Considerations

### Caching Strategy

- **No caching**: Always generate fresh data
- **Rationale**: Process state changes frequently
- **Optimization**: Use seqlock for consistent reads without locks

### Scalability

- **O(1) lookup**: Hash table for PID → Task
- **O(n) enumeration**: Linear scan for readdir (acceptable for /proc)
- **Lazy generation**: Only format data when read

### Memory Usage

- **Zero persistent storage**: All data generated on-demand
- **Stack allocation**: Use stack buffers for formatting
- **No allocations**: Avoid heap allocations in read path

## Debug Interfaces

### /proc/debug/pty

```
PTY  Master  Slave  FG_PGID  Buf_In  Buf_Out
0    open    open   123      0/4096  0/4096
1    open    closed -        0/4096  0/4096
2    closed  closed -        0/4096  0/4096
```

### /proc/debug/sessions

```
Session 1 (leader: 1)
  PG 1: [1]
  
Session 123 (leader: 123, tty: pts/0, fg: 125)
  PG 123: [123]
  PG 125: [125, 126, 127]
  PG 130: [130]
```

### /proc/debug/locks

```
Lock                    Acquisitions  Contentions  Max_Wait_us
global_task_list        12345         10           150
pty_table               5678          2            50
session_table           3456          0            0
```

## Integration Points

### With ps Utility

```rust
// ps reads /proc to enumerate processes
fn list_processes() -> Vec<ProcessInfo> {
    let mut procs = Vec::new();
    
    for entry in readdir("/proc") {
        if let Ok(pid) = entry.name.parse::<Pid>() {
            let stat = read_file(&format!("/proc/{}/stat", pid))?;
            let info = parse_stat(stat)?;
            procs.push(info);
        }
    }
    
    procs
}
```

### With Shell

```rust
// Shell uses /proc/self for introspection
fn get_own_pgid() -> Pid {
    let stat = read_file("/proc/self/stat")?;
    parse_pgid_from_stat(stat)
}
```

### With Monitoring Tools

```rust
// Monitor system resources
fn get_memory_usage() -> MemInfo {
    let meminfo = read_file("/proc/meminfo")?;
    parse_meminfo(meminfo)
}

fn get_cpu_usage() -> Vec<CpuStat> {
    let stat = read_file("/proc/stat")?;
    parse_cpu_stats(stat)
}
```

## Error Handling

| Error Code | Condition |
|------------|-----------|
| ESRCH      | Process does not exist |
| EPERM      | Permission denied |
| ENOENT     | File does not exist |
| EAGAIN     | Retry (seqlock conflict) |
| EINVAL     | Invalid argument |

## State Consistency

### Snapshot Guarantees

- **Per-file consistency**: Each file read is atomic
- **Cross-file consistency**: Not guaranteed (process state may change)
- **Retry mechanism**: Seqlock ensures consistent snapshots

### Example: Consistent Read

```rust
// Reading /proc/<pid>/stat
let seq = task.seqlock.read_begin();
let pid = task.pid;
let ppid = task.ppid;
let state = task.state;
// ... read other fields
if !task.seqlock.read_retry(seq) {
    // Data changed during read, retry
    return Err(EAGAIN);
}
// Data is consistent
```

## Future Enhancements

- **/proc/<pid>/maps**: Memory mappings
- **/proc/<pid>/fd/**: File descriptor symlinks
- **/proc/<pid>/environ**: Environment variables
- **/proc/sys/**: Kernel parameters (sysctl)
- **/proc/net/**: Network statistics
- **/proc/filesystems**: Supported filesystems
- **Write support**: For /proc/sys configuration
- **Binary formats**: For performance-critical data

## Best Practices

### For Kernel Developers

1. **Use seqlock**: For lock-free reads of process state
2. **Handle races**: Process may exit during read
3. **Minimize allocations**: Use stack buffers
4. **Format efficiently**: Avoid unnecessary string operations
5. **Check permissions**: Enforce security policies

### For Application Developers

1. **Parse carefully**: Handle format variations
2. **Retry on EAGAIN**: Seqlock conflicts are transient
3. **Cache sparingly**: /proc data changes frequently
4. **Handle ESRCH**: Process may exit between readdir and read
5. **Use /proc/self**: For introspection

## Debugging

### Logging

```
[cpu0][pid=123][proc] Read /proc/123/stat
[cpu1][pid=124][proc] Readdir /proc
[cpu0][pid=123][proc] Process 125 exited during read (ESRCH)
```

### Metrics

- Reads per file type
- EAGAIN retry count
- ESRCH error count
- Average read latency

## Performance Targets

- **File read latency**: < 50 µs
- **Directory enumeration**: < 1 ms for 100 processes
- **Memory overhead**: 0 bytes (no caching)
- **CPU overhead**: < 1% for typical workloads
