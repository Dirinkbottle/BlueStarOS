#![no_std]
#![no_main]
//read 和 write系统调用
use core::usize;
use user_lib::sys_yield;
use user_lib::{StdinBuffer, String, getchar, print, println, readline};
extern crate user_lib;

#[no_mangle]
pub fn main()->usize{
    let buf:[u8;100]=[0;100];
    for i in 0..1{
        readline(buf.as_ptr() as usize, buf.len());
        let strr=String::from_utf8(buf.to_vec()).expect("Can't to string");
        println!("I read:{}",strr);
    }
    return 0;
}
