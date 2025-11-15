use core::mem;

use alloc::{string::ToString, sync::Arc,vec::Vec};
use crate::{bitmap::{SuperBlock, DiskInode, DiskInodeType, BitMapAlloctor, BitMapAlloctorTrait, Bitmap_AllocUnit, DirEntry}, vfs::*};
use alloc::string::String;
use alloc::vec;
use log::{debug, error, warn};

const BlueOSFileSystemMagic:u32 = 0x79614000;
pub const INODEBITMAP_COUNT:u32 =100;
pub const DATABITMAP_COUNT:u32  =1000;

/// 计算 inode 区域起始块号
fn get_inode_area_start_block() -> usize {
    1 + INODEBITMAP_COUNT as usize + DATABITMAP_COUNT as usize
}

/// 计算数据区域起始块号
fn get_data_area_start_block() -> usize {
    let inode_area_start = get_inode_area_start_block();
    // 假设每个 inode 需要 64 字节，每个块 512 字节，可以存储 8 个 inode
    // 最大 inode 数 = INODEBITMAP_COUNT * BLOCK_SIZE * 8
    let max_inodes = INODEBITMAP_COUNT as usize * BLOCK_SIZE * 8;
    let inode_size = core::mem::size_of::<DiskInode>();
    let inodes_per_block = BLOCK_SIZE / inode_size;
    let inode_area_blocks = (max_inodes + inodes_per_block - 1) / inodes_per_block;
    inode_area_start + inode_area_blocks
}

/// 根据 inode_id 计算 inode 在磁盘上的位置
pub fn get_inode_block_and_offset(inode_id: usize) -> (usize, usize) {
    let inode_area_start = get_inode_area_start_block();
    let inode_size = core::mem::size_of::<DiskInode>();
    let inodes_per_block = BLOCK_SIZE / inode_size;
    let block_id = inode_area_start + (inode_id / inodes_per_block);
    let offset = (inode_id % inodes_per_block) * inode_size;
    (block_id, offset)
}

/// 根据 data_index 计算数据块在磁盘上的块号
pub fn get_data_block_id(data_index: usize) -> usize {
    get_data_area_start_block() + data_index
}

/// 间接块：每个块存储 128 个块指针（512 字节 / 4 字节）
const POINTERS_PER_BLOCK: usize = BLOCK_SIZE / 4;

/// 读取间接块中的块指针（返回的是 data_index，需要转换为绝对块号）
/// 返回完整的指针数组，包括零值
fn read_indirect_block(block_device: &Arc<dyn BlockDeviceTrait>, indirect_block_id: usize) -> Vec<u32> {
    let mut block = [0u8; BLOCK_SIZE];
    let absolute_block_id = get_data_block_id(indirect_block_id);
    block_device.read_block(absolute_block_id, &mut block);
    let mut pointers = Vec::with_capacity(POINTERS_PER_BLOCK);
    for i in 0..POINTERS_PER_BLOCK {
        let ptr = u32::from_le_bytes([
            block[i * 4],
            block[i * 4 + 1],
            block[i * 4 + 2],
            block[i * 4 + 3],
        ]);
        pointers.push(ptr);
    }
    pointers
}

/// 写入间接块
fn write_indirect_block(block_device: &Arc<dyn BlockDeviceTrait>, indirect_block_id: usize, pointers: &[u32]) {
    let mut block = [0u8; BLOCK_SIZE];
    for (i, &ptr) in pointers.iter().enumerate().take(POINTERS_PER_BLOCK) {
        let bytes = ptr.to_le_bytes();
        block[i * 4..i * 4 + 4].copy_from_slice(&bytes);
    }
    let absolute_block_id = get_data_block_id(indirect_block_id);
    block_device.write_block(absolute_block_id, &block);
}

/// 获取 inode 的所有数据块指针（返回绝对块号）
fn get_all_data_blocks(block_device: &Arc<dyn BlockDeviceTrait>, disk_inode: &DiskInode) -> Vec<usize> {
    let mut blocks = Vec::new();
    
    // 直接块（存储的是 data_index）
    for &data_index in &disk_inode.direct_blocks {
        if data_index != 0 {
            blocks.push(get_data_block_id(data_index as usize));
        }
    }
    
    // 一级间接块
    if disk_inode.indirect_block != 0 {
        let indirect_blocks = read_indirect_block(block_device, disk_inode.indirect_block as usize);
        for data_index in indirect_blocks {
            if data_index != 0 {
                blocks.push(get_data_block_id(data_index as usize));
            }
        }
    }
    
    // 二级间接块
    if disk_inode.double_indirect != 0 {
        let level1_blocks = read_indirect_block(block_device, disk_inode.double_indirect as usize);
        for level1_data_index in level1_blocks {
            if level1_data_index != 0 {
                let level2_blocks = read_indirect_block(block_device, level1_data_index as usize);
                for level2_data_index in level2_blocks {
                    if level2_data_index != 0 {
                        blocks.push(get_data_block_id(level2_data_index as usize));
                    }
                }
            }
        }
    }
    
    // 三级间接块
    if disk_inode.triple_indirect != 0 {
        let level1_blocks = read_indirect_block(block_device, disk_inode.triple_indirect as usize);
        for level1_data_index in level1_blocks {
            if level1_data_index != 0 {
                let level2_blocks = read_indirect_block(block_device, level1_data_index as usize);
                for level2_data_index in level2_blocks {
                    if level2_data_index != 0 {
                        let level3_blocks = read_indirect_block(block_device, level2_data_index as usize);
                        for level3_data_index in level3_blocks {
                            if level3_data_index != 0 {
                                blocks.push(get_data_block_id(level3_data_index as usize));
                            }
                        }
                    }
                }
            }
        }
    }
    
    blocks
}

/// 从目录的数据块中读取所有目录项
fn read_dir_entries(block_device: &Arc<dyn BlockDeviceTrait>, disk_inode: &DiskInode) -> Vec<DirEntry> {
    let mut entries = Vec::new();
    let data_blocks = get_all_data_blocks(block_device, disk_inode);
    
    for block_id in data_blocks {
        let mut block = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut block);
        
        let mut offset = 0;
        while offset + DirEntry::SIZE <= BLOCK_SIZE {
            let entry_bytes = &block[offset..offset + DirEntry::SIZE];
            let entry = unsafe { &*(entry_bytes.as_ptr() as *const DirEntry) };
            
            if entry.inode_id == 0 {
                break; // 空目录项，结束
            }
            
            entries.push(*entry);
            offset += DirEntry::SIZE;
        }
    }
    
    entries
}

/// 在目录中添加新的目录项
fn add_dir_entry(block_device: &Arc<dyn BlockDeviceTrait>, disk_inode: &mut DiskInode, entry: DirEntry) -> Result<(), VfsError> {
    // 查找可用的数据块
    let data_blocks = get_all_data_blocks(block_device, disk_inode);
    
    // 尝试在现有块中添加
    for &block_id in &data_blocks {
        let mut block = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut block);
        
        let mut offset = 0;
        while offset + DirEntry::SIZE <= BLOCK_SIZE {
            let entry_bytes = &block[offset..offset + DirEntry::SIZE];
            let existing_entry = unsafe { &*(entry_bytes.as_ptr() as *const DirEntry) };
            
            if existing_entry.inode_id == 0 {
                // 找到空槽，写入新目录项
                let entry_bytes = unsafe {
                    core::slice::from_raw_parts(&entry as *const DirEntry as *const u8, DirEntry::SIZE)
                };
                block[offset..offset + DirEntry::SIZE].copy_from_slice(entry_bytes);
                block_device.write_block(block_id, &block);
                disk_inode.file_size += DirEntry::SIZE as u32;
                return Ok(());
            }
            
            offset += DirEntry::SIZE;
        }
    }
    
    // 没有可用空间，需要分配新块
    // 简化处理：只使用直接块
    for i in 0..12 {
        if disk_inode.direct_blocks[i] == 0 {
            // 分配新块
            let alloc_unit = BitMapAlloctor::alloc_datamap(1, Arc::clone(block_device))
                .ok_or(VfsError::InvalidOperation)?;
            if alloc_unit.datanode.is_empty() {
                return Err(VfsError::InvalidOperation);
            }
            
            let new_block_id = alloc_unit.datanode[0].0 as u32;
            disk_inode.direct_blocks[i] = new_block_id;
            
            // 写入目录项到新块
            let mut block = [0u8; BLOCK_SIZE];
            let entry_bytes = unsafe {
                core::slice::from_raw_parts(&entry as *const DirEntry as *const u8, DirEntry::SIZE)
            };
            block[0..DirEntry::SIZE].copy_from_slice(entry_bytes);
            let absolute_block_id = get_data_block_id(new_block_id as usize);
            block_device.write_block(absolute_block_id, &block);
            disk_inode.file_size += DirEntry::SIZE as u32;
            return Ok(());
        }
    }
    
    Err(VfsError::InvalidOperation) // 没有可用块
}

pub struct BlueosFileSystem{
    root:Arc<DirNode>,
}


impl VfsOps for BlueosFileSystem {
    fn get_fs_name(&self)->alloc::string::String {
        "blueosfilesystem".to_string()    
    }

    fn get_root_dir(&self)->Arc<dyn VfsNodeOps> {
        self.root.clone()
    }
    ///待实现
    fn mount(&self,path:&str,mount_point:MountPoint)->Result<(),VfsError> {
        Ok(())
    }
    ///待实现
    fn unmount(&self,path:&str)->Result<(),VfsError> {
        Ok(())
    }
    fn verify_file_system(&self)->bool {
        let block_device = match crate::vfs::get_block_device() {
            Some(dev) => dev,
            None => return false,
        };
        
        let mut read_buffer = [0u8;BLOCK_SIZE];
        block_device.read_block(0, &mut read_buffer);
        let super_block = unsafe{&*(read_buffer.as_ref() as *const _ as *const SuperBlock)};
        super_block.magic == BlueOSFileSystemMagic
    }
    
    fn initial_file_systeam(&self,block_device:Arc<dyn BlockDeviceTrait>)->Result<(),VfsError> {
        // 超级块初始化
        let super_block = SuperBlock{
            magic: BlueOSFileSystemMagic,
            inode_bitmap_block_count:INODEBITMAP_COUNT,
            data_bitmap_block_count:DATABITMAP_COUNT,
            pad_:[0u8;500]
        };
        let super_block_sz:[u8;512] = unsafe{ mem::transmute(super_block)};
        block_device.write_block(0,&super_block_sz);
        
        // 位图初始化
        let empty_bitmap = [0u8;BLOCK_SIZE];
        for i in 1..=INODEBITMAP_COUNT as usize {
            block_device.write_block(i, &empty_bitmap);
        }
        let data_bitmap_start = 1 + INODEBITMAP_COUNT as usize;
        for i in 0..DATABITMAP_COUNT as usize {
            block_device.write_block(data_bitmap_start + i, &empty_bitmap);
        }
        
        // 分配根目录 inode (inode_id = 0)
        let mut root_inode_bitmap = [0u8; BLOCK_SIZE];
        block_device.read_block(1, &mut root_inode_bitmap);
        root_inode_bitmap[0] |= 1;
        block_device.write_block(1, &root_inode_bitmap);
        
        // 初始化根目录的 DiskInode
        // 根目录需要至少一个数据块来存储 "." 和 ".." 目录项
        let root_data_alloc = BitMapAlloctor::alloc_datamap(1, Arc::clone(&block_device))
            .ok_or(VfsError::InvalidOperation)?;
        if root_data_alloc.datanode.is_empty() {
            return Err(VfsError::InvalidOperation);
        }
        
        let root_data_index = root_data_alloc.datanode[0].0 as u32;
        let mut root_inode = DiskInode {
            file_size: 0,
            direct_blocks: {
                let mut blocks = [0u32; 12];
                blocks[0] = root_data_index;
                blocks
            },
            indirect_block: 0,
            double_indirect: 0,
            triple_indirect: 0,
            file_type: DiskInodeType::Dir,
            permission: 0o755,
            create_time: 0,
            access_time: 0,
            modify_time: 0,
            pad: [0; 2],
        };
        
        // 写入根目录的 DiskInode
        let (block_id, offset) = get_inode_block_and_offset(0);
        let mut inode_block = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut inode_block);
        let inode_bytes = unsafe {
            core::slice::from_raw_parts(
                &root_inode as *const DiskInode as *const u8,
                core::mem::size_of::<DiskInode>()
            )
        };
        inode_block[offset..offset + core::mem::size_of::<DiskInode>()].copy_from_slice(inode_bytes);
        block_device.write_block(block_id, &inode_block);
        
        // 初始化根目录的数据块：添加 "." 和 ".." 目录项
        let dot_entry = DirEntry::new(0, ".", DiskInodeType::Dir).unwrap();
        let dotdot_entry = DirEntry::new(0, "..", DiskInodeType::Dir).unwrap();
        
        let root_data_block_id = get_data_block_id(root_data_index as usize);
        let mut root_data_block = [0u8; BLOCK_SIZE];
        
        // 写入 "." 目录项
        let dot_bytes = unsafe {
            core::slice::from_raw_parts(&dot_entry as *const DirEntry as *const u8, DirEntry::SIZE)
        };
        root_data_block[0..DirEntry::SIZE].copy_from_slice(dot_bytes);
        
        // 写入 ".." 目录项
        let dotdot_bytes = unsafe {
            core::slice::from_raw_parts(&dotdot_entry as *const DirEntry as *const u8, DirEntry::SIZE)
        };
        root_data_block[DirEntry::SIZE..DirEntry::SIZE * 2].copy_from_slice(dotdot_bytes);
        
        block_device.write_block(root_data_block_id, &root_data_block);
        root_inode.file_size = (DirEntry::SIZE * 2) as u32;
        
        // 更新根目录的 DiskInode（更新 file_size）
        let mut inode_block2 = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut inode_block2);
        let inode_bytes2 = unsafe {
            core::slice::from_raw_parts(
                &root_inode as *const DiskInode as *const u8,
                core::mem::size_of::<DiskInode>()
            )
        };
        inode_block2[offset..offset + core::mem::size_of::<DiskInode>()].copy_from_slice(inode_bytes2);
        block_device.write_block(block_id, &inode_block2);
        
        // 验证超级块
        let mut read_buffer = [0u8;BLOCK_SIZE];
        block_device.read_block(0, &mut read_buffer);
        let magic = unsafe{&*(read_buffer.as_ref() as *const _ as *const SuperBlock)}.magic;
        assert_eq!(magic,BlueOSFileSystemMagic,"BlueosFileSystem initialed failed");
        Ok(())
    }
}


impl BlueosFileSystem {
    ///创建文件系统实例
    pub fn new()->Arc<Self>{
        let root_dir = DirNode::new_root_dir_with_inode(0); // '/'目录，inode_id=0
        Arc::new(
            BlueosFileSystem { 
                root: root_dir,
            }
        )
    }
    
    //获取超级块
    pub fn get_super_block()->Option<SuperBlock>{
        let block_device = crate::vfs::get_block_device()?;
        let mut read_buffer = [0u8;BLOCK_SIZE];
        block_device.read_block(0, &mut read_buffer);
        let super_block = 
        unsafe{*(read_buffer.as_ref() as *const _ as *const SuperBlock)}
        ;
        Some(super_block)
    }
}


///VfsNodeOps for DirNode
impl VfsNodeOps for DirNode {

    ///创建文件或者目录（基于磁盘）
    fn create(&self,name:&str,tp:NodeType)->Result<Arc<dyn VfsNodeOps>,VfsError> {
        let block_device = crate::vfs::get_block_device().ok_or(VfsError::InvalidOperation)?;
        
        let mut children = self.children.lock();
        if children.contains_key(name){
            return Err(VfsError::AlreadyExists);
        }
        
        // 分配 inode 和 data 块
        let data_blocks_needed = match tp {
            NodeType::Dir => 1,
            NodeType::File => 0,
        };
        
        let alloc_unit = BitMapAlloctor::alloc_datamap(data_blocks_needed, Arc::clone(&block_device))
            .ok_or(VfsError::InvalidOperation)?;
        
        let inode_id = alloc_unit.inode.0 as usize;
        let file_type = match tp {
            NodeType::Dir => DiskInodeType::Dir,
            NodeType::File => DiskInodeType::File,
        };
        
        // 创建 DiskInode
        let mut disk_inode = DiskInode {
            file_size: 0,
            direct_blocks: {
                let mut blocks = [0u32; 12];
                for (i, data_idx) in alloc_unit.datanode.iter().take(12).enumerate() {
                    blocks[i] = data_idx.0 as u32;
                }
                blocks
            },
            indirect_block: 0,
            double_indirect: 0,
            triple_indirect: 0,
            file_type,
            permission: 0o644,
            create_time: 0, // TODO: 获取当前时间
            access_time: 0,
            modify_time: 0,
            pad: [0; 2],
        };
        
        // 写入 DiskInode 到磁盘
        let (block_id, offset) = get_inode_block_and_offset(inode_id);
        let mut inode_block = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut inode_block);
        let inode_bytes = unsafe {
            core::slice::from_raw_parts(&disk_inode as *const DiskInode as *const u8, core::mem::size_of::<DiskInode>())
        };
        inode_block[offset..offset + core::mem::size_of::<DiskInode>()].copy_from_slice(inode_bytes);
        block_device.write_block(block_id, &inode_block);
        
        // 创建新节点
        let new_node:Arc<dyn VfsNodeOps> = match tp {
            NodeType::Dir => {
                let dir_node = DirNode::new_with_inode(name.to_string(), inode_id);
                let self_arc = unsafe { Arc::from_raw(self as *const Self) };
                let parent_weak = Arc::downgrade(&(self_arc.clone() as Arc<dyn VfsNodeOps>));
                dir_node.set_parent_weak(parent_weak);
                Arc::into_raw(self_arc);
                dir_node as Arc<dyn VfsNodeOps>
            }
            NodeType::File => {
                let file_node = FileNode::new_with_inode(name.to_string(), inode_id);
                let self_arc = unsafe { Arc::from_raw(self as *const Self) };
                let parent_weak = Arc::downgrade(&(self_arc.clone() as Arc<dyn VfsNodeOps>));
                file_node.set_parent_weak(parent_weak);
                Arc::into_raw(self_arc);
                file_node as Arc<dyn VfsNodeOps>
            }
        };
        
        // 在父目录中添加目录项
        let mut parent_disk_inode = self.read_disk_inode().ok_or(VfsError::InvalidOperation)?;
        let dir_entry = DirEntry::new(inode_id as u32, name, file_type)
            .ok_or(VfsError::InvalidOperation)?;
        
        // 添加目录项到父目录
        add_dir_entry(&block_device, &mut parent_disk_inode, dir_entry)?;
        
        // 写回更新后的父目录 DiskInode
        self.write_disk_inode(&parent_disk_inode)?;
        
        // 更新内存缓存
        children.insert(name.to_string(), new_node.clone());
        Ok(new_node)
    }
    fn find_child_node(&self,name:&str)->Option<Arc<dyn VfsNodeOps>> {
        // 先查内存缓存
        {
            let children = self.children.lock();
            if let Some(node) = children.get(name) {
                return Some(node.clone());
            }
        }
        
        // 内存中没有，从磁盘加载
        let block_device = crate::vfs::get_block_device()?;
        let disk_inode = self.read_disk_inode()?;
        let entries = read_dir_entries(&block_device, &disk_inode);
        
        let mut children = self.children.lock();
        for entry in entries {
            if let Some(entry_name) = entry.get_name() {
                if entry_name == name {
                    // 找到匹配的目录项，创建节点
                    let file_type = match entry.file_type {
                        1 => NodeType::File,
                        2 => NodeType::Dir,
                        _ => continue,
                    };
                    
                    let node: Arc<dyn VfsNodeOps> = match file_type {
                        NodeType::Dir => {
                            let dir_node = DirNode::new_with_inode(entry_name.to_string(), entry.inode_id as usize);
                            let self_arc = unsafe { Arc::from_raw(self as *const Self) };
                            let parent_weak = Arc::downgrade(&(self_arc.clone() as Arc<dyn VfsNodeOps>));
                            dir_node.set_parent_weak(parent_weak);
                            Arc::into_raw(self_arc);
                            dir_node as Arc<dyn VfsNodeOps>
                        }
                        NodeType::File => {
                            let file_node = FileNode::new_with_inode(entry_name.to_string(), entry.inode_id as usize);
                            let self_arc = unsafe { Arc::from_raw(self as *const Self) };
                            let parent_weak = Arc::downgrade(&(self_arc.clone() as Arc<dyn VfsNodeOps>));
                            file_node.set_parent_weak(parent_weak);
                            Arc::into_raw(self_arc);
                            file_node as Arc<dyn VfsNodeOps>
                        }
                    };
                    
                    children.insert(entry_name.to_string(), node.clone());
                    return Some(node);
                }
            }
        }
        
        None
    }
    fn get_attribute(&self)->FileAttribute {
        let attribute=self.metadata.lock();
        // 从磁盘读取文件大小
        let size = self.read_disk_inode()
            .map(|di| di.file_size as usize)
            .unwrap_or(0);
        FileAttribute { 
            tp:NodeType::Dir, 
            size, 
            permission: attribute.permission,
            create_time: attribute.create_time, 
            modify_time: attribute.modify_time,
         }
    }
    fn get_parent(&self)->Option<Arc<dyn VfsNodeOps>> {
        self.parent.lock().as_ref().and_then(|w| w.upgrade())
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
        let block_device = crate::vfs::get_block_device().ok_or(VfsError::InvalidOperation)?;
        let mut children = self.children.lock();
        let target = children.get(path).ok_or(VfsError::NotFound)?;
        
        //如果是目录检查是否为空
        if target.get_type() == NodeType::Dir{
            if !target.list_allnode_string().is_empty(){
                return Err(VfsError::NotEmpty);
            }
        }
        
        // 获取要删除节点的 inode_id 和数据块索引
        let (inode_id, data_indices) = match target.get_type() {
            NodeType::Dir => {
                unsafe {
                    let raw = Arc::as_ptr(target) as *const DirNode;
                    if raw.is_null() {
                        return Err(VfsError::InvalidOperation);
                    }
                    let dir_ref = &*raw;
                    let disk_inode = dir_ref.read_disk_inode().ok_or(VfsError::InvalidOperation)?;
                    let data_indices: Vec<_> = get_all_data_blocks(&block_device, &disk_inode)
                        .iter()
                        .map(|&abs_block_id| {
                            // 将绝对块号转换回 data_index
                            let data_index = abs_block_id - get_data_area_start_block();
                            crate::bitmap::data_index(data_index as u32)
                        })
                        .collect();
                    (dir_ref.inode_id, data_indices)
                }
            }
            NodeType::File => {
                unsafe {
                    let raw = Arc::as_ptr(target) as *const FileNode;
                    if raw.is_null() {
                        return Err(VfsError::InvalidOperation);
                    }
                    let file_ref = &*raw;
                    let disk_inode = file_ref.read_disk_inode().ok_or(VfsError::InvalidOperation)?;
                    let data_indices: Vec<_> = get_all_data_blocks(&block_device, &disk_inode)
                        .iter()
                        .map(|&abs_block_id| {
                            // 将绝对块号转换回 data_index
                            let data_index = abs_block_id - get_data_area_start_block();
                            crate::bitmap::data_index(data_index as u32)
                        })
                        .collect();
                    (file_ref.inode_id, data_indices)
                }
            }
        };
        
        // 回收资源
        let alloc_unit = Bitmap_AllocUnit {
            inode: crate::bitmap::inode_index(inode_id as u8),
            datanode: data_indices,
        };
        BitMapAlloctor::dealloc_datamap(alloc_unit, block_device.clone());
        
        // 清除被删除节点的parent引用
        match target.get_type() {
            NodeType::Dir => {
                unsafe {
                    let raw = Arc::as_ptr(target) as *const DirNode;
                    if !raw.is_null() {
                        let dir_ref = &*raw;
                        *dir_ref.parent.lock() = None;
                    }
                }
            }
            NodeType::File => {
                unsafe {
                    let raw = Arc::as_ptr(target) as *const FileNode;
                    if !raw.is_null() {
                        let file_ref = &*raw;
                        *file_ref.parent.lock() = None;
                    }
                }
            }
        }
        children.remove(path);
        Ok(())
    }
    fn rename(&self,old_path:&str,new_path:&str)->Result<(),VfsError> {
        // 获取父节点
        let parent = self.get_parent().ok_or(VfsError::InvalidOperation)?;
        // 确保父节点是目录
        if parent.get_type() != NodeType::Dir {
            return Err(VfsError::InvalidOperation);
        }
        // 从父节点的children中移除旧键名并插入新键名
        // 由于parent是Arc<dyn VfsNodeOps>，我们需要转换为DirNode
        unsafe {
            let parent_raw = Arc::as_ptr(&parent) as *const DirNode;
            if !parent_raw.is_null() {
                let parent_dir = &*parent_raw;
                let mut parent_children = parent_dir.children.lock();
                // 获取当前节点（通过old_path）
                if let Some(node) = parent_children.remove(old_path) {
                    // 检查新键名是否已存在
                    if parent_children.contains_key(new_path) {
                        // 如果新键名已存在，恢复旧键名
                        parent_children.insert(old_path.to_string(), node);
                        return Err(VfsError::AlreadyExists);
                    }
                    // 插入新键名
                    parent_children.insert(new_path.to_string(), node);
                } else {
                    return Err(VfsError::NotFound);
                }
            } else {
                return Err(VfsError::InvalidOperation);
            }
        }
        // 更新自己的name
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
        // 从磁盘读取文件大小
        let size = self.read_disk_inode()
            .map(|di| di.file_size as usize)
            .unwrap_or(0);
        FileAttribute { 
            tp: NodeType::File, 
            size,
            permission: metadata.permission, 
            create_time: metadata.create_time, 
            modify_time: metadata.modify_time
         }
    }
    fn get_parent(&self)->Option<Arc<dyn VfsNodeOps>> {
        self.parent.lock().as_ref().and_then(|w| w.upgrade())
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
        // 从磁盘读取 DiskInode
        let disk_inode = self.read_disk_inode().ok_or(VfsError::InvalidOperation)?;
        let file_size = disk_inode.file_size as usize;
        
        //检查偏移有效性
        if offset >= file_size {
            return Ok(0);
        }
        
        //计算实际读取长度
        let read_len = (file_size - offset).min(buf.len());
        let mut bytes_read = 0;
        let mut current_offset = offset;
        
        // 从数据块读取数据
        let block_device = crate::vfs::get_block_device().ok_or(VfsError::InvalidOperation)?;
        let data_blocks = get_all_data_blocks(&block_device, &disk_inode);
        
        while bytes_read < read_len && current_offset < file_size {
            let block_idx = current_offset / BLOCK_SIZE;
            if block_idx >= data_blocks.len() {
                break; // 超出已分配块范围
            }
            
            let block_offset = current_offset % BLOCK_SIZE;
            let remaining_in_block = BLOCK_SIZE - block_offset;
            let to_read = (read_len - bytes_read).min(remaining_in_block);
            
            let block_id = data_blocks[block_idx];
            let mut block = [0u8; BLOCK_SIZE];
            block_device.read_block(block_id, &mut block);
            
            buf[bytes_read..bytes_read + to_read].copy_from_slice(&block[block_offset..block_offset + to_read]);
            bytes_read += to_read;
            current_offset += to_read;
        }
        
        Ok(bytes_read)
    }
    fn remove(&self,path:&str)->Result<(),VfsError> {
        Err(VfsError::NotADir)
    }
    fn rename(&self,old_path:&str,new_path:&str)->Result<(),VfsError> {
        if old_path.eq("/"){
            return Err(VfsError::InvalidOperation);
        }
        // 获取父节点
        let parent = self.get_parent().ok_or(VfsError::InvalidOperation)?;
        // 确保父节点是目录
        if parent.get_type() != NodeType::Dir {
            return Err(VfsError::InvalidOperation);
        }
        // 从父节点的children中移除旧键名并插入新键名
        unsafe {
            let parent_raw = Arc::as_ptr(&parent) as *const DirNode;
            if !parent_raw.is_null() {
                let parent_dir = &*parent_raw;
                let mut parent_children = parent_dir.children.lock();
                // 获取当前节点（通过old_path）
                // 注意：old_path应该是文件名，不是完整路径
                if let Some(node) = parent_children.remove(old_path) {
                    // 检查新键名是否已存在
                    if parent_children.contains_key(new_path) {
                        // 如果新键名已存在，恢复旧键名
                        parent_children.insert(old_path.to_string(), node);
                        return Err(VfsError::AlreadyExists);
                    }
                    // 插入新键名
                    parent_children.insert(new_path.to_string(), node);
                } else {
                    return Err(VfsError::NotFound);
                }
            } else {
                return Err(VfsError::InvalidOperation);
            }
        }
        // 更新自己的name
        *self.name.lock() = new_path.to_string();
        Ok(())
    }
    fn truncate(&self,new_size:usize)->Result<(),VfsError> {
        let mut disk_inode = self.read_disk_inode().ok_or(VfsError::InvalidOperation)?;
        let current_size = disk_inode.file_size as usize;
        
        if new_size < current_size {
            // 截断：释放不需要的块
            let old_blocks = (current_size + BLOCK_SIZE - 1) / BLOCK_SIZE;
            let new_blocks = (new_size + BLOCK_SIZE - 1) / BLOCK_SIZE;
            
            if new_blocks < old_blocks {
                let block_device = crate::vfs::get_block_device().ok_or(VfsError::InvalidOperation)?;
                let data_blocks = get_all_data_blocks(&block_device, &disk_inode);
                // 回收不需要的块
                let mut dealloc_indices = Vec::new();
                for i in new_blocks..old_blocks.min(data_blocks.len()) {
                    let abs_block_id = data_blocks[i];
                    let data_index = abs_block_id - get_data_area_start_block();
                    dealloc_indices.push(crate::bitmap::data_index(data_index as u32));
                    // 清除直接块指针（简化处理，只处理直接块）
                    if i < 12 {
                        disk_inode.direct_blocks[i] = 0;
                    }
                }
                
                if !dealloc_indices.is_empty() {
                    let alloc_unit = Bitmap_AllocUnit {
                        inode: crate::bitmap::inode_index(0), // 不回收 inode
                        datanode: dealloc_indices,
                    };
                    // 只回收数据块，不回收 inode
                    for data_idx in &alloc_unit.datanode {
                        let data_bit_idx = data_idx.0 as usize;
                        let data_byte_idx = data_bit_idx / 8;
                        let data_bit_offset = data_bit_idx % 8;
                        let data_block_idx = data_byte_idx / BLOCK_SIZE;
                        let data_bitmap_start = 1 + INODEBITMAP_COUNT as usize;
                        
                        let mut data_block = vec![0u8; BLOCK_SIZE];
                        block_device.read_block(data_bitmap_start + data_block_idx, &mut data_block);
                        let byte_in_block = data_byte_idx % BLOCK_SIZE;
                        data_block[byte_in_block] &= !(1 << data_bit_offset);
                        block_device.write_block(data_bitmap_start + data_block_idx, &data_block);
                    }
                }
            }
        }
        
        disk_inode.file_size = new_size as u32;
        self.write_disk_inode(&disk_inode)?;
        Ok(())
    }
    fn write_at(&self,offset:usize,buf:&[u8])->Result<usize,VfsError> {
        let block_device = crate::vfs::get_block_device().ok_or(VfsError::InvalidOperation)?;
        
        // 读取当前的 DiskInode
        let mut disk_inode = self.read_disk_inode().ok_or(VfsError::InvalidOperation)?;
        let current_size = disk_inode.file_size as usize;
        let new_size = (offset + buf.len()).max(current_size);
        
        // 计算需要的块数
        let blocks_needed = (new_size + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let current_blocks = (current_size + BLOCK_SIZE - 1) / BLOCK_SIZE;
        
        // 如果需要更多块，分配新的数据块
        if blocks_needed > current_blocks {
            let additional_blocks = blocks_needed - current_blocks;
            
            // 分配新的数据块
            let alloc_unit = BitMapAlloctor::alloc_datamap(additional_blocks, block_device.clone())
                .ok_or(VfsError::InvalidOperation)?;
            
            if alloc_unit.datanode.len() < additional_blocks {
                return Err(VfsError::InvalidOperation);
            }
            
            // 将新分配的数据块索引写入 DiskInode
            for (i, data_idx) in alloc_unit.datanode.iter().enumerate() {
                let target_block_idx = current_blocks + i;
                
                if target_block_idx < 12 {
                    // 使用直接块
                    disk_inode.direct_blocks[target_block_idx] = data_idx.0 as u32;
                } else if target_block_idx < 12 + POINTERS_PER_BLOCK {
                    // 使用一级间接块
                    if disk_inode.indirect_block == 0 {
                        // 分配间接块
                        let indirect_alloc = BitMapAlloctor::alloc_datamap(1, block_device.clone())
                            .ok_or(VfsError::InvalidOperation)?;
                        if indirect_alloc.datanode.is_empty() {
                            return Err(VfsError::InvalidOperation);
                        }
                        disk_inode.indirect_block = indirect_alloc.datanode[0].0 as u32;
                    }
                    
                    // 读取间接块
                    let mut indirect_pointers = read_indirect_block(&block_device, disk_inode.indirect_block as usize);
                    
                    // 确保有足够的空间
                    while indirect_pointers.len() <= target_block_idx - 12 {
                        indirect_pointers.push(0);
                    }
                    
                    // 写入新的指针
                    indirect_pointers[target_block_idx - 12] = data_idx.0 as u32;
                    
                    // 写回间接块
                    write_indirect_block(&block_device, disk_inode.indirect_block as usize, &indirect_pointers);
                } else if target_block_idx < 12 + POINTERS_PER_BLOCK + POINTERS_PER_BLOCK * POINTERS_PER_BLOCK {
                    // 使用二级间接块
                    let level1_idx = (target_block_idx - 12 - POINTERS_PER_BLOCK) / POINTERS_PER_BLOCK;
                    let level2_idx = (target_block_idx - 12 - POINTERS_PER_BLOCK) % POINTERS_PER_BLOCK;
                    
                    // 分配或获取二级间接块
                    if disk_inode.double_indirect == 0 {
                        let double_indirect_alloc = BitMapAlloctor::alloc_datamap(1, block_device.clone())
                            .ok_or(VfsError::InvalidOperation)?;
                        if double_indirect_alloc.datanode.is_empty() {
                            return Err(VfsError::InvalidOperation);
                        }
                        disk_inode.double_indirect = double_indirect_alloc.datanode[0].0 as u32;
                    }
                    
                    // 读取二级间接块（包含一级间接块指针）
                    let mut level1_pointers = read_indirect_block(&block_device, disk_inode.double_indirect as usize);
                    
                    // 确保有足够的空间
                    while level1_pointers.len() <= level1_idx {
                        level1_pointers.push(0);
                    }
                    
                    // 分配或获取一级间接块
                    if level1_pointers[level1_idx] == 0 {
                        let level1_alloc = BitMapAlloctor::alloc_datamap(1, block_device.clone())
                            .ok_or(VfsError::InvalidOperation)?;
                        if level1_alloc.datanode.is_empty() {
                            return Err(VfsError::InvalidOperation);
                        }
                        level1_pointers[level1_idx] = level1_alloc.datanode[0].0 as u32;
                        // 写回二级间接块
                        write_indirect_block(&block_device, disk_inode.double_indirect as usize, &level1_pointers);
                    }
                    
                    // 读取一级间接块（包含数据块指针）
                    let mut level2_pointers = read_indirect_block(&block_device, level1_pointers[level1_idx] as usize);
                    
                    // 确保有足够的空间
                    while level2_pointers.len() <= level2_idx {
                        level2_pointers.push(0);
                    }
                    
                    // 写入数据块指针
                    level2_pointers[level2_idx] = data_idx.0 as u32;
                    
                    // 写回一级间接块
                    write_indirect_block(&block_device, level1_pointers[level1_idx] as usize, &level2_pointers);
                } else if target_block_idx < 12 + POINTERS_PER_BLOCK + POINTERS_PER_BLOCK * POINTERS_PER_BLOCK + POINTERS_PER_BLOCK * POINTERS_PER_BLOCK * POINTERS_PER_BLOCK {
                    // 使用三级间接块
                    let base_offset = 12 + POINTERS_PER_BLOCK + POINTERS_PER_BLOCK * POINTERS_PER_BLOCK;
                    let triple_offset = target_block_idx - base_offset;
                    let level1_idx = triple_offset / (POINTERS_PER_BLOCK * POINTERS_PER_BLOCK);
                    let level2_idx = (triple_offset % (POINTERS_PER_BLOCK * POINTERS_PER_BLOCK)) / POINTERS_PER_BLOCK;
                    let level3_idx = triple_offset % POINTERS_PER_BLOCK;
                    
                    // 分配或获取三级间接块
                    if disk_inode.triple_indirect == 0 {
                        let triple_indirect_alloc = BitMapAlloctor::alloc_datamap(1, block_device.clone())
                            .ok_or(VfsError::InvalidOperation)?;
                        if triple_indirect_alloc.datanode.is_empty() {
                            return Err(VfsError::InvalidOperation);
                        }
                        disk_inode.triple_indirect = triple_indirect_alloc.datanode[0].0 as u32;
                    }
                    
                    // 读取三级间接块（包含二级间接块指针）
                    let mut level1_pointers = read_indirect_block(&block_device, disk_inode.triple_indirect as usize);
                    
                    // 确保有足够的空间
                    while level1_pointers.len() <= level1_idx {
                        level1_pointers.push(0);
                    }
                    
                    // 分配或获取二级间接块
                    if level1_pointers[level1_idx] == 0 {
                        let level1_alloc = BitMapAlloctor::alloc_datamap(1, block_device.clone())
                            .ok_or(VfsError::InvalidOperation)?;
                        if level1_alloc.datanode.is_empty() {
                            return Err(VfsError::InvalidOperation);
                        }
                        level1_pointers[level1_idx] = level1_alloc.datanode[0].0 as u32;
                        // 写回三级间接块
                        write_indirect_block(&block_device, disk_inode.triple_indirect as usize, &level1_pointers);
                    }
                    
                    // 读取二级间接块（包含一级间接块指针）
                    let mut level2_pointers = read_indirect_block(&block_device, level1_pointers[level1_idx] as usize);
                    
                    // 确保有足够的空间
                    while level2_pointers.len() <= level2_idx {
                        level2_pointers.push(0);
                    }
                    
                    // 分配或获取一级间接块
                    if level2_pointers[level2_idx] == 0 {
                        let level2_alloc = BitMapAlloctor::alloc_datamap(1, block_device.clone())
                            .ok_or(VfsError::InvalidOperation)?;
                        if level2_alloc.datanode.is_empty() {
                            return Err(VfsError::InvalidOperation);
                        }
                        level2_pointers[level2_idx] = level2_alloc.datanode[0].0 as u32;
                        // 写回二级间接块
                        write_indirect_block(&block_device, level1_pointers[level1_idx] as usize, &level2_pointers);
                    }
                    
                    // 读取一级间接块（包含数据块指针）
                    let mut level3_pointers = read_indirect_block(&block_device, level2_pointers[level2_idx] as usize);
                    
                    // 确保有足够的空间
                    while level3_pointers.len() <= level3_idx {
                        level3_pointers.push(0);
                    }
                    
                    // 写入数据块指针
                    level3_pointers[level3_idx] = data_idx.0 as u32;
                    
                    // 写回一级间接块
                    write_indirect_block(&block_device, level2_pointers[level2_idx] as usize, &level3_pointers);
                } else {
                    // 超过三级间接块支持的范围
                    return Err(VfsError::InvalidOperation);
                }
            }
            
            // 写回 DiskInode（包含新分配的块指针）
            self.write_disk_inode(&disk_inode)?;
        }
        
        // 写入数据到磁盘
        let data_blocks = get_all_data_blocks(&block_device, &disk_inode);
        let mut bytes_written = 0;
        let mut current_offset = offset;
        
        while bytes_written < buf.len() {
            let block_idx = current_offset / BLOCK_SIZE;
            if block_idx >= data_blocks.len() {
                break; // 超出已分配块范围
            }
            
            let block_offset = current_offset % BLOCK_SIZE;
            let remaining_in_block = BLOCK_SIZE - block_offset;
            let to_write = (buf.len() - bytes_written).min(remaining_in_block);
            
            let block_id = data_blocks[block_idx];
            let mut block = [0u8; BLOCK_SIZE];
            block_device.read_block(block_id, &mut block);
            
            block[block_offset..block_offset + to_write].copy_from_slice(&buf[bytes_written..bytes_written + to_write]);
            block_device.write_block(block_id, &block);
            
            bytes_written += to_write;
            current_offset += to_write;
        }
        
        // 更新文件大小并写回 DiskInode
        disk_inode.file_size = new_size as u32;
        self.write_disk_inode(&disk_inode)?;
        
        Ok(bytes_written)
    }
}