pub const  SYS_WRITE:usize = 1;



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



pub fn sys_write(fd_type:usize,buffer_ptr:usize,buffer_len:usize)->isize{
    sys_call(SYS_WRITE, [buffer_ptr,fd_type,buffer_len])
}