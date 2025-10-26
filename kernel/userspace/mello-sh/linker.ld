/* Linker script for mello-sh userspace program */

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

    .got : {
        *(.got .got.*)
    }

    .got.plt : {
        *(.got.plt .got.plt.*)
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
