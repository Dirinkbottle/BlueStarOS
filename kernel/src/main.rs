//!内核主函数
//! bss段初始化，日志系统初始化，物理页帧分配系统初始化，内核地址空间初始化，激活内核地址空间
//#! 
//#![deny(missing_docs)]
//#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(panic_info_message,alloc,panic_internals,const_trait_impl,effects)]
use core::arch::global_asm;

mod sbi;
mod driver;  // driver 必须在 console 之前加载，因为 console 依赖 driver
#[macro_use]
mod console;
mod panic;
mod config;
mod logger;
mod memory;
mod sync;
mod syscall;
mod trap;
mod time;
mod task;
mod fs;
mod ffi;

use alloc::string::String;
use log::{debug, trace, warn};
use crate::driver::{init_global_block_device, get_global_block_device};
use riscv::asm;
use crate::config::{ebss, sbss};
use crate::driver::{BLOCK_DEVICE, BlockDevice, test_block_write_read};
use crate::task::run_first_task;
use crate::time::{ set_next_timeInterupt};
use crate::trap::{enable_timer_interupt, rather_global_interrupt, set_kernel_trap_handler};
extern crate alloc;
use crate::{config::*, logger::kernel_info_debug, memory::allocator_init};
use crate::memory::init_frame_allocator;
use crate::memory::MapSet;
use BlueosFS::*;
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
pub fn blue_main() -> ! {//永远不会返回
    kernel_init(); //bss，日志，分配器初始化
    set_kernel_trap_handler();//初始化陷阱入口，应该在地址空间激活前开启
    KERNEL_SPACE.lock().activate();//激活地址空间
    rather_global_interrupt();//愿意处理全局中断使能
    enable_timer_interupt();//开启全局时间中断使能
    set_next_timeInterupt();//第一次开启时钟中断
    warn!("All right,kernel Will end\n");
    debug!("stext {:#x}",__kernel_trap as usize);
    debug!("traper {:#x}",straper as usize);
    debug!("trap refume virtualaddr:{:#x}",__kernel_refume as usize - __kernel_trap as usize + TRAP_BOTTOM_ADDR);
    BLOCK_DEVICE.lock().initial_block_device();//初始化块设备
    
    // 初始化全局块设备并设置到 BlueosFS
    init_global_block_device();
    let block_device = get_global_block_device().expect("Failed to get global block device");
    BlueosFS::set_global_block_device(block_device);
    
    initial_root_filesystem();//初始化根文件系统（包含格式化检查）
    
    // 将测试文件加载到文件系统
    use crate::fs::make_testfile;
    //make_testfile();
    
    //test_block_device();
    //test_block_write_read();
    run_first_task();
    panic!("Kernel End");

}
