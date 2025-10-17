    .section .text.entry
    .globl _blue_start
_blue_start:
    la sp, kernel_stack_top
    la t0,kernel_trap_stack_top
    csrrw t0,sscratch,t0
    call blue_main

    .section .bss.stack
    .globl kernel_stack_lower_bound
kernel_stack_lower_bound:
    .space 4096 * 16
    .globl kernel_stack_top
kernel_stack_top:

.space 4096
#下面为了简化，加一个特殊的内核专用异常处理栈
.global kernel_trap_stack_bottom
.global kernel_trap_stack_top

kernel_trap_stack_bottom:
.space 4096 *16
#64kb
kernel_trap_stack_top:



#ld:从内存加载64到寄存器 la 将符号地址赋值给寄存器