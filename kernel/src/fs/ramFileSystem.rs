use alloc::{string::ToString, sync::Arc};

use crate::fs::vfs::{DirNode, NodeType, VfsOps};

///RamFileSystem
/// 速度快，掉电丢失
/// 有rust 的rail特性，不用手动释放内存




pub struct RamFileSystem{
    root:Arc<DirNode>//根目录节点
}


impl VfsOps for RamFileSystem {
    fn get_fs_name(&self)->alloc::string::String {
        "ramfs".to_string()
    }
    fn get_root_dir(&self)->Arc<dyn super::vfs::VfsNodeOps> {
        self.root.clone()
    }
    fn mount(&self,path:&str,mount_point:super::vfs::MountPoint)->Result<(),super::vfs::VfsError> {
        Ok(())
    }
    fn unmount(&self,path:&str)->Result<(),super::vfs::VfsError> {
        Ok(())   
    }
}

impl RamFileSystem {
    ///创建空文件系统实例
    pub fn new()->Arc<Self>{
        Arc::new(
            RamFileSystem { root: DirNode::new_root_dir() }
        )
    }

    ///创建带有一定初始目录的文件系统 /tmp /dev /proc
    pub fn initial_common_ramfilesystem()->Arc<Self>{
        let fs=Self::new();

        let root =fs.get_root_dir();
        //创建基本目录
        root.create("tmp", NodeType::Dir);
        root.create("dev", NodeType::Dir);
        root.create("proc", NodeType::Dir);
        fs
    }
}



