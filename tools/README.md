# MelloOS Development Tools

This directory contains various tools for building, testing, and debugging MelloOS.

## Directory Structure

```
tools/
├── qemu/           # QEMU virtualization and testing scripts
├── debug/          # Debugging tools and utilities
├── testing/        # Build verification and testing scripts
└── README.md       # This file
```

## Quick Reference

### QEMU Testing
```bash
# Basic QEMU launch (4 CPUs with KVM)
./qemu/qemu.sh

# SMP testing with different CPU counts
./qemu/qemu.sh -smp 2 -enable-kvm
./qemu/qemu.sh -preset smp4

# Quick SMP test scripts
./qemu/qemu-test-smp2.sh    # 2 CPUs optimized
./qemu/qemu-test-smp4.sh    # 4 CPUs optimized

# Debug mode with extensive logging
./qemu/qemu-debug-smp.sh
```

### Testing & Verification
```bash
# Automated boot testing
./testing/test_boot.sh
./testing/test_boot.sh -smp 4

# Build verification
./testing/verify_build.sh
```

### Debugging
```bash
# GDB debugging for SMP issues
gdb -x debug/gdb-smp.gdb

# Triple fault analysis
./debug/analyze-triple-fault.sh
```

## Tool Categories

### 🚀 QEMU Tools (`qemu/`)
- **qemu.sh**: Main QEMU launcher with SMP support and presets
- **qemu-test-smp2.sh**: Optimized 2-CPU testing script
- **qemu-test-smp4.sh**: Optimized 4-CPU testing script  
- **qemu-debug-smp.sh**: Debug mode with extensive logging
- **qemu-smp2.sh**: Legacy 2-CPU script (redirects to main)

### 🐛 Debug Tools (`debug/`)
- **gdb-smp.gdb**: GDB script for SMP debugging
- **analyze-triple-fault.sh**: Triple fault analysis utility

### 🧪 Testing Tools (`testing/`)
- **test_boot.sh**: Automated kernel boot testing with SMP support
- **verify_build.sh**: Build verification and validation

## Usage Examples

### Basic Development Workflow
```bash
# 1. Build and test
make build && make iso

# 2. Verify build
./testing/verify_build.sh

# 3. Test boot
./testing/test_boot.sh

# 4. Run with QEMU
./qemu/qemu.sh -preset smp4
```

### SMP Testing Workflow
```bash
# Test different CPU configurations
./testing/test_boot.sh -smp 1    # Single CPU
./testing/test_boot.sh -smp 2    # Dual CPU
./testing/test_boot.sh -smp 4    # Quad CPU

# Interactive SMP testing
./qemu/qemu-test-smp2.sh         # Quick 2-CPU test
./qemu/qemu-test-smp4.sh         # Quick 4-CPU test
```

### Debugging Workflow
```bash
# For SMP-related issues
./qemu/qemu-debug-smp.sh         # Run with debug output
gdb -x debug/gdb-smp.gdb         # Attach GDB debugger

# For triple faults
./debug/analyze-triple-fault.sh  # Analyze crash logs
```

## Adding New Tools

When adding new tools, follow this organization:

- **QEMU/Virtualization**: Add to `qemu/` directory
- **Debugging/Analysis**: Add to `debug/` directory  
- **Testing/Verification**: Add to `testing/` directory

Make sure to:
1. Add executable permissions (`chmod +x`)
2. Include help text (`--help` option)
3. Update this README with usage examples
4. Follow existing naming conventions