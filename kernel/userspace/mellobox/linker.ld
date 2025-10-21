/* Linker script for mellobox userspace binary */

ENTRY(_start)

SECTIONS
{
    /* Start at 1MB to avoid NULL pointer region */
    . = 0x100000;

    .text : {
        *(.text._start)
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }

    /DISCARD/ : {
        *(.eh_frame)
        *(.note.*)
    }
}
