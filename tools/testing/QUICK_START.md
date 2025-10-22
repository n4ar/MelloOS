# Integration Tests - Quick Start Guide

## TL;DR

```bash
# Build the system
make iso

# Run all tests
./tools/testing/test_advanced_userland.sh

# Or run quick tests (skip long stability test)
./tools/testing/test_advanced_userland.sh -quick
```

## Individual Test Suites

```bash
# PTY subsystem
./tools/testing/test_pty_integration.sh

# Job control (fg, bg, signals)
./tools/testing/test_job_control.sh

# Pipelines (cmd1 | cmd2 | cmd3)
./tools/testing/test_pipeline.sh

# I/O redirection (>, >>, <)
./tools/testing/test_io_redirection.sh

# Interactive session (boot to shell)
./tools/testing/test_interactive_session.sh

# Stability (long-running, 2 minutes default)
./tools/testing/test_stability.sh
```

## Common Options

```bash
# Custom timeout
./tools/testing/test_pty_integration.sh -timeout 60

# Short stability test (30s)
./tools/testing/test_stability.sh -short

# Long stability test (5 minutes)
./tools/testing/test_stability.sh -long

# Help
./tools/testing/test_advanced_userland.sh -h
```

## Expected Output

### Success
```
✓ ALL TESTS PASSED!
```

### Failure
```
✗ SOME TESTS FAILED
```
Check the detailed output for specific failures.

## Troubleshooting

### "mellos.iso not found"
```bash
make iso
```

### Tests timeout
```bash
# Increase timeout
./tools/testing/test_pty_integration.sh -timeout 120
```

### Need more details
Check the temporary files listed at the end of test output:
- Results file: Pass/fail for each test case
- QEMU output file: Complete serial console output

## For More Information

See `INTEGRATION_TESTS.md` for complete documentation.
