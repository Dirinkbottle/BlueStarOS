use core::ffi::{c_char, CStr, c_void};
use core::panic;

use log::debug;
use crate::console::print;

// 简单的字符串打印函数
#[no_mangle]
pub extern "C" fn rust_print_str(s: *const c_char) {
    unsafe {
        if let Ok(c_str) = CStr::from_ptr(s).to_str() {
            debug!("C:{}", c_str);
        }
    }
}

// 打印格式化后的字符串片段
#[no_mangle]
pub extern "C" fn rust_print_formatted(str: *const c_char, len: usize) {
    unsafe {
        if !str.is_null() && len > 0 {
            let slice = core::slice::from_raw_parts(str as *const u8, len);
            if let Ok(s) = core::str::from_utf8(slice) {
                print(format_args!("{}", s));
            }
        }
    }
}

// 打印单个字符
#[no_mangle]
pub extern "C" fn rust_print_char(c: c_char) {
    print(format_args!("{}", c as u8 as char));
}

// 打印整数（十进制）
#[no_mangle]
pub extern "C" fn rust_print_int(value: i64) {
    print(format_args!("{}", value));
}

// 打印无符号整数（十进制）
#[no_mangle]
pub extern "C" fn rust_print_uint(value: u64) {
    print(format_args!("{}", value));
}

// 打印十六进制（小写）
#[no_mangle]
pub extern "C" fn rust_print_hex_lower(value: u64) {
    print(format_args!("{:x}", value));
}

// 打印十六进制（大写）
#[no_mangle]
pub extern "C" fn rust_print_hex_upper(value: u64) {
    print(format_args!("{:X}", value));
}

// 打印八进制
#[no_mangle]
pub extern "C" fn rust_print_oct(value: u64) {
    print(format_args!("{:o}", value));
}

// 打印指针
#[no_mangle]
pub extern "C" fn rust_print_ptr(ptr: *const c_void) {
    print(format_args!("{:p}", ptr));
}

