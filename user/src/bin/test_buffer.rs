#![no_std]
#![no_main]

use user_lib::sys_write;

const FD_TYPE_STDOUT: usize = 2;
const TEST_SIZE: usize = 200; // 测试跨页情况

// 静态测试缓冲区
static mut TEST_BUFFER: [u8; TEST_SIZE] = [0u8; TEST_SIZE];

#[no_mangle]
pub fn main() {
    unsafe {
        // 填充有规律的数据：索引 % 256
        for i in 0..TEST_SIZE {
            TEST_BUFFER[i] = (i % 256) as u8;
        }
        
        // 通过系统调用发送给内核验证
        sys_write(FD_TYPE_STDOUT, TEST_BUFFER.as_ptr() as usize, TEST_SIZE);
    }
}

