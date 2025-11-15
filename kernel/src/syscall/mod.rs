mod syscall;
use log::error;

use crate::syscall::syscall::*;
pub const GET_TIME:usize   =0;     //获取系统时间
pub const SYS_WRITE:usize  =1;     //stdin write系统调用
pub const SYS_READ:usize   =2;     //stdin read系统调用
pub const SYS_EXIT:usize   =3;     //exit程序结束，运行下一个程序
pub const SYS_YIELD:usize  =4;     //主动放弃cpu
pub const SYS_MAP:usize    =5;     //mmap映射系统调用
pub const SYS_UNMAP:usize  =6;     //unmap映射系统调用
pub const SYS_CREATE:usize =7;     //文件节点创建系统调用
pub const SYS_DELETE:usize =8;     //文件删除系统调用
pub const SYS_MKDIR:usize  =9;     //文件夹创建系统调用
pub const SYS_FORK:usize   =10;    //fork系统调用
pub const SYS_EXEC:usize   =11;    //exec系统调用
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
            sys_read(arg[0], arg[1], arg[2])
        }
        SYS_EXIT=>{
            //error!("exit call");
            sys_exit(arg[0])
        }
        SYS_YIELD=>{
            sys_yield()
        }
        SYS_MAP=>{
            sys_map(arg[0], arg[1])
        }
        SYS_UNMAP=>{
            sys_unmap(arg[0], arg[1])
        }
        SYS_CREATE=>{
            sys_create(arg[0])
        }
        SYS_DELETE=>{
            sys_delete(arg[0])
        }
        SYS_MKDIR=>{
            sys_mkdir(arg[0])
        }
        SYS_FORK=>{
            sys_fork()
        }
        SYS_EXEC=>{
            sys_exec(arg[0])
        }
        
        _ => {
            panic!("Unknown Syscall type: {}", id);
        }
    }
}