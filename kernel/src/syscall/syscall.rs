
use core::mem::size_of;
use alloc::vec::Vec;
use log::{debug, error};

use crate::{config::PAGE_SIZE, memory::{PageTable, VirAddr, VirNumber}, task::TASK_MANAER, time::{TimeVal, get_time_ms}};

const FD_TYPE_STDIN:usize=1;
const FD_TYPE_STDOUT:usize=2;



///addr:用户传入的时间结构体地址 目前映射处理错误，因为还没有任务这个概念
fn syscall_get_time(addr:*mut TimeVal){  //考虑是否跨页面  
      let vpn=(addr as usize)/PAGE_SIZE;
      let offset=VirAddr(addr as usize).offset();
      // 获取当前页表的临时视图
      let mut table = PageTable::get_current_pagetable_view();
      let mut frame_pointer=table.get_mut_byte(VirNumber(vpn)).expect("Big Error!");

   //判断是否跨页 跨页需要特殊处理
   let len=size_of::<TimeVal>();
   if vpn !=(addr as usize -1 +len)/PAGE_SIZE{
      //跨页
      //let new_frame_pointer=table.get_mut_byte(VirNumber(vpn+1)); 不重复申请，节省内存
      if table.is_maped(VirNumber(vpn+1)){
         //并且存在合法映射,拼接两个页面
        let mut time_val:&mut TimeVal;
         unsafe {
           time_val= &mut *((frame_pointer as *mut _ as usize+offset) as *mut TimeVal);
            *time_val=TimeVal{
               sec:get_time_ms()/1000,
               ms:get_time_ms()
            }
         }
      }else { 
          //PageFault!!!!!! 下一个页面没有有效映射
          panic!("InValid Memory write!!")
      }
      
   }


}


///这个指针是用户空间的指针，应该解地址
pub fn sys_write(source_buffer:usize,fd_target:usize,buffer_len:usize)->isize{//用户空间缓冲数组，应该以\0结束
  match fd_target {
      FD_TYPE_STDOUT=>{
         let user_satp=TASK_MANAER.get_current_stap();
         //debug!("current task satp:{:#x},source buffer:{:#x} buffer_len:{}",user_satp,source_buffer,buffer_len);
         let buffer = PageTable::get_mut_slice_from_satp(user_satp, buffer_len, VirAddr(source_buffer));
         let len:usize=buffer.iter().map(|slic|{slic.len()}).sum();
         for i in buffer{            
            print!("{}",core::str::from_utf8(i).expect("Ilegal utf8 char!"));
         }
         
         return len as isize;
      }
      FD_TYPE_STDIN=>{
         return -1;
      }
      _=>{
         panic!("Unsupport fd_type");
      }
  }
  
}


///exit系统调用，一般main程序return后在这里处理退出码，目前为简便panic实现
pub fn sys_exit(exit_code:usize)->isize{
//程序return的返回码在这里进行判断和处理,目前全部都认为panic


   match exit_code{
      0=>{
         panic!("Program Exit Normaly With Code:{}",exit_code)
      }
      _=>{
         panic!("Program Exit with code:{}",exit_code);
      }
   }
}


