/* User Entry Trampoline Assembly
 * 
 * This file provides assembly functions for transitioning from kernel mode (ring 0)
 * to user mode (ring 3) using the IRET instruction.
 * 
 * Uses AT&T syntax for GNU assembler compatibility.
 */

.section .text

/* Constants for user segments */
.set USER_CODE_SEG, 0x3B    /* Ring 3 code segment (0x38 | 3) */
.set USER_DATA_SEG, 0x43    /* Ring 3 data segment (0x40 | 3) */

/* User space address limit for validation */
.set USER_LIMIT_LOW, 0x00000000
.set USER_LIMIT_HIGH, 0x00008000

.global user_entry_trampoline
.type user_entry_trampoline, @function

/* 
 * Transition to user mode (ring 3)
 * 
 * Arguments:
 *   %rdi = entry_point (user program entry point)
 *   %rsi = user_stack_top (user stack pointer)
 * 
 * This function never returns - it transitions to user mode
 */
user_entry_trampoline:
    /* Disable interrupts during transition setup */
    cli
    
    /* Validate arguments are canonical user addresses */
    /* Check entry_point is canonical user address (< USER_LIMIT) */
    movq $USER_LIMIT_HIGH, %rax
    shlq $32, %rax
    addq $USER_LIMIT_LOW, %rax
    cmpq %rax, %rdi
    jae .invalid_entry
    
    /* Check user_stack is canonical user address (< USER_LIMIT) */
    cmpq %rax, %rsi
    jae .invalid_stack
    
    /* Prepare IRET stack frame for transition to ring 3
     * Stack layout (pushed in reverse order):
     * [SS]     - User data segment with RPL=3
     * [RSP]    - User stack pointer
     * [RFLAGS] - Processor flags with IF=1
     * [CS]     - User code segment with RPL=3
     * [RIP]    - User program entry point
     */
    
    /* Push SS (user data segment with RPL=3) */
    pushq $USER_DATA_SEG
    
    /* Push RSP (user stack pointer - validated above) */
    pushq %rsi
    
    /* Push RFLAGS (current flags with IF enabled) */
    pushfq
    orq $0x200, (%rsp)   /* Set IF (interrupt enable) flag */
    
    /* Push CS (user code segment with RPL=3) */
    pushq $USER_CODE_SEG
    
    /* Push RIP (entry point - validated above) */
    pushq %rdi
    
    /* Clear all general-purpose registers for security
     * This prevents information leakage from kernel to user space */
    xorq %rax, %rax
    xorq %rbx, %rbx
    xorq %rcx, %rcx
    xorq %rdx, %rdx
    xorq %rdi, %rdi    /* Entry point no longer needed */
    xorq %rsi, %rsi    /* Stack pointer no longer needed */
    xorq %r8, %r8
    xorq %r9, %r9
    xorq %r10, %r10
    xorq %r11, %r11
    xorq %r12, %r12
    xorq %r13, %r13
    xorq %r14, %r14
    xorq %r15, %r15
    
    /* Clear frame pointer */
    xorq %rbp, %rbp
    
    /* Transition to user mode (ring 3)
     * IRET will:
     * - Pop RIP, CS, RFLAGS, RSP, SS from stack
     * - Set CPL=3 (from CS.RPL)
     * - Enable interrupts (from RFLAGS.IF)
     * - Switch to user stack
     */
    iretq

/* Error handlers for invalid transitions */
.invalid_entry:
    movq $0xDEAD0001, %rdi         /* Error code for invalid entry point */
    call kernel_panic_invalid_user_transition
    
.invalid_stack:
    movq $0xDEAD0002, %rdi         /* Error code for invalid stack */
    call kernel_panic_invalid_user_transition

.size user_entry_trampoline, . - user_entry_trampoline

/* 
 * Get current privilege level for testing
 * Returns the current privilege level (0-3) in RAX
 */
.global get_current_privilege_level
.type get_current_privilege_level, @function

get_current_privilege_level:
    movw %cs, %ax          /* Get current code segment */
    andq $3, %rax          /* Extract RPL (bits 0-1) */
    ret

.size get_current_privilege_level, . - get_current_privilege_level

/*
 * Read current RIP for debugging
 * Returns the current instruction pointer in RAX
 */
.global read_current_rip
.type read_current_rip, @function

read_current_rip:
    leaq (%rip), %rax      /* Load effective address of next instruction */
    ret

.size read_current_rip, . - read_current_rip