use alloc::collections::btree_map::BTreeMap;
use bitflags::bitflags;
use alloc::vec::Vec;
use log::{debug, error, trace};
use riscv::paging::PTE;
use core::arch::asm;
    use riscv::register::satp;
    use crate::task::file_loader;

use crate::{config::*, memory::{address::*, alloc_frame, frame_allocator::FramTracker}};
use crate::trap::no_return_start;
use crate::trap::TrapFunction;
///开始和结束，一个范围,自动[start,end] start地址自动向下取整，end也向下取整，因为virnumrange用于代码映射，防止代码缺失, startva/PAGE =num+offset ,从num开始，endva/pagesize=endva+offset由于闭区间所以向下取整,防止多映射
pub struct VirNumRange(pub VirNumber,pub VirNumber);
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

impl VirNumRange {
    ///VirNumRange初始化 传入起始地址和结束地址,闭区间都需要映射 [start,end] start地址自动向下取整，end也向下取整
    pub fn new(start:VirAddr,end:VirAddr)->Self{
        let start_vpn=start.floor_down();
        let end_vpn=end.floor_down();
        VirNumRange(start_vpn, end_vpn)//闭区间，都需要映射
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
#[derive(PartialEq,Clone, Copy)]
pub enum MapType {
    Indentical,
    Maped
}
pub struct MapArea{
    ///虚拟页号范围,闭区间
    range:VirNumRange,
    flags:MapAreaFlags,//访问标志   
    pub frames:BTreeMap<VirNumber,FramTracker>,//Maparea 持有的物理页
    map_type:MapType,
}
pub struct MapSet{
    ///页表
    pub table:PageTable,
    areas:Vec<MapArea>,
}
impl MapArea {
    ///range,闭区间
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
            }
        };
        page_table.map(vpn, ppn, self.flags.into());
        //debug!("Map Aread map vpn:{} -> ppn:{}",vpn.0,ppn.0);
    }

    ///映射分割和挂载MapArea所有段,闭区间全部映射
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


    ///复制MAPED映射的数据到物理页帧,maped方式才调用它(不包含判断)  必须按照elf格式的顺序复制,传入的data需要自行截断，有栈等映射不需要复制数据
    pub fn copy_data(&mut self,data:Option<&[u8]>,table:&mut PageTable){
        let mut start: usize = 0;
        let mut current_vpn = self.range.0;
        if let None =data{return;}//不需要复制数据的话就返回了
        let len = data.expect("No data").len();
        loop {
            let src = &data.expect("No data")[start..len.min(start + PAGE_SIZE)];
            let dst =&mut table.get_mut_byte(current_vpn).expect("Cant get mut slice")[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }
    
    
}
impl MapSet {

    ///获取当前memset的table临时借用
    pub fn get_table(&mut self)->&mut PageTable{
        &mut self.table    
    }

    ///TODO:
    ///从elf解析数据创建应用地址空间 Mapset entry user_stack,kernel_sp心智空间有限，姑且这样吧,外部库不值得研究，内核全部完成后自己实现xmas.映射trap，trapcontext，userstack，kernelstack，userheap。
    /// appid从1开始
    pub fn from_elf(appid:usize,elf_data:&[u8])->(Self,usize,VirAddr,usize){ 
        let mut memory_set = Self::new_bare();
        // map program headers of elf, with U flag
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirNumber(0);//为elf结尾所在段+1
        let entry_point = elf.header.pt2.entry_point();
        debug!("ELF entry point: {:#x}, program headers: {}", entry_point, ph_count);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirAddr = VirAddr(ph.virtual_addr() as usize);
                let end_va: VirAddr = VirAddr((ph.virtual_addr() + ph.mem_size()) as usize);
                let mut map_perm = MapAreaFlags::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapAreaFlags::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapAreaFlags::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapAreaFlags::X;
                }
                
                debug!("  [{}] Mapping segment: [{:#x}, {:#x}), perm: {:?}", 
                       i, start_va.0, end_va.0, map_perm);
                
                max_end_vpn=end_va.floor_up();
                memory_set.add_area(VirNumRange::new(start_va, end_va), MapType::Maped, map_perm,Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]) );
            }
        }
        
        //程序地址空间创建完成，接下来是
        // 映射陷阱
        memory_set.map_traper();
        //映射上下文
        memory_set.map_trapContext();
        //映射普通用户栈
        let userstack_start_vpn=VirNumber(max_end_vpn.0+1);//留guradpage
        let userstack_end_vpn=VirNumber(userstack_start_vpn.0+1);
        let user_sp:VirAddr=VirAddr(userstack_end_vpn.0*PAGE_SIZE + PAGE_SIZE);//因为结尾不包含，属于下一个页面
        debug!("  Mapping user stack: vpn={:#x}, sp={:#x}", userstack_start_vpn.0, user_sp.0);
        memory_set.add_area(VirNumRange(userstack_start_vpn,userstack_end_vpn), MapType::Maped, MapAreaFlags::W | MapAreaFlags::R | MapAreaFlags::U, None);
        //映射用户堆
        let userheap_start_end_vpn = VirNumber(userstack_start_vpn.0+1);//无需guardpage，堆不会向下溢出
        debug!("  Mapping user heap: vpn={:#x}", userheap_start_end_vpn.0);
        memory_set.add_area(VirNumRange(userheap_start_end_vpn, userheap_start_end_vpn), MapType::Maped, MapAreaFlags::R | MapAreaFlags::W | MapAreaFlags::U, None);
        //映射内核栈
        //debug!("Kernel stack start viadr:{:#x} appid:{}",TRAP_BOTTOM_ADDR-(PAGE_SIZE+PAGE_SIZE)*appid,appid);
        let strat_kernel_vpn =VirAddr(TRAP_BOTTOM_ADDR-(PAGE_SIZE+KERNEL_STACK_SIZE)*appid).strict_into_virnum();//隔了一个guardpage 
        let end_kernel_vpn=VirAddr(TRAP_BOTTOM_ADDR-((PAGE_SIZE+KERNEL_STACK_SIZE)*appid)+KERNEL_STACK_SIZE-PAGE_SIZE).strict_into_virnum();
        let kernel_stack_top =TRAP_BOTTOM_ADDR-((PAGE_SIZE+KERNEL_STACK_SIZE)*appid)+KERNEL_STACK_SIZE;//保命
        //debug!("Kernel stack vpn:{}",strat_adn_end_vpn.0);
    
        KERNEL_SPACE.lock().add_area(VirNumRange(strat_kernel_vpn, end_kernel_vpn), MapType::Maped, MapAreaFlags::R | MapAreaFlags::W, None);
        (
            memory_set,
            entry_point as usize,
            user_sp,
            kernel_stack_top
        )
    }


    ///这个map_one函数会在第0个self.area里面随便分配一个帧然后映射
    pub fn map_one(){

    }

    fn new_bare()->Self{
        MapSet{
            table:PageTable::new(),
            areas:Vec::new(),
        }
    }

    ///在目前的地址空间页表里面映射陷阱
    pub fn map_traper(&mut self,){
        let kernel_trape:usize=straper as usize;//内核陷阱起始物理地址
        self.table.map(VirAddr(TRAP_BOTTOM_ADDR).into(), PhysiAddr(kernel_trape as usize).into(), PTEFlags::X | PTEFlags::R);

       
    }

    ///映射陷阱上下文
    pub fn map_trapContext(&mut self){
        let trapcontext_addr:VirAddr = VirAddr(TRAP_CONTEXT_ADDR);
        self.add_area(VirNumRange(trapcontext_addr.strict_into_virnum(), trapcontext_addr.strict_into_virnum()), MapType::Maped, MapAreaFlags::R | MapAreaFlags::W, None);
    }

    ///目前不可用
    ///映射特殊用户库没返回的情况，可以直接切换任务或者panic，保证内核稳定,目前就在TrapContext后面巴，如果后续报错，则需要特殊处理。！！！！！！！！！！！！！！！！！！！！！！
    ///只映射了处理函数一个页，可能不够 目前不能用
   // pub fn map_user_start_return(&mut self){
     //   let userlib_start_retunr:usize=USERLIB_START_RETURN_HIGNADDR;
       // let map_vnumber=VirAddr(userlib_start_retunr).strict_into_virnum();//严格对齐
        //let start_return_phyaddr =PhysiAddr(no_return_start as usize).floor_down();
        //self.table.map(map_vnumber, start_return_phyaddr, PTEFlags::U | PTEFlags::X | PTEFlags::R);//用户唯一可以访问的高地址

   // }


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

    ///输入range，maptype和flags 自动处理maparea的映射和物理帧挂载以及对应memset的pagetable映射,处理数据的复制映射   但是映射用户栈不需要数据
    pub fn add_area(&mut self,range:VirNumRange,map_type :MapType,flags:MapAreaFlags,data:Option<&[u8]>){
        let mut area=MapArea::new(range, flags, map_type);
        area.map_all(&mut self.table);//映射area
        if let MapType::Maped = map_type{//maped方式要复制数据
            area.copy_data(data, &mut self.table);
        }
        self.areas.push(area);
    } 

    pub fn new_kernel()->Self{
        let mut mem_set =MapSet::new_bare();

        //映射陷阱
        mem_set.map_traper();

        //映射代码段
        let text_range = VirNumRange::new(VirAddr(stext as usize), VirAddr(etext as usize));//range封装过
        mem_set.add_area(text_range, MapType::Indentical,  MapAreaFlags::R | MapAreaFlags::X  ,None);


        //映射rodata段
        let rodata_range = VirNumRange::new(VirAddr(srodata as usize), VirAddr(erodata as usize));//range封装过
        mem_set.add_area(rodata_range, MapType::Indentical, MapAreaFlags::R,None);
        //trace!("{} {}\n",rodata_start_vpn.0,rodata_end_vpn.0);

    
        // 映射内核数据段
        let data_range = VirNumRange::new(VirAddr(sdata as usize), VirAddr(edata as usize));//range封装过
        mem_set.add_area(data_range, MapType::Indentical, MapAreaFlags::R | MapAreaFlags::W,None);
       // trace!("{} {}\n",data_start.0,data_end.0);

        //映射bss段
        let bss_range = VirNumRange::new(VirAddr(sbss as usize), VirAddr(ebss as usize));//range封装过
        mem_set.add_area(bss_range, MapType::Indentical, MapAreaFlags::R | MapAreaFlags::W,None);
       // trace!("{} {}\n",bss_start.0,bss_end.0);
        
        // 映射物理内存(必须手动构造range区间)，phystart需要向上取整,end需要手动-1 range
        let phys_start =VirAddr(ekernel as usize).floor_up();
        let phys_end =VirAddr(ekernel as usize + MEMORY_SIZE-PAGE_SIZE).floor_down(); //ekernel 为结束地址 end需要手动-1 range
        let phys_range = VirNumRange(phys_start,phys_end);
        mem_set.add_area(phys_range, MapType::Indentical, MapAreaFlags::W | MapAreaFlags::R, None);
       // trace!("{} {}\n",phys_start.0,phys_end.0);

        //内核地址空间映射完成
        let vdr:VirAddr=phys_end.into();
        debug!("Kernle AddressSet Total Memory:{} MB,Kernel Size:{}MB",(vdr.0 -skernel as usize)/MB,(ekernel as usize -skernel as usize)/MB);
        
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