#![no_main]
#![no_std]
#![feature(linkage,panic_info_message,)]
extern crate alloc;
mod panic;
mod syscall;
mod console;
pub use alloc::string::String;
use buddy_system_allocator::LockedHeap;
///BlueStarOS标准用户库
const USER_HEAP_SIZE:usize=40960;
static mut USER_HEAP_SPACE:[usize;USER_HEAP_SIZE]=[0;USER_HEAP_SIZE];
#[global_allocator]
static mut USER_HEAP_ALLOCTER:LockedHeap=LockedHeap::empty();
#[link_section = ".text.entry"]
#[no_mangle]
pub extern "C" fn _start()->!{
    unsafe {
        USER_HEAP_ALLOCTER.lock().init(USER_HEAP_SPACE.as_ptr() as usize, USER_HEAP_SIZE);
    }
    let code=main();
    sys_exit(code);
    panic!("_start UnReachBle!");
}



#[linkage ="weak"]
#[no_mangle]
fn main()->usize{
  return 1;
}

pub fn getchar()->char{
   syscall::sys_read(FD_TYPE_STDIN,0, 1) as u8 as char
}

pub fn readline(ptr:usize,len:usize)->isize{//返回读取的字符数量 目前实现比较原始，后期封装
  syscall::sys_read(FD_TYPE_STDIN, ptr, len)
}

pub fn map(start:usize,len:usize)->isize{
  syscall::sys_map(start, len)
}

pub fn unmap(start:usize,len:usize)->isize{
  syscall::sys_unmap(start, len)
}


use crate::panic::panic;

pub use self::syscall::*;
pub use self::console::*;
