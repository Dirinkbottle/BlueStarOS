const SYS_WRITE:usize = 1;//write系统调用
const SYS_READ:usize = 2;//read系统调用
const SYS_EXIT:usize=3;//exit程序结束，运行下一个程序
const SYS_YIELD:usize=4;//主动放弃一次cpu
///syscall封装 3个参数版本
pub fn sys_call(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id
        );
    }
    ret
}

pub fn sys_read(fd_type:usize,buffer_ptr:usize,buffer_len:usize)->isize{
    sys_call(SYS_READ, [buffer_ptr,fd_type,buffer_len])
}

pub fn sys_write(fd_type:usize,buffer_ptr:usize,buffer_len:usize)->isize{
    sys_call(SYS_WRITE, [buffer_ptr,fd_type,buffer_len])
}


///永远不返回 里面有loop封装为！
pub fn sys_exit(exit_code:usize)->!{//sys_exit 
    sys_call(SYS_EXIT, [exit_code,0,0]);
    loop {
        
    }
}

///主动放弃一次cpu
pub fn sys_yield(){
    sys_call(SYS_YIELD, [0;3]);
}

