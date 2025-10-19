#针对内核trap处理，先不使用trapcontext的结构体
#启用高级宏
.altmacro
.macro SAVE_GP n
    sd x\n , \n*8(sp)
.endm

.macro REFUME_GP n
    ld x\n,\n*8(sp)
.endm
.section .text.traper
    .global __kernel_trap
    .global __kernel_refume

#x权限硬性要求对齐
.align 2
__kernel_trap:
#切换到内核陷入栈,函数栈暂存
csrrw sp,sscratch,sp
#预分13+2csr=15个被调用者保存寄存器配栈空间，保存所有通用寄存器
addi sp,sp,-128 #没必要 内核栈仅仅是保存数据，不运行代码  为了16字节对齐
sd x1 , 0(sp)
sd x8 , 1*8(sp)
sd x9 , 2*8(sp)
.set n,18
.rept 10
    SAVE_GP %n
    .set n,n+1
.endr
#保存csr寄存器
csrr x8,sstatus
csrr x9,sepc
sd x8,13*8(sp)
sd x9,14*8(sp)
#保存完毕，把内核栈交换为 内核运行栈 
csrrw sp,sscratch,sp
#需要考虑参数传递 依次a0,a1,a2,a3,a4 sstatus sepc
csrr a0,sstatus
csrr a1,sepc
#下面两个用完及弃
csrr a2,scause
csrr a3,stval

ld t0,kernel_trap_handler

jr t0



#x权限硬性要求对齐
.align 2
__kernel_refume:
#切换为内核陷入栈
csrrw sp,sscratch,sp
#恢复被调用者保存寄存器

#先恢复csr寄存器
ld x8,13*8(sp)
ld x9,14*8(sp)

csrw sstatus,x8
csrw sepc,x9

ld x1,0(sp)
ld x8,1*8(sp)
ld x9,2*8(sp)
.set n,18
.rept 10
    REFUME_GP %n
    .set n,n+1
.endr

#恢复内核栈指针
addi sp,sp,128#为了16字节对齐
#切换栈为运行栈
csrrw sp,sscratch,sp

sret
#回到触发异常的那条指令


