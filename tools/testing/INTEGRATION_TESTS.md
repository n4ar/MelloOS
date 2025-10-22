# Advanced Userland & Shell Integration Tests

This directory contains comprehensive integration tests for Phase 6.6 - Advanced Userland & Shell Environment.

## Overview

The integration test suite validates the complete functionality of:
- PTY (pseudo-terminal) subsystem
- Job control and signal handling
- Shell pipeline execution
- I/O redirection
- Interactive terminal sessions
- System stability and resource management

## Test Suites

### 1. PTY Integration Test (`test_pty_integration.sh`)

Tests the PTY subsystem functionality.

**What it tests:**
- PTY pair allocation (`/dev/ptmx`, `/dev/pts/N`)
- Termios configuration (ICANON, ECHO, ISIG)
- ANSI escape sequence parsing
- Window resize and SIGWINCH delivery
- PTY read/write operations
- Line buffering in canonical mode

**Requirements covered:** 1.1, 1.2, 1.3, 1.4, 1.5

**Usage:**
```bash
./test_pty_integration.sh
./test_pty_integration.sh -timeout 45
```

### 2. Job Control Test (`test_job_control.sh`)

Tests shell job control functionality.

**What it tests:**
- Background job execution (`&`)
- Job table management (`jobs` command)
- SIGTSTP (Ctrl-Z) handling
- Foreground/background switching (`fg`, `bg`)
- SIGCONT delivery
- Process group management (setpgid, tcsetpgrp)
- SIGCHLD handling

**Requirements covered:** 2.1, 2.2, 2.3, 2.4, 2.5, 2.6

**Usage:**
```bash
./test_job_control.sh
./test_job_control.sh -timeout 60
```

### 3. Pipeline Test (`test_pipeline.sh`)

Tests shell pipeline functionality.

**What it tests:**
- Simple two-stage pipelines (`cmd1 | cmd2`)
- Multi-stage pipelines (`cmd1 | cmd2 | cmd3`)
- Process group management (all processes in same PGID)
- Exit status propagation (last command)
- Pipe buffer handling
- File descriptor management (dup2, close)

**Requirements covered:** 3.1, 3.8

**Usage:**
```bash
./test_pipeline.sh
./test_pipeline.sh -timeout 30
```

### 4. I/O Redirection Test (`test_io_redirection.sh`)

Tests shell I/O redirection functionality.

**What it tests:**
- Output redirection (`> file`)
- Append redirection (`>> file`)
- Input redirection (`< file`)
- Combined pipes and redirects
- File descriptor management (dup2)
- File creation and content verification

**Requirements covered:** 3.2

**Usage:**
```bash
./test_io_redirection.sh
./test_io_redirection.sh -timeout 30
```

### 5. Interactive Session Test (`test_interactive_session.sh`)

Tests complete interactive terminal session.

**What it tests:**
- System boot and terminal startup
- Shell prompt display (< 10ms target)
- Basic commands (`ls`, `cd`, `pwd`)
- /proc filesystem access
- Process listing (`ps aux`)
- Command execution and response time
- System stability during interactive use

**Requirements covered:** 8.1, 8.4

**Usage:**
```bash
./test_interactive_session.sh
./test_interactive_session.sh -timeout 45
```

### 6. Stability Test (`test_stability.sh`)

Long-running tests for system stability.

**What it tests:**
- Extended runtime stability (no kernel panics)
- Repeated command execution
- Zombie process detection and reaping
- Memory leak detection
- Resource exhaustion handling
- Lock contention and deadlock detection
- SMP safety on multiple CPUs

**Requirements covered:** 12.1, 12.2, 12.4, 12.5

**Usage:**
```bash
./test_stability.sh                # Default: 120s
./test_stability.sh -short         # 30s
./test_stability.sh -long          # 300s (5 minutes)
./test_stability.sh -timeout 180   # Custom timeout
```

## Master Test Runner

### `test_advanced_userland.sh`

Runs all integration tests in sequence.

**Usage:**
```bash
# Run all tests
./test_advanced_userland.sh

# Run all tests except stability (quick mode)
./test_advanced_userland.sh -quick

# Run specific test suites
./test_advanced_userland.sh -pty
./test_advanced_userland.sh -job -pipeline
./test_advanced_userland.sh -interactive -stability

# Set custom timeout for all tests
./test_advanced_userland.sh -timeout 60
```

**Options:**
- `-all` - Run all test suites (default)
- `-pty` - Run PTY integration tests only
- `-job` - Run job control tests only
- `-pipeline` - Run pipeline tests only
- `-io` - Run I/O redirection tests only
- `-interactive` - Run interactive session tests only
- `-stability` - Run stability tests only
- `-quick` - Run all tests except stability
- `-timeout N` - Set timeout for each test suite
- `-h, --help` - Show help message

## Prerequisites

Before running tests:

1. Build the kernel and userland:
   ```bash
   make clean
   make iso
   ```

2. Ensure QEMU is installed:
   ```bash
   which qemu-system-x86_64
   ```

3. Ensure you have sufficient disk space for temporary files

## Test Output

Each test script produces:
- **Console output**: Real-time test progress and results
- **Results file**: Detailed pass/fail for each test case
- **QEMU output file**: Complete serial console output from the VM

Output files are saved to temporary directories (displayed at end of test).

## Interpreting Results

### Success
```
✓ ALL TESTS PASSED!
```
All test cases passed. The feature is working correctly.

### Partial Failure
```
✗ SOME TESTS FAILED
```
Some test cases failed. Review the detailed output to identify issues.

### No Tests Completed
```
⚠ NO TESTS COMPLETED
```
The system failed to boot or crashed early. Check:
- Kernel build errors
- Boot configuration issues
- Critical kernel bugs

## Common Issues

### Test Timeout
If tests timeout without completing:
- Increase timeout: `-timeout 120`
- Check if system is hanging
- Review QEMU output for kernel panics

### ISO Not Found
```
Error: mellos.iso not found. Run 'make iso' first.
```
Solution: Run `make iso` to build the bootable image.

### QEMU Not Found
```
qemu-system-x86_64: command not found
```
Solution: Install QEMU for your platform.

### Inconsistent Results
If tests pass sometimes and fail other times:
- May indicate race conditions
- Run stability test for extended period
- Check SMP-related issues

## Integration with CI/CD

These tests can be integrated into continuous integration:

```bash
#!/bin/bash
# CI test script

set -e

# Build
make clean
make iso

# Run quick tests (skip long stability test)
./tools/testing/test_advanced_userland.sh -quick

# Exit with test result
exit $?
```

## Development Workflow

When developing Phase 6.6 features:

1. **During development**: Run specific test suite
   ```bash
   ./test_pty_integration.sh
   ```

2. **Before commit**: Run quick tests
   ```bash
   ./test_advanced_userland.sh -quick
   ```

3. **Before release**: Run full test suite including stability
   ```bash
   ./test_advanced_userland.sh
   ```

## Test Coverage

| Feature | Test Suite | Coverage |
|---------|-----------|----------|
| PTY allocation | PTY Integration | ✓ |
| Termios | PTY Integration | ✓ |
| ANSI parsing | PTY Integration | ✓ |
| SIGWINCH | PTY Integration | ✓ |
| Background jobs | Job Control | ✓ |
| fg/bg commands | Job Control | ✓ |
| SIGTSTP/SIGCONT | Job Control | ✓ |
| Process groups | Job Control, Pipeline | ✓ |
| Pipelines | Pipeline | ✓ |
| I/O redirection | I/O Redirection | ✓ |
| Shell prompt | Interactive Session | ✓ |
| Basic commands | Interactive Session | ✓ |
| /proc filesystem | Interactive Session | ✓ |
| Zombie reaping | Stability | ✓ |
| Memory leaks | Stability | ✓ |
| Kernel panics | Stability | ✓ |
| SMP safety | Stability | ✓ |

## Future Enhancements

Potential additions to the test suite:

- **Performance benchmarks**: Measure latency and throughput
- **Stress tests**: High load scenarios
- **Fuzzing**: Random input testing for robustness
- **Regression tests**: Specific bug reproduction tests
- **UTF-8 tests**: International character handling
- **Security tests**: Permission and isolation validation

## Contributing

When adding new tests:

1. Follow the existing test script structure
2. Use consistent output formatting
3. Include help text (`-h` option)
4. Document in this README
5. Add to master test runner if appropriate

## References

- Design Document: `.kiro/specs/advanced-userland-shell/design.md`
- Requirements: `.kiro/specs/advanced-userland-shell/requirements.md`
- Tasks: `.kiro/specs/advanced-userland-shell/tasks.md`
