/*
 * AP (Application Processor) Trampoline Code
 * 
 * This code is copied to physical address 0x8000 and executed by APs
 * when they receive a SIPI (Startup Inter-Processor Interrupt).
 * 
 * The trampoline transitions the AP from 16-bit real mode through
 * 32-bit protected mode to 64-bit long mode, then jumps to the
 * Rust entry point.
 */

.text
.code16

/* Trampoline data structure layout at 0x8000 */
.set TRAMPOLINE_BASE,       0x8000
.set TRAMPOLINE_GDT32,      0x8100
.set TRAMPOLINE_GDT64,      0x8200
.set TRAMPOLINE_STACK_PTR,  0x8300
.set TRAMPOLINE_ENTRY_PTR,  0x8308
.set TRAMPOLINE_CR3,        0x8310
.set TRAMPOLINE_CPU_ID,     0x8318
.set TRAMPOLINE_APIC_ID,    0x8320
.set TRAMPOLINE_LAPIC_ADDR, 0x8328

.globl trampoline_start
.globl trampoline_end

trampoline_start:
    cli                         /* Disable interrupts */
    
    /* Set up segments for real mode */
    xorw    %ax, %ax
    movw    %ax, %ds
    movw    %ax, %es
    movw    %ax, %ss
    
    /* Enable A20 line using fast A20 gate */
    inb     $0x92, %al
    orb     $0x02, %al
    outb    %al, $0x92
    
    /* Load 32-bit GDT */
    lgdt    (gdt32_desc - trampoline_start + TRAMPOLINE_BASE)
    
    /* Enter protected mode: set CR0.PE */
    movl    %cr0, %eax
    orl     $0x1, %eax          /* Set PE bit */
    movl    %eax, %cr0
    
    /* Far jump to 32-bit protected mode code segment */
    ljmp    $0x08, $(protected_mode_entry - trampoline_start + TRAMPOLINE_BASE)

.code32
protected_mode_entry:
    /* Set up 32-bit data segments */
    movw    $0x10, %ax
    movw    %ax, %ds
    movw    %ax, %es
    movw    %ax, %ss
    movw    %ax, %fs
    movw    %ax, %gs
    
    /* Enable PAE (Physical Address Extension): set CR4.PAE */
    movl    %cr4, %eax
    orl     $0x20, %eax         /* Set PAE bit (bit 5) */
    movl    %eax, %cr4
    
    /* Load CR3 with kernel page table */
    movl    (TRAMPOLINE_CR3), %eax
    movl    %eax, %cr3
    
    /* Enable long mode and NX: set EFER.LME and EFER.NXE */
    movl    $0xC0000080, %ecx   /* EFER MSR */
    rdmsr
    orl     $0x900, %eax        /* Set LME (bit 8) and NXE (bit 11) */
    wrmsr
    
    /* Enable paging: set CR0.PG */
    movl    %cr0, %eax
    orl     $0x80000000, %eax   /* Set PG bit (bit 31) */
    movl    %eax, %cr0
    
    /* Load 64-bit GDT */
    lgdt    (gdt64_desc - trampoline_start + TRAMPOLINE_BASE)
    
    /* Far jump to 64-bit long mode code segment */
    ljmp    $0x08, $(long_mode_entry - trampoline_start + TRAMPOLINE_BASE)

.code64
long_mode_entry:
    /* Set up 64-bit data segments */
    xorw    %ax, %ax
    movw    %ax, %ds
    movw    %ax, %es
    movw    %ax, %fs
    movw    %ax, %gs
    
    /* Set up stack segment with proper data selector */
    movw    $0x10, %ax
    movw    %ax, %ss
    
    /* Load stack pointer */
    movq    (TRAMPOLINE_STACK_PTR), %rsp
    
    /* Verify stack is valid */
    testq   %rsp, %rsp
    jz      .Lhalt_error
    
    /* Align stack to 16-byte boundary */
    /* x86-64 calling convention requires RSP % 16 == 8 before call */
    andq    $~0xF, %rsp        /* Clear low 4 bits */
    subq    $8, %rsp           /* Adjust so RSP % 16 == 8 */
    
    /* Load arguments for Rust function */
    movq    (TRAMPOLINE_CPU_ID), %rdi     /* First arg: cpu_id */
    movq    (TRAMPOLINE_APIC_ID), %rsi    /* Second arg: apic_id (only low byte used) */
    movq    (TRAMPOLINE_LAPIC_ADDR), %rdx /* Third arg: lapic_address */
    
    /* Load entry point address */
    movq    (TRAMPOLINE_ENTRY_PTR), %rax
    
    /* Verify entry point is valid */
    testq   %rax, %rax
    jz      .Lhalt_error
    
    /* Debug: Write 'R' to serial (about to jump) */
    pushq   %rax
    pushq   %rdx
    movb    $'R', %al
    movw    $0x3F8, %dx
    outb    %al, %dx
    popq    %rdx
    popq    %rax
    
    /* Double-check address is in higher-half */
    movq    %rax, %rcx
    shrq    $48, %rcx
    testq   %rcx, %rcx
    jz      .Lbad_address
    
    /* Debug: 'J' before jump */
    pushq   %rax
    pushq   %rdx
    movb    $'J', %al
    movw    $0x3F8, %dx
    outb    %al, %dx
    popq    %rdx
    popq    %rax
    
    /* Save entry point and jump to lower-half wrapper */
    movq    %rax, %r15
    leaq    (rust_entry_wrapper - trampoline_start + TRAMPOLINE_BASE), %rax
    jmp     *%rax
    
.Lbad_address:
    pushq   %rdx
    movb    $'Z', %al
    movw    $0x3F8, %dx
    outb    %al, %dx
    popq    %rdx
    jmp     .Lhalt_error
    
.Lhalt_error:
    cli
    hlt
    jmp     .Lhalt_error

/* Lower-half wrapper that calls higher-half Rust */
rust_entry_wrapper:
    /* Debug: 'W' = entered wrapper */
    pushq   %rdx
    movb    $'W', %al
    movw    $0x3F8, %dx
    outb    %al, %dx
    popq    %rdx
    
    /* Arguments are already in rdi, rsi, rdx from earlier */
    /* Entry point is in r15 */
    callq   *%r15
    
    /* Should never return */
    pushq   %rdx
    movb    $'N', %al
    movw    $0x3F8, %dx
    outb    %al, %dx
    popq    %rdx
    cli
    hlt

/* 32-bit GDT for protected mode transition */
.align 16
gdt32:
    .quad   0x0000000000000000  /* Null descriptor */
    .quad   0x00CF9A000000FFFF  /* Code segment: base=0, limit=4GB, executable, readable */
    .quad   0x00CF92000000FFFF  /* Data segment: base=0, limit=4GB, writable */

gdt32_desc:
    .word   23                                              /* Limit (3 entries * 8 - 1) */
    .long   gdt32 - trampoline_start + TRAMPOLINE_BASE      /* Base */

/* 64-bit GDT for long mode */
.align 16
gdt64:
    .quad   0x0000000000000000  /* Null descriptor */
    .quad   0x00209A0000000000  /* Code segment: L=1, 64-bit, executable, readable */
    .quad   0x0000920000000000  /* Data segment: 64-bit, writable */

gdt64_desc:
    .word   23                                              /* Limit (3 entries * 8 - 1) */
    .long   gdt64 - trampoline_start + TRAMPOLINE_BASE      /* Base */

.align 16
trampoline_end:
