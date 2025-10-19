const  MSEC:usize=1000;
use riscv::register::time;

use crate::config::CPU_CIRCLE;



pub struct TimeVal{
    pub sec:usize,
    pub ms:usize,
}


///返回毫秒数
pub fn get_time()->usize{
    let current=(time::read()*MSEC)/CPU_CIRCLE;//先×再除防止精度丢失
    current
}