use crate::{task::TaskContext, trap::TrapContext};
extern "C"{
        pub fn kernel_stack_lower_bound();
        pub fn kernel_stack_top();
        pub fn kernel_trap_stack_top();
        pub fn kernel_trap_stack_bottom();
        pub fn ekernel();
        pub fn skernel();
        pub fn stext();
        pub fn etext();
        pub fn srodata();
        pub fn erodata();
        pub fn sdata();
        pub fn edata();
        pub fn sbss();
        pub fn ebss();
        ///内核陷阱地址
        pub fn __kernel_trap();
        ///内核陷阱恢复地址
        pub fn __kernel_refume();
        ///内核陷阱的物理起始地址
        pub fn straper();
        ///用户程序专用陷阱物理起始地址
        pub fn utraper();
        pub fn app_start();//测试应用地址
        pub fn app_end();//测试应用地址
        pub fn __switch(need_swapout:*const TaskContext,need_swapin:*const TaskContext);//任务切换汇编函数
        ///内核的trap独立运行栈 栈顶
        pub fn kernel_trap_run_stack_top();
        ///内核的trap独立运行栈 栈底
        pub fn kernel_trap_run_stack_bottom();
        ///应用列表起始地址
        pub fn app_list_start();
        ///应用列表结束地址
        pub fn app_list_end();
}
///MB的简单封装
pub const  MB:usize=1024*1024;
pub const  PAGE_SIZE:usize=4096;//每个页面大小4kb
pub const KERNEL_HEAP_SIZE:usize=1*MB;//内核堆大小
pub const KERNEL_STACK_SIZE:usize=PAGE_SIZE*4;//应用内核栈有四个页面的大小
pub static mut KERNEL_HEADP:[u8;KERNEL_HEAP_SIZE]=[0;KERNEL_HEAP_SIZE];//内核堆实例
pub const  PAGE_SIZE_BITS:usize=12;//2^12=4096 4kb
pub const MEMORY_SIZE:usize=40*MB;//总可用空闲物理内存大小100个页
pub const CPU_CIRCLE:usize=12_500_000;
///使用虚拟高地址并且刚好留够一个页面,代表开始的第一个地址
pub const TRAP_BOTTOM_ADDR:usize=usize::MAX-PAGE_SIZE+1;
///每个app的trap context (高地址)
pub const TRAP_CONTEXT_ADDR:usize=TRAP_BOTTOM_ADDR-PAGE_SIZE;
///用户start函数在用户地址空间的起始映射地址，不携带页帧，直接操作页表映射 D
pub const USERLIB_START_RETURN_HIGNADDR:usize=TRAP_CONTEXT_ADDR-PAGE_SIZE;
pub const HIGNADDRESS_MASK:usize=0xFFFFFFE000000000;//0xFFFFFFFFFFFFF000 hb *0xfffffffffffff070
///每秒多少次时钟中断
pub const TIME_FREQUENT:usize=100;


///任务初始ticket(优先级)
pub const TASK_TICKET:usize=100;
///初始大数
pub const BIG_INT:usize=1_000_000;

use lazy_static::lazy_static;
use crate::{MapSet, sync::UPSafeCell};
lazy_static!{
        pub static ref KERNEL_SPACE:UPSafeCell<MapSet> =unsafe {
            UPSafeCell::new( MapSet::new_kernel())//内核地址空间，必须持有,从来不会丢弃
        };
}

/// 获取应用程序数量
pub fn get_app_num() -> usize {
    unsafe {
        let app_list_start_addr = app_list_start as usize;
        let app_list_end_addr = app_list_end as usize;
        // 每个应用占用2个usize（start和end地址）
        (app_list_end_addr - app_list_start_addr) / (core::mem::size_of::<usize>() * 2)
    }
}

/// 获取第 app_id 个应用的数据切片（app_id 从 0 开始）
pub fn get_app_data(app_id: usize) -> &'static [u8] {
    let app_num = get_app_num();
    if app_id >= app_num {
        panic!("Application id {} out of range! Total apps: {}", app_id, app_num);
    }
    
    unsafe {
        let app_list_start_addr = app_list_start as usize;
        // 应用列表是一个 usize 数组，存储每个应用的起始和结束地址
        let app_list = core::slice::from_raw_parts(
            app_list_start_addr as *const usize,
            app_num * 2
        );
        
        let app_start_addr = app_list[app_id * 2];
        let app_end_addr = app_list[app_id * 2 + 1];
        let app_size = app_end_addr - app_start_addr;
        
        core::slice::from_raw_parts(app_start_addr as *const u8, app_size)
    }
}
