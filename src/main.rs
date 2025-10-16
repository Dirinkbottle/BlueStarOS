#![no_std]
#![no_main]
#![feature(panic_info_message,allocator_api,sync_unsafe_cell)]
mod config;
mod memory;
mod uart;
use core::{alloc::Layout, fmt::Write, panic::PanicInfo,};
use crate::uart::{GLOBAL_UART, UART,};
use memory::{STACK_ALLOCER,stack_allocer};
use config::{_stack_top,_kernel_start,_kernel_end,_stack_bottom};

/// Rust 入口函数
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    UART::init_uart();//serial init
    stack_allocer::init(_kernel_start as usize, _kernel_start as usize + 1024*4); //4kb
    // 主循环
    print_hex!(_kernel_start as usize);
    print_hex!(_stack_bottom as usize);
    print_hex!(_stack_top as usize);
    print_hex!(_kernel_end as usize);
    println!("Hello, BlueStarOS!");
    loop {
        
    }
}

/// Panic 处理函数
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {
       
    }
}

