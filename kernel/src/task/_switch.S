#任务切换
.altmacro
.macro SAVE_GPN n 
    sd s\n,(\n+2)*8(a0)
.endm
.macro REFUME_GPN n 
    ld s\n,(\n+2)*8(a1)
.endm

.section .text.switch
.global __switch
__switch:
    #保存s0-s11 called registers
    sd ra,(a0) #保存ra
    sd sp,8(a0)
    .set n , 0
.rept 12
    SAVE_GPN %n
    .set n ,n+1
.endr


#恢复执行任务
.set n,0
.rept 12
    REFUME_GPN %n
    .set n,n+1
.endr
    ld ra,(a1)
    ld sp,8(a1) #sp初始为应用内核栈 sp的地址是否是当前页表的地址，这很重要 ,因为接下来还要在内核执行代码
    ret#不是sret

#switch本质是任务状态切换，地址空间切换等移动到trap->特权级的切换