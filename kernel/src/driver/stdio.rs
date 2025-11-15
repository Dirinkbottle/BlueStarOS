use core::fmt::{self, Write};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use lazy_static::lazy_static;
use log::debug;
use log::error;
use crate::memory::PageTable;
use crate::memory::VirAddr;
use crate::{sbi, sync::UPSafeCell, task::TASK_MANAER};
use BlueosFS::{FileAttribute, FileDescriptor, NodeType, VfsError, VfsNodeOps, FileFlags};

/// 标准输出文件节点
pub struct Stdout;

/// 标准输入文件节点
pub struct Stdin;

///Stdout文件抽象
impl VfsNodeOps for Stdout {
    /// 写入数据到标准输出
    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize, VfsError> {
        // 直接写入字节，不进行UTF-8转换，避免panic
        // 对于ASCII字符，直接输出；对于多字节UTF-8字符，逐字节输出
        for &byte in buf {
            sbi::putc(byte as usize);
        }
        Ok(buf.len())
    }

    /// 获取文件属性
    fn get_attribute(&self) -> FileAttribute {
        FileAttribute {
            tp: NodeType::File,
            size: 0, // 标准输出没有固定大小
            permission: 0o666, // 读写权限
            create_time: 0,
            modify_time: 0,
        }
    }

    fn get_type(&self) -> NodeType {
        NodeType::File
    }
}

///标准输入文件（向后兼容）

impl Stdin {
    ///调用栈顶必须为traphandler！！！，因为其中有TASK_MANAER.suspend_and_run_task();
    pub fn get_char() -> u8 {
        //直接调用sbi接口，返回一个字符，没有字符就挂起
        let cha = sbi::get_char() as u8;

        if cha == 0 {
            TASK_MANAER.suspend_and_run_task();//没有字符就切换任务
        }
        
        cha
    }
}



///Stdin文件抽象
impl VfsNodeOps for Stdin {//调用栈顶必须是traphandler
    /// 从标准输入读取数据 遇到\n自动返回 
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize, VfsError> {
        // 忽略offset，标准输入是顺序读取的
        let mut read_count = 0;

        /* 首先清空输入缓冲区 */
        buf.iter_mut().for_each(|ptr|{*ptr=0});

        /* 逐个读取字符 */
        for char_adr in buf {
            let cha = sbi::get_char();
            debug!("DEBUG:getchar:{}",cha);

            /* 遇到\n提前结束 */
            if cha as u8 == 13{
                *char_adr = cha as u8;
                read_count+=1;
                return Ok(read_count);
            }

            /*  空字符挂起进程 */
            if cha == 0 {
                TASK_MANAER.suspend_and_run_task();
            }

            *char_adr = cha as u8;
            read_count+=1;
        }




        Ok(read_count)
    }

    /// 获取文件属性
    fn get_attribute(&self) -> FileAttribute {
        FileAttribute {
            tp: NodeType::File,
            size: 0, // 标准输入没有固定大小
            permission: 0o444, // 只读权限
            create_time: 0,
            modify_time: 0,
        }
    }

    /// 返回节点类型为文件
    fn get_type(&self) -> NodeType {
        NodeType::File
    }
}

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for cha in s.chars() {
            sbi::putc(cha as usize);
        }
        Ok(())
    }
}

/// 打印函数
pub fn print(fmt: fmt::Arguments) {
    let mut stdout = Stdout;
    stdout.write_fmt(fmt).unwrap()
}

/// 全局标准输入文件描述符
lazy_static! {
    pub static ref STDIN_FD: UPSafeCell<FileDescriptor> = UPSafeCell::new(
        FileDescriptor::new(Arc::new(Stdin), FileFlags::read_only())
    );

    pub static ref STDOUT_FD: UPSafeCell<FileDescriptor> = UPSafeCell::new(
        FileDescriptor::new(Arc::new(Stdout), FileFlags::write_only())
    );
}

