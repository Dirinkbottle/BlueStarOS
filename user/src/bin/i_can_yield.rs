#![no_std]
#![no_main]

use core::usize;
use user_lib::sys_yield;
use user_lib::{StdinBuffer, String, getchar, print, println, readline};
extern crate user_lib;

#[no_mangle]
pub fn main()->usize{
    for i in 0..1000{
        sys_yield();
        println!("YIELD!");
    }
    return 0;
}
