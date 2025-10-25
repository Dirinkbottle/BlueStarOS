mod syscall;
use log::error;

use crate::syscall::syscall::*;
pub const GET_TIME:usize=0;//获取系统时间
pub const SYS_WRITE:usize=1;//stdin write系统调用
pub const SYS_READ:usize=2;//stdin read系统调用
pub const SYS_EXIT:usize=3;//exit程序结束，运行下一个程序


///id: 系统调用号
///args:接受1个usize参数
///返回值：通过 x10 (a0) 寄存器返回给用户态
pub fn syscall_handler(id:usize,arg:[usize;3]) -> isize {
    match id {
        GET_TIME => {
            0  // 暂未实现
        }
        SYS_WRITE => {
            ///bufferpoint fd_type buffer_len
            sys_write(arg[0], arg[1], arg[2])
           
        }
        SYS_READ => {
            -1  // 暂未实现
        }
        SYS_EXIT=>{
            sys_exit(arg[0]);
            return -1;//不会执行的
        }
        _ => {
            panic!("Unknown Syscall type: {}", id);
        }
    }
}