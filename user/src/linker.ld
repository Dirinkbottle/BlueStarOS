OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x2000;

SECTIONS
{
    . = BASE_ADDRESS;

    . = ALIGN(4K);
    .text : {
        *(.text .text.*)
    }

    . = ALIGN(4K);
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(4K);
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(4K);
    .bss : {
        sbss = .;
        *(.bss.stack)
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    /DISCARD/ : {
        *(.eh_frame)
    }
}