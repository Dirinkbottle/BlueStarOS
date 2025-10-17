use buddy_system_allocator::LockedHeap;
use log::trace;
use crate::{config::{KERNEL_HEADP, KERNEL_HEAP_SIZE, PAGE_SIZE}, memory::address::*,sync::UPSafeCell};
use core::cell::UnsafeCell;
use lazy_static::lazy_static;
#[global_allocator]
pub static ALLOCATOR:LockedHeap=LockedHeap::empty();
use alloc::vec::Vec;

pub fn allocator_init(){
    unsafe{
        ALLOCATOR.lock().init(KERNEL_HEADP.as_ptr() as usize,KERNEL_HEAP_SIZE);
    }
    trace!("Kernel HeapAlloctor init, can use size:{}MB , mount on KERNEL_HEADP",KERNEL_HEAP_SIZE/(1024*1024));
}


pub struct FrameAlloctor{
    start:usize,
    end:usize,
    recycle:Vec<usize>
}

trait FrameAllocatorTrait{
    fn new()->Self;
    fn alloc(&mut self)->Option<FramTracker>;
    fn dealloc(&mut self,ppn:usize);
}
impl FrameAllocatorTrait for FrameAlloctor{
    fn new()->Self {
        FrameAlloctor{
            start:0,
            end:0,
            recycle:Vec::new()
        }
    }
    fn alloc(&mut self)->Option<FramTracker>{
        if let Some(ppn)=self.recycle.pop(){
            trace!("recycle frame:ppn:{}",ppn);
            Some(FramTracker::new(PhysiNumber(ppn)))
        }else if self.start<self.end{
            let ppn=self.start;
            self.start+=1;
            trace!("new frame:ppn:{}",ppn);
            Some(FramTracker::new(PhysiNumber(ppn)))
        }else{
            panic!("no more frame!");
        }
    }

    fn dealloc(&mut self,ppn:usize) {
        //页号合法性检查
        if ppn>= self.start || ppn>self.end || self.recycle.contains(&ppn){
            panic!("frame ppn:{} is not valid!",ppn);
        }
    }
}

impl FrameAlloctor {
    pub fn init(&mut self,start:usize,end:usize){
        self.start=VirAddr(start).floor_up().0/PAGE_SIZE;
        self.end=VirAddr(end).floor_down().0/PAGE_SIZE;
        self.recycle=Vec::new(); 
        trace!("frame allocator init: start ppn:{} end ppn:{} size:{}MB",self.start,self.end,(end-start)/1024/1024);
    }
}


#[derive(Debug,Clone)]
pub struct FramTracker{
    pub ppn:PhysiNumber
}
impl FramTracker{
    fn new(ppn:PhysiNumber)->Self{
        FramTracker{
            ppn
        }
    }
}
lazy_static!{
    pub static ref FRAME_ALLOCATOR:UPSafeCell<FrameAlloctor>= 
    unsafe {
        UPSafeCell::new(FrameAlloctor::new())
    };
}
pub fn init_frame_allocator(start:usize,end:usize){
    FRAME_ALLOCATOR.lock().init(start,end);
}
pub fn alloc_frame()->Option<FramTracker>{
    FRAME_ALLOCATOR.lock().alloc()
}

pub fn dealloc_frame(ppn:usize){
    FRAME_ALLOCATOR.lock().dealloc(ppn);
}

impl Drop for FramTracker {
    fn drop(&mut self) {
        dealloc_frame(self.ppn.0);
        trace!("free frame:ppn:{}",self.ppn.0);
    }
}
