use core::fmt::Write;

use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use log::debug;
use log::error;
use crate::memory::PageTable;
use crate::memory::VirAddr;
use crate::{sbi, sync::UPSafeCell, task::TASK_MANAER};

///标准输入文件
pub struct STDIN;

impl STDIN {
    ///调用栈顶必须为traphandler！！！，因为其中有TASK_MANAER.suspend_and_run_task();
    pub fn get_char()->u8{
        //直接调用sbi接口，返回一个字符，没有字符就挂起
       let cha= sbi::get_char() as u8;

       if cha ==0 {
         TASK_MANAER.suspend_and_run_task();//没有字符就切换任务
       }
        
        
        cha
       
    }

    pub fn readline(buffer:usize,buffer_len:usize)->usize{
         let user_satp=TASK_MANAER.get_current_stap();
         let mut multi_buffer = PageTable::get_mut_slice_from_satp(user_satp, buffer_len, VirAddr(buffer));
         let mut read_cont:usize=0;

         //首先清空缓冲区
         multi_buffer.iter_mut()
         .for_each(|sli|{
            sli.iter_mut()
            .for_each(|mut cha|{
                *cha=0;
            });
         });
         debug!("{:?}",multi_buffer);
       
        for sli in multi_buffer{
            for cha_mut in sli{
                let cha=STDIN::get_char();
                debug!("Input :{}",cha);
                if cha==13{//换行符号 
                    *cha_mut=cha;
                    read_cont+=1;
                    debug!("Read Line Read char:{}",read_cont);
                    return read_cont;
                }
                *cha_mut=cha;
                read_cont+=1;
            }
        }
        read_cont

    }

  
}

