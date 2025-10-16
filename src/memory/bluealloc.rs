use core::{alloc::{ GlobalAlloc, Layout}, cell::UnsafeCell, ptr,cell::SyncUnsafeCell};
use crate::{ uart,print};
pub struct stack_allocer{
    inner:UnsafeCell<Inner>
}

struct Inner{
    start:usize,//可用内存起始地址
    end:usize,//可用内存结束地址
    stack_ptr:usize,//指向未分配区域
}

impl stack_allocer {
    pub fn init(start:usize,end:usize){
        unsafe {
            STACK_ALLOCER.get().write(Some(stack_allocer { inner: UnsafeCell::new(Inner { start: start, end: end, stack_ptr: start }) }));           
            print!("STACK_ALLOCER INIT SUCCESSFUL!\n");
        }
    }

    pub fn mem_alloc(&self,layout:Layout)->Option<*mut u8>{
        let inner=unsafe {
            &mut *self.inner.get()
        };
        let size=layout.size();//分配大小
        let align=layout.align();
        let need_align=(align-(inner.stack_ptr)%align)%align; //还需要多少才对齐
        //判断是否超出可用范围
        if inner.stack_ptr+size+need_align>inner.end{
            print!("Memory out of range!");
            return None;//内存不足
        }
        let align_addr=inner.stack_ptr+size+need_align;//保证内存对齐
        let new_ptr:usize=inner.stack_ptr+need_align;//对齐后的栈指针
        inner.stack_ptr=new_ptr+size;//更新分配数据
        return Some(new_ptr as *mut u8);//应该返回分配后的起始地址
    }
    pub fn mem_deadalloc(&self,ptr:*mut u8,layout:Layout){
        let inner=unsafe {
            &mut *self.inner.get()
        };
        let orign_ptr=ptr as usize;
        let size=layout.size();//分配大小

        if orign_ptr+size!=inner.stack_ptr{//内存显然不对其
            print!("Memory not align!");
            return;
        }
        inner.stack_ptr-=size;
    }
}


unsafe impl GlobalAlloc for stack_allocer {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self.mem_alloc(layout){
            Some(ptr)=>{
                ptr
            }
            None=>{
                ptr::null_mut()
            }
        }
       
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.mem_deadalloc(ptr, layout);
    }
}

unsafe impl Sync for stack_allocer {}
    

pub static STACK_ALLOCER:SyncUnsafeCell<Option<stack_allocer>>=SyncUnsafeCell::new(None);
