//!内核主函数
//! bss段初始化，日志系统初始化，物理页帧分配系统初始化，内核地址空间初始化，激活内核地址空间
//#! 
//#![deny(missing_docs)]
//#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(panic_info_message,alloc,panic_internals)]
use core::arch::global_asm;

#[macro_use]
mod console;
mod sbi;
mod panic;
mod config;
mod logger;
mod memory;
mod sync;
mod syscall;
mod trap;
mod time;
mod task;
use log::{debug, trace, warn};
use riscv::asm;
use crate::config::{ebss, sbss};
use crate::task::run_first_task;
use crate::time::{ set_next_timeInterupt};
use crate::trap::{enable_timer_interupt, set_kernel_trap_handler};
extern crate alloc;
use crate::{config::*, logger::kernel_info_debug, memory::allocator_init};
use crate::memory::init_frame_allocator;
use crate::memory::MapSet;
global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("app.asm"));
/// clear BSS segment
pub fn clear_bss() {
    extern "C" {
        pub fn sbss();
        pub fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}
pub fn kernel_init(){
    clear_bss();//清空bss
    logger::init();//日志初始化 - 必须先初始化日志才能使用 debug!
    kernel_info_debug();//打印内核日志
    allocator_init();//内核堆，分配器初始化
    init_frame_allocator(ekernel as usize,ekernel as usize +MEMORY_SIZE);//物理内存页分配器初始化
}
/// the rust entry-point of os
#[no_mangle]
pub fn blue_main() -> ! {
    kernel_init(); //bss，日志，分配器初始化
    set_kernel_trap_handler();//初始化陷阱入口，应该在地址空间激活前开启
    KERNEL_SPACE.lock().activate();//激活地址空间
   // enable_timer_interupt();//开启全局时间中断使能
    //set_next_timeInterupt();//第一次开启时钟中断
    // kernel_space.translate_test();
    warn!("All right,kernel Will end\n");
    debug!("stext {:#x}",__kernel_trap as usize);
    debug!("traper {:#x}",straper as usize);
    debug!("trap refume virtualaddr:{:#x}",__kernel_refume as usize - __kernel_trap as usize + TRAP_BOTTOM_ADDR);
    run_first_task();
    panic!("Kernel End");

}
