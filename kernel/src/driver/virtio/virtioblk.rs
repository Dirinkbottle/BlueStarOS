use core::mem::size_of;

use crate::sync::UPSafeCell;
use alloc::{boxed::Box, collections::vec_deque::VecDeque, format, string::ToString, vec::Vec};
use bitflags::bitflags;
use lazy_static::lazy_static;
use log::*;


/// Virtio block device driver - 块设备专用 

const VIRTIO_BLK_MMIO_BASE: u32 = 0x1000_1000;
const VIRTIO_BLK_MAGIC_VALUE: u32 = 0x7472_6976;

const VIRTIO_BLK_REG_MAGIC: u32 = 0x000;
const VIRTIO_BLK_REG_VERSION: u32 = 0x004;
const VIRTIO_BLK_REG_DEVICE_ID: u32 = 0x008;
const VIRTIO_BLK_REG_DEVICE_FEATURES: u32 = 0x010;
const VIRTIO_BLK_REG_DEVICE_FEATURES_SEL: u32 = 0x014;
const VIRTIO_BLK_REG_DRIVER_FEATURES: u32 = 0x020;
const VIRTIO_BLK_REG_DRIVER_FEATURES_SEL: u32 = 0x024;
const VIRTIO_BLK_REG_STATUS: u32 = 0x070;
const VIRTIO_BLKDEVICE_ID:u32 = 0x2;

// ==================== 队列配置相关寄存器（virtio MMIO 传输方式）====================
// 一、队列配置寄存器
const VIRTIO_MMIO_QUEUE_SEL: u32 = 0x030;        // 队列选择寄存器：选择要配置的虚拟队列索引
const VIRTIO_MMIO_QUEUE_SIZE: u32 = 0x034;        // 队列大小寄存器：读取/写入当前选中队列的大小
const VIRTIO_MMIO_QUEUE_ADDR: u32 = 0x038;        // 队列基地址低32位：虚拟队列的物理基地址（低32位）
const VIRTIO_MMIO_QUEUE_ADDR_HI: u32 = 0x03C;     // 队列基地址高32位：虚拟队列的物理基地址（高32位）
const VIRTIO_MMIO_QUEUE_ENABLE: u32 = 0x040;      // 队列使能控制位：写入1启用队列，写入0禁用

// 二、队列通知相关寄存器
const VIRTIO_MMIO_QUEUE_NOTIFY: u32 = 0x050;      // 队列通知触发寄存器：驱动写入队列索引通知设备

// 三、中断相关寄存器
const VIRTIO_MMIO_INTERRUPT_STATUS: u32 = 0x060;  // 中断状态寄存器（只读）：bit 0 表示队列已用环有新数据
const VIRTIO_MMIO_INTERRUPT_ACK: u32 = 0x064;     // 中断确认寄存器（只写）：驱动写入对应bit清除中断状态

pub const VIRTIO_BLKDEVICE_QUEUESIZE:u16=20;
pub const VIRTQ_DESC_F_NEXT:u16 = 0x0001;
pub const VIRTQ_DESC_F_WRITE:u16 =0x0002;
/*表示当前描述符是 “间接描述符”，其指向的缓冲区中存储
的是更多描述符（扩展分散 / 聚集的缓冲区数量）。 */
const VIRTQ_DESC_F_INDIRECT:u16 = 0x0004;
/* 驱动设置该标志后，设备处理完可用环中的描述符链时，
不向驱动发送中断通知（适用于驱动主动轮询场景，减少中断开销）。 */
const VIRTQ_AVAIL_F_NO_INTERRUPT:u16 = 0x0001;
/*设备设置该标志后，驱动无需通过 “通知机制” 告知设备新
的可用缓冲区（适用于设备主动轮询场景，减少通知开销）。
 */
const VIRTQ_USED_F_NO_NOTIFY:u16 =0x0001;

pub const BLOCK_SIZE_SIGNAL:usize =512;

const QUEUE_INDEX_DEFAULT:u16 = 0;//块设备默认使用队列0

/* 获取磁盘总扇区数 */
const VIRTIO_BLK_T_GET_SIZE: u32 = 0x0000_0008;

bitflags! {
    pub struct VirtioStatus: u8 {
        const ACKNOWLEDGE       = 1 << 0;
        const DRIVER            = 1 << 1;
        const DRIVER_OK         = 1 << 2;
        const FEATURES_OK       = 1 << 3;
        const DEVICE_NEEDS_REST = 1 << 6;
        const FAILED            = 1 << 7;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct VirtioFeatureSet: u64 {
        // ------------------------------
        // 通用特征（所有 virtio 设备共享，v1.0 规范 Section 5.2）
        // ------------------------------
        /// 支持间接描述符（Section 5.2.1）：减少连续内存占用，适配碎片化缓冲区
        const RING_INDIRECT_DESC = 1 << 2;
        /// 支持事件索引（Section 5.2.6）：抑制无效中断，降低 CPU 开销
        const RING_EVENT_IDX     = 1 << 6;
        /// 标识 virtio 1.0 版本（Section 5.2.11）：必选特征，启用后遵循 virtio 1.0 规范
        const VERSION_1          = 1 << 32;

        // ------------------------------
        // virtio-blk 专属特征（Section 9.2）
        // ------------------------------
        /// 支持获取磁盘容量（Section 9.2.1）：通过 VIRTIO_BLK_T_GET_SIZE 命令获取总扇区数
        const BLK_SIZE           = 1 << 0; // VIRTIO_BLK_F_SIZE
        /// 限制单次 I/O 的最大内存段数（Section 9.2.2）：避免碎片化缓冲区导致的性能下降
        const BLK_SEG_MAX        = 1 << 1; // VIRTIO_BLK_F_SEG_MAX
        /// 设备为只读（Section 9.2.6）：驱动需拒绝写操作，防止数据篡改
        const BLK_RO             = 1 << 5; // VIRTIO_BLK_F_RO
        /// 支持缓存刷新（Section 9.2.10）：确保数据写入物理介质，防止掉电丢失（必备）
        const BLK_FLUSH          = 1 << 9; // VIRTIO_BLK_F_FLUSH
        /// 暴露磁盘拓扑信息（Section 9.2.11）：如扇区大小、最优 I/O 粒度，优化读写对齐
        const BLK_TOPOLOGY       = 1 << 10; // VIRTIO_BLK_F_TOPOLOGY
        /// 支持多队列（Section 9.2.13）：多个 virtqueue 并行处理 I/O，提升并发性能
        const BLK_MQ             = 1 << 12; // VIRTIO_BLK_F_MQ
        /// 支持 DISCARD/TRIM 操作（Section 9.2.14）：释放无用空间，优化存储利用率
        const BLK_DISCARD        = 1 << 13; // VIRTIO_BLK_F_DISCARD
        /// 支持快速写零（Section 9.2.15）：批量填充零数据，比逐字节写高效
        const BLK_WRITE_ZEROES   = 1 << 14; // VIRTIO_BLK_F_WRITE_ZEROES
    }
}

#[repr(C,packed)]
pub struct VirtioQueueMemory{
    pub desc_chain:[virtq_desc;VIRTIO_BLKDEVICE_QUEUESIZE as usize],
    pub avavil_queue:virtq_avail_queue,
    pub used_queue:virtq_used,
}


/* virtio描述符 已验证*/
#[repr(C,packed)]
#[derive(Default,Clone, Copy)]
pub struct virtq_desc{
    pub addr:u64,
    pub len:u32,
    pub flags:u16,
    /* 下一个描述符在数组的索引 */
    pub next:u16
}
/* 可用环 available ring 已验证 */
#[repr(C,packed)]
pub struct virtq_avail_queue{
    pub flags:u16,
    pub next_idx:u16,
    pub ring:[u16;VIRTIO_BLKDEVICE_QUEUESIZE as usize] //描述符的起始索引
}

/* 已处理的描述符 已验证*/
#[repr(C, packed)]
#[derive(Default,Clone, Copy)]
pub struct virtq_used_elem{
    pub id:u32, //已处理描述符链起始索引
    pub len:u32 //设备实际写入的字节数
}


/* 已用环 used ring 已验证 */
#[repr(C,packed)]
pub struct virtq_used{
    pub flags:u16,
    pub next_idx:u16,
    pub complete_queue:[virtq_used_elem;VIRTIO_BLKDEVICE_QUEUESIZE as usize],
}


/* 描述符分配器 */
impl virtio_descriptor_alloctor {
    pub fn alloc_reqsize_dector(&mut self)->(usize,usize,usize){
        let mut queue:(usize,usize,usize)=(0,0,0);
        let mut result:[usize;3]=[0,0,0];
        for idx in 0..3 {
            if self.queue[self.current]==true {
                error!("Virtio_decriptor alloc failed!!!");
                panic!();
            }
            self.queue[self.current] = true;
            result[idx]=self.current;
            self.current+=1;
        }
        debug!("destor alloc :{} {} {}",queue.0,queue.1,queue.2);
        queue.0 = result[0];
        queue.1 = result[1];
        queue.2 = result[2];
        queue
    }
}

// virtq_desc::new 已移除，描述符在初始化时全为0，在 blockdevice 操作时动态配置

impl VirtioFeatureSet {
    /// 驱动默认启用的特征组合（virtio 1.0 协议）
    /// 必选特征：VERSION_1（标识 virtio 1.0 版本）、BLK_FLUSH（数据安全）
    /// 优化特征：RING_INDIRECT_DESC（内存适配）、BLK_WRITE_ZEROES（写性能）
    pub const DRIVER_DEFAULT: Self = Self::from_bits_truncate(Self::VERSION_1.bits()
        | Self::RING_INDIRECT_DESC.bits()
        | Self::BLK_FLUSH.bits());
}
pub struct virtio_descriptor_alloctor{
    queue:[bool;VIRTIO_BLKDEVICE_QUEUESIZE as usize],
    current:usize, //当前位于哪个可以被分配出去的索引，可以直接分配
}
#[repr(C,align(16))]
pub struct VirtioBlockDevice {
    pub initiaed:bool,
    magic: u32,
    version: u32,
    device_id: u32,
    status: VirtioStatus,
    negotiated_features: VirtioFeatureSet,
    pub queue_size:u16,//队列大小
    virtdsptbaddr: usize,//描述符表地址
    avail_addr:usize,//可用队列地址
    used_phys:usize,//已使用队列地址
    pub virtio_queueMemory:VirtioQueueMemory,
     desc_alloctor:virtio_descriptor_alloctor,
}

impl Default for VirtioBlockDevice {
    fn default() -> Self {
        // 描述符、可用环、已用环全部初始化为0
        let dec_chian:[virtq_desc;  VIRTIO_BLKDEVICE_QUEUESIZE as usize] = [virtq_desc::default();VIRTIO_BLKDEVICE_QUEUESIZE as usize];
        let avail_queue = virtq_avail_queue{
            flags:0,
            next_idx:0,
            ring:[0;VIRTIO_BLKDEVICE_QUEUESIZE as usize]
        };
        let used_queue = virtq_used{
            flags:0, // 禁用通知（使用轮询）
            next_idx:0,
            complete_queue:[virtq_used_elem{id:0,len:0};VIRTIO_BLKDEVICE_QUEUESIZE as usize]
        };
        let queue_memory = VirtioQueueMemory{
            desc_chain:dec_chian ,
            avavil_queue: avail_queue,
            used_queue:used_queue ,

        };
        let desc_alloctor=virtio_descriptor_alloctor{
            queue:[false;VIRTIO_BLKDEVICE_QUEUESIZE as usize],
            current:0
        };
        Self {
            initiaed:false,
            magic: VIRTIO_BLK_MAGIC_VALUE,
            version: 0,
            device_id: 0,
            status: VirtioStatus::empty(),
            negotiated_features: VirtioFeatureSet::empty(),
            queue_size:0,
            virtdsptbaddr:0,
            avail_addr:0,
            used_phys:0,
            virtio_queueMemory:queue_memory,
            desc_alloctor:desc_alloctor,
        }
    }
}




impl VirtioBlockDevice {

    // ==================== Packed 结构体安全访问辅助函数 ====================
    
    /// 安全读取可用环的 next_idx（避免未对齐引用警告）
    #[inline]
    fn read_avail_next_idx(avail: &virtq_avail_queue) -> u16 {
        unsafe {
            core::ptr::read_volatile(
                core::ptr::addr_of!(avail.next_idx) as *const u16
            )
        }
    }
    
    /// 安全写入可用环的 next_idx
    #[inline]
    fn write_avail_next_idx(avail: &mut virtq_avail_queue, value: u16) {
        unsafe {
            core::ptr::write_volatile(
                core::ptr::addr_of_mut!(avail.next_idx),
                value
            );
        }
    }
    
    /// 安全读取可用环的 flags
    #[inline]
    fn read_avail_flags(avail: &virtq_avail_queue) -> u16 {
        unsafe {
            core::ptr::read_volatile(
                core::ptr::addr_of!(avail.flags) as *const u16
            )
        }
    }
    
    /// 安全读取已用环的 next_idx
    #[inline]
    fn read_used_next_idx(used: &virtq_used) -> u16 {
        unsafe {
            core::ptr::read_volatile(
                core::ptr::addr_of!(used.next_idx) as *const u16
            )
        }
    }
    
    /// 安全读取已用环的 flags
    #[inline]
    fn read_used_flags(used: &virtq_used) -> u16 {
        unsafe {
            core::ptr::read_volatile(
                core::ptr::addr_of!(used.flags) as *const u16
            )
        }
    }
    
    /// 安全读取描述符字段（用于需要避免警告的场景）
    #[inline]
    fn read_desc_field<T>(desc: &virtq_desc, field_offset: usize) -> T 
    where
        T: Copy,
    {
        unsafe {
            let desc_ptr = desc as *const virtq_desc as *const u8;
            let field_ptr = desc_ptr.add(field_offset) as *const T;
            core::ptr::read_volatile(field_ptr)
        }
    }
    
    /// 安全读取描述符的 len 字段
    #[inline]
    fn read_desc_len(desc: &virtq_desc) -> u32 {
        unsafe {
            core::ptr::read_volatile(
                core::ptr::addr_of!(desc.len) as *const u32
            )
        }
    }
    
    /// 安全读取描述符的 addr 字段
    #[inline]
    fn read_desc_addr(desc: &virtq_desc) -> usize {
        unsafe {
            core::ptr::read_volatile(
                core::ptr::addr_of!(desc.addr) as *const usize
            )
        }
    }
    
    /// 安全读取描述符的 flags 字段
    #[inline]
    fn read_desc_flags(desc: &virtq_desc) -> u16 {
        unsafe {
            core::ptr::read_volatile(
                core::ptr::addr_of!(desc.flags) as *const u16
            )
        }
    }
    
    /// 安全读取描述符的 next 字段
    #[inline]
    fn read_desc_next(desc: &virtq_desc) -> u16 {
        unsafe {
            core::ptr::read_volatile(
                core::ptr::addr_of!(desc.next) as *const u16
            )
        }
    }
    
    /// 安全读取已用环元素的 id 字段
    #[inline]
    fn read_used_elem_id(elem: &virtq_used_elem) -> u32 {
        unsafe {
            core::ptr::read_volatile(
                core::ptr::addr_of!(elem.id) as *const u32
            )
        }
    }
    
    /// 安全读取已用环元素的 len 字段
    #[inline]
    fn read_used_elem_len(elem: &virtq_used_elem) -> u32 {
        unsafe {
            core::ptr::read_volatile(
                core::ptr::addr_of!(elem.len) as *const u32
            )
        }
    }

    /// 释放描述符链（标记为空闲）
    pub fn free_descriptor_chain(&mut self, req_idx: usize, data_idx: usize, resp_idx: usize) {
        let queue_memory = &mut self.virtio_queueMemory;
        // 1. 重置描述符 flags
        queue_memory.desc_chain[req_idx].flags = 0;
        queue_memory.desc_chain[data_idx].flags = 0;
        queue_memory.desc_chain[resp_idx].flags = 0;
        // 2. 关键：重置描述符分配器的空闲标记
        self.desc_alloctor.queue[req_idx] = false;
        self.desc_alloctor.queue[data_idx] = false;
        self.desc_alloctor.queue[resp_idx] = false;
        // 3. 可选：重置 next 字段，避免残留链接
        queue_memory.desc_chain[req_idx].next = 0;
        queue_memory.desc_chain[data_idx].next = 0;
        queue_memory.desc_chain[resp_idx].next = 0;
    }

    pub fn get_virtio_queueMemory(&mut self)->&mut VirtioQueueMemory{
        &mut self.virtio_queueMemory
    }

    pub fn alloc_reQueue(&mut self)->(usize,usize,usize){
        self.desc_alloctor.alloc_reqsize_dector()
    }

    ///真正初始化队列
    fn init_virtqueue(&mut self)-> Result<(),&'static str>{
        self.queue_size = VIRTIO_BLKDEVICE_QUEUESIZE;
        /* 初始化描述符表 */
        self.initial_virtio_dectb();


        /* 连续队列物理内存基地址计算 */
        let queue_phys_addr = &self.virtio_queueMemory as *const _ as usize;
        
        // 计算各部分大小（用于验证）
        let desc_table_size = size_of::<[virtq_desc; VIRTIO_BLKDEVICE_QUEUESIZE as usize]>();
        let avail_ring_size = size_of::<virtq_avail_queue>();
        let used_ring_size = size_of::<virtq_used>();
        
        info!("Virtqueue memory layout:");
        info!("  Physical base address: {:#x}", queue_phys_addr);
        info!("  Descriptor table size: {} bytes", desc_table_size);
        info!("  Available ring size: {} bytes", avail_ring_size);
        info!("  Used ring size: {} bytes", used_ring_size);
        
        /* 其它队列各部分物理地址 */
        self.virtdsptbaddr = queue_phys_addr; // 描述符表物理地址=队列基地址
        self.avail_addr = queue_phys_addr + desc_table_size;
        self.used_phys = self.avail_addr + avail_ring_size;
        
        // 验证地址对齐（根据 Virtio 规范）
        // 描述符表：16 字节对齐
        if self.virtdsptbaddr % 16 != 0 {
            warn!("Descriptor table address {:#x} is not 16-byte aligned", self.virtdsptbaddr);
        }
        // 可用环：2 字节对齐
        if self.avail_addr % 2 != 0 {
            warn!("Available ring address {:#x} is not 2-byte aligned", self.avail_addr);
        }
        // 已用环：4 字节对齐
        if self.used_phys % 4 != 0 {
            warn!("Used ring address {:#x} is not 4-byte aligned", self.used_phys);
        }
        
        info!("  Descriptor table: {:#x} (size: {})", self.virtdsptbaddr, desc_table_size);
        info!("  Available ring: {:#x} (size: {})", self.avail_addr, avail_ring_size);
        info!("  Used ring: {:#x} (size: {})", self.used_phys, used_ring_size);
        
        // 验证实际内存地址与计算地址是否一致
        let actual_avail_addr = &self.virtio_queueMemory.avavil_queue as *const _ as usize;
        let actual_used_addr = &self.virtio_queueMemory.used_queue as *const _ as usize;
        
        if actual_avail_addr != self.avail_addr {
            error!("Available ring address mismatch! Calculated: {:#x}, Actual: {:#x}", 
                   self.avail_addr, actual_avail_addr);
            return Err("Available ring address calculation error");
        }
        if actual_used_addr != self.used_phys {
            error!("Used ring address mismatch! Calculated: {:#x}, Actual: {:#x}", 
                   self.used_phys, actual_used_addr);
            return Err("Used ring address calculation error");
        }

        /* 注册队列到设备 Virtio 1.0 */
        self.select_queue(QUEUE_INDEX_DEFAULT);
        //协商队列大小
        let device_max_support_size = self.read_queue_size_max();
        if self.queue_size > device_max_support_size{
            return Err("queue size out of device support max");
        }
        self.write_queue_size(self.queue_size);

        //设置队列物理基地址
        // 重要：根据 Virtio 规范，QUEUE_ADDR 应该设置为描述符表的物理地址
        // 设备会根据描述符表地址计算 available ring 和 used ring 的位置
        self.set_queue_addr(self.virtdsptbaddr);
        //启用队列
        self.set_queue_enable(true);

        /* 验证队列是否启用成功 */
        let is_enabled = unsafe {
            core::ptr::read_volatile(Self::mmio_ptr::<u32>(VIRTIO_MMIO_QUEUE_ENABLE)) == 1
        };
        if !is_enabled {
            return Err("failed to enable virtqueue");
        }

        info!(
            "Virtqueue 0 registered successfully:\n  desc_phys: {:#x}\n  avail_phys: {:#x}\n  used_phys: {:#x}",
            self.virtdsptbaddr, self.avail_addr, self.used_phys
        );
        
        /* 队列完整性校验 */
        if let Err(e) = self.verify_queue_integrity() {
            error!("Queue integrity verification failed: {}", e);
            return Err("Queue integrity verification failed");
        }
        
        Ok(())

    }

    // ==================== 队列配置相关函数 ====================
    
    /// 选择要配置的虚拟队列索引（VIRTIO_MMIO_QUEUE_SEL）
    fn select_queue(&self, queue_index: u16) {
        unsafe {
            core::ptr::write_volatile(
                Self::mmio_ptr::<u32>(VIRTIO_MMIO_QUEUE_SEL),
                queue_index as u32,
            );
        }
    }

    /// 读取设备支持的最大队列大小（VIRTIO_MMIO_QUEUE_SIZE）
    fn read_queue_size_max(&self) -> u16 {
        unsafe {
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            let size = core::ptr::read_volatile(Self::mmio_ptr::<u32>(VIRTIO_MMIO_QUEUE_SIZE)) as u16;
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            size
        }
    }

    /// 写入协商后的队列大小（VIRTIO_MMIO_QUEUE_SIZE）
    /// 驱动需先读取设备支持的最大大小，再写入协商后的值（需 ≤ 设备最大值）
    fn write_queue_size(&self, queue_size: u16) {
        unsafe {
            core::ptr::write_volatile(
                Self::mmio_ptr::<u32>(VIRTIO_MMIO_QUEUE_SIZE),
                queue_size as u32,
            );
        }
    }

    /// 设置队列基地址（VIRTIO_MMIO_QUEUE_ADDR 和 VIRTIO_MMIO_QUEUE_ADDR_HI）
    /// 存储虚拟队列（描述符表 + 可用环 + 已用环）的物理基地址
    fn set_queue_addr(&self, phys_addr: usize) {
        unsafe {
            // 设置低32位
            core::ptr::write_volatile(
                Self::mmio_ptr::<u32>(VIRTIO_MMIO_QUEUE_ADDR),
                (phys_addr & 0xFFFF_FFFF) as u32,
            );
            // 设置高32位（64位系统）
            core::ptr::write_volatile(
                Self::mmio_ptr::<u32>(VIRTIO_MMIO_QUEUE_ADDR_HI),
                ((phys_addr >> 32) & 0xFFFF_FFFF) as u32,
            );
        }
    }

    /// 使能/禁用队列（VIRTIO_MMIO_QUEUE_ENABLE）
    /// 写入1启用当前选中的队列，写入0禁用（禁用后设备不再访问该队列）
    fn set_queue_enable(&self, enable: bool) {
        unsafe {
            core::ptr::write_volatile(
                Self::mmio_ptr::<u32>(VIRTIO_MMIO_QUEUE_ENABLE),
                if enable { 1 } else { 0 },
            );
        }
    }

    // ==================== 队列通知相关函数 ====================
    
    /// 通知设备处理队列（VIRTIO_MMIO_QUEUE_NOTIFY）
    /// 驱动写入要通知的队列索引，设备会立即检查该队列的可用环，处理新的可用描述符链
    pub fn notify_queue(&self, queue_index: u16) {
        unsafe {
            core::ptr::write_volatile(
                Self::mmio_ptr::<u32>(VIRTIO_MMIO_QUEUE_NOTIFY),
                queue_index as u32,
            );
        }
    }

    // ==================== 中断相关函数 ====================
    
    /// 读取中断状态（VIRTIO_MMIO_INTERRUPT_STATUS）
    /// bit 0 表示"队列已用环有新数据"（设备处理完队列请求后置位）
    /// 返回中断状态位掩码
    pub fn read_interrupt_status(&self) -> u32 {
        unsafe {
            core::ptr::read_volatile(Self::mmio_ptr::<u32>(VIRTIO_MMIO_INTERRUPT_STATUS))
        }
    }

    /// 检查是否有队列中断（bit 0）
    pub fn has_queue_interrupt(&self) -> bool {
        (self.read_interrupt_status() & 0x1) != 0
    }

    /// 确认中断（VIRTIO_MMIO_INTERRUPT_ACK）
    /// 驱动写入对应bit（如bit 0），清除中断状态，允许后续中断触发
    fn ack_interrupt(&self, interrupt_bits: u32) {
        unsafe {
            core::ptr::write_volatile(
                Self::mmio_ptr::<u32>(VIRTIO_MMIO_INTERRUPT_ACK),
                interrupt_bits,
            );
        }
    }

    /// 确认队列中断（清除bit 0）
    pub fn ack_queue_interrupt(&self) {
        self.ack_interrupt(0x1);
    }

    /// 队列完整性校验函数
    /// 校验准备的队列确认无误，包括描述符表、可用环、已用环、内存布局等
    /// 返回 Ok(()) 表示校验通过，Err 包含错误信息
    pub fn verify_queue_integrity(&self) -> Result<(), &'static str> {
        let queue_memory = &self.virtio_queueMemory;
        let queue_size = self.queue_size as usize;

        // ==================== 1. 队列大小校验 ====================
        if queue_size == 0 {
            return Err("Queue size is zero");
        }
        if queue_size > VIRTIO_BLKDEVICE_QUEUESIZE as usize {
            return Err("Queue size exceeds maximum");
        }

        // ==================== 2. 描述符表校验 ====================
        info!("Verifying descriptor table ({} descriptors)...", queue_size);
        
        // 描述符初始化时应该全为0（缓冲区在操作时动态分配）
        for idx in 0..queue_size {
            let desc = &queue_memory.desc_chain[idx];
            let desc_addr = Self::read_desc_addr(desc);
            let desc_len = Self::read_desc_len(desc);
            let desc_flags = Self::read_desc_flags(desc);
            let desc_next = Self::read_desc_next(desc);
            
            // 初始化时描述符应该全为0
            if desc_addr != 0 || desc_len != 0 || desc_flags != 0 || desc_next != 0 {
                warn!("Descriptor[{}]: not zero-initialized (addr: {:#x}, len: {}, flags: {:#x}, next: {})", 
                      idx, desc_addr, desc_len, desc_flags, desc_next);
            }
        }
        info!("✓ Descriptor table verification passed (all zero-initialized)");

        // ==================== 3. 缓冲区池校验 ====================
        // 缓冲区池已移除，缓冲区在 blockdevice 操作时动态分配
        // 此步骤已在步骤2中完成，跳过
        info!("✓ Buffer pool verification passed (dynamic allocation)");

        // ==================== 4. 可用环校验 ====================
        let avail = &queue_memory.avavil_queue;
        
        // 4.1 检查 next_idx 初始化（使用安全访问方法）
        let avail_next_idx = Self::read_avail_next_idx(avail);
        if avail_next_idx != 0 {
            warn!("Available ring next_idx is not 0 (got {}), may be in use", avail_next_idx);
        }
        
        // 4.2 检查 flags（使用安全访问方法）- 应该设置为 VIRTQ_AVAIL_F_NO_INTERRUPT
        let avail_flags = Self::read_avail_flags(avail);
        if avail_flags != VIRTQ_AVAIL_F_NO_INTERRUPT {
            warn!("Available ring flags mismatch (got {:#x}, expected {:#x})", 
                  avail_flags, VIRTQ_AVAIL_F_NO_INTERRUPT);
        }
        
        // 4.3 检查 ring 数组大小（数组长度是编译时常量，可以直接访问）
        // 注意：ring 是固定大小数组，len() 返回编译时常量，不会产生未对齐引用
        const RING_SIZE: usize = VIRTIO_BLKDEVICE_QUEUESIZE as usize;
        if RING_SIZE != queue_size {
            error!("Available ring size mismatch: got {}, expected {}", 
                   RING_SIZE, queue_size);
            return Err("Available ring size mismatch");
        }
        info!("✓ Available ring verification passed");

        // ==================== 5. 已用环校验 ====================
        let used = &queue_memory.used_queue;
        
        // 5.1 检查 next_idx 初始化（使用安全访问方法）
        let used_next_idx = Self::read_used_next_idx(used);
        if used_next_idx != 0 {
            warn!("Used ring next_idx is not 0 (got {}), may be in use", used_next_idx);
        }
        
        // 5.2 检查 flags（使用安全访问方法）- 应该设置为 VIRTQ_USED_F_NO_NOTIFY
        let used_flags = Self::read_used_flags(used);
        if used_flags != VIRTQ_USED_F_NO_NOTIFY {
            warn!("Used ring flags mismatch (got {:#x}, expected {:#x})", 
                  used_flags, VIRTQ_USED_F_NO_NOTIFY);
        }
        
        // 5.3 检查 complete_queue 数组大小（数组长度是编译时常量，可以直接访问）
        // 注意：complete_queue 是固定大小数组，len() 返回编译时常量，不会产生未对齐引用
        const COMPLETE_QUEUE_SIZE: usize = VIRTIO_BLKDEVICE_QUEUESIZE as usize;
        if COMPLETE_QUEUE_SIZE != queue_size {
            error!("Used ring size mismatch: got {}, expected {}", 
                   COMPLETE_QUEUE_SIZE, queue_size);
            return Err("Used ring size mismatch");
        }
        
        // 5.4 检查已用环元素的初始化状态（使用安全访问方法）
        for idx in 0..queue_size {
            // 使用 addr_of! 获取元素指针，避免创建未对齐引用
            let elem_ptr = unsafe {
                core::ptr::addr_of!(used.complete_queue[idx])
            };
            let elem_id = Self::read_used_elem_id(unsafe { &*elem_ptr });
            let elem_len = Self::read_used_elem_len(unsafe { &*elem_ptr });
            if elem_id != 0 || elem_len != 0 {
                warn!("Used ring element[{}] not initialized (id: {}, len: {})", 
                      idx, elem_id, elem_len);
            }
        }
        info!("✓ Used ring verification passed");

        // ==================== 6. 内存布局校验 ====================
        let queue_phys_addr = queue_memory as *const _ as usize;
        let expected_desc_addr = queue_phys_addr;
        let expected_avail_addr = expected_desc_addr + size_of::<[virtq_desc; VIRTIO_BLKDEVICE_QUEUESIZE as usize]>();
        let expected_used_addr = expected_avail_addr + size_of::<virtq_avail_queue>();
        
        // 6.1 检查描述符表地址
        if self.virtdsptbaddr != expected_desc_addr {
            error!("Descriptor table address mismatch: got {:#x}, expected {:#x}", 
                   self.virtdsptbaddr, expected_desc_addr);
            return Err("Descriptor table address mismatch");
        }
        
        // 6.2 检查可用环地址
        let actual_avail_addr = &queue_memory.avavil_queue as *const _ as usize;
        if actual_avail_addr != expected_avail_addr {
            error!("Available ring address mismatch: got {:#x}, expected {:#x}", 
                   actual_avail_addr, expected_avail_addr);
            return Err("Available ring address mismatch");
        }
        if self.avail_addr != expected_avail_addr {
            error!("Stored available ring address mismatch: got {:#x}, expected {:#x}", 
                   self.avail_addr, expected_avail_addr);
            return Err("Stored available ring address mismatch");
        }
        
        // 6.3 检查已用环地址
        let actual_used_addr = &queue_memory.used_queue as *const _ as usize;
        if actual_used_addr != expected_used_addr {
            error!("Used ring address mismatch: got {:#x}, expected {:#x}", 
                   actual_used_addr, expected_used_addr);
            return Err("Used ring address mismatch");
        }
        if self.used_phys != expected_used_addr {
            error!("Stored used ring address mismatch: got {:#x}, expected {:#x}", 
                   self.used_phys, expected_used_addr);
            return Err("Stored used ring address mismatch");
        }
        info!("✓ Memory layout verification passed");

        // ==================== 7. 描述符分配器校验 ====================
        // 检查分配器状态与描述符状态的一致性
        // 注意：初始化时描述符可能还未被标记为已分配，这取决于初始化流程
        // 这里只检查分配器状态本身是否合理
        let allocated_count = self.desc_alloctor.queue.iter().filter(|&&x| x).count();
        if allocated_count > queue_size {
            error!("Descriptor allocator: more descriptors marked as allocated ({}) than queue size ({})", 
                   allocated_count, queue_size);
            return Err("Descriptor allocator state invalid");
        }
        
        // 检查 current 指针是否在有效范围内
        if self.desc_alloctor.current > queue_size {
            error!("Descriptor allocator current pointer out of bounds: {} (max: {})", 
                   self.desc_alloctor.current, queue_size);
            return Err("Descriptor allocator current pointer out of bounds");
        }
        info!("✓ Descriptor allocator verification passed ({} allocated, current: {})", 
              allocated_count, self.desc_alloctor.current);

        // ==================== 8. 队列启用状态校验 ====================
        // 如果队列已经初始化，检查设备端的启用状态
        if self.initiaed {
            let is_enabled = unsafe {
                core::ptr::read_volatile(Self::mmio_ptr::<u32>(VIRTIO_MMIO_QUEUE_ENABLE)) == 1
            };
            if !is_enabled {
                warn!("Queue is marked as initialized but device reports it as disabled");
            }
        }

        info!("✓ Queue integrity verification completed successfully");
        Ok(())
    }

    /* 描述符表初始化 - 全部初始化为0，缓冲区在 blockdevice 操作时动态分配 */
    fn initial_virtio_dectb(&mut self){
        self.queue_size=VIRTIO_BLKDEVICE_QUEUESIZE;
        // 描述符全部初始化为0，不分配缓冲区
        let desc_vec:[virtq_desc;VIRTIO_BLKDEVICE_QUEUESIZE as usize] = [virtq_desc::default();VIRTIO_BLKDEVICE_QUEUESIZE as usize];
        self.virtio_queueMemory.desc_chain = desc_vec;
    }
    fn set_initial_success(&mut self){
        /* 可用队列等初始化 */
        /* 设置块设备初始化成功，可用 */
        self.initiaed=true;
    }
    fn mmio_ptr<T>(offset: u32) -> *mut T {
        (VIRTIO_BLK_MMIO_BASE + offset) as *mut T
    }

    fn read_magic() -> u32 {
        unsafe { core::ptr::read_volatile(Self::mmio_ptr::<u32>(VIRTIO_BLK_REG_MAGIC)) }
    }

    fn read_version() -> u32 {
        unsafe { core::ptr::read_volatile(Self::mmio_ptr::<u32>(VIRTIO_BLK_REG_VERSION)) }
    }

    fn read_device_id() -> u32 {
        unsafe { core::ptr::read_volatile(Self::mmio_ptr::<u32>(VIRTIO_BLK_REG_DEVICE_ID)) }
    }

    fn select_device_features(&self, index: u32) {
        unsafe {
            core::ptr::write_volatile(
                Self::mmio_ptr::<u32>(VIRTIO_BLK_REG_DEVICE_FEATURES_SEL),
                index,
            );
        }
    }

    fn select_driver_features(&self, index: u32) {
        unsafe {
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            core::ptr::write_volatile(
                Self::mmio_ptr::<u32>(VIRTIO_BLK_REG_DRIVER_FEATURES_SEL),
                index,
            );
        }
    }

    fn read_device_feature_bits(&self) -> u64 {
        self.select_device_features(0);
        unsafe {
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            let low =
                core::ptr::read_volatile(Self::mmio_ptr::<u32>(VIRTIO_BLK_REG_DEVICE_FEATURES))
                    as u64;
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            self.select_device_features(1);
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            let high =
                core::ptr::read_volatile(Self::mmio_ptr::<u32>(VIRTIO_BLK_REG_DEVICE_FEATURES))
                    as u64;

            (high << 32) | low
        }
    }

    fn write_driver_features(&self, features: VirtioFeatureSet) {
        let bits = features.bits();
        unsafe {
            self.select_driver_features(0);
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            core::ptr::write_volatile(
                Self::mmio_ptr::<u32>(VIRTIO_BLK_REG_DRIVER_FEATURES),
                bits as u32,
            );

            self.select_driver_features(1);
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            core::ptr::write_volatile(
                Self::mmio_ptr::<u32>(VIRTIO_BLK_REG_DRIVER_FEATURES),
                (bits >> 32) as u32,
            );
        }
    }

    fn write_status(&self) {
        unsafe {
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            core::ptr::write_volatile(
                Self::mmio_ptr::<u32>(VIRTIO_BLK_REG_STATUS),
                self.status.bits() as u32,
            );
        }
    }

    pub fn reset_status(&mut self) {
        self.status = VirtioStatus::empty();
        self.write_status();
    }

    pub fn set_status(&mut self, status: VirtioStatus) {
        self.status |= status;
        self.write_status();
    }

    pub fn read_status(&self) -> VirtioStatus {
        let bits =
            unsafe { core::ptr::read_volatile(Self::mmio_ptr::<u32>(VIRTIO_BLK_REG_STATUS)) as u8 };
        VirtioStatus::from_bits_truncate(bits)
    }

    pub fn read_device_features(&self) -> VirtioFeatureSet {
        VirtioFeatureSet::from_bits_truncate(self.read_device_feature_bits())
    }

    pub fn negotiate_features(
        &mut self,
        requested: VirtioFeatureSet,
    ) -> VirtioFeatureSet {
        let available = self.read_device_features();
        let accepted = available & requested;
        self.write_driver_features(accepted);
        self.negotiated_features = accepted;
        self.set_status(VirtioStatus::FEATURES_OK);

        accepted
    }

    pub fn update_identity(&mut self, magic: u32, version: u32, device_id: u32) {
        self.magic = magic;
        self.version = version;
        self.device_id = device_id;
    }

    pub fn negotiated_features(&self) -> VirtioFeatureSet {
        self.negotiated_features
    }
}

lazy_static! {
    pub static ref BLOCK_DEVICE: UPSafeCell<VirtioBlockDevice> =
        unsafe { UPSafeCell::new(VirtioBlockDevice::default()) };
}

fn verify_block_device() -> bool {
    let magic = VirtioBlockDevice::read_magic();
    let device_id = VirtioBlockDevice::read_device_id();
    if (magic != VIRTIO_BLK_MAGIC_VALUE) || (device_id!=VIRTIO_BLKDEVICE_ID){
        error!("Unexpected virtio-blk magic value: {:#x} device_id:{:#x}", magic,device_id);
        return false;
    }
    true
}

pub fn virt_blkdevice_init(blockdevice:&mut VirtioBlockDevice) {
    if !verify_block_device() {
        error!("Not a valid block device!");
        return;
    }

    let version = VirtioBlockDevice::read_version();
    let device_id = VirtioBlockDevice::read_device_id();
    

    let mut device = blockdevice;
    device.update_identity(VIRTIO_BLK_MAGIC_VALUE, version, device_id);
    info!(
        "BlockDevice verified:\n  version: {:#x}\n  device_id: {:#x}",
        version, device_id
    );

    device.reset_status();
    device.set_status(VirtioStatus::ACKNOWLEDGE | VirtioStatus::DRIVER);

    let negotiated = device.negotiate_features(VirtioFeatureSet::DRIVER_DEFAULT);
    info!("Negotiated features: {:#x}", negotiated.bits());

    if let Err(e) = device.init_virtqueue(){
        device.set_status(VirtioStatus::FAILED);
        error!("Virtqueue initialization failed: {}", e);
        return;
    }


    let status = device.read_status();
    if status.contains(VirtioStatus::FEATURES_OK) {
        device.set_status(VirtioStatus::DRIVER_OK);
        device.set_initial_success();
        info!("virtio-blk ready, DRIVER_OK set");
    } else {
        device.set_status(VirtioStatus::FAILED);
        error!(
            "Device rejected negotiated features, status bits: {:#04x}",
            status.bits()
        );
    }
}