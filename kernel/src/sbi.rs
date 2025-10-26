use core::arch::asm;


const SET_TIMER:usize=0;
const PUTC_CALLID:usize=1;
const GETCHAR_CALLID:usize=2;
const SHUTDOWN_CALLID:usize=8;


#[inline(always)]
fn sbi_call(callid:usize,arg0:usize,arg1:usize,arg2:usize)->isize{
    let mut result;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") arg0 => result,
            in("x11") arg1,
            in("x12") arg2,
            in("x16") 0,
            in("x17") callid,
        );
    }
    result
}
///向串口输出一个字符
pub fn putc(cha:usize){
    sbi_call(PUTC_CALLID, cha, 0, 0);
}
///从sbi console读取一个字符->should be called by plic  已经实现了阻塞!!!!
pub fn get_char()->isize{//非阻塞 -1没有字符，>=0ascii码
    sbi_call(GETCHAR_CALLID, 0, 0, 0)
}

pub fn shutdown()->!{
    sbi_call(SHUTDOWN_CALLID, 0, 0, 0);
    panic!("It should shutdown!");
}
///设置下一次的时钟中断
pub fn set_next_timetriger(timer:usize){
    sbi_call(SET_TIMER, timer, 0, 0);
}

