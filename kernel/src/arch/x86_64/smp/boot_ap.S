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
    
    /* Enable long mode: set EFER.LME */
    movl    $0xC0000080, %ecx   /* EFER MSR */
    rdmsr
    orl     $0x100, %eax        /* Set LME bit (bit 8) */
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
    movw    $0x10, %ax
    movw    %ax, %ds
    movw    %ax, %es
    movw    %ax, %ss
    movw    %ax, %fs
    movw    %ax, %gs
    
    /* Load stack pointer */
    movq    (TRAMPOLINE_STACK_PTR), %rsp
    
    /* Load CPU ID into first argument register (rdi) */
    movq    (TRAMPOLINE_CPU_ID), %rdi
    
    /* Jump to Rust entry point */
    movq    (TRAMPOLINE_ENTRY_PTR), %rax
    jmp     *%rax

/* 32-bit GDT for protected mode transition */
.align 16
gdt32:
    .quad   0x0000000000000000  /* Null descriptor */
    .quad   0x00CF9A000000FFFF  /* Code segment: base=0, limit=4GB, executable, readable */
    .quad   0x00CF92000000FFFF  /* Data segment: base=0, limit=4GB, writable */

gdt32_desc:
    .word   gdt32_desc - gdt32 - 1  /* Limit */
    .long   gdt32 - trampoline_start + TRAMPOLINE_BASE  /* Base */

/* 64-bit GDT for long mode */
.align 16
gdt64:
    .quad   0x0000000000000000  /* Null descriptor */
    .quad   0x00AF9A000000FFFF  /* Code segment: 64-bit, executable, readable */
    .quad   0x00AF92000000FFFF  /* Data segment: 64-bit, writable */

gdt64_desc:
    .word   gdt64_desc - gdt64 - 1  /* Limit */
    .long   gdt64 - trampoline_start + TRAMPOLINE_BASE  /* Base */

.align 16
trampoline_end:
