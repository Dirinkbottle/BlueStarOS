mod task;
use crate::config::*;

///临时文件加载器
pub fn file_loader()->&'static [u8]{
   let file_start_addr:usize= app_start as usize;
   let file_end_addr:usize = app_end as usize;//结束后的第一个地址
   let data_len:usize=file_end_addr - file_start_addr;
   unsafe {
        core::slice::from_raw_parts(file_start_addr as *const u8, data_len)
   }
}


pub use task::*;