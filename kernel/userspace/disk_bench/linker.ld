/* Linker script for userspace programs */

ENTRY(_start)

SECTIONS
{
    /* Userspace programs start at 0x400000 */
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
        *(.note.*)
    }
}
