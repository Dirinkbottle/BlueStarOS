#陷入之前 sstatus的spp字段，和spec（触发异常的命令地址），scause/stval会被设置为原因和附加信息
#注意写入栈时应该先向上挪动

.altmacro
.macro SAVE_GR n
    sd x\n, \n*8(sp)
.endm

.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
#定义通用寄存器的保存和恢复的宏

    .section .text.traper
    .global __alltraped
    .global __restore

.align 2
#千万记得对齐
__alltraped:
    #注意 sscratch 初始时已经被初始化为内核陷阱栈
    csrrw sp sscratch sp
    #预留空间 32*8=256
    addi sp sp -256 
    #交换陷入前后的sp栈指针，内核陷入的话也没事的
    sd x1,1*8(sp)
    sd x3,3*8(sp)
    .set n,5 x5~x31
    .rept 27
        SAVE_GR %n
        .set n,n+1
    .endr
    csrr t0,sstatus
    csrr t1,sepc
    sd t0,32*8(sp)
    sd t1,33*8(sp)
    #保存特殊状态寄存器，方便后续恢复
    csrr t2,sscratch
    #内核栈
    sd t2,2*8(sp)
    sfence.vma
    ld t3, trap_handler
    jr t3
    #别用call指令，会返回的,jr不保存返回地址
    



#sd r1 addr 将r1寄存器的值写入addr内存地址   addi rd r1,imd 把一个寄存器的值和一个12位立即数立刻相加
