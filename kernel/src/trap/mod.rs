
use core::{arch::global_asm, panic};
use crate::{config::*, task::TASK_MANAER, time::set_next_timeInterupt};
use log::{debug, error, };
use riscv::register::{scause::{self, Exception, Trap}, sie::Sie, sscratch, sstatus::{self, SPP, Sstatus}, stval, stvec, utvec::TrapMode};
use crate::syscall::*;//系统调用
use riscv::register::sie;
use riscv::register::scause::Interrupt;
use core::arch::asm;


pub enum TrapFunction{
    USERHANDLER,
    KERNELHANDLER
}

#[repr(C)]
#[repr(align(8))]  // 确保 8 字节对齐
pub struct TrapContext{
     ///32个寄存器完全保存
     pub x:[usize;32],
     ///陷入状态
     pub sstatus:Sstatus, //32*8(sp)
     ///返回地址
     pub sepc_entry_point:usize,//33*8(sp)
     ///内核地址空间satp
     pub kernel_satp:usize,//34*8(sp)
     ///内核栈指针
     pub kernel_sp:usize,//35*8(sp)
     ///陷阱处理程序
     pub trap_handler:usize,//36*8(sp)
}

extern "C" {
    fn __kernel_trap_handler_ptr();  // trap.asm 中定义的 handler 地址存储位置
}

impl TrapContext {
    /// 初始化应用的 TrapContext,设置usersp
    pub fn init_app_trap_context(
        entry: usize,
        kernel_satp: usize,
        trap_handler: usize,
        kernel_sp: usize,
        user_sp:usize,
    ) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);  // 设置返回用户态
        let mut register = [0; 32];
        //x2
        register[2]=user_sp;
        TrapContext {
            x: register,               // 通用寄存器初始化为 0，x[2](sp) 会在外部设置
            sstatus,
            sepc_entry_point: entry,   // 用户程序入口
            kernel_satp,              // 内核页表
            kernel_sp,                // 内核栈指针
            trap_handler,             // trap 处理函数
        }
    }
}

///设置sstatus的sie开启全局中断使能，设置sie寄存器的第五位（从0开始）开启具体时钟中断
pub fn enable_timer_interupt(){
    unsafe {
     sstatus::set_sie();
     sie::set_stimer(); 
    }
    debug!("TIMER INTERUPT ENABLE!");
}



pub fn set_kernel_trap_handler(){
    unsafe {
        let handler_ptr=__kernel_trap_handler_ptr as usize;
        let hander_func=kernel_trap_handler as usize;
        let trap_entry = TRAP_BOTTOM_ADDR as usize;
        core::ptr::write_volatile(handler_ptr as *mut usize, hander_func);
        let verify_addr = core::ptr::read_volatile(handler_ptr as *mut usize);
        if verify_addr != hander_func{
        debug!("Handler function addr:    {:#x}", hander_func);
        debug!("Handler verify addr:    {:#x}", verify_addr);
            panic!("Trap set failed!");
        }
        stvec::write(trap_entry, TrapMode::Direct);
        debug!("Kernel TrapHandler func addr    :{:#x}",hander_func);
        debug!("Verify Kernel TrapHandler func addr    :{:#x}",verify_addr);
        debug!("Traper Set Success!");
    }
}

pub fn set_kernel_forbid(){
    unsafe {
        stvec::write(kernel_traped_forbid as usize, TrapMode::Direct);
    }
}

/// 第一次进入用户态的入口点
/// __switch 会跳转到这里，设置好 trap 环境后跳转到用户态
#[no_mangle]
pub extern "C" fn app_entry_point() {
    //set_kernel_trap_handler();
    let trap_cx_ptr = TRAP_CONTEXT_ADDR;
    let user_satp = TASK_MANAER.get_current_stap();
    let restore_va = __kernel_refume as usize - __kernel_trap as usize + TRAP_BOTTOM_ADDR;
   // let restore_va = __kernel_refume as usize;
    // trace!("[kernel] trap_return: ..before return");
   debug!("Welcome to app entry point!!!");
   empty();
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",         // jump to new addr of __restore asm function
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,      // a0 = virt addr of Trap Context
            in("a1") user_satp,        // a1 = phy addr of usr page table
            options(noreturn)
        );
    }
}


#[no_mangle]
pub extern "C" fn empty(){

}

use riscv::register::sepc;
///handler必须返回到trap里面去
#[no_mangle]
pub extern "C" fn kernel_trap_handler(){//内核专属trap（目前不应该被调用）
    //set_kernel_forbid();
    let scauses = scause::read();
    let sepc_val = sepc::read();
    let stval_val = stval::read();
        match scauses.cause(){
        Trap::Exception(Exception::UserEnvCall)=>{
            panic!("User syscalled");
        }
        Trap::Exception(Exception::IllegalInstruction)=>{
            panic!("User IllegalInstruction at {:#x}", sepc_val)
        }
        Trap::Exception(Exception::InstructionPageFault)=>{
            panic!("User InstructionPageFault at {:#x}, accessing {:#x}", sepc_val, stval_val)
        }
        Trap::Exception(Exception::LoadPageFault)=>{
            panic!("User LoadPageFault at {:#x}, accessing {:#x}", sepc_val, stval_val)
        }
        Trap::Exception(Exception::StorePageFault)=>{
            panic!("User StorePageFault at {:#x}, accessing {:#x}", sepc_val, stval_val)
        }
        Trap::Interrupt(Interrupt::SupervisorTimer)=>{
            set_next_timeInterupt();
        }
        _=>{
            panic!("Unknown trap from user: {:?}", scauses.cause())
        }
    }
app_entry_point();//传入特定参数，返回回去
}


#[no_mangle]
pub extern "C" fn kernel_traped_forbid(){//内核专属trap目前只支持时钟设置
let scauses = scause::read();
        match scauses.cause(){
        Trap::Interrupt(Interrupt::SupervisorTimer)=>{
            set_next_timeInterupt();
        }
        _=>{
            panic!("Unknown trap from user: {:?}", scauses.cause())
        }
    }
}

global_asm!(include_str!("trap.asm"));
