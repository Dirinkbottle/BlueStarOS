#![no_std]
#![no_main]

use core::usize;

use user_lib::{StdinBuffer, String, getchar, print, println, readline, sys_yield};
extern crate user_lib;

#[no_mangle]
pub fn main()->usize{
   let mut i=0;
       println!("no:{}",i);
    for i in 0..1{
        println!("no:{}",i);
    }


for i in 0..1{
        println!("no:{}",i);
    }

    return 0;
}
