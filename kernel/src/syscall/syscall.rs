
use core::mem::size_of;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use log::{debug, error};
use crate::sbi::shutdown;
use crate::task::ProcessId;
use crate::{config::PAGE_SIZE, memory::{PageTable, VirAddr, VirNumber}, task::TASK_MANAER, time::{TimeVal, get_time_ms}};
use BlueosFS::VfsError;
use alloc::vec;
use crate::memory::MapSet;





///SYS_EXEC系统调用
pub fn sys_exec(path_ptr: usize)->isize{
    let inner = TASK_MANAER.task_que_inner.lock();
    let current_index = inner.current;
  //  let current_task = &mut inner.task_queen[current_index];

    //pid过滤
  
    //let new_memset = MapSet::from_elf(old_appid, elf_data);


    return 0;

}

///SYS_FORK系统调用
pub fn sys_fork()->isize{
    let mut inner = TASK_MANAER.task_que_inner.lock();
    let current_index = inner.current;
    let current_task = &mut inner.task_queen[current_index];
    let mut bad_task = current_task.clone();

    let arc =Arc::new(bad_task);
    /* 建立父子关系 */
    current_task.add_children(arc.clone());

    drop(inner);

    //bad_task.pid=ProcessId(0);//设置fork后的进程pid为0 全复制
    


    /* 把任务添加到任务队列 */
   // TASK_MANAER.task_que_inner.lock().task_queen.push_back(bad_task);

    return 0;



}





/// 从用户空间读取 null 结尾的 C 风格字符串
/// 最大读取长度为 4096 字节，避免读取过长的字符串
fn read_c_string_from_user(path_ptr: usize) -> Result<String, VfsError> {
    const MAX_PATH_LEN: usize = 4096;
    
    // 获取当前任务的页表
    let user_satp = TASK_MANAER.get_current_stap();
    
    // 读取用户空间字节，最多读取 MAX_PATH_LEN 字节
    let buffer = PageTable::get_mut_slice_from_satp(user_satp, MAX_PATH_LEN, VirAddr(path_ptr));
    
    // 拼接所有切片并查找 null 终止符
    let mut path_bytes = Vec::new();
    for slice in buffer {
        // 查找 null 字节
        if let Some(null_pos) = slice.iter().position(|&b| b == 0) {
            // 找到 null 字节，只取到 null 之前的部分
            path_bytes.extend_from_slice(&slice[..null_pos]);
            break;
        } else {
            // 没有找到 null 字节，添加整个切片
            path_bytes.extend_from_slice(slice);
            // 如果已经达到最大长度，停止读取
            if path_bytes.len() >= MAX_PATH_LEN {
                return Err(VfsError::InvalidOperation);
            }
        }
    }
    
    // 转换为字符串
    String::from_utf8(path_bytes)
        .map_err(|_| VfsError::InvalidOperation)
}






///sys_create系统调用 专门创建文件
/// path_ptr: 用户空间路径字符串指针（以 null 结尾）
pub fn sys_create(path_ptr: usize) -> isize {
    // 从用户空间读取路径字符串
    let path_str = match read_c_string_from_user(path_ptr) {
        Ok(path) => path,
        Err(_) => return -1, // 路径读取失败
    };
    
    // 调用文件系统 API 创建文件
    match BlueosFS::create_file(&path_str) {
        Ok(_) => 0,
        Err(_) => -1, // 创建失败（路径无效、权限错误、已存在等）
    }
}

///sys_mkdir系统调用 创建文件夹
/// path_ptr: 用户空间路径字符串指针（以 null 结尾）
pub fn sys_mkdir(path_ptr: usize) -> isize {
    // 从用户空间读取路径字符串
    let path_str = match read_c_string_from_user(path_ptr) {
        Ok(path) => path,
        Err(_) => return -1, // 路径读取失败
    };
    
    // 调用文件系统 API 创建目录
    match BlueosFS::create_dir(&path_str) {
        Ok(_) => 0,
        Err(_) => -1, // 创建失败
    }
}

///sys_delete系统调用 删除文件或者文件夹
/// path_ptr: 用户空间路径字符串指针（以 null 结尾）
/// 注意：暂时只支持文件删除，删除非空目录会返回错误
pub fn sys_delete(path_ptr: usize) -> isize {
    // 从用户空间读取路径字符串
    let path_str = match read_c_string_from_user(path_ptr) {
        Ok(path) => path,
        Err(_) => return -1, // 路径读取失败
    };
    
    // 调用文件系统 API 删除文件或目录
    // 如果删除目录且目录非空，会返回 VfsError::NotEmpty
    match BlueosFS::remove(&path_str) {
        Ok(_) => 0,
        Err(_) => -1, // 删除失败（路径不存在、权限错误、目录非空等）
    }
}

///mmap系统调用
/// startaddr:usize size:长度
pub fn sys_map(start:usize,size:usize)->isize{
    let inner=TASK_MANAER.task_que_inner.lock();
    let current=inner.current;
    drop(inner);
    let mut inner=TASK_MANAER.task_que_inner.lock();
    let mut memset=&mut inner.task_queen[current].memory_set;
    memset.mmap(VirAddr(start), size)
    //inner自动销毁
}

///unmap系统调用
/// startaddr:usize size:长度
pub fn sys_unmap(start:usize,size:usize)->isize{
    let inner=TASK_MANAER.task_que_inner.lock();
    let current=inner.current;
    drop(inner);
    let mut inner=TASK_MANAER.task_que_inner.lock();
    let memset=&mut inner.task_queen[current].memory_set;
    debug!("SYSCALL_UNMAP:ADDR{:#x} LEN:{}",start,size);
    let resu=memset.unmap_range(VirAddr(start), size);
    //销毁inner,也可以自动销毁
    drop(inner);
    resu
}



///addr:用户传入的时间结构体地址 目前映射处理错误，因为还没有任务这个概念
fn syscall_get_time(addr:*mut TimeVal){  //考虑是否跨页面  
      let vpn=(addr as usize)/PAGE_SIZE;
      let offset=VirAddr(addr as usize).offset();
      // 获取当前页表的临时视图
      let mut table = PageTable::get_kernel_table_layer();
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
/// 使用文件描述符进行写入
pub fn sys_write(source_buffer: usize, fd_target: usize, buffer_len: usize) -> isize {
    // 获取文件描述符
    let fd = match TASK_MANAER.get_current_fd(fd_target) {
        Some(fd) => fd,
        None => return -1, // 文件描述符不存在
    };

    // 获取当前任务的页表进行地址转换
    let user_satp = TASK_MANAER.get_current_stap();
    let buffer = PageTable::get_mut_slice_from_satp(user_satp, buffer_len, VirAddr(source_buffer));
    
    // 计算总长度并准备写入缓冲区
    let total_len: usize = buffer.iter().map(|slic| slic.len()).sum();
    let mut write_buffer = Vec::with_capacity(total_len);
    
    // 将用户空间的数据复制到内核缓冲区
    for slice in buffer {
        write_buffer.extend_from_slice(slice);
    }

    // 使用文件描述符写入
    match fd.write(&write_buffer) {
        Ok(written) => written as isize,
        Err(_) => -1,
    }
}
///sysread调用 traphandler栈顶
/// 使用文件描述符进行读取
pub fn sys_read(source_buffer: usize, fd_target: usize, buffer_len: usize) -> isize {
    // 获取文件描述符
    let fd = match TASK_MANAER.get_current_fd(fd_target) {
        Some(fd) => fd,
        None => return -1, // 文件描述符不存在
    };

    // 获取当前任务的页表进行地址转换
    let user_satp = TASK_MANAER.get_current_stap();
    let mut buffer = PageTable::get_mut_slice_from_satp(user_satp, buffer_len, VirAddr(source_buffer));
    
    // 计算总缓冲区大小
    let total_len: usize = buffer.iter().map(|slic| slic.len()).sum();
    let mut read_buffer = vec![0u8; total_len];

    // 使用文件描述符读取
    let read_len = match fd.read(&mut read_buffer) {
        Ok(len) => len,
        Err(_) => return -1,
    };


    // 将读取的数据复制回用户空间缓冲区
    let mut offset = 0;
    for slice in buffer.iter_mut() {
        let slice_len = slice.len();
        if offset + slice_len > read_len {
            let remaining = read_len - offset;
            slice[..remaining].copy_from_slice(&read_buffer[offset..offset + remaining]);
            break;
        }
        slice.copy_from_slice(&read_buffer[offset..offset + slice_len]);
        offset += slice_len;
    }

    read_len as isize
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


