#![no_std]
#![no_main]

use core::usize;
use user_lib::{map, sys_unmap, sys_yield};
use user_lib::{StdinBuffer, String, getchar, print, println, readline};
extern crate user_lib;

#[no_mangle]
pub fn main()->usize{
    let test_addr:usize=0x99999;
    let len=4096;
    let result=map(test_addr, len);
    let mut kl;
    unsafe {
        let ptr=test_addr as *mut u8;
        *ptr=66;
        kl=*ptr;
    }
    println!("test mmap:{} I read:{}",result,kl);
    //接下来unmap
    let resu=sys_unmap(test_addr, len);
    println!("unmap result:{}",resu);
        let bb;
    //接下来程序应该会被kill
    unsafe {
        let aa =test_addr as *mut u8;
        bb=*aa;
    }
    println!("Real Read bb:{}??",bb);
    return 0;
}
