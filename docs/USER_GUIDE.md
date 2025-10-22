# MelloOS User Guide

## Introduction

Welcome to MelloOS! This guide will help you get started with the interactive shell environment, terminal emulator, and command-line utilities.

## Getting Started

### Booting MelloOS

1. Build the system:
   ```bash
   make
   ```

2. Run in QEMU:
   ```bash
   make run
   ```

3. The system will boot and present you with the mello-term terminal emulator running mello-sh shell.

### First Login

When the system boots, you'll see a prompt like:
```
[user@mellos /]$ 
```

The prompt format is: `[username@hostname current_directory]$`

## Shell Features

### Basic Commands

The shell supports standard command execution:

```bash
# Run a command
ls /

# Run with arguments
ls -la /proc

# Change directory
cd /proc

# Print working directory
pwd
```

### Command History

- Commands are stored in history during your session
- Use up/down arrows to navigate history (future feature)
- History is lost when shell exits

### Built-in Commands

#### cd - Change Directory

Change the current working directory.

```bash
cd /proc          # Change to /proc
cd ..             # Go up one directory
cd                # Go to home directory
cd /usr/bin       # Absolute path
```

#### pwd - Print Working Directory

Display the current directory path.

```bash
pwd
# Output: /proc
```

#### echo - Print Text

Print arguments to standard output.

```bash
echo Hello World
# Output: Hello World

echo -n "No newline"
# Output: No newline (without trailing newline)

echo -e "Line 1\nLine 2"
# Output: Line 1
#         Line 2
```

#### export - Set Environment Variable

Set or modify environment variables.

```bash
export PATH=/bin:/usr/bin
export LANG=C.UTF-8
export MY_VAR=value
```

#### unset - Remove Environment Variable

Remove an environment variable.

```bash
unset MY_VAR
```

#### exit - Exit Shell

Exit the shell with optional exit code.

```bash
exit          # Exit with code 0
exit 1        # Exit with code 1
```

#### which - Locate Command

Show the full path of a command.

```bash
which ls
# Output: /bin/ls
```

### Job Control

#### Background Jobs

Run commands in the background using `&`:

```bash
sleep 100 &
# Output: [1] 123
```

The output shows:
- `[1]` - Job number
- `123` - Process ID

#### jobs - List Jobs

Display all background and stopped jobs.

```bash
jobs
# Output:
# [1]+ Running    sleep 100 &
# [2]- Stopped    cat
```

Format: `[job_id][current] state    command`
- `+` indicates current job
- `-` indicates previous job

#### fg - Foreground Job

Bring a background or stopped job to the foreground.

```bash
fg          # Bring current job to foreground
fg %1       # Bring job 1 to foreground
fg %2       # Bring job 2 to foreground
```

If the job was stopped, it will be resumed.

#### bg - Background Job

Resume a stopped job in the background.

```bash
bg          # Resume current job in background
bg %1       # Resume job 1 in background
```

#### Keyboard Shortcuts

- **Ctrl-C**: Send SIGINT to foreground job (interrupt)
- **Ctrl-Z**: Send SIGTSTP to foreground job (suspend)
- **Ctrl-D**: Send EOF (exit shell if at empty prompt)
- **Ctrl-\\**: Send SIGQUIT to foreground job (quit with core dump)

### Pipelines

Connect commands using pipes (`|`):

```bash
# Simple pipeline
ls / | grep proc

# Multi-stage pipeline
cat file.txt | grep error | wc -l

# Complex pipeline
ps aux | grep mello | awk '{print $2}'
```

The output of each command becomes the input of the next.

### I/O Redirection

#### Input Redirection

Redirect input from a file:

```bash
cat < input.txt
grep pattern < file.txt
```

#### Output Redirection

Redirect output to a file:

```bash
# Overwrite file
echo "Hello" > output.txt

# Append to file
echo "World" >> output.txt
```

#### Combining Redirections

```bash
# Input and output
cat < input.txt > output.txt

# Pipeline with redirection
ls / | grep proc > results.txt
```

## Mellobox Utilities

Mellobox provides common UNIX utilities in a single binary.

### ls - List Directory Contents

List files and directories.

```bash
# Basic listing
ls

# Long format
ls -l

# Show hidden files
ls -a

# Long format with hidden files
ls -la

# Human-readable sizes
ls -lh

# Specific directory
ls /proc
```

**Output format (long):**
```
-rw-r--r-- 1 user group 1234 Jan 01 12:00 file.txt
drwxr-xr-x 2 user group 4096 Jan 01 12:00 directory
```

### cp - Copy Files

Copy files and directories.

```bash
# Copy file
cp source.txt dest.txt

# Copy directory recursively
cp -r source_dir dest_dir

# Interactive mode (prompt before overwrite)
cp -i file.txt backup.txt

# Verbose mode
cp -v file.txt backup.txt
```

### mv - Move/Rename Files

Move or rename files and directories.

```bash
# Rename file
mv old.txt new.txt

# Move file
mv file.txt /tmp/

# Interactive mode
mv -i file.txt /tmp/

# Verbose mode
mv -v file.txt /tmp/
```

### rm - Remove Files

Delete files and directories.

```bash
# Remove file
rm file.txt

# Remove directory recursively
rm -r directory

# Force removal (no prompts)
rm -f file.txt

# Interactive mode
rm -i file.txt

# Remove multiple files
rm file1.txt file2.txt file3.txt
```

**Warning:** Be careful with `rm -rf` - it will delete everything without confirmation!

### cat - Concatenate Files

Display file contents.

```bash
# Display file
cat file.txt

# Display multiple files
cat file1.txt file2.txt

# Number lines
cat -n file.txt

# Display from stdin
cat
(type text, press Ctrl-D to end)
```

### grep - Search Text

Search for patterns in files.

```bash
# Search in file
grep pattern file.txt

# Case-insensitive search
grep -i pattern file.txt

# Recursive search
grep -r pattern directory/

# Show line numbers
grep -n pattern file.txt

# Search in multiple files
grep pattern *.txt

# Use with pipeline
ps aux | grep mello
```

### ps - Process Status

List running processes.

```bash
# Basic listing
ps

# All processes
ps -a

# User-oriented format
ps -u

# Include processes without TTY
ps -x

# Combined (common usage)
ps aux
```

**Output format:**
```
PID  TTY      TIME CMD
123  pts/0    00:00:01 mello-sh
124  pts/0    00:00:00 ps
```

### kill - Send Signal to Process

Send signals to processes.

```bash
# Send SIGTERM (default)
kill 123

# Send specific signal by name
kill -TERM 123
kill -INT 123
kill -KILL 123

# Send specific signal by number
kill -9 123    # SIGKILL
kill -15 123   # SIGTERM

# Kill process group
kill -TERM -123
```

**Common signals:**
- `SIGTERM (15)`: Graceful termination
- `SIGKILL (9)`: Force kill (cannot be caught)
- `SIGINT (2)`: Interrupt (like Ctrl-C)
- `SIGHUP (1)`: Hangup

### mkdir - Make Directory

Create directories.

```bash
# Create directory
mkdir mydir

# Create parent directories
mkdir -p path/to/directory
```

### touch - Create Empty File

Create empty files or update timestamps.

```bash
# Create new file
touch newfile.txt

# Update timestamp
touch existing.txt

# Create multiple files
touch file1.txt file2.txt file3.txt
```

### pwd - Print Working Directory

Display current directory (also available as shell built-in).

```bash
pwd
# Output: /home/user
```

### true / false - Exit Status

Commands that always succeed or fail.

```bash
true
echo $?    # Output: 0

false
echo $?    # Output: 1
```

Useful in scripts and conditionals.

## Terminal Features

### ANSI Escape Sequences

The terminal supports standard ANSI/VT escape sequences:

- **Cursor movement**: Arrow keys, Home, End
- **Colors**: 16-color palette (8 normal + 8 bright)
- **Text attributes**: Bold, underline, reverse
- **Screen control**: Clear screen, cursor positioning

### UTF-8 Support

The terminal fully supports UTF-8 encoding:

```bash
# Display UTF-8 text
echo "Hello 世界 🌍"

# UTF-8 in filenames
touch "файл.txt"
ls -l файл.txt

# UTF-8 in command arguments
grep "สวัสดี" file.txt
```

### Scrollback Buffer

- The terminal maintains a scrollback buffer of 10,000 lines
- Scroll up/down to view history (implementation-specific)
- Oldest lines are automatically discarded

### Copy and Paste

- Select text with mouse or keyboard
- Copy to clipboard
- Paste from clipboard

## /proc Filesystem

The /proc filesystem provides system and process information.

### Process Information

```bash
# View process status
cat /proc/123/stat

# View command line
cat /proc/123/cmdline

# View human-readable status
cat /proc/123/status

# View current process
cat /proc/self/stat
```

### System Information

```bash
# Memory information
cat /proc/meminfo

# CPU information
cat /proc/cpuinfo

# System uptime
cat /proc/uptime

# System statistics
cat /proc/stat
```

### Debug Information

```bash
# PTY allocation table
cat /proc/debug/pty

# Session tree
cat /proc/debug/sessions

# Lock statistics
cat /proc/debug/locks
```

## Common Tasks

### Viewing System Status

```bash
# List all processes
ps aux

# Check memory usage
cat /proc/meminfo

# Check CPU information
cat /proc/cpuinfo

# System uptime
cat /proc/uptime
```

### File Management

```bash
# Create directory structure
mkdir -p project/src project/docs

# Copy project files
cp -r old_project new_project

# Find files
ls -R / | grep filename

# Search in files
grep -r "TODO" project/
```

### Process Management

```bash
# Start background job
long_running_command &

# List jobs
jobs

# Bring to foreground
fg %1

# Stop foreground job
# (press Ctrl-Z)

# Resume in background
bg %1

# Kill job
kill %1
```

### Working with Pipelines

```bash
# Count files
ls / | wc -l

# Find and display
find / -name "*.txt" | xargs cat

# Process list filtering
ps aux | grep mello | awk '{print $2}'

# Log analysis
cat /var/log/messages | grep error | wc -l
```

## Known Limitations

### Current Limitations

1. **No command line editing**: Arrow keys for history navigation not yet implemented
2. **No tab completion**: File and command completion not available
3. **No wildcards**: Glob patterns (`*.txt`) not expanded by shell
4. **No variables**: Shell variables (`$VAR`) not supported
5. **No conditionals**: `if`, `while`, `for` not implemented
6. **No functions**: Shell functions not supported
7. **No aliases**: Command aliases not available
8. **Limited redirection**: No stderr redirection (`2>`)
9. **No here documents**: `<<EOF` syntax not supported
10. **No process substitution**: `<(command)` not available

### Workarounds

**For wildcards:**
```bash
# Instead of: ls *.txt
# Use:
ls | grep "\.txt$"
```

**For variables:**
```bash
# Instead of: echo $PATH
# Use:
export PATH=/bin:/usr/bin  # Set once
# No way to read back
```

**For conditionals:**
```bash
# Instead of: if [ -f file ]; then cat file; fi
# Use:
cat file  # Will error if not found
```

## Tips and Tricks

### Efficient Command Usage

1. **Use pipelines**: Chain commands instead of intermediate files
   ```bash
   # Good
   cat file | grep pattern | sort
   
   # Avoid
   cat file > temp1
   grep pattern temp1 > temp2
   sort temp2
   ```

2. **Background long tasks**: Use `&` for long-running commands
   ```bash
   long_task &
   # Continue working
   ```

3. **Check job status**: Use `jobs` to monitor background tasks
   ```bash
   jobs
   fg %1  # When ready to interact
   ```

### Keyboard Efficiency

- **Ctrl-C**: Quickly stop misbehaving commands
- **Ctrl-Z**: Pause command to run something else, then `fg` to resume
- **Ctrl-D**: Quick exit from shell

### Process Management

1. **Find process ID**:
   ```bash
   ps aux | grep program_name
   ```

2. **Kill by name** (manual):
   ```bash
   ps aux | grep program_name
   kill <pid>
   ```

3. **Monitor processes**:
   ```bash
   # Repeatedly check
   while true; do ps aux; sleep 1; done
   ```

## Troubleshooting

### Command Not Found

```bash
$ mycommand
Error: Command not found: mycommand
```

**Solutions:**
1. Check if command exists: `which mycommand`
2. Check PATH: `echo $PATH` (if variables supported)
3. Use full path: `/bin/mycommand`

### Permission Denied

```bash
$ cat /root/file
Error: Permission denied
```

**Solutions:**
1. Check file permissions: `ls -l /root/file`
2. Run as appropriate user
3. Check if file exists: `ls /root/`

### Job Control Issues

**Job won't stop with Ctrl-Z:**
- Some programs may ignore SIGTSTP
- Try Ctrl-C to terminate instead

**Can't bring job to foreground:**
- Check job exists: `jobs`
- Use correct job number: `fg %1`

### Terminal Issues

**Garbled output:**
- Terminal may be in wrong mode
- Exit and restart shell
- Check for binary output: `cat binary_file` can corrupt terminal

**No echo:**
- Terminal echo may be disabled
- Exit and restart shell

## Getting Help

### Documentation

- **Architecture docs**: See `docs/architecture/` for technical details
- **Developer guide**: See `docs/DEVELOPER_GUIDE.md` for development info
- **Troubleshooting**: See `docs/TROUBLESHOOTING_GUIDE.md` for common issues

### Command Help

Most utilities support `-h` or `--help`:
```bash
ls --help
grep --help
```

### System Information

Check system status:
```bash
cat /proc/meminfo    # Memory
cat /proc/cpuinfo    # CPU
cat /proc/uptime     # Uptime
ps aux               # Processes
```

## Next Steps

1. **Explore the system**: Try different commands and utilities
2. **Read architecture docs**: Understand how the system works
3. **Experiment with job control**: Practice fg/bg/jobs
4. **Use pipelines**: Combine commands for powerful operations
5. **Check /proc**: Learn about system internals

## Appendix: Quick Reference

### Shell Built-ins

| Command | Description |
|---------|-------------|
| cd      | Change directory |
| pwd     | Print working directory |
| echo    | Print text |
| export  | Set environment variable |
| unset   | Remove environment variable |
| jobs    | List jobs |
| fg      | Foreground job |
| bg      | Background job |
| exit    | Exit shell |
| which   | Locate command |

### Mellobox Utilities

| Command | Description |
|---------|-------------|
| ls      | List directory |
| cp      | Copy files |
| mv      | Move/rename files |
| rm      | Remove files |
| cat     | Display files |
| grep    | Search text |
| ps      | List processes |
| kill    | Send signal |
| mkdir   | Make directory |
| touch   | Create file |
| pwd     | Print directory |
| true    | Exit success |
| false   | Exit failure |

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl-C   | Interrupt (SIGINT) |
| Ctrl-Z   | Suspend (SIGTSTP) |
| Ctrl-D   | EOF / Exit |
| Ctrl-\\  | Quit (SIGQUIT) |

### Common Signals

| Signal  | Number | Description |
|---------|--------|-------------|
| SIGHUP  | 1      | Hangup |
| SIGINT  | 2      | Interrupt |
| SIGQUIT | 3      | Quit |
| SIGKILL | 9      | Kill (force) |
| SIGTERM | 15     | Terminate |
| SIGTSTP | 20     | Stop |
| SIGCONT | 18     | Continue |
