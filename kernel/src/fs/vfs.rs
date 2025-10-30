use core::str;

use alloc::collections::btree_map::BTreeMap;
use alloc::rc::Weak;
use alloc::vec;
use alloc::{sync::Arc, vec::Vec};
use alloc::string::{String, ToString};
use spin::Mutex;
use spin::RwLock;




///VFS抽象层
/// VfsNodeOps 文件和目录需要实现的通用操作
pub trait VfsNodeOps:Sync+Send {
    ///通用功能区域------------------------
    ///获取节点属性
    fn get_attribute(&self)->FileAttribute;
    ///返回节点类型
    fn get_type(&self)->NodeType;
    ///返回父节点引用
    fn get_parent(&self)->Option<Arc<dyn VfsNodeOps>>;
    ///重命名
    fn rename(&self,old_path:&str,new_path:&str)->Result<(),VfsError>;
    ///移动节点
    fn mv(&self,old_path:&str,new_path:&str)->Result<(),VfsError>;
    ///----------------------------
    
    ////目录专用功能-----------------------------
    ///创建节点 name为文件名字或者目录名字，不支持多级路径
    fn create(&self,name:&str,tp:NodeType)->Result<Arc<dyn VfsNodeOps>,VfsError>;
    ///删除节点
    fn remove(&self,path:&str)->Result<(),VfsError>;
    ///查找子节点
    fn find_child_node(&self,path:&str)->Option<Arc<dyn VfsNodeOps>>;
    ///列出所有子节点名字
    fn list_allnode_string(&self)->Vec<String>;
    ///--------------------------------------------

    ////文件专用功能---------------------------------
    ///在指定位置读
    fn read_at(&self,offset:usize,buf:&mut [u8])->Result<usize,VfsError>;
    ///在指定偏移写
    fn write_at(&self,offset:usize,buf:&[u8])->Result<usize,VfsError>;
    ///阶段文件或者扩展文件到指定大学
    fn truncate(&self,new_size:usize)->Result<(),VfsError>;
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
}
///挂载点
pub struct MountPoint{
    pub path:String,
    pub fs:Arc<dyn VfsOps>,
}


///文件节点
pub struct FileNode{
    content:RwLock<Vec<u8>>,//内容
   // parent:Mutex<Option<Weak<dyn VfsNodeOps>>>,//父节点引用
    metadata:Mutex<FileMetadata>,//文件元数据
    name:Mutex<String>//文件名字
}   
///目录节点
pub struct DirNode{
    children:Mutex<BTreeMap<String,Arc<dyn VfsNodeOps>>>,//子节点
    parent:Mutex<Option<Arc<dyn VfsNodeOps>>>,//父节点
    metadata:Mutex<FileMetadata>,//元数据
    name:Mutex<String>//名字
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
    tp:NodeType,
    ///节点大学
    pub size:usize,
    ///权限
    permission:u16,
    ///创建时间
    create_time:u64,
    ///修改时间
    modify_time:u64
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



///DirNode默认实现
impl DirNode {
    ///创建目录节点
    pub fn new(name:String)->Arc<Self>{
        let children=Mutex::new(BTreeMap::new());
        let parent = Mutex::new(None);
        let metadata = Mutex::new(FileMetadata{
            permission:0o755,
            create_time:0,
            modify_time:0,
        });
        Arc::new(DirNode { children, parent, metadata, name: Mutex::new(name) })
    }
    ///创建根目录节点
    pub fn new_root_dir()->Arc<Self>{
        let name="/".to_string();
        Self::new(name)
    }
}

///VfsNodeOps for DirNode
impl VfsNodeOps for DirNode {

    ///创建文件或者目录
    /// name 为文件夹或者文件名字
    fn create(&self,name:&str,tp:NodeType)->Result<Arc<dyn VfsNodeOps>,VfsError> {
        let mut children = self.children.lock();
        //检查节点是否存在
        if children.contains_key(name){
            return Err(VfsError::AlreadyExists);
        }
        //创建新节点
        let new_node:Arc<dyn VfsNodeOps> =match tp {
                NodeType::Dir=>{
                    DirNode::new(name.to_string())
                }
                NodeType::File=>{
                    FileNode::new(name.to_string())
                }
        };
        children.insert(name.to_string(), new_node.clone());
        Ok(new_node)
    }
    fn find_child_node(&self,name:&str)->Option<Arc<dyn VfsNodeOps>> {
        //返回子节点
        self.children.lock().get(name).cloned()

    }
    fn get_attribute(&self)->FileAttribute {
        let attribute=self.metadata.lock();
        FileAttribute { 
            tp:NodeType::Dir, 
            size: self.children.lock().len(), 
            permission: attribute.permission,
            create_time: attribute.create_time, 
            modify_time: attribute.modify_time,
         }
    }
    fn get_parent(&self)->Option<Arc<dyn VfsNodeOps>> {
        self.parent.lock().clone()
    }
    fn get_type(&self)->NodeType {
        NodeType::Dir
    }
    fn list_allnode_string(&self)->Vec<String> {
        self.children.lock().keys().cloned().collect()
    }
    fn mv(&self,old_path:&str,new_path:&str)->Result<(),VfsError> {
        Ok(())
    }
    fn read_at(&self,offset:usize,buf:&mut [u8])->Result<usize,VfsError> {
        Err(VfsError::NotAFile)
    }
    fn remove(&self,path:&str)->Result<(),VfsError> {
        let mut children = self.children.lock();
        let target = children.get(path).ok_or(VfsError::NotFound)?;
        //如果是目录检查是否为空
        if target.get_type() == NodeType::Dir{
            if !target.list_allnode_string().is_empty(){
                return Err(VfsError::NotEmpty);
            }
        }
        children.remove(path);
        Ok(())

    }
    fn rename(&self,old_path:&str,new_path:&str)->Result<(),VfsError> {
        *self.name.lock() = new_path.to_string();
        Ok(())
    }
    fn truncate(&self,new_size:usize)->Result<(),VfsError> {
        Err(VfsError::NotAFile)
    }
    fn write_at(&self,offset:usize,buf:&[u8])->Result<usize,VfsError> {
        Err(VfsError::NotAFile)
    }
}


impl FileNode {
    pub fn new(name:String,)->Arc<Self>{
       let file_node= FileNode{
            content:RwLock::new(Vec::new()),
            metadata:Mutex::new(FileMetadata{
                permission:0o644,
                create_time:0,
                modify_time:0
            }),
            name:Mutex::new(name)
        };
        Arc::new(file_node)
    }
}

///为文件node实现抽象
impl VfsNodeOps for FileNode {
    fn create(&self,path:&str,tp:NodeType)->Result<Arc<dyn VfsNodeOps>,VfsError> {
        Err(VfsError::NotADir)
    }
    fn find_child_node(&self,path:&str)->Option<Arc<dyn VfsNodeOps>> {
        None
    }
    fn get_attribute(&self)->FileAttribute {
        let metadata = self.metadata.lock();
        FileAttribute { 
            tp: NodeType::File, 
            size: self.content.read().len(),
            permission: metadata.permission, 
            create_time: metadata.create_time, 
            modify_time: metadata.modify_time
         }
    }
    fn get_parent(&self)->Option<Arc<dyn VfsNodeOps>> {
        self.get_parent()
    }
    fn get_type(&self)->NodeType {
        NodeType::File
    }
    fn list_allnode_string(&self)->Vec<String> {
        Vec::new()
        
    }
    fn mv(&self,old_path:&str,new_path:&str)->Result<(),VfsError> {
        Ok(())
    }
    fn read_at(&self,offset:usize,buf:&mut [u8])->Result<usize,VfsError> {
        //offset 从0开始
        let content = &self.content;
        //检查偏移有效性
        if offset >= content.read().len(){
            return Ok(0);
        }
        //计算实际读取长度
        let read_len = (content.read().len() -offset).min(buf.len());

        //复制数据
        buf[..read_len].copy_from_slice(&content.read()[offset..offset+read_len]);

        Ok(read_len)
    }
    fn remove(&self,path:&str)->Result<(),VfsError> {
        Err(VfsError::NotADir)
    }
    fn rename(&self,old_path:&str,new_path:&str)->Result<(),VfsError> {
        *self.name.lock()=new_path.to_string();
        Ok(())
    }
    fn truncate(&self,new_size:usize)->Result<(),VfsError> {
        let content = &self.content;
        content.write().resize(new_size, 0);
        Ok(())
    }
    fn write_at(&self,offset:usize,buf:&[u8])->Result<usize,VfsError> {
        let content = &self.content;
        //检查偏移有效性
        if offset+buf.len() > content.read().len(){
            content.write().resize(buf.len(), 0);
        }
        //写入数据
        content.write()[offset..offset+buf.len()].copy_from_slice(&buf);
        Ok(buf.len())
    }
}