use bitflags::bitflags;
use crate::{config::PAGE_SIZE, memory::frame_allocator::*};
use alloc::vec::Vec;
pub struct VirNumber(pub usize);
pub struct PhysiNumber(pub usize);
pub struct VirAddr(pub usize);
pub struct PhysiAddr(pub usize);
pub struct PageTableEntry(pub usize);
pub struct PageTable{
    root_ppn:PhysiNumber,
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
impl Clone for PhysiNumber {
    fn clone(&self) -> Self {
        PhysiNumber(self.0)
    }
}
impl Copy for PhysiNumber {}

impl Clone for VirNumber {
    fn clone(&self) -> Self {
        VirNumber(self.0)
    }
}
impl Copy for VirNumber {}




impl VirNumber {
    pub fn index(&self)-> [usize;3] {//返回三级索引 SV39规范
        let mut vpn=self.0;
        let mut idx:[usize;3]=[0;3];
        for i in (0..3).rev(){
            idx[i]=vpn&511;//不是512
            vpn>>=9;
        }
        idx
    }
}

impl PhysiNumber {
    pub fn get_pte_array(&self)->&'static [PageTableEntry;512]{//根据根ppn返回页表数组
        let pte_array_ptr=(self.0*PAGE_SIZE) as usize as *const [PageTableEntry;512];
        unsafe{&*pte_array_ptr}
    }
}

impl PageTableEntry {
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits_truncate(self.0 & 255)
    }
    pub fn index(&self) -> [PhysiNumber; 3] {
        let mut ppn=(self.0 << 10) >> 20;//去掉低10位
        let mut ppn_array:[PhysiNumber;3]=[PhysiNumber(0);3];
        for i in (0..3).rev(){
            ppn_array[i]=PhysiNumber(ppn&511);
            ppn>>=9;
        }
        ppn_array
    }
}

impl PageTable {
    fn find_pte_vpn(&self,VirNum:VirNumber)->Option<&PageTableEntry>{
        let vpn=VirNum.0;
        let idx:[usize;3]=VirNum.index();
        let mut pte_array=self.root_ppn.get_pte_array();
        for (id,index) in idx.iter().enumerate(){
            if id==2{
                return Some(&pte_array[*index]);//最后一级
            }
            let pte=&pte_array[*index];
            if !pte.flags().contains(PTEFlags::V){//页表是否合法
                return None;
            }
            let next_ppn=pte.index();
            pte_array=next_ppn[id].get_pte_array();
        }
        None
    }
}