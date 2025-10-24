#![no_std]
#![no_main]

use user_lib::{print,println};
extern crate user_lib;

#[no_mangle]
pub fn main(_argc:usize,_argv:&[&str]){
    println!("You are a Good Boy:{}\n",123);
    println!("Not addr :{}\n",123);
}
