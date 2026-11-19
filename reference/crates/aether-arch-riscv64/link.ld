OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS
{
    . = 0x80200000;

    .text : {
        *(.text._start)
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .sdata : {
        *(.sdata .sdata.*)
        *(.sbss .sbss.*)
    }

    .bss : {
        *(.bss .bss.*)
    }

    . = ALIGN(16);
    . += 65536;
    _stack_top = .;

    /DISCARD/ : {
        *(.comment*)
        *(.note*)
        *(.eh_frame*)
        *(.eh_frame_hdr)
    }
}
