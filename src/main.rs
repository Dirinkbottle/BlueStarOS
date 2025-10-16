//! The main module and entrypoint
//!
//! The operating system and app also starts in this module. Kernel code starts
//! executing from `entry.asm`, after which [`rust_main()`] is called to
//! initialize various pieces of functionality [`clear_bss()`]. (See its source code for
//! details.)
//!
//! We then call [`println!`] to display `Hello, world!`.
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
use log::trace;
extern crate alloc;
use crate::{config::{ekernel, skernel}, memory::allocator_init};
global_asm!(include_str!("entry.asm"));
/// clear BSS segment
pub fn clear_bss() {
    extern "C" {
        pub fn sbss();
        pub fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}
fn info_trace(){
    trace!("KERNEL START ADDRESS:{:#x} END ADDRESS:{:#x}",skernel as usize,ekernel as usize);

}

/// the rust entry-point of os
#[no_mangle]
pub fn blue_main() -> ! {
    logger::init();//日志初始化
    info_trace();
    allocator_init();//内核堆，分配器初始化
    panic!("test panic!");
    loop {
        
    }

}
