; AP (Application Processor) Trampoline Code
; 
; This code is copied to physical address 0x8000 and executed by APs
; when they receive a SIPI (Startup Inter-Processor Interrupt).
; 
; The trampoline transitions the AP from 16-bit real mode through
; 32-bit protected mode to 64-bit long mode, then jumps to the
; Rust entry point.

[BITS 16]

; Trampoline data structure layout at 0x8000
TRAMPOLINE_BASE         equ 0x8000
TRAMPOLINE_GDT32        equ 0x8100
TRAMPOLINE_GDT64        equ 0x8200
TRAMPOLINE_STACK_PTR    equ 0x8300
TRAMPOLINE_ENTRY_PTR    equ 0x8308
TRAMPOLINE_CR3          equ 0x8310
TRAMPOLINE_CPU_ID       equ 0x8318

global trampoline_start
global trampoline_end

trampoline_start:
    cli                         ; Disable interrupts
    
    ; Set up segments for real mode
    xor     ax, ax
    mov     ds, ax
    mov     es, ax
    mov     ss, ax
    
    ; Enable A20 line using fast A20 gate
    in      al, 0x92
    or      al, 0x02
    out     0x92, al
    
    ; Load 32-bit GDT
    lgdt    [gdt32_desc - trampoline_start + TRAMPOLINE_BASE]
    
    ; Enter protected mode: set CR0.PE
    mov     eax, cr0
    or      eax, 0x1            ; Set PE bit
    mov     cr0, eax
    
    ; Far jump to 32-bit protected mode code segment
    jmp     0x08:(protected_mode_entry - trampoline_start + TRAMPOLINE_BASE)

[BITS 32]
protected_mode_entry:
    ; Set up 32-bit data segments
    mov     ax, 0x10
    mov     ds, ax
    mov     es, ax
    mov     ss, ax
    mov     fs, ax
    mov     gs, ax
    
    ; Enable PAE (Physical Address Extension): set CR4.PAE
    mov     eax, cr4
    or      eax, 0x20           ; Set PAE bit (bit 5)
    mov     cr4, eax
    
    ; Load CR3 with kernel page table
    mov     eax, [TRAMPOLINE_CR3]
    mov     cr3, eax
    
    ; Enable long mode: set EFER.LME
    mov     ecx, 0xC0000080     ; EFER MSR
    rdmsr
    or      eax, 0x100          ; Set LME bit (bit 8)
    wrmsr
    
    ; Enable paging: set CR0.PG
    mov     eax, cr0
    or      eax, 0x80000000     ; Set PG bit (bit 31)
    mov     cr0, eax
    
    ; Load 64-bit GDT
    lgdt    [gdt64_desc - trampoline_start + TRAMPOLINE_BASE]
    
    ; Far jump to 64-bit long mode code segment
    jmp     0x08:(long_mode_entry - trampoline_start + TRAMPOLINE_BASE)

[BITS 64]
long_mode_entry:
    ; Set up 64-bit data segments
    mov     ax, 0x10
    mov     ds, ax
    mov     es, ax
    mov     ss, ax
    mov     fs, ax
    mov     gs, ax
    
    ; Load stack pointer
    mov     rsp, [TRAMPOLINE_STACK_PTR]
    
    ; Load CPU ID into first argument register (rdi)
    mov     rdi, [TRAMPOLINE_CPU_ID]
    
    ; Jump to Rust entry point
    mov     rax, [TRAMPOLINE_ENTRY_PTR]
    jmp     rax

; 32-bit GDT for protected mode transition
align 16
gdt32:
    dq      0x0000000000000000  ; Null descriptor
    dq      0x00CF9A000000FFFF  ; Code segment: base=0, limit=4GB, executable, readable
    dq      0x00CF92000000FFFF  ; Data segment: base=0, limit=4GB, writable

gdt32_desc:
    dw      gdt32_desc - gdt32 - 1                          ; Limit
    dd      gdt32 - trampoline_start + TRAMPOLINE_BASE      ; Base

; 64-bit GDT for long mode
align 16
gdt64:
    dq      0x0000000000000000  ; Null descriptor
    dq      0x00AF9A000000FFFF  ; Code segment: 64-bit, executable, readable
    dq      0x00AF92000000FFFF  ; Data segment: 64-bit, writable

gdt64_desc:
    dw      gdt64_desc - gdt64 - 1                          ; Limit
    dd      gdt64 - trampoline_start + TRAMPOLINE_BASE      ; Base

align 16
trampoline_end:
