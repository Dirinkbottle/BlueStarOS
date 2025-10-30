use alloc::{string::ToString, sync::{self, Arc}, vec::Vec};
use spin::{Mutex};
use alloc::string::String;
use crate::fs::vfs::{MountPoint, NodeType, VfsError, VfsNodeOps, VfsOps};
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
    /// 传入的path 要求以/开头
    /// 支持相对路径和绝对路径/......
    pub fn look_node(&self,path:&str)->Result<Arc<dyn VfsNodeOps>,VfsError>{
        //可能的路径 / /root/ /root/name 空
        //实例 /root/mnt/d/abc 挂载点/root.mbt/d
        //空路径处理，返回主文件系统根节点
        if path.is_empty() || path=="/"{
            return Ok(self.main_fs.get_root_dir());
        }
        //普通情况，解析路径
        //判断这些路径是否在挂载点中存在
        for mp in self.mount_points.lock().iter(){
            if path.starts_with(&mp.path){
                //说明在挂载点里面
                let sub_path = &path[mp.path.len()..];//以'/'开头
                if sub_path.is_empty(){
                    return Ok(mp.fs.get_root_dir());
                }
                //返回查找结果
                return self.find_in_node(mp.fs.get_root_dir(), &sub_path);
            }
        }

        //在主文件系统里面查找
        self.find_in_node(self.main_fs.get_root_dir(), path)
    }

    ///通用查找函数
    pub fn find_in_node(&self,node:Arc<dyn VfsNodeOps>,path:&str)->Result<Arc<dyn VfsNodeOps>,VfsError>{
        //去除开头的/
        let path = path.trim_start_matches("/");

        //去除后可能为空
        if path.is_empty(){
            return Ok(node);
        }

        //分割路径
        let parts:Vec<&str> = path.split("/").filter(|st|{!st.is_empty()}).collect();//去除/  /root/mnt/d/a -> root mnt d a 
        //逐级查找
        let mut current_node=node; 
        for part in parts{
            match part{
                "."=>{
                    continue;
                }
                ".."=>{
                    current_node=current_node.get_parent().ok_or(VfsError::NotFound)?;
                }

                name =>{
                    current_node = current_node
                    .find_child_node(name)
                    .ok_or(VfsError::NotFound)?;
                }
            }
        }
        Ok(current_node)

    }

    ///分离路径和名称 保留开头的/
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
        //查找父目录
        let parent=self.look_node(parent_path)?;
        //在父目录里面创建子节点
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



