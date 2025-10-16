ENTRY(_start)

SECTIONS {
    . = 0x80000000;       /* QEMU 默认起始地址 */
    . = ALIGN(4K);
        _kernel_start = .;
    /* 文本段 (代码) - 4K对齐 */
    .text ALIGN(4K) : {
        *(.text.entry)    /* 启动代码 */
        *(.text .text.*)  /* 其他代码 */
        . = ALIGN(4K);    /* 确保段尾4K对齐 */
    }
    
    /* 只读数据段 - 4K对齐 */
    .rodata ALIGN(4K) : { 
        *(.rodata .rodata.*) 
        *(.srodata .srodata.*)
        . = ALIGN(4K);    /* 确保段尾4K对齐 */
    }
    
    /* 数据段 - 4K对齐 */
    .data ALIGN(4K) : { 
        *(.data .data.*) 
        *(.sdata .sdata.*)
        . = ALIGN(4K);    /* 确保段尾4K对齐 */
    }

    .bss ALIGN(4K) : {
        *(.bss .bss.*)
        . = ALIGN(4K);    /* 确保段尾4K对齐 */
    }
    
    /* 栈空间 - 4K对齐 */
    .stack ALIGN(4K) : {
        _stack_bottom = .;
        . += 64K;         /* 64KB栈空间 */
        . = ALIGN(4K);    /* 确保段尾4K对齐 */
        _stack_top = .;
        . = ALIGN(4K);
        _kernel_end = .;
    }
        
    
    /* 丢弃调试信息 */
    /DISCARD/ : { 
        *(.eh_frame)
        *(.debug*)
        *(.comment)
    }
}