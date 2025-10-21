# Troubleshooting Documentation

Debugging guides and issue resolution for MelloOS development.

## Documents

- **[troubleshooting.md](troubleshooting.md)**: General troubleshooting guide
- **[smp-ap-boot-issues.md](smp-ap-boot-issues.md)**: ⭐ **Complete guide to AP boot issues and solutions**
- **[DEBUG-SMP-TRIPLE-FAULT.md](DEBUG-SMP-TRIPLE-FAULT.md)**: SMP triple fault debugging
- **[smp-boot-debug.md](smp-boot-debug.md)**: SMP boot process debugging  
- **[smp-safety.md](smp-safety.md)**: SMP safety and synchronization guidelines
- **[smp-triple-fault-fix.md](smp-triple-fault-fix.md)**: Specific triple fault fixes

## Common Issues

### Boot Problems
- Check [troubleshooting.md](troubleshooting.md) for general boot issues
- For SMP boot problems, see [smp-boot-debug.md](smp-boot-debug.md)

### Triple Faults
- Start with [DEBUG-SMP-TRIPLE-FAULT.md](DEBUG-SMP-TRIPLE-FAULT.md)
- Apply fixes from [smp-triple-fault-fix.md](smp-triple-fault-fix.md)

### SMP Issues
- **Start here:** [smp-ap-boot-issues.md](smp-ap-boot-issues.md) - Complete guide with all major issues and fixes
- Review [smp-safety.md](smp-safety.md) for synchronization guidelines
- Use [smp-boot-debug.md](smp-boot-debug.md) for boot-specific problems

### Key Issues Resolved
1. **LAPIC Address Corruption** - Register preservation in trampoline assembly
2. **CPU ID Corruption** - Syscall MSR initialization parameter passing
3. **CPU_COUNT Synchronization** - Single source of truth between modules

See [smp-ap-boot-issues.md](smp-ap-boot-issues.md) for detailed explanations and solutions.