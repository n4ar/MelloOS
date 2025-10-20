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
TRAMPOLINE_APIC_ID      equ 0x8320
TRAMPOLINE_LAPIC_ADDR   equ 0x8328

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
    
    ; Load CR3 with kernel page table (load full 64-bit value)
    mov     eax, [TRAMPOLINE_CR3]
    mov     edx, [TRAMPOLINE_CR3 + 4]
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
    
    ; Now we're in compatibility mode (32-bit code in 64-bit mode)
    ; Load 64-bit GDT
    lgdt    [gdt64_desc - trampoline_start + TRAMPOLINE_BASE]
    
    ; Far jump to 64-bit long mode code segment
    jmp     0x08:(long_mode_entry - trampoline_start + TRAMPOLINE_BASE)

[BITS 64]
long_mode_entry:
    ; Set up 64-bit data segments (use null selector for most segments in 64-bit mode)
    xor     ax, ax
    mov     ds, ax
    mov     es, ax
    mov     fs, ax
    mov     gs, ax
    
    ; Set up stack segment with proper data selector
    mov     ax, 0x10
    mov     ss, ax
    
    ; Load stack pointer from trampoline data (identity mapped)
    mov     rsp, [TRAMPOLINE_STACK_PTR]
    
    ; Verify stack is valid (should be non-zero)
    test    rsp, rsp
    jz      .halt_error
    
    ; Align stack to 16-byte boundary
    ; x86-64 calling convention requires RSP % 16 == 8 before call
    and     rsp, ~0xF       ; Clear low 4 bits
    sub     rsp, 8          ; Adjust so RSP % 16 == 8
    
    ; Load CPU ID into first argument register (rdi)
    mov     rdi, [TRAMPOLINE_CPU_ID]
    
    ; Load APIC ID into second argument register (rsi)
    mov     rsi, [TRAMPOLINE_APIC_ID]
    
    ; Load LAPIC address into third argument register (rdx)
    mov     rdx, [TRAMPOLINE_LAPIC_ADDR]
    
    ; Load entry point address
    mov     rax, [TRAMPOLINE_ENTRY_PTR]
    
    ; Verify entry point is valid
    test    rax, rax
    jz      .halt_error
    
    ; Debug: Write 'R' to serial (about to jump to Rust)
    push    rax
    mov     al, 'R'
    mov     dx, 0x3F8
    out     dx, al
    pop     rax
    
    ; Double-check address is in higher-half
    mov     rcx, rax
    shr     rcx, 48
    test    rcx, rcx
    jz      .bad_address  ; If upper 16 bits are 0, not a kernel address!
    
    ; Debug: 'J' before jump
    push    rax
    mov     al, 'J'
    mov     dx, 0x3F8
    out     dx, al
    pop     rax
    
    ; Save entry point and jump to lower-half wrapper instead
    mov     r15, rax        ; Save entry point in r15
    lea     rax, [rust_entry_wrapper - trampoline_start + TRAMPOLINE_BASE]
    jmp     rax
    
    ; If we get here somehow...
    mov     al, 'F'
    mov     dx, 0x3F8
    out     dx, al
    jmp     .halt_error

; Lower-half wrapper that sets everything up and calls higher-half Rust
rust_entry_wrapper:
    ; Debug: 'W' = entered wrapper
    mov     al, 'W'
    mov     dx, 0x3F8
    out     dx, al
    
    ; Arguments are already in rdi, rsi, rdx from earlier
    ; Entry point is in r15
    ; Just call it
    call    r15
    
    ; Should never return
    mov     al, 'N'
    mov     dx, 0x3F8
    out     dx, al
    cli
    hlt
    
.bad_address:
    mov     al, 'Z'
    mov     dx, 0x3F8
    out     dx, al
    jmp     .halt_error
    
    ; This should NEVER be reached - if we see 'E', jmp failed!
    mov     al, 'E'
    mov     dx, 0x3F8
    out     dx, al
    jmp     .halt_error
    
.halt_error:
    ; If we get here, something went wrong
    cli
    hlt
    jmp     .halt_error

; 32-bit GDT for protected mode transition
align 16
gdt32:
    dq      0x0000000000000000  ; Null descriptor
    dq      0x00CF9A000000FFFF  ; Code segment: base=0, limit=4GB, executable, readable
    dq      0x00CF92000000FFFF  ; Data segment: base=0, limit=4GB, writable

gdt32_desc:
    dw      23                                              ; Limit (3 entries * 8 - 1)
    dd      gdt32 - trampoline_start + TRAMPOLINE_BASE      ; Base

; 64-bit GDT for long mode
align 16
gdt64:
    dq      0x0000000000000000  ; Null descriptor
    dq      0x00209A0000000000  ; Code segment: 64-bit, L=1, executable, readable
    dq      0x0000920000000000  ; Data segment: 64-bit, writable

gdt64_desc:
    dw      23                                              ; Limit (3 entries * 8 - 1)
    dd      gdt64 - trampoline_start + TRAMPOLINE_BASE      ; Base

align 16
trampoline_end:
