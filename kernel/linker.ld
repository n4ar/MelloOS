/* MelloOS Kernel Linker Script */
/* This script defines the memory layout and sections for the kernel */

/* Entry point of the kernel */
ENTRY(_start)

/* Define the program headers */
PHDRS
{
    text    PT_LOAD FLAGS((1 << 0) | (1 << 2)) ; /* Execute + Read */
    rodata  PT_LOAD FLAGS((1 << 2)) ;            /* Read only */
    data    PT_LOAD FLAGS((1 << 1) | (1 << 2)) ; /* Write + Read */
}

SECTIONS
{
    /* Kernel is loaded at 0xFFFFFFFF80000000 (higher half) */
    /* This is a common address for 64-bit kernels */
    . = 0xFFFFFFFF80000000;

    /* Text section - executable code */
    .text : {
        *(.text .text.*)
    } :text

    /* Read-only data section */
    .rodata : {
        *(.rodata .rodata.*)
    } :rodata

    /* Limine requests section - must be in its own section */
    /* The bootloader looks for this section to find requests */
    .requests : {
        KEEP(*(.requests_start_marker))
        KEEP(*(.requests))
        KEEP(*(.requests_end_marker))
    } :rodata

    /* Data section - initialized read-write data */
    .data : {
        *(.data .data.*)
    } :data

    /* BSS section - uninitialized data (zeroed by bootloader) */
    .bss : {
        *(COMMON)
        *(.bss .bss.*)
    } :data

    /* Discard unnecessary sections */
    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
    }
}
