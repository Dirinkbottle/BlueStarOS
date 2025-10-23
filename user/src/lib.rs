#![no_main]
#![no_std]
#![feature(linkage)]
mod panic;
mod syscall;
///BlueStarOS标准用户库
//无main情况
#[linkage ="weak"]
#[no_mangle]
fn main(_argc:usize,_argv:&[&str]){
    sys_write();//也是syswrite目前
}


#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start(){
    let st =["sd","as"];
    main(0,&st);
}


pub use self::syscall::*;

