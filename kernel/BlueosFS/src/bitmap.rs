use alloc::{sync::Arc, vec::Vec};

use crate::{BlockDeviceTrait, vfs::BLOCK_SIZE};
use crate::{BlueosFileSystem, DATABITMAP_COUNT, INODEBITMAP_COUNT};
use alloc::vec;
pub struct inode_index(pub u8);
pub struct data_index(pub u32);  // 改为 u32 以支持更多数据块

///位图分配单元 一个inode n个datamap
pub struct Bitmap_AllocUnit{
    pub inode:inode_index,
    pub datanode:Vec<data_index>
}

///Inode数据结构
#[repr(C)]
#[derive(Clone, Copy)]
pub enum DiskInodeType {
    File = 1,
    Dir = 2
}

/// 类似 ext2 的简化 inode 结构
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DiskInode{
    pub file_size: u32,              // 文件大小（字节）
    pub direct_blocks: [u32; 12],    // 12 个直接块指针
    pub indirect_block: u32,          // 一级间接块指针
    pub double_indirect: u32,         // 二级间接块指针
    pub triple_indirect: u32,         // 三级间接块指针
    pub file_type: DiskInodeType,     // 文件类型
    pub permission: u16,               // 权限（类似 Unix 权限位）
    pub create_time: u32,              // 创建时间（Unix 时间戳）
    pub access_time: u32,              // 访问时间
    pub modify_time: u32,              // 修改时间
    pub pad: [u8; 2],                  // 对齐填充
}

/// 目录项结构（类似 ext2_dir_entry）
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct DirEntry {
    pub inode_id: u32,                // 指向的 inode 号
    pub name_len: u8,                 // 文件名长度
    pub file_type: u8,                // 文件类型（DiskInodeType as u8）
    pub name: [u8; 59],               // 文件名（最大 59 字节，总大小 64 字节）
}

impl DirEntry {
    pub const SIZE: usize = 64;       // 目录项固定大小 64 字节
    
    /// 创建新的目录项
    pub fn new(inode_id: u32, name: &str, file_type: DiskInodeType) -> Option<Self> {
        if name.len() > 59 {
            return None; // 文件名太长
        }
        
        let mut entry = DirEntry {
            inode_id,
            name_len: name.len() as u8,
            file_type: file_type as u8,
            name: [0; 59],
        };
        
        entry.name[..name.len()].copy_from_slice(name.as_bytes());
        Some(entry)
    }
    
    /// 获取文件名
    pub fn get_name(&self) -> Option<&str> {
        core::str::from_utf8(&self.name[..self.name_len as usize]).ok()
    }
}


#[repr(C)]
#[derive(Clone, Copy)]
pub struct SuperBlock{
    pub magic:u32,
    pub inode_bitmap_block_count:u32,
    pub data_bitmap_block_count:u32,
    pub pad_:[u8;(BLOCK_SIZE - 4*3) as usize]
}


///位图分配器 一次返回一个inode n个datanode 一个datanode可以储存512字节 返回一个datanode 的vec，存放
/// 可用的datanode索引
pub struct BitMapAlloctor;
pub trait BitMapAlloctorTrait {
    ///分配函数
    fn alloc_datamap(count:usize,block_device:Arc<dyn BlockDeviceTrait>)->Option<Bitmap_AllocUnit>;
    ///回收函数
    fn dealloc_datamap(unit:Bitmap_AllocUnit,block_device:Arc<dyn BlockDeviceTrait>)->bool;
}




impl BitMapAlloctorTrait for  BitMapAlloctor {
    fn alloc_datamap(count:usize,block_device:Arc<dyn BlockDeviceTrait>)->Option<Bitmap_AllocUnit> {
        use log::{debug, error, warn};
        debug!("[BitMapAlloctor::alloc_datamap] Start: count={}", count);
        
        let inode_bitmap_count:usize=INODEBITMAP_COUNT as usize;
        let data_bitmap_count:usize= DATABITMAP_COUNT as usize;
        debug!("[BitMapAlloctor::alloc_datamap] inode_bitmap_count={}, data_bitmap_count={}", 
               inode_bitmap_count, data_bitmap_count);
        
        ///inode位图从1号块开始（块0是超级块）
        let inode_bitmap_start_blockid = 1;
        let inode_bitmap_end_blockid = 1 + inode_bitmap_count - 1;
        /// data位图从inode位图块结束之后开始
        let data_bitmap_start_blockid = inode_bitmap_end_blockid + 1;
        let data_bitmap_end_blockid = data_bitmap_start_blockid + data_bitmap_count - 1;
        debug!("[BitMapAlloctor::alloc_datamap] inode_bitmap: blocks {}..={}", 
               inode_bitmap_start_blockid, inode_bitmap_end_blockid);
        debug!("[BitMapAlloctor::alloc_datamap] data_bitmap: blocks {}..={}", 
               data_bitmap_start_blockid, data_bitmap_end_blockid);
        
        ///inode bitmap 缓存
        debug!("[BitMapAlloctor::alloc_datamap] Reading inode bitmap from disk...");
        let mut inodemap_buffer:Vec<u8> = vec![0; BLOCK_SIZE * inode_bitmap_count];
        for (i, chunk) in inodemap_buffer.chunks_mut(BLOCK_SIZE).enumerate() {
            block_device.read_block(inode_bitmap_start_blockid + i, chunk);
        }
        debug!("[BitMapAlloctor::alloc_datamap] Inode bitmap read, size={} bytes", inodemap_buffer.len());
        
        ///datanode bitmap 缓存
        debug!("[BitMapAlloctor::alloc_datamap] Reading data bitmap from disk...");
        let mut datamap_buffer:Vec<u8> = vec![0; BLOCK_SIZE * data_bitmap_count];
        for (i, chunk) in datamap_buffer.chunks_mut(BLOCK_SIZE).enumerate() {
            block_device.read_block(data_bitmap_start_blockid + i, chunk);
        }
        debug!("[BitMapAlloctor::alloc_datamap] Data bitmap read, size={} bytes", datamap_buffer.len());

        /// 查找空闲的 inode（按位查找）
        debug!("[BitMapAlloctor::alloc_datamap] Searching for free inode...");
        let mut inode_number: Option<u32> = None;
        let max_inode_bits = inode_bitmap_count * BLOCK_SIZE * 8;
        debug!("[BitMapAlloctor::alloc_datamap] max_inode_bits={}", max_inode_bits);
        for bit_idx in 0..max_inode_bits {
            let byte_idx = bit_idx / 8;
            let bit_offset = bit_idx % 8;
            let byte = inodemap_buffer[byte_idx];
            if (byte & (1 << bit_offset)) == 0 {
                // 找到空闲 inode，标记为已分配
                debug!("[BitMapAlloctor::alloc_datamap] Found free inode at bit_idx={}", bit_idx);
                inodemap_buffer[byte_idx] |= 1 << bit_offset;
                inode_number = Some(bit_idx as u32);
                // 写回对应的位图块
                let block_idx = byte_idx / BLOCK_SIZE;
                let block_start = block_idx * BLOCK_SIZE;
                let block_data = &inodemap_buffer[block_start..block_start + BLOCK_SIZE];
                block_device.write_block(inode_bitmap_start_blockid + block_idx, block_data);
                debug!("[BitMapAlloctor::alloc_datamap] Inode bitmap updated and written to block {}", 
                       inode_bitmap_start_blockid + block_idx);
                break;
            }
        }
        
        let inode_number = match inode_number {
            Some(n) if n <= u8::MAX as u32 => {
                debug!("[BitMapAlloctor::alloc_datamap] Allocated inode: {}", n);
                n as u8
            }
            Some(n) => {
                error!("[BitMapAlloctor::alloc_datamap] ERROR: inode_number {} exceeds u8::MAX", n);
                return None;
            }
            None => {
                error!("[BitMapAlloctor::alloc_datamap] ERROR: No free inode found! max_inode_bits={}", max_inode_bits);
                return None; // 没有空闲 inode
            }
        };

        /// 查找空闲的 data 块（按位查找）
        debug!("[BitMapAlloctor::alloc_datamap] Searching for {} free data blocks...", count);
        let mut datanode_number: Vec<data_index> = Vec::new();
        let max_data_bits = data_bitmap_count * BLOCK_SIZE * 8;
        debug!("[BitMapAlloctor::alloc_datamap] max_data_bits={}", max_data_bits);
        let mut allocated_count = 0;
        
        for bit_idx in 0..max_data_bits {
            if allocated_count >= count {
                break;
            }
            let byte_idx = bit_idx / 8;
            let bit_offset = bit_idx % 8;
            let byte = datamap_buffer[byte_idx];
            if (byte & (1 << bit_offset)) == 0 {
                // 找到空闲 data 块，标记为已分配
                debug!("[BitMapAlloctor::alloc_datamap] Found free data block at bit_idx={}", bit_idx);
                datamap_buffer[byte_idx] |= 1 << bit_offset;
                datanode_number.push(data_index(bit_idx as u32));
                allocated_count += 1;
                // 写回对应的位图块
                let block_idx = byte_idx / BLOCK_SIZE;
                let block_start = block_idx * BLOCK_SIZE;
                let block_data = &datamap_buffer[block_start..block_start + BLOCK_SIZE];
                block_device.write_block(data_bitmap_start_blockid + block_idx, block_data);
            }
        }

        debug!("[BitMapAlloctor::alloc_datamap] Allocated {}/{} data blocks", datanode_number.len(), count);
        if datanode_number.len() < count {
            // 如果分配的数据块不足，需要回滚已分配的 inode
            error!("[BitMapAlloctor::alloc_datamap] ERROR: Failed to allocate enough data blocks! Got {}, needed {}", 
                   datanode_number.len(), count);
            debug!("[BitMapAlloctor::alloc_datamap] Rolling back inode allocation...");
            let byte_idx = inode_number as usize / 8;
            let bit_offset = inode_number as usize % 8;
            let block_idx = byte_idx / BLOCK_SIZE;
            let mut rollback_block = vec![0u8; BLOCK_SIZE];
            block_device.read_block(inode_bitmap_start_blockid + block_idx, &mut rollback_block);
            rollback_block[byte_idx % BLOCK_SIZE] &= !(1 << bit_offset);
            block_device.write_block(inode_bitmap_start_blockid + block_idx, &rollback_block);
            debug!("[BitMapAlloctor::alloc_datamap] Inode rollback completed");
            return None;
        }

        let result = Bitmap_AllocUnit{
            inode:inode_index(inode_number),
            datanode:datanode_number
        };
        debug!("[BitMapAlloctor::alloc_datamap] Success: inode_id={}, data_blocks={}", 
               result.inode.0, result.datanode.len());
        Some(result)
        
    }
    fn dealloc_datamap(unit:Bitmap_AllocUnit,block_device:Arc<dyn BlockDeviceTrait>)->bool {
        let inode_bitmap_count:usize=INODEBITMAP_COUNT as usize;
        let data_bitmap_count:usize= DATABITMAP_COUNT as usize;
        ///inode位图从1号块开始
        let inode_bitmap_start_blockid = 1;
        let inode_bitmap_end_blockid = 1 + inode_bitmap_count - 1;
        /// data位图从inode位图块结束之后开始
        let data_bitmap_start_blockid = inode_bitmap_end_blockid + 1;
        let data_bitmap_end_blockid = data_bitmap_start_blockid + data_bitmap_count - 1;

        /// 回收 inode：按位操作
        let inode_bit_idx = unit.inode.0 as usize;
        let inode_byte_idx = inode_bit_idx / 8;
        let inode_bit_offset = inode_bit_idx % 8;
        let inode_block_idx = inode_byte_idx / BLOCK_SIZE;
        
        // 读取对应的位图块
        let mut inode_block = vec![0u8; BLOCK_SIZE];
        block_device.read_block(inode_bitmap_start_blockid + inode_block_idx, &mut inode_block);
        
        // 检查该位是否已分配
        let byte_in_block = inode_byte_idx % BLOCK_SIZE;
        if (inode_block[byte_in_block] & (1 << inode_bit_offset)) == 0 {
            // 该 inode 未被分配，返回错误
            return false;
        }
        
        // 清除位图标记
        inode_block[byte_in_block] &= !(1 << inode_bit_offset);
        block_device.write_block(inode_bitmap_start_blockid + inode_block_idx, &inode_block);

        /// 回收 data 块：按位操作
        for data_idx in &unit.datanode {
            let data_bit_idx = data_idx.0 as usize;
            let data_byte_idx = data_bit_idx / 8;
            let data_bit_offset = data_bit_idx % 8;
            let data_block_idx = data_byte_idx / BLOCK_SIZE;
            
            // 读取对应的位图块
            let mut data_block = vec![0u8; BLOCK_SIZE];
            block_device.read_block(data_bitmap_start_blockid + data_block_idx, &mut data_block);
            
            // 检查该位是否已分配
            let byte_in_block = data_byte_idx % BLOCK_SIZE;
            if (data_block[byte_in_block] & (1 << data_bit_offset)) == 0 {
                // 该 data 块未被分配，跳过（可能已经被回收）
                continue;
            }
            
            // 清除位图标记
            data_block[byte_in_block] &= !(1 << data_bit_offset);
            block_device.write_block(data_bitmap_start_blockid + data_block_idx, &data_block);
        }

        true
    }
}