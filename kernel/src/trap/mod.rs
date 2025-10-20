
use core::arch::global_asm;
use crate::{config::*, time::set_next_timeInterupt};
use log::{debug, error, };
use riscv::register::{scause::{self, Exception, Trap}, sie::Sie, sscratch, sstatus::{self, Sstatus}, stvec, utvec::TrapMode};
use crate::syscall::*;//系统调用
use riscv::register::sie;
use riscv::register::scause::Interrupt;
pub struct TrapContext{
     ///32个寄存器完全保存
     x:[usize;32],
     ///陷入状态
     sstatus:Sstatus,
     ///返回地址
     spec:usize,
     ///内核地址空间stap
     kernel_stap:usize,
     ///陷阱处理程序
     trap_handler:usize,//陷阱处理程序上下文
}

extern "C" {
    fn __kernel_trap_handler_ptr();  // trap.asm 中定义的 handler 地址存储位置
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

#[no_mangle]
pub extern "C" fn trap_handler(){
    debug!("Traper")
}


///handler必须返回到trap里面去
#[no_mangle]
pub extern "C" fn kernel_trap_handler(){
    let scauses=scause::read();
    match scauses.cause(){
        Trap::Exception(Exception::IllegalInstruction)=>{
            panic!("IllegalInstruction Error.... ")
        }
        Trap::Exception(Exception::LoadPageFault)=>{
            //缺页，🥲
            panic!("Page Fault!!!.... ")
        }
        Trap::Interrupt(Interrupt::SupervisorTimer)=>{
            set_next_timeInterupt();
        }
        _=>{
            panic!("Kernel Traped can't Refumed Error.... ")
        }
    }
}//在这里自己会返回到trap.asm




global_asm!(include_str!("trap.asm"));