use core::mem::size_of;

use crate::driver::{VirtioBlockDevice, virtio::virtioblk::BLOCK_DEVICE};
use crate::driver::virtio::virtioblk::{BLOCK_SIZE_SIGNAL, VIRTQ_DESC_F_WRITE, virt_blkdevice_init};
use crate::driver::virtio::virtioblk::VirtioQueueMemory;
use crate::driver::virtio::virtioblk::{VIRTQ_DESC_F_NEXT, VIRTIO_BLKDEVICE_QUEUESIZE};
use crate::driver::virtio::virtioblk::virtq_used_elem;
use core::sync::atomic;
use log::{error, warn, info, debug};
use core::sync::atomic::Ordering;
use alloc::boxed::Box;
/* 请求类型 */
/* 读 */
const VIRTIO_BLK_T_IN:u32 = 0; 
/* 写 */
const VIRTIO_BLK_T_OUT:u32 = 1;
/* 响应类型 */
const VIRTIO_BLK_S_OK:usize = 0;     // 成功
const VIRTIO_BLK_S_FAILED:usize = 1; // 失败

///trait抽象层->api层
#[repr(C,packed)]
struct VirtioReq{
    /* 请求类型 */
    pub type_ :u32,
    /* 保留字段 */
    pub reserved:u32,
    /* 起始扇区号 */
    pub sector:u64
}

#[repr(C,packed)]
struct VirtioResp{
    /* 响应状态 0成功 1失败 */
    pub status:u8 
}

pub trait BlockDevice {
     fn initial_block_device(&mut self);
     fn write_blk(&self, sector: u64, user_buffer: &[u8]);
     fn read_blk(&self,sector:u64,user_buffer:&mut [u8]);
}

impl BlockDevice for VirtioBlockDevice {
    fn initial_block_device(&mut self) {
        virt_blkdevice_init(self);
    }
    fn read_blk(&self,sector:u64,user_buffer:&mut [u8]) {
       
    }
    fn write_blk(&self, sector: u64, user_buffer: &[u8]) {

    }
        

}


