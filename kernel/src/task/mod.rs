mod task;
use crate::config::*;

///临时文件加载器
pub fn file_loader()->&'static [u8]{
let statr:usize=app_start as usize;//开始占据的地址
let end:usize=app_end as usize;//结束后的空地址
let len=end-statr;

unsafe {
    core::slice::from_raw_parts(statr as *const u8, len)
}

}