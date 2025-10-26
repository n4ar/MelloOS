---
inclusion: always
---

# MelloOS Build Process

## Standard Build Sequence

**MANDATORY:** Always follow this exact sequence when building or testing MelloOS:

```bash
make clean
make build
make iso
./tools/qemu.sh
```

### Build Steps Explained

1. **make clean** - Removes all build artifacts (prevents stale code issues)
2. **make build** - Compiles kernel and all userspace programs
3. **make iso** - Creates bootable ISO image with Limine bootloader
4. **./tools/qemu.sh** - Launches QEMU with correct configuration

### Common Mistakes to Avoid

❌ **NEVER do this:**
- `make run` - May use stale ISO, skips proper build steps
- `make build && make run` - Skips ISO creation step
- Building without `make clean` first - May have stale artifacts
- Using `make` alone - Incomplete build

✅ **ALWAYS do this:**
```bash
# One-liner (preferred)
make clean && make build && make iso && ./tools/qemu.sh

# Or step-by-step
make clean
make build
make iso
./tools/qemu.sh
```

## When to Use Full Build Sequence

Execute the full build sequence after:
- Modifying any kernel source code
- Modifying any userspace program
- Changing build configuration (Cargo.toml, Makefile, linker scripts)
- Updating dependencies
- Testing filesystem changes
- Debugging boot or runtime issues
- **ANY code change you want to test**

## Development Build Modes

### Debug Build (Default)
```bash
make clean && make build && make iso
```
- Includes debug symbols
- No optimizations
- Larger binary size
- Easier debugging with GDB

### Release Build (Optimized)
```bash
make clean && make build-release && make iso-release
```
- Full optimizations
- Smaller binary size
- Faster execution
- Use for performance testing

### Development Build with sccache
```bash
SCCACHE=1 make build-dev
make iso-dev
```
- Faster incremental builds
- Caches compilation artifacts
- Useful for rapid iteration
- Requires sccache installed

## QEMU Execution Rules

**CRITICAL:** QEMU is a long-running process and must be handled correctly.

### Correct QEMU Usage

```bash
# Use controlBashProcess with action "start"
# This runs QEMU as a background process
./tools/qemu.sh
```

**Why background process:**
- QEMU doesn't terminate automatically
- Allows monitoring output while continuing work
- Prevents blocking other operations
- Can be stopped when needed

### Monitoring QEMU

```bash
# Use getProcessOutput to view QEMU output
# Use listProcesses to see running QEMU instances
# Use controlBashProcess with action "stop" to terminate
```

### QEMU Scripts Available

- `./tools/qemu.sh` - Standard single-core boot
- `./tools/qemu/qemu-smp2.sh` - 2-core SMP testing
- `./tools/qemu/qemu-test-smp4.sh` - 4-core SMP testing
- `./tools/qemu/qemu-debug-smp.sh` - SMP with GDB debugging

## Quick Verification (No Boot Test)

For syntax and type checking only (does NOT test runtime behavior):

```bash
cd kernel && cargo check
```

**Important:** `cargo check` verifies compilation but does NOT:
- Test actual system behavior
- Verify boot process
- Test userspace integration
- Validate runtime correctness

**Always follow with full build sequence to verify functionality.**

## Build Targets Reference

### Kernel Targets
- `make build` - Build kernel (debug)
- `make build-release` - Build kernel (optimized)
- `make build-dev` - Build kernel (dev mode with sccache)

### Userspace Targets
- `make userspace` - Build all userspace programs
- `make userspace-dev` - Build userspace (dev mode with sccache)

### ISO Targets
- `make iso` - Create bootable ISO (debug)
- `make iso-release` - Create bootable ISO (release)
- `make iso-dev` - Create bootable ISO (dev mode)

### Utility Targets
- `make clean` - Remove all build artifacts
- `make run` - ⚠️ Avoid - use full sequence instead

## Integration with Workflow

### After Code Changes
1. Run `cargo check` immediately (see cargo-check-policy.md)
2. Fix any errors found
3. Execute full build sequence
4. Monitor QEMU serial output for runtime issues

### Before Testing Features
1. Ensure clean build: `make clean`
2. Build everything: `make build && make iso`
3. Launch QEMU as background process
4. Run relevant test scripts from `tools/testing/`

### Before Committing
1. Full clean build passes
2. System boots successfully in QEMU
3. Relevant integration tests pass
4. No warnings in build output

## Troubleshooting Build Issues

**Stale artifacts:** Always `make clean` first
**Linker errors:** Check linker.ld files haven't been corrupted
**Missing symbols:** Ensure all dependencies are built
**ISO boot failure:** Verify Limine bootloader configuration
**QEMU won't start:** Check for port conflicts or stale processes

## Summary

**Golden Rule:** `make clean && make build && make iso && ./tools/qemu.sh`

This ensures you're always testing the latest code with a fresh, complete build.