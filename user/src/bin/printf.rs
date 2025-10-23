#![no_std]
#![no_main]

use user_lib::sys_write;
extern crate user_lib;


pub fn main(){
    sys_write();
}
