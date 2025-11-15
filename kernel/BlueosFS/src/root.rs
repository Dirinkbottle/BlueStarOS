use alloc::{string::ToString, sync::{self, Arc}, vec::Vec};
use log::debug;
use spin::{Mutex};
use alloc::string::String;
use crate::vfs::{MountPoint, NodeType, VfsError, VfsNodeOps, VfsOps};
///RootFileSyste, 根文件系统管理器
pub struct RootFileSystem{
    main_fs:Arc<dyn VfsOps>,
    mount_points:Mutex<Vec<MountPoint>>
}
impl RootFileSystem {
    ///创建全局唯一的根文件系统管理器
    pub fn new(fs:Arc<dyn VfsOps>)->Self{
        Self {
             main_fs:fs,
             mount_points:Mutex::new(Vec::new()) 
            }
    }


    ///挂载操作
    pub fn mount(&self,path:&str,fs:Arc<dyn VfsOps>)->Result<(),VfsError>{
        //路径有效性验证
        if !path.starts_with("/"){
            return Err(VfsError::InvalidPath);
        }

        //检查挂载点是否已经存在
        let mut mount_pointst = self.mount_points.lock();
        for mp in mount_pointst.iter(){
            if mp.path == path{
                return Err(VfsError::AlreadyExists);
            }
        }

        mount_pointst.push(MountPoint { 
            path: path.to_string(), 
            fs: fs
        });

        Ok(())
    }
    ///取消挂载操作
    pub fn unmount(&self,path:&str)->Result<(),VfsError>{
        let mut mount_points = self.mount_points.lock();

        let position = mount_points
        .iter()
        .position(|mp|{
            mp.path==path
        }).ok_or(VfsError::NotFound)?;

        mount_points.remove(position);
        Ok(())

    }
    ///核心操作，路径查找和遍历
    /// 传入的path 必须是绝对路径
    pub fn look_node(&self,path:&str)->Result<Arc<dyn VfsNodeOps>,VfsError>{
        if path.is_empty() || path=="/"{
            return Ok(self.main_fs.get_root_dir());
        }
        
        let mount_points = self.mount_points.lock();
        for mp in mount_points.iter(){
            if path.starts_with(&mp.path){
                let sub_path = &path[mp.path.len()..];
                if sub_path.is_empty(){
                    return Ok(mp.fs.get_root_dir());
                }
                return self.find_in_node(mp.fs.get_root_dir(), &sub_path);
            }
        }
        drop(mount_points);

        self.find_in_node(self.main_fs.get_root_dir(), path)
    }

    ///通用查找函数
    pub fn find_in_node(&self,node:Arc<dyn VfsNodeOps>,path:&str)->Result<Arc<dyn VfsNodeOps>,VfsError>{
        let path = path.trim_start_matches("/");
        if path.is_empty(){
            return Ok(node);
        }

        let parts:Vec<&str> = path.split("/").filter(|st|{!st.is_empty()}).collect();
        let mut current_node=node; 
        for part in parts.iter() {
            match *part {
                "."=> continue,
                ".."=> {
                    current_node = current_node.get_parent().ok_or(VfsError::NotFound)?;
                }
                name => {
                    current_node = current_node.find_child_node(name).ok_or(VfsError::NotFound)?;
                }
            }
        }
        Ok(current_node)
    }

    ///分离路径和名称 保留开头的/  因此必须是绝对路径
    fn split_path<'a>(&self,path:&'a str)->Result<(&'a str,&'a str), VfsError>{
        if !path.starts_with("/"){
            return Err(VfsError::InvalidPath);
        }
        //去除多余的/
        let path =path.trim_end_matches('/');
        //找到最后一次匹配的/的位置
        match path.rfind('/'){
            Some(pos)=>{
                let parent =if pos==0{"/"}else{&path[..pos]};
                let name = &path[pos+1..];
                if name.is_empty(){
                    return Err(VfsError::InvalidPath);
                }
               return Ok((parent,name));
            }
            None=>{Err(VfsError::InvalidPath)}
        }
        
    }

    ///创建节点
    pub fn create_node(&self,path:&str,node_type:NodeType)->Result<Arc<dyn VfsNodeOps>,VfsError>{
        let (parent_path,name) = self.split_path(path)?;
        let parent = self.look_node(parent_path)?;
        parent.create(name, node_type)
    }

    ///root层创建文件api
    pub fn create_file(&self,path:&str)->Result<Arc<dyn VfsNodeOps>,VfsError>{
       self.create_node(path, NodeType::File)
    }
    ///创建文件夹api
    pub fn create_dir(&self,path:&str)->Result<Arc<dyn VfsNodeOps>,VfsError>{
        self.create_node(path, NodeType::Dir)
    }
    ///删除节点（文件或者目录）
    pub fn remove(&self,path:&str)->Result<(),VfsError>{
        let (parent_path,name) = self.split_path(path)?;
        let parent = self.look_node(parent_path)?;
        parent.remove(name)
    }
    ///返回主文件系统
    pub fn main_fs(&self)->Arc<dyn VfsOps>{
        self.main_fs.clone()
    }
    ///列出所有挂载点
    pub fn list_all_mountpoints(&self)->Vec<String>{
        self.mount_points.lock().iter().map(|mp|{mp.path.clone()}).collect()
    }
}



