# SMP Boot Debugging Guide

## Problem Summary

When attempting to bring up Application Processors (APs) in SMP mode, the system experiences a triple fault immediately after sending SIPI to AP#1. The system then reboots continuously.

## Symptoms

1. BSP successfully initializes and sends INIT IPI to AP#1
2. BSP sends first SIPI to AP#1
3. Triple fault occurs (indicated by system reboot)
4. No AP boot messages appear in serial output

## What Has Been Tried

### 1. Identity Mapping for Low Memory ✅
- Created identity mapping for 0x0-0x1FFFFF using 2MB huge page
- This covers the trampoline code at 0x8000
- Mapping is confirmed to be created successfully

### 2. GDT Descriptor Fixes ✅
- Fixed 64-bit GDT descriptors to use correct format:
  - Code: `0x00209A0000000000` (L=1, 64-bit)
  - Data: `0x0000920000000000`
- Fixed GDT limits to be correct (23 bytes for 3 entries)

### 3. Trampoline Code Improvements ✅
- Simplified long mode entry point
- Removed unnecessary register moves
- Used direct memory access for trampoline data
- Added error handling for invalid stack/entry point

### 4. CR3 Loading Fix ✅
- Fixed CR3 loading in 32-bit mode to handle 64-bit physical address
- Load both EAX and EDX for full 64-bit value

## Remaining Issues

### Possible Root Causes

1. **Page Table Issue**
   - The kernel page table (loaded into CR3) might not have proper mappings
   - Higher-half kernel addresses might not be accessible from AP
   - Stack pointer might point to unmapped memory

2. **Stack Allocation**
   - AP stack is allocated with `kmalloc()` which returns virtual address
   - Stack might not be properly mapped in the page table
   - Stack might be in higher-half but AP can't access it yet

3. **Entry Point Address**
   - `ap_entry64` function address might be virtual (higher-half)
   - AP might not be able to jump to higher-half address immediately

4. **GDT Base Address**
   - GDT descriptors use 32-bit base address
   - In 64-bit mode, this might cause issues if GDT is in higher-half

## Recommended Next Steps

### 1. Use QEMU Monitor for Debugging

```bash
qemu-system-x86_64 -monitor stdio -d int,cpu_reset ...
```

This will show:
- CPU state at triple fault
- Which instruction caused the fault
- Register values (RIP, RSP, CR3, etc.)

### 2. Add Serial Debug Output to Trampoline

Modify `boot_ap.asm` to write to serial port at each stage:
- After entering protected mode
- After enabling PAE
- After enabling long mode
- Before jumping to Rust code

### 3. Verify Page Table Mappings

Add debug code to print:
- CR3 value being passed to AP
- Stack address being passed to AP
- Entry point address being passed to AP
- Verify these addresses are properly mapped

### 4. Use Lower-Half Addresses

Consider using lower-half addresses for:
- AP stack (allocate in first 2MB)
- AP entry point (create trampoline in lower-half)
- Transition to higher-half after AP is running

### 5. Test with Single AP First

Modify code to only bring up AP#1, making debugging easier.

## Code Locations

- Trampoline code: `kernel/src/arch/x86_64/smp/boot_ap.asm`
- SMP initialization: `kernel/src/arch/x86_64/smp/mod.rs`
- Identity mapping: `kernel/src/arch/x86_64/smp/mod.rs::identity_map_low_memory()`
- AP entry point: `kernel/src/arch/x86_64/smp/mod.rs::ap_entry64()`

## Temporary Workaround

SMP is currently disabled to allow single-core testing. To re-enable:

1. In `kernel/src/main.rs`, uncomment the `init_smp()` call
2. Comment out the single-core fallback code

## References

- Intel SDM Volume 3, Chapter 8: Multiple-Processor Management
- OSDev Wiki: SMP
- OSDev Wiki: Trampoline

## Status

**BLOCKED**: Requires deeper debugging with QEMU monitor or GDB to identify exact fault location.

**Priority**: HIGH - SMP is a core feature for Phase 5

**Estimated Effort**: 4-8 hours with proper debugging tools
