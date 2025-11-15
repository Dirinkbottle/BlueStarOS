#![no_std]
#![no_main]
//read 和 write系统调用
use core::usize;
use user_lib::sys_yield;
use user_lib::{StdinBuffer, String, getchar, print, println, readline};
extern crate user_lib;

#[no_mangle]
pub fn main()->usize{
    
    return 0;
}
