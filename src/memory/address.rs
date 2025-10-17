use core::ops::Add;

use bitflags::bitflags;
use log::{debug, error, warn};
use riscv::addr;
use crate::{config::*, memory::frame_allocator::*};
use alloc::vec::Vec;
use alloc::vec;
#[derive(Debug,Clone,Copy,PartialEq, Eq, PartialOrd, Ord)]
pub struct VirNumber(pub usize);
#[derive(Debug,Clone,Copy)]
pub struct PhysiNumber(pub usize);
#[derive(Debug,Clone,Copy)]
pub struct VirAddr(pub usize);
#[derive(Debug,Clone,Copy)]
pub struct PhysiAddr(pub usize);
#[derive(Debug,Clone,Copy)]
#[repr(C)]
pub struct PageTableEntry(pub usize);
pub struct PageTable{
    pub root_ppn:PhysiNumber,
    entries:Vec<FramTracker>,
}
bitflags! {
    pub struct PTEFlags: usize {
        /// Valid
        const V = 1 << 0;
        /// Readable
        const R = 1 << 1;
        /// Writable
        const W = 1 << 2;
        /// eXecutable
        const X = 1 << 3;
        /// User
        const U = 1 << 4;
        /// Global
        const G = 1 << 5;
        /// Accessed
        const A = 1 << 6;
        /// Dirty
        const D = 1 << 7;
    }
}


impl From<PhysiNumber> for PhysiAddr {
    fn from(value: PhysiNumber) -> Self {
        PhysiAddr(value.0<<PAGE_SIZE_BITS)
    }
}
impl From<VirNumber> for VirAddr {
    fn from(value: VirNumber) -> Self {
        VirAddr(value.0<<PAGE_SIZE_BITS)
    }
}
impl From<PhysiAddr> for PhysiNumber {
    fn from(value: PhysiAddr) -> Self {
        PhysiNumber(value.0>>PAGE_SIZE_BITS)
    }
}
impl From<VirAddr> for VirNumber {
    fn from(value: VirAddr) -> Self {
        VirNumber(value.0>>PAGE_SIZE_BITS)
    }
}



impl VirNumber {
 pub fn index(&self) -> [usize; 3] {
        let  vpn = self.0;
        let mut idx: [usize; 3] = [0; 3];
        // SV39: VPN[2] (最高位) -> VPN[1] -> VPN[0] (最低位)
        idx[0] = (vpn >> 18) & 0x1FF;  // 27-18位
        idx[1] = (vpn >> 9) & 0x1FF;   // 18-9位  
        idx[2] = vpn & 0x1FF;          // 8-0位
        idx
    }
}

impl VirAddr {
    pub fn floor_up(&self)->Self{
        let addr=self.0;
        if addr%PAGE_SIZE==0{
            VirAddr(addr)
        }else{
            VirAddr((addr/PAGE_SIZE+1)*PAGE_SIZE)
        }
    }
    pub fn floor_down(&self)->Self{
        let addr=self.0;
        VirAddr((addr/PAGE_SIZE)*PAGE_SIZE)
    }
    pub fn offset(&self)->usize{
        self.0 & 4095
    }
}



impl PageTableEntry {
    pub fn new(ppn:usize,flags:PTEFlags)->Self{
        PageTableEntry( (ppn<<10) | flags.bits()) // 页表项不持有frametracer
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits_truncate(self.0 & 255) //目前跳过2位的rsw保留位
    }
    pub fn ppn(&self)->PhysiNumber{
         PhysiNumber((self.0 >> 10) & ((1 << 44) - 1))
    }
    pub fn is_valid(&self)->bool{
        self.flags().contains(PTEFlags::V)
    }
}

impl PageTable {
    pub fn new()->Self{
        let mut root_frame=alloc_frame().expect("failed to alloc frame for page table");
        PageTable{
            root_ppn:PhysiNumber(root_frame.ppn.0),
            entries:vec![root_frame], //把根页面挂下面 正确，获取所有权
        }
    }

    pub fn translate(&mut self,VDDR:VirAddr)->Option<PhysiAddr>{
        match self.find_pte_vpn(VDDR.into()){
            Some(pte)=>{
                let ppn=pte.ppn();
                let addr=(ppn.0*PAGE_SIZE)+VDDR.offset();
                Some(PhysiAddr(addr))
            }
            None=>{
                None
            }
        }
    }

    pub fn get_pte_array(&self,phynum:usize)->&'static mut [PageTableEntry]{
        let phyaddr:PhysiAddr=PhysiNumber(phynum).into();
        unsafe{core::slice::from_raw_parts_mut(phyaddr.0 as  *mut PageTableEntry, 512)}
    }

    ///查找但是不创建新页表项
    fn find_pte_vpn(&mut self,VirNum:VirNumber)->Option<&mut PageTableEntry>{
        let mut current_ppn=self.root_ppn.0;
        let mut idx=VirNum.index();
        let mut pte_array=self.get_pte_array(current_ppn);
        for (id,index) in idx.iter().enumerate(){

            let entry=&mut pte_array[*index];
                        
                 if id==2{//最后一级
                    return Some(entry);
                }
            if !entry.is_valid(){
                return  None;//不合法
            }
            current_ppn=entry.ppn().0;
            pte_array=self.get_pte_array(current_ppn);
        }
        None
    }

    pub fn map(&mut self,vpn:VirNumber,ppn:PhysiNumber,flags:PTEFlags){//map是需要传入对应vpn和ppn的
        let pte=self.find_or_create_pte_vpn(vpn).expect("Failed When Map");

        if pte.is_valid(){
            //说明之前已经存在对应的映射了,给个警告级别的提示，因为可能有重叠的
            warn!("MAP error！vpn:{}has maped before, pte exist ppn:{}",vpn.0,pte.ppn().0)
        }
        *pte=PageTableEntry::new(ppn.0,flags|PTEFlags::V); //否则创建映射
    }

    fn find_or_create_pte_vpn(&mut self,VirNum:VirNumber)->Option<&mut PageTableEntry>{
        let mut current_ppn=self.root_ppn.0;
        let mut idx=VirNum.index();
        let mut pte_array=self.get_pte_array(current_ppn);
        for (id,index) in idx.iter().enumerate(){

            let entry=&mut pte_array[*index];
                        
                 if id==2{//最后一级
                    return Some(entry);
                }
            if !entry.is_valid(){
                //不存在页表，开始创建页表
                let frame=alloc_frame().expect("Frame alloc failed on pte alloc");
                let ppn =frame.ppn.0;
                *entry=PageTableEntry::new(ppn, PTEFlags::V);
                self.entries.push(frame);
            }
            current_ppn=entry.ppn().0;
            pte_array=self.get_pte_array(current_ppn);
        }
        None
    }

        ///获取适用于satp的token
    pub fn satp_token(&self)->usize{
          debug!("token: root ppn:{}", self.root_ppn.0);
          // MODE (8 for Sv39) | ASID (0) | PPN
          (8 << 60) | (self.root_ppn.0)
    }

    
}