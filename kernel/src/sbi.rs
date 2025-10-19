use core::arch::asm;

const PUTC_CALLID:usize=1;
const SHUTDOWN_CALLID:usize=8;

#[inline(always)]
fn sbi_call(callid:usize,arg0:usize,arg1:usize,arg2:usize)->usize{
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
pub fn putc(cha:usize){
    sbi_call(PUTC_CALLID, cha, 0, 0);
}


pub fn shutdown()->!{
    sbi_call(SHUTDOWN_CALLID, 0, 0, 0);
    panic!("It should shutdown!");
}