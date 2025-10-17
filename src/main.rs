//!内核主函数
//! bss段初始化，日志系统初始化，物理页帧分配系统初始化，内核地址空间初始化，激活内核地址空间
//#! 
//#![deny(missing_docs)]
//#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(panic_info_message,alloc)]
use core::arch::global_asm;

#[macro_use]
mod console;
mod sbi;
mod panic;
mod config;
mod logger;
mod memory;
mod sync;
mod trap;
use log::{debug, trace, warn};
use crate::config::{ebss, sbss};
use crate::trap::set_kernel_trap_handler;
extern crate alloc;
use crate::{config::{MEMORY_SIZE, ekernel, skernel}, logger::kernel_info_debug, memory::allocator_init};
use crate::memory::init_frame_allocator;
use crate::memory::MapSet;
global_asm!(include_str!("entry.asm"));
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
    set_kernel_trap_handler();//设置内核陷入入口
    logger::init();//日志初始化
    kernel_info_debug();//打印内核日志
    allocator_init();//内核堆，分配器初始化
    init_frame_allocator(ekernel as usize,ekernel as usize +MEMORY_SIZE);//物理内存页分配器初始化

}

/// the rust entry-point of os
#[no_mangle]
pub fn blue_main() -> ! {
    kernel_init(); //陷阱入口，bss，日志，分配器初始化
    let mut kernel_space= MapSet::new_kernel();//内核地址空间，必须持有,从来不会丢弃
    kernel_space.activate();
   // kernel_space.translate_test();
    warn!("All right,kernel Will end");
    panic!("Kernel End");

}
