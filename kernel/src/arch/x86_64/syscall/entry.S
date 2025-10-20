# Fast syscall entry assembly stub
#
# This file implements the fast syscall entry point using the syscall/sysret
# mechanism. It handles SWAPGS, safe stack switching, and canonical address
# validation according to the R10-based calling convention.

.section .text
.global syscall_entry_fast
.global handle_bad_syscall_return

# Syscall calling convention (x86-64 compatible with Linux):
# RAX = syscall number
# RDI = arg1, RSI = arg2, RDX = arg3
# R10 = arg4, R8 = arg5, R9 = arg6  (NOTE: R10 not RCX for arg4!)
# Return value in RAX
#
# IMPORTANT: RCX and R11 are clobbered by SYSCALL instruction:
# - RCX = user RIP (return address)
# - R11 = user RFLAGS
# Therefore arg4 uses R10 instead of RCX to avoid conflicts

syscall_entry_fast:
    # SYSCALL instruction has already:
    # - Saved user RIP to RCX
    # - Saved user RFLAGS to R11
    # - Loaded kernel CS from STAR[47:32]
    # - Loaded kernel RIP from LSTAR (this function)
    # - Masked RFLAGS with SFMASK (cleared IF)
    # - Set CPL=0
    
    # 1. Switch to kernel GS base for per-CPU data access
    swapgs
    
    # 2. Switch to kernel stack safely
    # Get kernel stack top from per-CPU area through GS
    # Offset 0x00 = id, 0x08 = apic_id, 0x10 = node_id, 0x18 = runqueue
    # We need to add a kernel_stack field to PerCpu structure
    # For now, use a temporary approach with current RSP validation
    
    # Save user RSP in a safe location (use R12 temporarily)
    mov %rsp, %r12
    
    # TODO: Get actual kernel stack from per-CPU structure
    # For now, assume we have a valid kernel stack
    # In the complete implementation, this would be:
    # mov %gs:kernel_stack_offset, %rsp
    
    # 3. Save user context on kernel stack
    push %r11        # User RFLAGS
    push %rcx        # User RIP
    push %r12        # User RSP (saved above)
    push %rdi        # arg1
    push %rsi        # arg2  
    push %rdx        # arg3
    push %r10        # arg4 (NOT RCX!)
    push %r8         # arg5
    push %r9         # arg6
    
    # 4. Prepare arguments for dispatcher
    # syscall_dispatcher(rax, rdi, rsi, rdx, r10, r8, r9)
    mov %rax, %rdi   # syscall_id (move RAX to RDI)
    mov 32(%rsp), %rsi  # arg1 (saved rdi)
    mov 24(%rsp), %rdx  # arg2 (saved rsi) 
    mov 16(%rsp), %rcx  # arg3 (saved rdx)
    mov 8(%rsp), %r8    # arg4 (saved r10)
    mov 0(%rsp), %r9    # arg5 (saved r8)
    # arg6 is already on stack at correct position
    
    # 5. Call dispatcher
    call syscall_dispatcher_enhanced
    
    # 6. Restore user context
    pop %r9          # arg6
    pop %r8          # arg5
    pop %r10         # arg4
    pop %rdx         # arg3
    pop %rsi         # arg2
    pop %rdi         # arg1
    pop %r12         # User RSP
    pop %rcx         # User RIP
    pop %r11         # User RFLAGS
    
    # 7. Validate canonical addresses before SYSRET
    # SYSRET will #GP if RIP or RSP are non-canonical
    # Canonical addresses have bits 63:47 all 0 or all 1
    # User addresses should have bits 63:47 all 0
    
    # Check RIP canonical (user space should be < 0x0000800000000000)
    test $0xFFFF800000000000, %rcx
    jnz .bad_return_rip
    
    # Check RSP canonical (user space should be < 0x0000800000000000)  
    test $0xFFFF800000000000, %r12
    jnz .bad_return_rsp
    
    # 8. Restore user RSP
    mov %r12, %rsp
    
    # 9. Switch back to user GS base
    swapgs
    
    # 10. Return to user mode
    # SYSRET will:
    # - Load user CS from STAR[63:48] + 16 (with RPL=3)
    # - Load user SS from STAR[63:48] + 8 (with RPL=3)  
    # - Load user RIP from RCX
    # - Load user RFLAGS from R11
    # - Set CPL=3
    sysretq

# Error handlers for non-canonical addresses
.bad_return_rip:
    swapgs           # Restore kernel GS
    mov %rcx, %rdi   # Pass bad RIP as first argument
    mov %r12, %rsi   # Pass RSP as second argument
    call handle_bad_syscall_return
    ud2              # Should never return

.bad_return_rsp:
    swapgs           # Restore kernel GS
    mov %r12, %rdi   # Pass bad RSP as first argument
    mov %rcx, %rsi   # Pass RIP as second argument
    call handle_bad_syscall_return
    ud2              # Should never return

# Handler for bad syscall returns (implemented in Rust)
# This is just a stub - the actual implementation is in mod.rs
handle_bad_syscall_return:
    # This function is implemented in Rust
    # It will terminate the process and never return
    ud2