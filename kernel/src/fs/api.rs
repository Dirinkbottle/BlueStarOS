use core::{f32::consts::E};

use alloc::{string::String, sync::Arc, vec::Vec};
use bitflags::bitflags;
///FileSystem API 对用户友好接口
/// 封装VFS层



use lazy_static::lazy_static;
use log::info;
use riscv::register::sepc;
use spin::{Mutex};

use crate::fs::{ramFileSystem::RamFileSystem, root::{self, RootFileSystem}, vfs::{NodeType, VfsError, VfsNodeOps}};
///初始化全局根文件系统
lazy_static!{
    pub static ref ROOT_FS:Mutex<Option<Arc<RootFileSystem>>>=Mutex::new(None);
}



///文件系统初始化
pub fn initial_root_filesystem(){
    //创建主文件系统
    let main_fs_ramfs = RamFileSystem::initial_common_ramfilesystem();
    //创建根文件系统管理器
    let root_fs = Arc::new(RootFileSystem::new(main_fs_ramfs));
    //保存到全局
    *ROOT_FS.lock()=Some(root_fs);
    info!("RootFileSystem Initial complete!")
}

///获取全局根文件管理系统
pub fn get_rootfs()->Result<Arc<RootFileSystem>,VfsError>{
    ROOT_FS.lock().clone().ok_or(VfsError::InvalidOperation)
}


///文件描述符 FileDescriptor
///维护打开的文件状态
pub struct FileDescriptor{
    node:Arc<dyn  VfsNodeOps>,
    offset:Mutex<usize>,
    flags:FileFlags
}


/// 文件打开标志
#[derive(Clone, Copy)]
pub struct FileFlags {
    pub read: bool,      // 可读
    pub write: bool,     // 可写
    pub append: bool,    // 追加模式
    pub create: bool,    // 如果不存在则创建
    pub truncate: bool,  // 截断文件
}

impl FileFlags {
    /// 只读模式
    pub fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            append: false,
            create: false,
            truncate: false,
        }
    }
    
    /// 只写模式
    pub fn write_only() -> Self {
        Self {
            read: false,
            write: true,
            append: false,
            create: false,
            truncate: false,
        }
    }
    
    /// 读写模式
    pub fn read_write() -> Self {
        Self {
            read: true,
            write: true,
            append: false,
            create: false,
            truncate: false,
        }
    }
    
    /// 创建新文件（写入）
    pub fn create() -> Self {
        Self {
            read: false,
            write: true,
            append: false,
            create: true,
            truncate: true,
        }
    }
    
    /// 追加模式
    pub fn append() -> Self {
        Self {
            read: false,
            write: true,
            append: true,
            create: true,
            truncate: false,
        }
    }
}



impl FileDescriptor {
    ///创建新文件描述符
    pub fn new(node:Arc<dyn VfsNodeOps>,flags:FileFlags)->Self{
        let offset =if flags.append{
            //追加模式 定位到末尾
            node.get_attribute().size
        }else if flags.truncate{
            let _ =node.truncate(0);
            0
        }else {
            0
        };
        Self { node,  offset:Mutex::new(offset), flags }
    }
    ///读取数据
    pub fn read(&self,buf:&mut [u8])->Result<usize,VfsError>{
        if !self.flags.read{
            return Err(VfsError::PermissionDenied);
        }
        
        
       let read_len =  self.node.read_at(*self.offset.lock(), buf)?;
       *self.offset.lock() +=read_len;
       Ok(read_len)
    }
    ///写入数据
    pub fn write(&self,buf:&[u8])->Result<usize,VfsError>{
        if !self.flags.write{
            return Err(VfsError::PermissionDenied);
        }

        let offset = self.offset.lock();
        let write_len = self.node.write_at(*offset, buf)?;
        Ok(write_len)
    }
    ///定位到指定位置
    pub fn seek(&self,pos:usize)->Result<usize,VfsError>{
        let mut offset =self.offset.lock();
        *offset = pos;
        Ok(*offset)
    }
    ///获取当前位置
    pub fn tell(&self)->usize{
        *self.offset.lock()
    }
    ///获取文件大小 
    pub fn size(&self)->usize{
        self.node.get_attribute().size
    }
}



///高层api函数
///打开文件
///path /tmp/test.txt 
pub fn open(path:&str,flags:FileFlags)->Result<FileDescriptor,VfsError>{
    let root_fs = get_rootfs()?;
    //查找文件
    let node = match root_fs.look_node(path){
        Ok(node)=>{
            //文件存在
            if flags.truncate{
                node.truncate(0);
            }
            node
        }
        Err(VfsError::NotFound)=>{
            //文件不存在
            if flags.create{
                //创建文件
                root_fs.create_file(path)?
            }else {
                return Err(VfsError::NotFound);
            }
        }
        Err(e)=>{
            return Err(e);
        }
    };
    //是否是文件
    if node.get_type()!=NodeType::File{
        return Err(VfsError::NotAFile);
    }
    Ok(FileDescriptor::new(node, flags))

}


///创建文件 path可带/
pub fn create_file(path:&str)->Result<(),VfsError>{
    let root_fs = get_rootfs()?;
    root_fs.create_file(path)?;
    Ok(())
}
///创建目录
pub fn create_dir(path:&str)->Result<(),VfsError>{
    let root_fs = get_rootfs()?;
    root_fs.create_dir(path)?;
    Ok(())
}
///删除文件或目录
pub fn remove(path:&str)->Result<(),VfsError>{
    let root_fs = get_rootfs()?;
    root_fs.remove(path)
}
///列出目录内容
pub fn list_dir(path:&str)->Result<Vec<String>,VfsError>{
    let root_fs = get_rootfs()?;
    let node =root_fs.look_node(path)?;
    if node.get_type()!=NodeType::Dir{
        return Err(VfsError::NotADir);
    }
    Ok(node.list_allnode_string())
}
///读取整个文件
pub fn read_file(path:&str)->Result<Vec<u8>,VfsError>{
    let fd = open(path, FileFlags::read_only())?;
    let size = fd.size();
    let mut buf = Vec::with_capacity(size);
    buf.resize(size, 0);
    fd.read(&mut buf)?;
    Ok(buf)
}
///写入整个文件
pub fn write_file(path:&str,data:&[u8])->Result<(),VfsError>{
    let fd=open(path, FileFlags::create())?;
    fd.write(data)?;
    Ok(())
}
///追加数据到文件末尾
pub fn append_file(path:&str,data:&[u8])->Result<(),VfsError>{
    let fd =open(path, FileFlags::append())?;
    fd .write(data)?;
    Ok(())
}
///检查路径是否存在
pub fn exists(path:&str)->bool{
    let root_fs =match get_rootfs(){
        Ok(fs)=>{fs}
        Err(_)=>{return false;}
    };
    root_fs.look_node(path).is_ok()
}
///路径是否是文件
pub fn is_file(path:&str)->bool{
    let root_fs =match get_rootfs(){
        Ok(fs)=>{fs}
        Err(_)=>{return false;}
    };
    match root_fs.look_node(path){
        Ok(node)=>{node.get_type()==NodeType::File}
        Err(_)=>{return false;}
    }
}
///路径是否是目录
pub fn is_dir(path:&str)->bool{
    let root_fs =match get_rootfs(){
        Ok(fs)=>{fs}
        Err(_)=>{return false;}
    };
    match root_fs.look_node(path){
        Ok(node)=>{node.get_type()==NodeType::Dir}
        Err(_)=>{return false;}
    }
}