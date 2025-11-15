use BlueosFS::BlockDeviceTrait;
use virtio_drivers::{Hal, VirtIOBlk, VirtIOHeader};
use lazy_static::*;
use alloc::{sync::Arc, vec::Vec};
use crate::driver::BlockDevice;
use crate::{memory::*};
use crate::sync::UPSafeCell;
use spin::Mutex;
const VIRTIO0: usize = 0x10001000;

lazy_static!{
    static ref QUEUE_FRAMES:UPSafeCell<Vec<FramTracker>> = UPSafeCell::new(Vec::new());
    /// 全局块设备实例
    pub static ref GLOBAL_BLOCK_DEVICE: Mutex<Option<Arc<dyn BlockDeviceTrait>>> = Mutex::new(None);
}

/// 初始化全局块设备
pub fn init_global_block_device() {
    let device = Arc::new(VirtBlk::new());
    *GLOBAL_BLOCK_DEVICE.lock() = Some(device);
}

/// 获取全局块设备
pub fn get_global_block_device() -> Option<Arc<dyn BlockDeviceTrait>> {
    GLOBAL_BLOCK_DEVICE.lock().clone()
}

pub struct VirtBlk(UPSafeCell<VirtIOBlk<'static,VirtioHal>>);



impl VirtBlk {
    pub fn new()->Self{
        VirtBlk(
            UPSafeCell::new(
                unsafe {
                    VirtIOBlk::new(&mut *(VIRTIO0 as *mut VirtIOHeader)).expect("failed new blk device")
                }
            )
        )
    }
}

impl BlockDevice for VirtBlk {
    fn initial_block_device(&mut self) {
        
    }
    fn read_blk(&self,sector:u64,user_buffer:&mut [u8]){
        self.0.lock().read_block(sector as usize, user_buffer).expect("failed to read block!")
    }
    fn write_blk(&self, sector: u64, user_buffer: &[u8]){
        self.0.lock()
        .write_block(sector as usize, user_buffer).expect("failed to write block")
    }
}


impl BlockDeviceTrait for VirtBlk {
    fn read_block(&self,block_id:usize,read_buffer:&mut [u8]) {
        self.read_blk(block_id as u64, read_buffer);
    }
    fn write_block(&self,block_id:usize,write_buffer:&[u8]) {
        self.write_blk(block_id as u64, write_buffer);
    }
}

pub struct VirtioHal;
impl Hal for VirtioHal {
    fn dma_alloc(pages: usize) -> virtio_drivers::PhysAddr {
        let mut base_ppn = PhysiNumber(0);
        for i in 0..pages {
            let frame_trace =alloc_frame().expect("no frame alloced");
            let ppn = frame_trace.ppn;
            if i==0{
                base_ppn = ppn;
            }
            QUEUE_FRAMES.lock().push(frame_trace);
        }
        let base_addr:PhysiAddr = base_ppn.into();
        base_addr.0
    }
    fn dma_dealloc(paddr: virtio_drivers::PhysAddr, pages: usize) -> i32 {
        let mut ppn:PhysiNumber = PhysiAddr(paddr).into();
        for _ in 0..pages {
            dealloc_frame(ppn.0);

            ppn.0+=1;
        }
        0
    }
    fn phys_to_virt(paddr: virtio_drivers::PhysAddr) -> virtio_drivers::VirtAddr {
        paddr
    }
    fn virt_to_phys(vaddr: virtio_drivers::VirtAddr) -> virtio_drivers::PhysAddr {
        vaddr
    }
}

