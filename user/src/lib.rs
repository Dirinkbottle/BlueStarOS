#![no_main]
#![no_std]
#![feature(linkage,panic_info_message,)]
extern crate alloc;
mod panic;
mod syscall;
mod console;
use buddy_system_allocator::LockedHeap;
///BlueStarOS标准用户库


const USER_HEAP_SIZE:usize=4096;
static mut USER_HEAP_SPACE:[usize;USER_HEAP_SIZE]=[0;USER_HEAP_SIZE];
#[global_allocator]
static  USER_HEAP_ALLOCTER:LockedHeap=LockedHeap::empty();



#[linkage ="weak"]
fn main(_argc:usize,_argv:&[&str]){
  panic!("No Main Function find!")
}


#[link_section = ".text.entry"]
#[no_mangle]
pub extern "C" fn _start(){

    unsafe {
        USER_HEAP_ALLOCTER.lock().init(USER_HEAP_SPACE.as_ptr() as usize, USER_HEAP_SIZE);
    }

    let st =["sd","as"];
    main(0,&st);
}


pub use self::syscall::*;
pub use self::console::*;
