# PTY Subsystem Architecture

## Overview

The Pseudo-Terminal (PTY) subsystem provides terminal emulation capabilities for MelloOS, enabling interactive shell sessions and terminal-based applications. It implements a master/slave architecture compatible with POSIX terminal semantics.

## Architecture

### Components

```
┌─────────────────────────────────────────────────┐
│         Terminal Emulator (mello-term)          │
│              User Space Application              │
└────────────────┬────────────────────────────────┘
                 │ PTY Master (/dev/ptmx)
                 │ read/write/ioctl
                 ↓
┌─────────────────────────────────────────────────┐
│           Kernel PTY Subsystem                  │
│  ┌──────────────────────────────────────────┐  │
│  │         PTY Master Side                  │  │
│  │  - Master file operations                │  │
│  │  - Master ring buffer                    │  │
│  │  - ioctl handlers                        │  │
│  └──────────────┬───────────────────────────┘  │
│                 │                                │
│  ┌──────────────▼───────────────────────────┐  │
│  │      Line Discipline Layer               │  │
│  │  - Canonical/Raw mode processing         │  │
│  │  - Echo handling                         │  │
│  │  - Signal generation (SIGINT, SIGTSTP)   │  │
│  │  - Special character processing          │  │
│  └──────────────┬───────────────────────────┘  │
│                 │                                │
│  ┌──────────────▼───────────────────────────┐  │
│  │         PTY Slave Side                   │  │
│  │  - Slave file operations                 │  │
│  │  - Slave ring buffer                     │  │
│  │  - Foreground process group tracking     │  │
│  └──────────────────────────────────────────┘  │
└────────────────┬────────────────────────────────┘
                 │ PTY Slave (/dev/pts/N)
                 │ read/write
                 ↓
┌─────────────────────────────────────────────────┐
│              Shell (mello-sh)                   │
│              User Space Application              │
└─────────────────────────────────────────────────┘
```

### Data Structures

#### PTY Pair

```rust
pub struct PtyPair {
    pub number: u32,              // PTY number (N in /dev/pts/N)
    pub master: PtyMaster,        // Master side state
    pub slave: PtySlave,          // Slave side state
    pub termios: Termios,         // Terminal settings
    pub winsize: Winsize,         // Window size
    pub lock: SpinLock<()>,       // Synchronization
}
```

#### Termios Structure

```rust
pub struct Termios {
    pub c_iflag: u32,    // Input modes
    pub c_oflag: u32,    // Output modes
    pub c_cflag: u32,    // Control modes
    pub c_lflag: u32,    // Local modes
    pub c_cc: [u8; 32],  // Control characters
}
```

**Key Flags:**
- `ICANON`: Canonical mode (line buffering)
- `ECHO`: Echo input characters
- `ISIG`: Generate signals for special characters
- `ICRNL`: Map CR to NL on input
- `ONLCR`: Map NL to CRNL on output

**Control Characters:**
- `VINTR` (Ctrl-C): Generate SIGINT
- `VSUSP` (Ctrl-Z): Generate SIGTSTP
- `VEOF` (Ctrl-D): End of file
- `VERASE` (Backspace): Erase character

## Data Flow

### Write Path (Terminal → Shell)

```
1. User types in terminal emulator
2. Terminal writes to PTY master
3. Data enters master → slave buffer
4. Line discipline processes:
   - Canonical mode: buffer until newline
   - Echo: copy to slave → master buffer
   - Special chars: generate signals
5. Shell reads from PTY slave
```

### Read Path (Shell → Terminal)

```
1. Shell writes to PTY slave
2. Data enters slave → master buffer
3. Line discipline processes:
   - Output processing (OPOST)
   - NL to CRNL conversion (ONLCR)
4. Terminal reads from PTY master
5. Terminal renders to screen
```

## Device Nodes

### /dev/ptmx (Master Multiplexer)

- Single device node for all PTY allocations
- `open()` allocates a new PTY pair
- Returns file descriptor for master side
- Use `ioctl(TIOCGPTN)` to get slave number

### /dev/pts/N (Slave Devices)

- Dynamically created for each PTY pair
- N is the PTY number (0, 1, 2, ...)
- Opened by shell/application
- Becomes controlling terminal via `ioctl(TIOCSCTTY)`

## Operations

### PTY Allocation

```rust
// User space
let master_fd = open("/dev/ptmx", O_RDWR);
let mut slave_num: u32 = 0;
ioctl(master_fd, TIOCGPTN, &mut slave_num);
let slave_path = format!("/dev/pts/{}", slave_num);
let slave_fd = open(&slave_path, O_RDWR);
```

### Terminal Configuration

```rust
// Get current settings
let mut termios = Termios::default();
ioctl(fd, TCGETS, &mut termios);

// Modify settings
termios.c_lflag |= ICANON | ECHO;
termios.c_cc[VINTR] = 3;  // Ctrl-C

// Apply settings
ioctl(fd, TCSETS, &termios);
```

### Window Size Management

```rust
// Set window size
let winsize = Winsize {
    ws_row: 24,
    ws_col: 80,
    ws_xpixel: 0,
    ws_ypixel: 0,
};
ioctl(master_fd, TIOCSWINSZ, &winsize);
// Kernel sends SIGWINCH to foreground process group
```

## Signal Generation

The PTY subsystem generates signals based on special characters:

| Character | Signal   | Condition        | Target              |
|-----------|----------|------------------|---------------------|
| Ctrl-C    | SIGINT   | ISIG flag set    | Foreground PGID     |
| Ctrl-Z    | SIGTSTP  | ISIG flag set    | Foreground PGID     |
| Ctrl-\    | SIGQUIT  | ISIG flag set    | Foreground PGID     |
| (resize)  | SIGWINCH | Window size change| Foreground PGID    |
| (close)   | SIGHUP   | Master closes    | Session leader      |

## Synchronization

### Lock Hierarchy

```
1. Global PTY table lock
2. PTY pair lock
3. Buffer locks (master/slave)
```

### Concurrency Considerations

- **Ring buffers**: Lock-free for single reader/writer
- **Termios updates**: Require PTY pair lock
- **Signal delivery**: Atomic operations for pending signals
- **Window resize**: Lock PTY pair during SIGWINCH delivery

## Performance Characteristics

- **Buffer size**: 4 KB per direction (master→slave, slave→master)
- **Latency**: < 10 µs for read/write operations
- **Throughput**: > 200 MB/s for bulk transfers
- **Max PTYs**: 256 concurrent pairs

## Error Handling

| Error Code | Condition |
|------------|-----------|
| ENODEV     | No PTY devices available |
| EIO        | I/O error on PTY |
| EINVAL     | Invalid ioctl request |
| ENOTTY     | File descriptor is not a TTY |
| EAGAIN     | Non-blocking I/O would block |

## Integration Points

### With Signal Subsystem

- PTY generates signals for special characters
- Signals delivered to foreground process group
- SIGWINCH on window resize

### With Process Groups

- PTY tracks foreground process group
- Background processes receive SIGTTIN/SIGTTOU
- Session leader receives SIGHUP on close

### With /proc Filesystem

- `/proc/<pid>/stat` includes TTY number
- `/proc/debug/pty` shows PTY allocation table

## State Machine

### PTY Lifecycle

```
[Unallocated] --open(/dev/ptmx)--> [Master Open]
                                         |
                                         v
                              open(/dev/pts/N)
                                         |
                                         v
                                   [Both Open]
                                         |
                                         v
                              close(master/slave)
                                         |
                                         v
                                  [Closing State]
                                         |
                                         v
                              (send SIGHUP)
                                         |
                                         v
                                   [Unallocated]
```

## Debugging

### Debug Files

- `/proc/debug/pty`: Shows all allocated PTY pairs
  - PTY number
  - Master/slave open status
  - Buffer fill levels
  - Foreground PGID

### Logging

```
[cpu0][pid=123][pty] Allocated PTY pair 0
[cpu1][pid=124][pty] Set foreground PGID to 125
[cpu0][pid=123][pty] Generated SIGINT for PGID 125
[cpu2][pid=123][pty] Window resize: 24x80 -> 30x100
```

## Future Enhancements

- **Multiple line disciplines**: Support for PPP, SLIP
- **PTY permissions**: Fine-grained access control
- **Flow control**: Hardware flow control (RTS/CTS)
- **Packet mode**: For terminal multiplexers
- **UTF-8 validation**: Kernel-level UTF-8 checking
