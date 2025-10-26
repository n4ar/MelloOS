# Requirements Document

## Introduction

This document specifies the requirements for Phase 6.6 - Advanced Userland & Shell Environment for MelloOS. The feature elevates the operating system from "can run ELF binaries" to "usable through a terminal" by implementing a PTY subsystem, terminal emulator, shell with job control, and POSIX-compatible coreutils. This phase prepares the system for native zsh porting (Phase 7) and terminfo support (Phase 8).

## Glossary

- **PTY_Subsystem**: Pseudo-terminal subsystem providing master/slave terminal pairs for terminal emulation
- **Termios**: Terminal I/O settings structure controlling terminal behavior (raw/canonical modes, echo, etc.)
- **Job_Control**: Shell feature allowing background/foreground process management and signal handling
- **Mello_Shell**: The native shell implementation (mello-sh) with POSIX-like features
- **Mellobox**: Multi-call binary implementing common UNIX utilities (similar to busybox)
- **Mello_Term**: Terminal emulator application providing VT/ANSI rendering
- **Process_Group**: Collection of processes that can be signaled together, identified by PGID
- **Foreground_Group**: The process group currently receiving keyboard input in a terminal
- **Proc_Filesystem**: Virtual filesystem at /proc providing process and system information
- **Signal**: Asynchronous notification sent to processes (SIGINT, SIGTSTP, SIGCHLD, etc.)
- **SIGWINCH**: Signal sent when terminal window size changes
- **Ioctl**: Input/output control system call for device-specific operations
- **VT_ANSI**: Video terminal ANSI escape sequence standard for terminal control

## Requirements

### Requirement 1

**User Story:** As a system user, I want a functional pseudo-terminal subsystem, so that I can interact with the shell through a terminal emulator

#### Acceptance Criteria

1. WHEN a process opens /dev/ptmx, THE PTY_Subsystem SHALL allocate a new pseudo-terminal master/slave pair
2. WHEN a PTY master is allocated, THE PTY_Subsystem SHALL create a corresponding /dev/pts/<n> slave device
3. THE PTY_Subsystem SHALL support termios structures with raw mode and canonical mode configurations
4. WHEN terminal window size changes, THE PTY_Subsystem SHALL deliver SIGWINCH to the foreground process group
5. THE PTY_Subsystem SHALL implement ioctl operations TIOCGWINSZ and TIOCSWINSZ for window size management

### Requirement 2

**User Story:** As a shell user, I want signal handling and job control, so that I can manage foreground and background processes

#### Acceptance Criteria

1. THE Mello_Shell SHALL implement process group management through setpgid and getpgrp system calls
2. THE Mello_Shell SHALL support terminal foreground group control through tcsetpgrp and tcgetpgrp
3. WHEN keyboard interrupt (Ctrl-C) occurs, THE PTY_Subsystem SHALL deliver SIGINT only to the Foreground_Group
4. WHEN keyboard suspend (Ctrl-Z) occurs, THE PTY_Subsystem SHALL deliver SIGTSTP only to the Foreground_Group
5. THE Mello_Shell SHALL handle SIGCHLD to detect child process state changes
6. THE Mello_Shell SHALL support SIGCONT for resuming stopped processes

### Requirement 3

**User Story:** As a shell user, I want a functional shell with pipes and redirects, so that I can compose commands and manage I/O

#### Acceptance Criteria

1. THE Mello_Shell SHALL parse and execute command pipelines with the pipe operator (|)
2. THE Mello_Shell SHALL support input redirection (<) and output redirection (>, >>)
3. THE Mello_Shell SHALL implement built-in commands: cd, pwd, echo, export, unset, jobs, fg, bg, exit, which
4. THE Mello_Shell SHALL execute external commands through fork and execve system calls
5. THE Mello_Shell SHALL display a prompt in the format [user@host cwd]$
6. THE Mello_Shell SHALL support background job execution with the ampersand operator (&)
7. THE Mello_Shell SHALL maintain command history during the session
8. WHEN a command pipeline completes, THE Mello_Shell SHALL propagate the exit status of the last command in the pipeline

### Requirement 4

**User Story:** As a system user, I want common UNIX utilities, so that I can perform file operations and system inspection

#### Acceptance Criteria

1. THE Mellobox SHALL implement as a multi-call binary supporting at least: ls, cp, mv, rm, cat, grep, ps, kill
2. WHEN invoked with a utility name, THE Mellobox SHALL execute the corresponding utility function
3. THE Mellobox SHALL support common command-line options for each utility
4. THE Mellobox SHALL return exit code 0 for success, 1 for errors, and 2 for usage errors
5. THE Mellobox SHALL handle UTF-8 encoded text correctly in all utilities

### Requirement 5

**User Story:** As a system user, I want a terminal emulator, so that I can interact with the shell visually

#### Acceptance Criteria

1. THE Mello_Term SHALL render VT/ANSI escape sequences correctly
2. THE Mello_Term SHALL support UTF-8 character encoding and display
3. THE Mello_Term SHALL provide scrollback buffer functionality
4. WHEN terminal window is resized, THE Mello_Term SHALL call ioctl(TIOCSWINSZ) and trigger SIGWINCH
5. THE Mello_Term SHALL support copy and paste operations

### Requirement 6

**User Story:** As a system administrator, I want a /proc filesystem, so that I can inspect process and system information

#### Acceptance Criteria

1. THE Proc_Filesystem SHALL provide /proc/<pid>/stat containing process state information
2. THE Proc_Filesystem SHALL provide /proc/<pid>/cmdline containing process command line
3. THE Proc_Filesystem SHALL provide /proc/<pid>/status containing detailed process status
4. THE Proc_Filesystem SHALL provide /proc/meminfo containing memory statistics
5. THE Proc_Filesystem SHALL provide /proc/cpuinfo containing CPU information
6. THE Proc_Filesystem SHALL provide /proc/uptime containing system uptime
7. THE Proc_Filesystem SHALL provide /proc/self as a symbolic link to the current process

### Requirement 7

**User Story:** As a kernel developer, I want necessary system calls, so that userland can implement terminal and shell features

#### Acceptance Criteria

1. THE Kernel SHALL implement system calls: fork, execve, wait4, kill, sigaction
2. THE Kernel SHALL implement system calls: setpgid, getpgrp, setsid, getsid, tcsetpgrp, tcgetpgrp
3. THE Kernel SHALL implement system calls: openat, read, write, pipe, pipe2, dup, dup2, fcntl, ioctl, getdents
4. THE Kernel SHALL implement system calls: clock_gettime, nanosleep, mmap, getcwd, chdir, uname, fstat, lstat
5. THE Kernel SHALL implement ioctl commands: TCGETS, TCSETS, TIOCGWINSZ, TIOCSWINSZ, TIOCSCTTY, TIOCSPGRP, TIOCGPGRP
6. THE Kernel SHALL implement fcntl commands: F_GETFD, F_SETFD, F_GETFL, F_SETFL for FD_CLOEXEC and O_NONBLOCK flags
7. THE Kernel SHALL generate SIGHUP when PTY master closes and deliver to foreground process group
8. THE Kernel SHALL generate SIGTTIN when background process group attempts to read from TTY
9. THE Kernel SHALL generate SIGTTOU when background process group attempts to write to TTY

### Requirement 8

**User Story:** As a system user, I want responsive terminal interaction, so that the system feels usable

#### Acceptance Criteria

1. WHEN Mello_Term starts, THE system SHALL display a shell prompt within 10 milliseconds
2. WHEN spawning /bin/true, THE system SHALL complete execution within 2 milliseconds
3. WHEN using pipes, THE system SHALL achieve throughput of at least 200 megabytes per second
4. WHEN listing 1000 directory entries, THE ls utility SHALL complete within 80 milliseconds
5. THE Kernel SHALL complete read and write system calls with median latency below 5 microseconds

### Requirement 9

**User Story:** As a system user, I want proper UTF-8 support, so that I can use international characters

#### Acceptance Criteria

1. THE system SHALL default to LANG=C.UTF-8 locale setting
2. THE Mello_Term SHALL render UTF-8 characters without splitting multi-byte sequences
3. THE Mello_Shell SHALL handle UTF-8 in command arguments and environment variables
4. THE Mellobox utilities SHALL process UTF-8 text correctly

### Requirement 10

**User Story:** As a system administrator, I want security and SMP safety, so that the system remains stable on multicore systems

#### Acceptance Criteria

1. THE Kernel SHALL set user pages with U=1 flag only where necessary
2. THE Kernel SHALL enforce W^X (write XOR execute) memory protection
3. THE Kernel SHALL validate all user pointers through copy_from_user before dereferencing
4. WHEN modifying page tables on multicore systems, THE Kernel SHALL perform TLB shootdown with acknowledgment
5. THE Kernel SHALL use per-CPU locks where possible to reduce lock contention

### Requirement 11

**User Story:** As a kernel developer, I want observability features, so that I can debug and monitor the system

#### Acceptance Criteria

1. THE Kernel SHALL log messages in format [cpuN][pid=X][subsys] message
2. THE Kernel SHALL track metrics: context switches, signals delivered, PTY bytes in/out
3. WHEN kernel panic occurs, THE Kernel SHALL dump: last system call, CR2, RIP, RSP, PID, PGID, TTY

### Requirement 12

**User Story:** As a system user, I want stable operation, so that I can use the terminal without crashes or resource leaks

#### Acceptance Criteria

1. WHEN terminal is resized, THE system SHALL not panic or deadlock
2. WHEN processes exit, THE system SHALL not create zombie processes
3. WHEN using job control, THE system SHALL not have race conditions
4. THE system SHALL not deadlock when PTY master and slave interact
5. THE system SHALL validate all ioctl structures to prevent malformed requests
6. THE Kernel SHALL use trylock or spinlock timeout mechanisms to mitigate priority inversion
