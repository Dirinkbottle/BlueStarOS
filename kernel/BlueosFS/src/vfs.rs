use core::str;

use alloc::collections::btree_map::BTreeMap;
use alloc::{sync::{Arc, Weak}, vec::Vec};
use alloc::string::{String, ToString};
use spin::Mutex;
use spin::RwLock;




///512byte 4096bit
pub const BLOCK_SIZE:usize = 512;

///块设备trait
pub trait BlockDeviceTrait:Send + Sync{
    fn read_block(&self,block_id:usize,read_buffer:&mut [u8]);
    fn write_block(&self,block_id:usize,write_buffer:&[u8]); 
}

//块设备抽象


///VFS抽象层
/// VfsNodeOps 文件和目录需要实现的通用操作
pub trait VfsNodeOps:Sync+Send {
    ///通用功能区域------------------------
    ///获取节点属性
    fn get_attribute(&self)->FileAttribute;
    ///返回节点类型
    fn get_type(&self)->NodeType;
    ///返回父节点引用
    fn get_parent(&self)->Option<Arc<dyn VfsNodeOps>>{
        None
    }
    ///重命名
    fn rename(&self,old_path:&str,new_path:&str)->Result<(),VfsError>{
        Err(VfsError::PermissionDenied)
    }
    ///移动节点
    fn mv(&self,old_path:&str,new_path:&str)->Result<(),VfsError>{
        Err(VfsError::PermissionDenied)
    }
    ///----------------------------
    
    ////目录专用功能-----------------------------
    ///创建节点 name为文件名字或者目录名字，不支持多级路径
    fn create(&self,name:&str,tp:NodeType)->Result<Arc<dyn VfsNodeOps>,VfsError>{
        Err(VfsError::PermissionDenied)
    }
    ///删除节点
    fn remove(&self,path:&str)->Result<(),VfsError>{
        Err(VfsError::PermissionDenied)
    }
    ///查找子节点
    fn find_child_node(&self,path:&str)->Option<Arc<dyn VfsNodeOps>>{
        None
    }
    ///列出所有子节点名字
    fn list_allnode_string(&self)->Vec<String>{
        Vec::new()
    }
    ///--------------------------------------------

    ////文件专用功能---------------------------------
    ///在指定位置读
    fn read_at(&self,offset:usize,buf:&mut [u8])->Result<usize,VfsError>{
        Err(VfsError::PermissionDenied)
    }
    ///在指定偏移写
    fn write_at(&self,offset:usize,buf:&[u8])->Result<usize,VfsError>{
        Err(VfsError::PermissionDenied)
    }
    ///阶段文件或者扩展文件到指定大学
    fn truncate(&self,new_size:usize)->Result<(),VfsError>{
        Err(VfsError::PermissionDenied)
    }
    //-----------------------------------------------
}

///文件系统抽象
pub trait VfsOps:Send+Sync {
    ///在当前文件系统挂载新的文件系统到指定路径
    fn mount(&self,path:&str,mount_point:MountPoint)->Result<(),VfsError>;
    ///取消挂载
    fn unmount(&self,path:&str)->Result<(),VfsError>;
    ///返回文件目录根目录节点
    fn get_root_dir(&self)->Arc<dyn VfsNodeOps>;
    ///返回文件系统类型名称
    fn get_fs_name(&self)->String;
    ///文件系统启动验证，验证文件系统是否正常
    fn verify_file_system(&self)->bool;
    ///文件系统初始化函数
    fn initial_file_systeam(&self,block_device:Arc<dyn BlockDeviceTrait>)->Result<(),VfsError>;

}
///挂载点
pub struct MountPoint{
    pub path:String,
    pub fs:Arc<dyn VfsOps>,
}


///文件节点
pub struct FileNode{
    pub inode_id: usize, // 磁盘上的 inode 编号
    pub parent:Mutex<Option<Weak<dyn VfsNodeOps>>>,//父节点引用
    pub metadata:Mutex<FileMetadata>,//文件元数据
    pub name:Mutex<String>//文件名字
}   
///目录节点
pub struct DirNode{
    pub inode_id: usize, // 磁盘上的 inode 编号
    pub children:Mutex<BTreeMap<String,Arc<dyn VfsNodeOps>>>,//子节点（内存缓存）
    pub parent:Mutex<Option<Weak<dyn VfsNodeOps>>>,//父节点
    pub metadata:Mutex<FileMetadata>,//元数据
    pub name:Mutex<String>//名字
}

use lazy_static::lazy_static;

/// 全局块设备实例（在 BlueosFS 模块中）
lazy_static! {
    static ref GLOBAL_BLOCK_DEVICE: Mutex<Option<Arc<dyn BlockDeviceTrait>>> = Mutex::new(None);
}

/// 设置全局块设备（由外部调用）
pub fn set_global_block_device(device: Arc<dyn BlockDeviceTrait>) {
    *GLOBAL_BLOCK_DEVICE.lock() = Some(device);
}

/// 获取全局块设备
pub fn get_block_device() -> Option<Arc<dyn BlockDeviceTrait>> {
    GLOBAL_BLOCK_DEVICE.lock().clone()
}

#[derive(Debug,Clone)]
pub struct FileMetadata{
    ///权限
    pub permission:u16,
    ///创建时间
    pub create_time:u64,
    ///修改时间
    pub modify_time:u64,
}

#[derive(Debug,Clone, Copy,PartialEq, Eq)]
pub struct FileAttribute{
    ///节点类型
    pub tp:NodeType,
    ///节点大学
    pub size:usize,
    ///权限
    pub permission:u16,
    ///创建时间
    pub create_time:u64,
    ///修改时间
    pub modify_time:u64
}

#[derive(Debug,Clone, Copy,PartialEq, Eq)]
pub enum NodeType {
    File,
    Dir
}

#[derive(Debug,Clone, Copy,PartialEq, Eq)]
pub enum VfsError {
    NotAFile,
    NotADir,
    InvalidPath,
    InvalidOperation,
    PermissionDenied,
    NotEmpty,
    AlreadyExists,
    NotFound
}



/// 辅助函数：设置节点的parent
fn set_node_parent(node: &Arc<dyn VfsNodeOps>, parent: &dyn VfsNodeOps) {
    // 由于trait object的限制，我们需要使用unsafe来转换
    // 但更安全的方法是使用Any trait，但VfsNodeOps没有实现Any
    // 所以我们使用一个不同的方法：通过类型判断
    // 实际上，最简单的方法是在创建节点时就传入parent
    // 但为了保持API兼容，我们使用unsafe转换
    unsafe {
        let node_ptr = Arc::as_ptr(node) as *const u8;
        // 尝试转换为DirNode
        // 由于我们不知道具体类型，我们需要一个不同的方法
        // 暂时先不实现，使用一个更简单的方法
    }
}



///DirNode默认实现
impl DirNode {
    ///创建目录节点（基于磁盘）
    pub fn new_with_inode(name:String, inode_id: usize)->Arc<Self>{
        let children=Mutex::new(BTreeMap::new());
        let parent = Mutex::new(None);
        let metadata = Mutex::new(FileMetadata{
            permission:0o755,
            create_time:0,
            modify_time:0,
        });
        Arc::new(DirNode { 
            inode_id,
            children, 
            parent, 
            metadata, 
            name: Mutex::new(name) 
        })
    }
    
    ///创建根目录节点（基于磁盘）
    pub fn new_root_dir_with_inode(inode_id: usize)->Arc<Self>{
        let name="/".to_string();
        Self::new_with_inode(name, inode_id)
    }
    
    /// 设置parent（内部方法）
    pub fn set_parent_weak(&self, parent: Weak<dyn VfsNodeOps>) {
        *self.parent.lock() = Some(parent);
    }
    
    /// 从磁盘读取 DiskInode
    pub fn read_disk_inode(&self) -> Option<crate::bitmap::DiskInode> {
        let block_device = get_block_device()?;
        let (block_id, offset) = crate::blueosfs::get_inode_block_and_offset(self.inode_id);
        let mut inode_block = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut inode_block);
        let disk_inode = unsafe {
            *(inode_block.as_mut_ptr().add(offset) as *const crate::bitmap::DiskInode)
        };
        Some(disk_inode)
    }
    
    /// 写入 DiskInode 到磁盘
    pub fn write_disk_inode(&self, disk_inode: &crate::bitmap::DiskInode) -> Result<(), VfsError> {
        let block_device = get_block_device().ok_or(VfsError::InvalidOperation)?;
        let (block_id, offset) = crate::blueosfs::get_inode_block_and_offset(self.inode_id);
        let mut inode_block = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut inode_block);
        let inode_bytes = unsafe {
            core::slice::from_raw_parts(
                disk_inode as *const crate::bitmap::DiskInode as *const u8,
                core::mem::size_of::<crate::bitmap::DiskInode>()
            )
        };
        inode_block[offset..offset + core::mem::size_of::<crate::bitmap::DiskInode>()].copy_from_slice(inode_bytes);
        block_device.write_block(block_id, &inode_block);
        Ok(())
    }
}



impl FileNode {
    pub fn new_with_inode(name:String, inode_id: usize)->Arc<Self>{
       let file_node= FileNode{
            inode_id,
            parent:Mutex::new(None),
            metadata:Mutex::new(FileMetadata{
                permission:0o644,
                create_time:0,
                modify_time:0
            }),
            name:Mutex::new(name)
        };
        Arc::new(file_node)
    }
    
    /// 设置parent（内部方法）
    pub fn set_parent_weak(&self, parent: Weak<dyn VfsNodeOps>) {
        *self.parent.lock() = Some(parent);
    }
    
    /// 从磁盘读取 DiskInode
    pub fn read_disk_inode(&self) -> Option<crate::bitmap::DiskInode> {
        let block_device = get_block_device()?;
        let (block_id, offset) = crate::blueosfs::get_inode_block_and_offset(self.inode_id);
        let mut inode_block = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut inode_block);
        let disk_inode = unsafe {
            &*(inode_block.as_ptr().add(offset) as *const crate::bitmap::DiskInode)
        };
        Some(*disk_inode)
    }
    
    /// 写入 DiskInode 到磁盘
    pub fn write_disk_inode(&self, disk_inode: &crate::bitmap::DiskInode) -> Result<(), VfsError> {
        let block_device = get_block_device().ok_or(VfsError::InvalidOperation)?;
        let (block_id, offset) = crate::blueosfs::get_inode_block_and_offset(self.inode_id);
        let mut inode_block = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut inode_block);
        let inode_bytes = unsafe {
            core::slice::from_raw_parts(
                disk_inode as *const crate::bitmap::DiskInode as *const u8,
                core::mem::size_of::<crate::bitmap::DiskInode>()
            )
        };
        inode_block[offset..offset + core::mem::size_of::<crate::bitmap::DiskInode>()].copy_from_slice(inode_bytes);
        block_device.write_block(block_id, &inode_block);
        Ok(())
    }
    
    /// 从磁盘读取数据块
    fn read_data_block(&self, data_index: usize, buf: &mut [u8]) -> Result<(), VfsError> {
        let block_device = get_block_device().ok_or(VfsError::InvalidOperation)?;
        let block_id = crate::blueosfs::get_data_block_id(data_index);
        let mut block = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut block);
        let read_len = buf.len().min(BLOCK_SIZE);
        buf[..read_len].copy_from_slice(&block[..read_len]);
        Ok(())
    }
    
    /// 写入数据块到磁盘
    fn write_data_block(&self, data_index: usize, buf: &[u8]) -> Result<(), VfsError> {
        let block_device = get_block_device().ok_or(VfsError::InvalidOperation)?;
        let block_id = crate::blueosfs::get_data_block_id(data_index);
        let mut block = [0u8; BLOCK_SIZE];
        if buf.len() <= BLOCK_SIZE {
            block[..buf.len()].copy_from_slice(buf);
        } else {
            block.copy_from_slice(&buf[..BLOCK_SIZE]);
        }
        block_device.write_block(block_id, &block);
        Ok(())
    }
}
