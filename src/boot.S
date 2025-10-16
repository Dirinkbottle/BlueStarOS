.section .text.entry
.globl _start
_start:
    # 设置栈指针
    la sp, _stack_top

call_main:
    # 调用 Rust 入口函数
    call kernel_main



    .section .bss
    .align 4
_stack_bottom:
    .skip 4096 * 16       # 64KB 栈空间
_stack_top: