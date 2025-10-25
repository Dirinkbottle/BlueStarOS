#![no_std]
#![no_main]

use user_lib::{print,println};
extern crate user_lib;

#[no_mangle]
pub fn main(_argc:usize,_argv:&[&str])->usize{
    println!("Hello,World!");
    return 0;
}
