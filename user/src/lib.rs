#![no_main]
#![no_std]
#![feature(linkage,panic_info_message,)]
extern crate alloc;
mod panic;
mod syscall;
mod console;
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
    let st =["sd","as"];
    sys_exit( main(0,&st));
    loop {
        //不可达
    }
}



#[linkage ="weak"]
#[no_mangle]
fn main(_argc:usize,_argv:&[&str])->usize{
  panic!("No Main Function find!")
}




pub use self::syscall::*;
pub use self::console::*;
