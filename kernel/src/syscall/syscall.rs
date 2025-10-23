
use core::mem::size_of;
use alloc::vec::Vec;
use log::error;

use crate::{config::PAGE_SIZE, memory::{PageTable, VirAddr, VirNumber}, task::TASK_MANAER, time::{TimeVal, get_time_ms}};





///addr:用户传入的时间结构体地址 目前映射处理错误，因为还没有任务这个概念
fn syscall_get_time(addr:*mut TimeVal){  //考虑是否跨页面  
      let vpn=(addr as usize)/PAGE_SIZE;
      let offset=VirAddr(addr as usize).offset();
      let mut table=unsafe {//获取页表
       &mut *PageTable::get_current_pagetable()
      };
      let mut frame_pointer=table.get_mut_byte(VirNumber(vpn)).expect("Big Error!");

   //判断是否跨页 跨页需要特殊处理
   let len=size_of::<TimeVal>();
   if vpn !=(addr as usize -1 +len)/PAGE_SIZE{
      //跨页
      //let new_frame_pointer=table.get_mut_byte(VirNumber(vpn+1)); 不重复申请，节省内存
      let table=unsafe {
          &mut *PageTable::get_current_pagetable()
      };

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
pub fn sys_write(source_buffer:usize){//用户空间缓冲数组，应该以\0结束
   panic!("Syscalled sys_write!!!");



   
   let task_satp= TASK_MANAER.get_current_stap();
   let task_table_mut_pointer = PageTable::crate_table_from_satp(task_satp);
   let mut task_pagetable = unsafe {
       &mut *task_table_mut_pointer
   };
   let mut page_start_phyaddr= task_pagetable.get_mut_byte(VirAddr(source_buffer).floor_down()).expect("Error source buffer!!");
   let phy_offset=VirAddr(source_buffer).offset();
   let last_physlice = &mut page_start_phyaddr[phy_offset..];
   let mut buffer:Vec<u8>=Vec::new();
   last_physlice.iter().for_each(|cha|{
      if *cha != b'|'{
         buffer.push(*cha);
      }
   });

}


