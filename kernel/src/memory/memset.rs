use alloc::collections::btree_map::BTreeMap;
use bitflags::bitflags;
use alloc::vec::Vec;
use log::{debug, error, trace};
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
    pub frames:BTreeMap<VirNumber,FramTracker>,//Maparea 持有的物理页
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
        if page_table.is_maped(vpn){return;}//如果映射过了就跳过,防止多个一个vpn对应多个ppn，但是只有最后的ppn有效
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

    ///通过虚拟页号释放一个页帧
    pub fn unmap_one(&mut self,table:&mut PageTable,vpn:VirNumber){
        if self.frames.contains_key(&vpn){
            self.frames.remove(&vpn.clone()).expect("Remove a exist vpn failed!!");//回收页帧
            table.unmap(vpn);
        }else{
            error!("MapArea try Unmap vpn:{} but not find vpn in this area",vpn.0);
        }
    }

    ///释放maparea所有页帧
    pub fn unmap_all(){

    }
    
    
}
impl MapSet {

    ///这个map_one函数会在第0个self.area里面随便分配一个帧然后映射
    pub fn map_one(){

    }

    fn new_bare()->Self{
        MapSet{
            table:PageTable::new(),
            areas:Vec::new(),
        }
    }

    pub fn map_traper(&mut self){
        let trape:usize=straper as usize;//陷阱起始物理地址
        self.table.map(VirAddr(TRAP_BOTTOM_ADDR).into(), PhysiAddr(straper as usize).into(), PTEFlags::X | PTEFlags::R);
        self.table.map(VirAddr(HIGNADDRESS_MASK | kernel_trap_stack_bottom as usize).into(), PhysiAddr(kernel_trap_stack_bottom as usize).into(), PTEFlags::W | PTEFlags::R);//映射内核陷阱栈
    }


    ///取消映射一个页面，通过该地址空间唯一的vpn查找
    pub fn umap_one(&mut self,vpn:VirNumber){
        self.areas.iter_mut().for_each(|area| {
            if area.frames.contains_key(&vpn) {//找到那个area持有唯一的vpn
                 //调用内部页表取消映射
                area.unmap_one(&mut self.table, vpn);
            }
        });
    }

    ///取消映射一个区间的vpn页面
    pub fn unmap_range(){
        //暂时用不到
    }

    pub fn add_area(&mut self,range:VirNumRange,map_type :MapType,flags:MapAreaFlags){
        let mut area=MapArea::new(range, flags, map_type);
        area.map_all(&mut self.table);//映射area
        self.areas.push(area);
    } 

    pub fn new_kernel()->Self{
        let mut mem_set =MapSet::new_bare();

        //映射陷阱
        mem_set.map_traper();

        //映射代码段
        let text_start_vpn = VirNumber(VirAddr(stext as usize).floor_down().0 / PAGE_SIZE);
        let text_end_vpn = VirNumber( VirAddr(etext as usize + PAGE_SIZE).floor_up().0 / PAGE_SIZE);
        mem_set.add_area(VirNumRange(text_start_vpn, text_end_vpn), MapType::Indentical, MapAreaFlags::R | MapAreaFlags::X);
        //trace!("{} {}\n",text_start_vpn.0,text_end_vpn.0);


        //映射rodata段
        let rodata_start_vpn = VirNumber(VirAddr(srodata as usize).floor_down().0 / PAGE_SIZE);
        let rodata_end_vpn = VirNumber(( VirAddr(erodata as usize + PAGE_SIZE).floor_up().0 / PAGE_SIZE));
        mem_set.add_area(VirNumRange(rodata_start_vpn, rodata_end_vpn), MapType::Indentical, MapAreaFlags::R);
        //trace!("{} {}\n",rodata_start_vpn.0,rodata_end_vpn.0);

    
        // 映射内核数据段
        let data_start = VirNumber(VirAddr(sdata as usize).floor_down().0 / PAGE_SIZE);
        let data_end = VirNumber(VirAddr(edata as usize).floor_up().0 / PAGE_SIZE);
        mem_set.add_area(
            VirNumRange(data_start, data_end),
            MapType::Indentical,
            MapAreaFlags::R | MapAreaFlags::W,
        );
       // trace!("{} {}\n",data_start.0,data_end.0);

        //映射bss段
        let bss_start = VirNumber(VirAddr(sbss as usize).floor_down().0 / PAGE_SIZE);
        let bss_end = VirNumber(VirAddr(ebss as usize).floor_up().0 / PAGE_SIZE);
        mem_set.add_area(
            VirNumRange(bss_start, bss_end),
            MapType::Indentical,
            MapAreaFlags::R | MapAreaFlags::W,
        );
       // trace!("{} {}\n",bss_start.0,bss_end.0);
        
        // 映射物理内存
        let phys_start = VirNumber(VirAddr(ekernel as usize).floor_down().0 / PAGE_SIZE);
        let phys_end = VirNumber(VirAddr(ekernel as usize+ MEMORY_SIZE).floor_up().0 / PAGE_SIZE);
        mem_set.add_area(
            VirNumRange(phys_start, phys_end),
            MapType::Indentical,
            MapAreaFlags::R | MapAreaFlags::W,
        );
       // trace!("{} {}\n",phys_start.0,phys_end.0);

        //内核地址空间映射完成
        let vdr:VirAddr=phys_end.into();
        debug!("Kernle AddressSet Total Memory:{} MB",(vdr.0 -skernel as usize) /1024/1024);
        
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