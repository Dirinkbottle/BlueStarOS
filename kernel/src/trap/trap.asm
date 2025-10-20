#针对内核trap处理，先不使用trapcontext的结构体
#启用高级宏
.altmacro
.macro SAVE_GP n
    sd s\n , (\n+1)*8(sp)
.endm

.macro REFUME_GP n
    ld s\n,(\n+1)*8(sp)
.endm
.section .text.traper
    .global __kernel_trap
    .global __kernel_refume
    .global __kernel_trap_handler_ptr

#x权限硬性要求对齐，确保 __kernel_trap 在段的开始处
.align 2
__kernel_trap:
#切换到内核陷入栈,函数栈暂存
csrrw sp,sscratch,sp
#预分配栈空间：ra(8) + s0-s11(12*8=96) + sstatus(8) + sepc(8) = 120字节，对齐到128
addi sp,sp,-128
sd ra, 0(sp)
.set n,0
.rept 12
    SAVE_GP %n
    .set n,n+1
.endr
#保存csr寄存器
csrr x8,sstatus
csrr x9,sepc
sd x8,104(sp)
sd x9,112(sp)
#保存完毕，把内核栈交换为 内核运行栈 
csrrw sp,sscratch,sp

#由于trap代码在高地址，无法使用call直接调用低地址的handler
#需要从trap段内的数据区加载handler地址并使用jalr间接跳转
#使用PC相对寻址加载同段内的handler指针（避免跨地址空间访问）
lla t0,__kernel_trap_handler_ptr  #PC相对加载handler指针地址
ld t0,0(t0)                       #从指针加载handler地址到t0
jalr ra,t0,0                      #间接跳转到handler



#x权限硬性要求对齐
.align 2
__kernel_refume:
#切换为内核陷入栈
csrrw sp,sscratch,sp
#恢复被调用者保存寄存器

#先恢复csr寄存器
ld x8,104(sp)
ld x9,112(sp)

csrw sstatus,x8
csrw sepc,x9

ld ra,0(sp)
.set n,0
.rept 12
    REFUME_GP %n
    .set n,n+1
.endr

#恢复内核栈指针
addi sp,sp,128#为了16字节对齐
#切换栈为运行栈
csrrw sp,sscratch,sp

sret
#回到触发异常的那条指令

#在trap代码段末尾存储handler地址（8字节对齐）
.align 3
__kernel_trap_handler_ptr:
    .dword 0

