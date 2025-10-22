/* Linker script for serial_test userspace program */

ENTRY(_start)

SECTIONS
{
    /* Load at 4MB mark in virtual memory */
    . = 0x400000;

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
        *(.note*)
    }
}
