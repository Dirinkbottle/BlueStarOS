use alloc::collections::btree_map::BTreeMap;
use bitflags::bitflags;
use alloc::vec::Vec;
use log::{debug, };
use core::arch::asm;
    use riscv::register::satp;

use crate::{config::*, memory::{address::*, alloc_frame, frame_allocator::FramTracker}};
pub struct VirNumRange(pub VirNumber,pub VirNumber);//开始和结束，一个范围
bitflags! {//MapAreaFlags 和 PTEFlags 起始全为0
    pub struct MapAreaFlags: usize {
        ///Readable
        const R = 1 << 1;
        ///Writable
        const W = 1 << 2;
        ///Excutable
        const X = 1 << 3;
        ///Accessible in U mode
        const U = 1 << 4;  //这里是maparea的标志 不要和页表标志搞混淆
    }
}
impl From<MapAreaFlags> for PTEFlags {
    fn from(value: MapAreaFlags) -> Self {
        match PTEFlags::from_bits(value.bits()){
            Some(pteflags)=>{pteflags}
            None=>{panic!("MapAreaFlags translate to PTEFlags Failed!")}
        }
    }
}
enum MapType {
    Indentical,
    Maped
}
pub struct MapArea{
    range:VirNumRange,//虚拟页号范围
    flags:MapAreaFlags,//访问标志   
    frames:BTreeMap<VirNumber,FramTracker>,//Maparea 持有的物理页
    map_type:MapType,
}
pub struct MapSet{
    table:PageTable,
    areas:Vec<MapArea>,
}
impl MapArea {
    pub fn new(range:VirNumRange,flags:MapAreaFlags,map_type:MapType)->Self{
        MapArea{
            range,
            flags,
            frames:BTreeMap::new(),
            map_type,
        }
    }
    pub fn map_one(&mut self,vpn:VirNumber,page_table:&mut PageTable){//带自动分配物理页帧的
        //可能是恒等和普通映射
        let ppn:PhysiNumber;
        match self.map_type{
            MapType::Indentical=>{
                ppn =PhysiNumber(vpn.0) //内核特权高大上，恒等映射 内核映射所有物理帧，但是不能占用和分配对应Framtracer，需要构建一个特殊页表
            }
            MapType::Maped=>{
               let frame= alloc_frame().expect("Memory Alloc Failed By map_one");
                ppn=frame.ppn;
                self.frames.insert(vpn,frame ); //管理最终pte对应的frametracer，分工明确 巧妙！！！！
                  debug!("Dymical frame: VPN {} -> PPN {}", vpn.0, ppn.0);
            }
        };
        page_table.map(vpn, ppn, self.flags.into());
        //debug!("Map Aread map vpn:{} -> ppn:{}",vpn.0,ppn.0);
    }

    pub fn map_all(&mut self,page_table:&mut PageTable){
        let start=self.range.0;
        let end=self.range.1;
        let mut current=start;
        while current.0<=end.0 {
            self.map_one(current, page_table);
            current.0+=1;
        }

    }
    
}
impl MapSet {
    fn new_bare()->Self{
        MapSet{
            table:PageTable::new(),
            areas:Vec::new(),
        }
    }

    pub fn add_area(&mut self,range:VirNumRange,map_type :MapType,flags:MapAreaFlags){
        let mut area=MapArea::new(range, flags, map_type);
        area.map_all(&mut self.table);//映射area
        self.areas.push(area);
    } 

    pub fn new_kernel()->Self{
        let mut mem_set =MapSet::new_bare();

        //映射代码段
        let text_start_vpn = VirNumber(VirAddr(stext as usize).floor_down().0 / PAGE_SIZE);
        let text_end_vpn = VirNumber( VirAddr(etext as usize + PAGE_SIZE).floor_up().0 / PAGE_SIZE);
        mem_set.add_area(VirNumRange(text_start_vpn, text_end_vpn), MapType::Indentical, MapAreaFlags::R | MapAreaFlags::X);
    
        //映射rodata段
        let rodata_start_vpn = VirNumber(VirAddr(srodata as usize).floor_down().0 / PAGE_SIZE);
        let rodata_end_vpn = VirNumber(( VirAddr(erodata as usize + PAGE_SIZE).floor_up().0 / PAGE_SIZE));
        mem_set.add_area(VirNumRange(rodata_start_vpn, rodata_end_vpn), MapType::Indentical, MapAreaFlags::R);
    
        // 映射内核数据段
        let data_start = VirNumber(VirAddr(sdata as usize).floor_down().0 / PAGE_SIZE);
        let data_end = VirNumber(VirAddr(edata as usize).floor_up().0 / PAGE_SIZE);
        mem_set.add_area(
            VirNumRange(data_start, data_end),
            MapType::Indentical,
            MapAreaFlags::R | MapAreaFlags::W,
        );

        //映射bss段
        let bss_start = VirNumber(VirAddr(sbss as usize).floor_down().0 / PAGE_SIZE);
        let bss_end = VirNumber(VirAddr(ebss as usize).floor_up().0 / PAGE_SIZE);
        mem_set.add_area(
            VirNumRange(bss_start, bss_end),
            MapType::Indentical,
            MapAreaFlags::R | MapAreaFlags::W,
        );
        
        // 映射物理内存
        let phys_start = VirNumber(VirAddr(ekernel as usize).floor_down().0 / PAGE_SIZE);
        let phys_end = VirNumber(VirAddr(ekernel as usize+ MEMORY_SIZE).floor_up().0 / PAGE_SIZE);
        mem_set.add_area(
            VirNumRange(phys_start, phys_end),
            MapType::Indentical,
            MapAreaFlags::R | MapAreaFlags::W,
        );

        //内核地址空间映射完成
        let vdr:VirAddr=phys_end.into();
        debug!("Kernle AddressSet Total Memory:{} MB",vdr.0/1024/1024);
        
        mem_set

    
    }
    
    pub fn translate_test(&mut self){
        self.areas.iter().for_each(|maparea|{
            (maparea.range.0.0..=maparea.range.1.0).for_each(|vpn| {
                let vdr:VirAddr=VirNumber(vpn).into();
                let addr=self.table.translate(vdr);
                debug!("Translate Test vddr:{:#x} ->Phyaddr:{:#x}",vdr.0,addr.unwrap().0)
            });
        } );
    }

    /// Change page table by writing satp CSR Register.
    pub fn activate(&self) {
         let satps = self.table.satp_token();
        debug!("Active PageTable: SATP = {:#x}", satps);
        unsafe {
            satp::write(satps);
            asm!("sfence.vma");
            debug!("Page Witch Successful!!!!!");
        }
    }


}