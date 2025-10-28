#![no_std]
#![no_main]

use core::usize;
use user_lib::{map, sys_yield};
use user_lib::{StdinBuffer, String, getchar, print, println, readline};
extern crate user_lib;

#[no_mangle]
pub fn main()->usize{
    let test_addr:usize=0x600000;
    let len=4096;
    let result=map(test_addr, len);
    println!("map result:{}",result);
    let b;
    unsafe {
        let a=test_addr as *mut u8;
        *a=66;
        b=*a;
    }
    println!("Then I read:{}",b);
    return 0;
}
