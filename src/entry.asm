    .section .text.entry
    .globl _blue_start
_blue_start:
    la sp, kernel_stack_top
    call blue_main

    .section .bss.stack
    .globl kernel_stack_lower_bound
kernel_stack_lower_bound:
    .space 4096 * 16
    .globl boot_stack_top
kernel_stack_top: