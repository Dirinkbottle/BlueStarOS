
use core::mem::size_of;
use alloc::vec::Vec;
use log::{debug, error};
use crate::sbi::shutdown;
use crate::{config::PAGE_SIZE, memory::{PageTable, VirAddr, VirNumber}, task::TASK_MANAER, time::{TimeVal, get_time_ms}};
use crate::driver::STDIN;
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
         panic!("Unsupport Write to fd_type");
      }
  }
  
}
///sysread调用 traphandler栈顶
pub fn sys_read(source_buffer:usize,fd_target:usize,buffer_len:usize)->isize{
   match fd_target{
      FD_TYPE_STDIN=>{
         if buffer_len==1{
            //getchar标志，同时写入缓冲，返回读取了多少字符，getchar就返回字符ascill即可
            STDIN::get_char() as isize
         }else {
            
             //写入用户标准缓冲返回读取了多少字符
             STDIN::readline(source_buffer, buffer_len) as isize
         }
      }
      _=>{
         panic!("UnSupport Read from FD_type")
      }
      
   }
}
///exit系统调用，一般main程序return后在这里处理退出码 任务调度型返回-1
///注意：这个函数永不返回！要么切换到其他任务，要么关机
pub fn sys_exit(exit_code:usize)->isize{
   match exit_code{
      0=>{
         error!("Program Exit Normaly With Code:{}",exit_code);
         TASK_MANAER.remove_current_task();//移除当前任务块,当前任务块就不存在了
         // 检查是否还有任务
         if TASK_MANAER.task_queen_is_empty() {
            error!("All tasks completed! Shutting down...");
            shutdown();
         }
         // 切换到其他任务，这个函数应该永不返回
         // 如果返回了，说明出现严重错误（比如只剩一个任务但没被删除）
         TASK_MANAER.suspend_and_run_task();
         // 如果执行到这里，说明suspend_and_run_task异常返回了
         // 这不应该发生，因为我们已经删除了当前任务
        // panic!("sys_exit: suspend_and_run_task should never return!");
        -1
      }
      _=>{
         panic!("Program Exit with code:{}",exit_code);
      }
   }
}

///主动放弃cpu 任务调度型返回-1 
pub fn sys_yield()->isize{
   TASK_MANAER.suspend_and_run_task();
   -1
}


