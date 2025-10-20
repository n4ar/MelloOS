# Troubleshooting Documentation

Debugging guides and issue resolution for MelloOS development.

## Documents

- **[troubleshooting.md](troubleshooting.md)**: General troubleshooting guide
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
- Review [smp-safety.md](smp-safety.md) for synchronization guidelines
- Use [smp-boot-debug.md](smp-boot-debug.md) for boot-specific problems