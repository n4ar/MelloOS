# Implementation Plan - Advanced Userland & Shell Environment

This implementation plan breaks down Phase 6.6 into incremental, actionable coding tasks. Each task builds on previous work and references specific requirements from the requirements document.

## Task List

- [x] 1. Set up project structure and core interfaces
  - Create directory structure for kernel PTY subsystem (`kernel/src/dev/pty/`)
  - Create directory structure for /proc filesystem (`kernel/src/fs/proc/`)
  - Create directory structure for signal infrastructure (`kernel/src/signal/`)
  - Create userland project directories (`kernel/userspace/mello-term/`, `kernel/userspace/mello-sh/`, `kernel/userspace/mellobox/`)
  - Define core data structures: `PtyPair`, `Termios`, `Winsize`, `ProcessGroup`, `Session`
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 7.1, 7.2_

- [x] 2. Implement PTY device driver core
- [x] 2.1 Create PTY allocation and device nodes
  - Implement `/dev/ptmx` master multiplexer device
  - Implement PTY pair allocation with unique numbering
  - Create `/dev/pts/<n>` slave device nodes dynamically
  - Implement `open()` handler for `/dev/ptmx` that allocates new PTY pair
  - Implement `ioctl(TIOCGPTN)` to get slave number
  - _Requirements: 1.1, 1.2_

- [x] 2.2 Implement termios structure and operations
  - Define `Termios` structure with c_iflag, c_oflag, c_cflag, c_lflag, c_cc fields
  - Implement default termios initialization (ICANON, ECHO, ISIG enabled)
  - Implement `ioctl(TCGETS)` to read termios settings
  - Implement `ioctl(TCSETS)` to write termios settings
  - Add support for critical flags: ICANON, ECHO, ISIG, ICRNL, INLCR, OPOST, ONLCR, IXON, IXOFF
  - Add support for control characters: VINTR, VSUSP, VEOF, VERASE, VMIN, VTIME
  - _Requirements: 1.3, 7.5_

- [x] 2.3 Implement PTY read/write operations
  - Create ring buffers for master→slave and slave→master data flow
  - Implement `read()` handler for PTY master (reads from slave output buffer)
  - Implement `write()` handler for PTY master (writes to slave input buffer)
  - Implement `read()` handler for PTY slave (reads from master output buffer)
  - Implement `write()` handler for PTY slave (writes to master input buffer)
  - Add canonical mode line buffering (buffer until newline)
  - Add echo support (echo input back to output)
  - _Requirements: 1.3, 8.3_

- [x] 2.4 Implement window size management
  - Define `Winsize` structure (ws_row, ws_col, ws_xpixel, ws_ypixel)
  - Implement `ioctl(TIOCGWINSZ)` to get window size
  - Implement `ioctl(TIOCSWINSZ)` to set window size
  - Generate SIGWINCH signal to foreground process group on resize
  - _Requirements: 1.4, 1.5_


- [x] 3. Implement signal infrastructure
- [x] 3.1 Extend Task structure for signals
  - Add signal-related fields to Task: signal_handlers, pending_signals, signal_mask
  - Define `SigAction` structure (handler, mask, flags)
  - Define `SigHandler` enum (Default, Ignore, Custom)
  - Initialize default signal handlers for new tasks
  - _Requirements: 2.5, 7.1_

- [x] 3.2 Implement signal delivery mechanism
  - Implement `send_signal(pid, signal)` kernel function
  - Add signal to task's pending_signals bitset atomically
  - Implement signal delivery on return to userspace
  - Handle default signal actions (terminate, stop, continue, ignore)
  - Implement custom signal handler invocation (save context, jump to handler)
  - _Requirements: 2.3, 2.4, 2.5, 7.1_

- [x] 3.3 Implement signal system calls
  - Implement `sys_sigaction()` to register signal handlers
  - Implement `sys_kill()` to send signals to processes
  - Add permission checks (same UID or root, same session)
  - Prevent SIGKILL/SIGSTOP to PID 1
  - _Requirements: 2.5, 7.1_

- [x] 3.4 Implement keyboard signal generation
  - Detect special characters in PTY input (Ctrl-C, Ctrl-Z, Ctrl-\)
  - Check ISIG flag in termios before generating signals
  - Map characters to signals: VINTR→SIGINT, VSUSP→SIGTSTP, VQUIT→SIGQUIT
  - Send signals only to foreground process group
  - _Requirements: 2.3, 2.4_

- [x] 3.5 Implement TTY background signals
  - Generate SIGHUP when PTY master closes
  - Generate SIGTTIN when background process reads from TTY
  - Generate SIGTTOU when background process writes to TTY
  - Deliver signals to appropriate process groups
  - _Requirements: 7.7, 7.8, 7.9_

- [x] 4. Implement process groups and sessions
- [x] 4.1 Extend Task structure for job control
  - Add fields to Task: pgid, sid, tty (controlling terminal)
  - Initialize pgid=pid, sid=pid for init process
  - Inherit pgid, sid, tty from parent on fork
  - _Requirements: 2.1, 2.2_

- [x] 4.2 Implement process group management syscalls
  - Implement `sys_setpgid(pid, pgid)` to set process group
  - Implement `sys_getpgrp()` to get current process group
  - Add validation: can only set pgid for self or children, must be in same session
  - _Requirements: 2.1, 7.2_

- [x] 4.3 Implement session management syscalls
  - Implement `sys_setsid()` to create new session
  - Detach from controlling terminal on setsid
  - Set sid=pid, pgid=pid for new session leader
  - Implement `sys_getsid(pid)` to get session ID
  - _Requirements: 2.2, 7.2_

- [x] 4.4 Implement terminal foreground control
  - Add foreground_pgid field to PTY slave
  - Implement `sys_tcsetpgrp(fd, pgid)` to set foreground group
  - Implement `sys_tcgetpgrp(fd)` to get foreground group
  - Implement `ioctl(TIOCSPGRP/TIOCGPGRP)` as aliases
  - Add validation: caller must be in same session, pgid must exist
  - _Requirements: 2.2, 7.2, 7.5_

- [x] 4.5 Implement controlling terminal acquisition
  - Implement `ioctl(TIOCSCTTY, 0)` to make TTY the controlling terminal
  - Restrict to session leaders without controlling terminal
  - Set task->tty to this TTY device
  - _Requirements: 7.5_


- [x] 5. Implement file descriptor management
- [x] 5.1 Implement fcntl system call
  - Implement `sys_fcntl(fd, cmd, arg)` dispatcher
  - Implement F_GETFD/F_SETFD for FD_CLOEXEC flag
  - Implement F_GETFL/F_SETFL for O_NONBLOCK and O_APPEND flags
  - Add FD flags field to file descriptor structure
  - _Requirements: 7.3, 7.6_

- [x] 5.2 Implement pipe2 system call
  - Implement `sys_pipe2(flags)` to create pipe with flags
  - Support O_CLOEXEC flag (set FD_CLOEXEC on both FDs)
  - Support O_NONBLOCK flag (set O_NONBLOCK on both FDs)
  - _Requirements: 7.3, 7.6_

- [x] 5.3 Implement dup2 system call
  - Implement `sys_dup2(oldfd, newfd)` to duplicate file descriptor
  - Close newfd if already open
  - Copy file descriptor to specific FD number
  - _Requirements: 7.3_

- [x] 5.4 Add FD_CLOEXEC handling to execve
  - Scan all file descriptors during execve
  - Close FDs with FD_CLOEXEC flag set
  - Keep other FDs open for new program
  - _Requirements: 7.6_

- [x] 6. Implement /proc filesystem
- [x] 6.1 Create /proc virtual filesystem infrastructure
  - Implement /proc filesystem registration
  - Create virtual file operations (open, read, readdir)
  - Implement dynamic file generation on read
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7_

- [x] 6.2 Implement /proc/<pid>/ files
  - Implement `/proc/<pid>/stat` with Linux-compatible format
  - Implement `/proc/<pid>/cmdline` with null-separated arguments
  - Implement `/proc/<pid>/status` with human-readable status
  - Include fields: pid, comm, state, ppid, pgid, sid, tty_nr, tpgid, utime, stime
  - _Requirements: 6.1, 6.2, 6.3_

- [x] 6.3 Implement /proc system-wide files
  - Implement `/proc/meminfo` with memory statistics
  - Implement `/proc/cpuinfo` with CPU information
  - Implement `/proc/uptime` with system uptime in seconds.fraction format
  - Implement `/proc/self` as symlink to current process
  - _Requirements: 6.4, 6.5, 6.6, 6.7_

- [x] 6.4 Add /proc debug files
  - Implement `/proc/debug/pty` showing PTY allocation table
  - Implement `/proc/debug/sessions` showing session/PGID tree
  - Implement `/proc/debug/locks` showing lock contention statistics
  - _Requirements: 11.1, 11.2_


- [x] 7. Implement mello-term terminal emulator
- [x] 7.1 Set up mello-term project structure
  - Create Cargo project for mello-term
  - Set up no_std environment with alloc
  - Define main entry point and initialization
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 7.2 Implement PTY master interaction
  - Open `/dev/ptmx` to allocate PTY pair
  - Get slave number with `ioctl(TIOCGPTN)`
  - Open slave side `/dev/pts/<n>`
  - Set up non-blocking I/O on master FD
  - _Requirements: 5.1, 5.4_

- [x] 7.3 Implement screen buffer and rendering
  - Define `ScreenBuffer` structure (rows, cols, cells array)
  - Define `Cell` structure (character, fg color, bg color, attributes)
  - Implement cursor tracking (row, col, visible)
  - Implement basic text rendering (write character at cursor position)
  - Implement scrolling when cursor reaches bottom
  - _Requirements: 5.1, 5.2_

- [x] 7.4 Implement ANSI escape sequence parser
  - Create state machine for ANSI/VT escape sequences
  - Parse CSI sequences: `ESC[<params><cmd>`
  - Implement cursor movement: ESC[A/B/C/D (up/down/right/left)
  - Implement clear screen: ESC[2J
  - Implement cursor positioning: ESC[<row>;<col>H
  - Implement SGR (color/attributes): ESC[<n>m
  - _Requirements: 5.1, 5.2_

- [x] 7.5 Implement UTF-8 rendering
  - Parse UTF-8 multi-byte sequences correctly
  - Use wcwidth-like rules for character width (0, 1, or 2 columns)
  - Handle wide characters (CJK) as 2-column cells
  - Never split combining character sequences
  - _Requirements: 5.2, 9.2_

- [x] 7.6 Implement keyboard input handling
  - Read keyboard input from user (platform-specific)
  - Map special keys to escape sequences (arrows, function keys)
  - Write input to PTY master
  - Handle Ctrl-C, Ctrl-Z, Ctrl-D correctly
  - _Requirements: 5.1_

- [x] 7.7 Implement scrollback buffer
  - Create scrollback buffer (VecDeque of lines)
  - Limit to 10,000 lines to prevent memory exhaustion
  - Evict oldest lines when limit reached
  - Allow scrolling through history
  - _Requirements: 5.3_

- [x] 7.8 Implement window resize handling
  - Detect terminal window resize events
  - Call `ioctl(TIOCSWINSZ)` with new dimensions
  - Verify SIGWINCH is delivered to shell
  - Redraw screen buffer with new dimensions
  - _Requirements: 5.4_

- [x] 7.9 Implement copy/paste support
  - Implement text selection with mouse or keyboard
  - Copy selected text to clipboard
  - Paste clipboard content to PTY master
  - _Requirements: 5.5_


- [x] 8. Implement mello-sh shell core
- [x] 8.1 Set up mello-sh project structure
  - Create Cargo project for mello-sh
  - Set up no_std environment with alloc
  - Define main entry point and shell loop
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [x] 8.2 Implement command line reading
  - Open controlling terminal (stdin/stdout/stderr)
  - Display prompt in format `[user@host cwd]$`
  - Read line from stdin (canonical mode)
  - Handle EOF (Ctrl-D) to exit shell
  - _Requirements: 3.5_

- [x] 8.3 Implement command parser
  - Create lexer to tokenize input (split on whitespace, handle quotes)
  - Parse simple commands (command + arguments)
  - Parse pipes: `cmd1 | cmd2 | cmd3`
  - Parse redirects: `< input.txt`, `> output.txt`, `>> append.txt`
  - Parse background operator: `cmd &`
  - Handle quoted strings and escape sequences
  - _Requirements: 3.1, 3.2, 3.6_

- [x] 8.4 Implement simple command execution
  - Fork child process
  - Set child's process group with `setpgid(0, 0)`
  - Set parent's view of child's pgid with `setpgid(child, child)`
  - Set foreground group with `tcsetpgrp(tty, child_pgid)`
  - Execute command with `execve()`
  - Parent waits with `wait4()` for child to complete
  - Restore shell as foreground group after child exits
  - _Requirements: 3.4, 8.2_

- [x] 8.5 Implement pipeline execution
  - Create pipes for each connection: `pipe()` or `pipe2()`
  - Fork child for each command in pipeline
  - Set up stdin/stdout redirection with `dup2()`
  - Put all children in same process group
  - Close unused pipe ends in parent and children
  - Wait for all children to complete
  - Return exit status of last command
  - _Requirements: 3.1, 3.8, 8.3_

- [x] 8.6 Implement I/O redirection
  - Parse redirect operators and filenames
  - Open files with appropriate flags (O_RDONLY, O_WRONLY|O_CREAT, O_APPEND)
  - Use `dup2()` to redirect stdin/stdout/stderr
  - Close original file descriptors after dup2
  - _Requirements: 3.2_

- [x] 8.7 Implement background job execution
  - Detect `&` operator at end of command
  - Fork and exec without waiting
  - Do not set as foreground group
  - Add job to job table
  - Print job ID and PID: `[1] 12345`
  - _Requirements: 3.6_

- [x] 8.8 Implement job table management
  - Define `Job` structure (id, pgid, command, state, background)
  - Define `JobState` enum (Running, Stopped, Done)
  - Maintain list of active jobs
  - Assign sequential job IDs
  - Update job state on SIGCHLD
  - _Requirements: 3.6_

- [x] 8.9 Implement SIGCHLD handler
  - Install SIGCHLD handler in shell
  - Call `wait4(-1, WNOHANG)` to reap children
  - Update job table with exit status or stop/continue events
  - Print job completion messages: `[1]+ Done    sleep 10`
  - _Requirements: 2.5_

- [x] 8.10 Implement command history
  - Maintain vector of executed commands
  - Add command to history after execution
  - Provide access to history (for future up/down arrow support)
  - _Requirements: 3.7_


- [x] 9. Implement shell built-in commands
- [x] 9.1 Implement cd built-in
  - Parse directory argument (default to $HOME)
  - Call `chdir()` system call
  - Update PWD environment variable
  - Handle errors (directory not found, permission denied)
  - _Requirements: 3.3_

- [x] 9.2 Implement pwd built-in
  - Call `getcwd()` system call
  - Print current working directory
  - _Requirements: 3.3_

- [x] 9.3 Implement echo built-in
  - Print arguments separated by spaces
  - Support `-n` flag (no trailing newline)
  - Support `-e` flag (interpret escape sequences)
  - _Requirements: 3.3_

- [x] 9.4 Implement export built-in
  - Parse `VAR=value` syntax
  - Add or update environment variable
  - Make variable available to child processes
  - _Requirements: 3.3_

- [x] 9.5 Implement unset built-in
  - Parse variable name
  - Remove from environment
  - _Requirements: 3.3_

- [x] 9.6 Implement jobs built-in
  - List all jobs in job table
  - Show job ID, state, and command
  - Format: `[1]+ Running    sleep 10 &`
  - _Requirements: 3.3_

- [x] 9.7 Implement fg built-in
  - Parse job ID (default to current job)
  - Find job in job table
  - Set job as foreground group with `tcsetpgrp()`
  - Send SIGCONT if job is stopped
  - Wait for job to complete or stop
  - _Requirements: 3.3, 2.6_

- [x] 9.8 Implement bg built-in
  - Parse job ID (default to current job)
  - Find job in job table
  - Send SIGCONT to resume job
  - Keep job in background (don't call tcsetpgrp)
  - _Requirements: 3.3, 2.6_

- [x] 9.9 Implement exit built-in
  - Parse optional exit code argument
  - Clean up resources (close FDs, free memory)
  - Exit shell with specified code
  - _Requirements: 3.3_

- [x] 9.10 Implement which built-in
  - Parse command name
  - Search PATH environment variable
  - Print full path to executable
  - Return error if not found
  - _Requirements: 3.3_


- [x] 10. Implement mellobox coreutils framework
- [x] 10.1 Set up mellobox project structure
  - Create Cargo project for mellobox
  - Set up no_std environment with alloc
  - Define multi-call binary dispatcher
  - Parse argv[0] to determine which utility to run
  - _Requirements: 4.1, 4.2_

- [x] 10.2 Implement argument parsing framework
  - Create simple getopt-style argument parser
  - Support short options: `-a -b -c`
  - Support combined short options: `-abc`
  - Support options with arguments: `-o file`
  - Parse remaining arguments as positional
  - _Requirements: 4.3_

- [x] 10.3 Implement error handling and exit codes
  - Define error types (NotFound, PermissionDenied, InvalidArgument, etc.)
  - Return 0 for success
  - Return 1 for runtime errors
  - Return 2 for usage errors
  - Print error messages to stderr
  - _Requirements: 4.4_

- [x] 11. Implement mellobox core utilities
- [x] 11.1 Implement ls utility
  - Open directory with `openat()` and `getdents()`
  - List directory entries
  - Support `-l` flag (long format with permissions, size, date)
  - Support `-a` flag (show hidden files)
  - Support `-h` flag (human-readable sizes)
  - Handle UTF-8 filenames correctly
  - _Requirements: 4.1, 4.3, 4.5, 8.4_

- [x] 11.2 Implement cp utility
  - Parse source and destination arguments
  - Open source file for reading
  - Create destination file for writing
  - Copy data in chunks
  - Support `-r` flag (recursive copy for directories)
  - Support `-i` flag (interactive, prompt before overwrite)
  - Support `-v` flag (verbose, print files copied)
  - _Requirements: 4.1, 4.3_

- [x] 11.3 Implement mv utility
  - Parse source and destination arguments
  - Try rename() first (same filesystem)
  - Fall back to copy+delete if rename fails
  - Support `-i` flag (interactive)
  - Support `-v` flag (verbose)
  - _Requirements: 4.1, 4.3_

- [x] 11.4 Implement rm utility
  - Parse file arguments
  - Unlink files
  - Support `-r` flag (recursive delete for directories)
  - Support `-f` flag (force, ignore errors)
  - Support `-i` flag (interactive, prompt before delete)
  - _Requirements: 4.1, 4.3_

- [x] 11.5 Implement cat utility
  - Parse file arguments (or read from stdin if none)
  - Open and read each file
  - Write contents to stdout
  - Support `-n` flag (number lines)
  - Handle UTF-8 correctly
  - _Requirements: 4.1, 4.3, 4.5_

- [x] 11.6 Implement grep utility
  - Parse pattern and file arguments
  - Read files line by line
  - Match pattern (simple substring or basic regex)
  - Print matching lines
  - Support `-i` flag (case-insensitive)
  - Support `-r` flag (recursive search in directories)
  - Support `-n` flag (show line numbers)
  - Handle UTF-8 correctly
  - _Requirements: 4.1, 4.3, 4.5_

- [x] 11.7 Implement ps utility
  - Read `/proc` to enumerate processes
  - Parse `/proc/<pid>/stat` for process information
  - Display PID, TTY, TIME, CMD
  - Support `-a` flag (all processes)
  - Support `-u` flag (user-oriented format)
  - Support `-x` flag (include processes without TTY)
  - _Requirements: 4.1, 4.3, 6.1_

- [x] 11.8 Implement kill utility
  - Parse signal and PID arguments
  - Default to SIGTERM if no signal specified
  - Call `kill()` system call
  - Support signal names: `-TERM`, `-INT`, `-KILL`
  - Support signal numbers: `-9`, `-15`
  - _Requirements: 4.1, 4.3_

- [x] 11.9 Implement additional utilities
  - Implement `mkdir` with `-p` flag (create parent directories)
  - Implement `touch` (create empty file or update timestamp)
  - Implement `echo` with `-n` and `-e` flags
  - Implement `pwd` (print working directory)
  - Implement `true` (exit 0)
  - Implement `false` (exit 1)
  - _Requirements: 4.1, 4.3, 4.4_


- [x] 12. Implement SMP safety and synchronization
- [x] 12.1 Define lock hierarchy
  - Document lock ordering: Global PTY table → Session → Process group → Task → PTY pair
  - Add lock ordering validation in debug builds
  - Implement trylock with timeout for PTY operations
  - _Requirements: 10.4, 12.6_

- [x] 12.2 Implement per-CPU data structures
  - Create per-CPU scheduler run queues
  - Create per-CPU statistics (context switches, signals delivered, syscalls)
  - Use CPU-local storage to reduce lock contention
  - _Requirements: 10.5_

- [x] 12.3 Implement TLB shootdown for page table modifications
  - Send IPI to all CPUs running a task when unmapping pages
  - Wait for acknowledgment before freeing physical pages
  - Implement IPI handler to flush TLB
  - _Requirements: 10.4_

- [x] 12.4 Add atomic operations for signal delivery
  - Use atomic bitset for pending_signals
  - Use atomic operations for signal handler registration
  - Ensure signal delivery is race-free
  - _Requirements: 12.3_

- [x] 12.5 Implement lock-free /proc reads
  - Use RCU or seqlock for reading process state
  - Avoid holding locks during /proc file generation
  - Handle races gracefully (process may exit during read)
  - _Requirements: 10.5_

- [x] 13. Implement security and validation
- [x] 13.1 Implement user pointer validation
  - Create `copy_from_user()` function with bounds checking
  - Create `copy_to_user()` function with bounds checking
  - Validate alignment and address range
  - Check page permissions (readable/writable)
  - _Requirements: 10.1, 10.3_

- [x] 13.2 Implement W^X memory protection
  - Ensure code pages are R+X, not W
  - Ensure data pages are R+W, not X
  - Ensure stack is R+W, not X (NX bit)
  - Validate page permissions on mmap and mprotect
  - _Requirements: 10.1, 10.2_

- [x] 13.3 Implement ioctl validation
  - Validate ioctl command numbers
  - Validate argument pointers with copy_from_user/copy_to_user
  - Check file descriptor type matches ioctl
  - Return EINVAL for unknown commands
  - _Requirements: 10.3_

- [x] 13.4 Implement signal security checks
  - Check sender UID matches target UID (or sender is root)
  - Check sender and target are in same session (or sender is root)
  - Prevent SIGKILL/SIGSTOP to PID 1
  - Validate signal handler addresses are in user code pages
  - _Requirements: 10.3_


- [x] 14. Implement observability and debugging
- [x] 14.1 Implement structured logging
  - Create logging macros with format: `[cpuN][pid=X][subsys] message`
  - Add log levels: ERROR, WARN, INFO, DEBUG, TRACE
  - Log important events: PTY allocation, signal delivery, process group changes
  - _Requirements: 11.1_

- [x] 14.2 Implement metrics collection
  - Create global metrics structure with atomic counters
  - Track context switches, signals delivered, PTY bytes in/out
  - Track per-syscall counters
  - Expose metrics via /proc/stat
  - _Requirements: 11.2_

- [x] 14.3 Implement panic dump
  - Enhance panic handler to dump task state
  - Print PID, PGID, SID, TTY, last syscall
  - Print register state: RIP, RSP, CR2
  - Print stack trace
  - _Requirements: 11.3_

- [x] 14.4 Add debug shell built-ins
  - Implement `debug-pty` to show PTY state
  - Implement `debug-jobs` to show detailed job table
  - Implement `debug-signals` to show pending signals
  - _Requirements: 11.1_

- [x] 15. Integration and end-to-end testing
- [x] 15.1 Create PTY integration test
  - Open PTY pair
  - Write ANSI sequences to master
  - Verify correct parsing and rendering
  - Test resize and SIGWINCH delivery
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

- [x] 15.2 Create job control integration test
  - Start background job: `sleep 10 &`
  - Verify job appears in jobs list
  - Send SIGTSTP to foreground job (Ctrl-Z)
  - Verify job state changes to Stopped
  - Resume with `fg` and `bg`
  - Verify SIGCONT delivery
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

- [x] 15.3 Create pipeline integration test
  - Execute: `echo hello | grep h | wc -l`
  - Verify all processes in same PGID
  - Verify correct output
  - Verify exit status propagation
  - _Requirements: 3.1, 3.8_

- [x] 15.4 Create I/O redirection integration test
  - Execute: `echo "hello world" > test.txt`
  - Execute: `cat test.txt | tr a-z A-Z > output.txt`
  - Verify file contents: "HELLO WORLD"
  - _Requirements: 3.2_

- [x] 15.5 Create interactive session test
  - Boot system and open mello-term
  - Verify prompt appears within 10ms
  - Execute: `ls /`, `cd /proc`, `cat /proc/self/stat`, `ps aux`
  - Verify all commands work correctly
  - _Requirements: 8.1, 8.4_

- [x] 15.6 Create stability test
  - Run system for extended period
  - Execute various commands repeatedly
  - Verify no kernel panics
  - Verify no zombie processes accumulate
  - Verify no memory leaks
  - _Requirements: 12.1, 12.2, 12.4, 12.5_


- [x] 16. Performance optimization and benchmarking
- [x] 16.1 Implement performance benchmarks
  - Create benchmark: shell startup time (`time mello-sh -c 'exit'`)
  - Create benchmark: process spawn (`time mello-sh -c 'for i in {1..100}; do /bin/true; done'`)
  - Create benchmark: pipe throughput (`dd if=/dev/zero bs=1M count=100 | cat > /dev/null`)
  - Create benchmark: directory listing (`time ls -la /usr/bin` with 1000+ entries)
  - Create benchmark: syscall latency (read/write 4 KiB hot cache)
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [x] 16.2 Optimize hot paths
  - Profile syscall entry/exit overhead
  - Optimize PTY read/write paths
  - Optimize signal delivery path
  - Reduce lock contention in scheduler
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [x] 16.3 Verify performance targets
  - Shell startup: < 10ms ✓
  - Spawn /bin/true: < 2ms per iteration ✓
  - Pipe throughput: > 200 MB/s ✓
  - ls 1000 entries: < 80ms ✓
  - Syscall latency: < 5µs median ✓
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [x] 17. UTF-8 and internationalization
- [x] 17.1 Set up locale support
  - Set default LANG=C.UTF-8
  - Support th_TH.UTF-8 for Thai language
  - Ensure all components respect locale setting
  - _Requirements: 9.1_

- [x] 17.2 Verify UTF-8 handling
  - Test multi-byte UTF-8 sequences in terminal
  - Test UTF-8 in command arguments
  - Test UTF-8 in environment variables
  - Test UTF-8 in file names (ls, cat, grep)
  - Verify no corruption or splitting of sequences
  - _Requirements: 9.2, 9.3, 9.4_

- [x] 18. Build system and integration
- [x] 18.1 Update kernel build system
  - Add PTY subsystem to kernel build
  - Add /proc filesystem to kernel build
  - Add signal infrastructure to kernel build
  - Update linker script if needed
  - _Requirements: All kernel requirements_

- [x] 18.2 Create userland build system
  - Add mello-term to build
  - Add mello-sh to build
  - Add mellobox to build
  - Create symlinks for mellobox utilities
  - Package userland binaries into initramfs or disk image
  - _Requirements: All userland requirements_

- [x] 18.3 Update boot process
  - Ensure /dev/ptmx and /dev/pts/ are created at boot
  - Mount /proc filesystem at boot
  - Start mello-term as init process (or from init)
  - Set up initial environment variables
  - _Requirements: 8.1_

- [x] 19. Documentation and handoff
- [x] 19.1 Update architecture documentation
  - Document PTY subsystem architecture
  - Document signal and job control implementation
  - Document /proc filesystem structure
  - Add diagrams for data flow and state machines
  - _Requirements: All requirements_

- [x] 19.2 Create user guide
  - Document shell features and built-in commands
  - Document mellobox utilities and options
  - Provide usage examples
  - Document known limitations
  - _Requirements: 3.3, 4.1_

- [x] 19.3 Create developer guide
  - Document how to add new syscalls
  - Document how to add new /proc files
  - Document how to add new mellobox utilities
  - Document debugging techniques
  - _Requirements: 11.1, 11.2, 11.3_

- [x] 19.4 Create troubleshooting guide
  - Document common issues and solutions
  - Document how to debug PTY issues
  - Document how to debug signal issues
  - Document how to debug job control issues
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5_

## Summary

This implementation plan provides a comprehensive, incremental approach to building Phase 6.6. Each task is concrete and actionable, with clear references to requirements. The plan follows a logical progression:

1. **Foundation** (Tasks 1-6): Kernel infrastructure (PTY, signals, process groups, /proc)
2. **Userland Core** (Tasks 7-11): Terminal emulator, shell, and coreutils
3. **Quality** (Tasks 12-14): SMP safety, security, observability
4. **Validation** (Tasks 15-17): Testing, performance, UTF-8
5. **Integration** (Tasks 18-19): Build system and documentation

Optional tasks (marked with `*`) focus on debugging tools and documentation that enhance development but are not critical for core functionality.
