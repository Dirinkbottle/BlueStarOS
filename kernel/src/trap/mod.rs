
use core::arch::global_asm;
use crate::config::*;
use log::{debug, error, };
use riscv::register::{scause::{self, Exception, Trap}, sscratch, sstatus::Sstatus, stvec, utvec::TrapMode};
use crate::syscall::*;//系统调用
#[repr(C)]
pub struct TrapContext{
    pub x:[usize;32],
    pub sstatus:Sstatus,
    pub spec:usize
}

pub fn set_kernel_trap_handler(){
    unsafe {
        sscratch::write(HIGNADDRESS_MASK | kernel_trap_stack_top as usize);
        stvec::write(TRAP_BOTTOM_ADDR as usize, TrapMode::Direct);// 0000000080201000 0000000080201000
    }
}

#[no_mangle]
pub extern "C" fn trap_handler(){
    debug!("Traper")
}


///handler必须返回到trap里面去
#[no_mangle]
pub extern "C" fn kernel_trap_handler(status:usize,sepc:usize,scause:usize,stval:usize){
    error!("Kernel Traped sstatus ={:#x},spec={:#x} scause:{:#x} stval:{:#x}",status,sepc,scause,stval);
    loop{}
    let scauses=scause::read();
    match scauses.cause(){
        Trap::Exception(Exception::IllegalInstruction)=>{
            panic!("IllegalInstruction Error.... ")
        }
        Trap::Exception(Exception::LoadPageFault)=>{
            //缺页，🥲
            panic!("Page Fault!!!.... ")
        }
        _=>{
            panic!("Kernel Traped can't Refumed Error.... ")
        }
    }
}//在这里自己会返回到trap.asm




global_asm!(include_str!("trap.asm"));