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
}
pub const KERNEL_HEAP_SIZE:usize=1024*1024*2;//内核堆大小1MB
pub static mut KERNEL_HEADP:[u8;KERNEL_HEAP_SIZE]=[0;KERNEL_HEAP_SIZE];//内核堆实例
pub const  PAGE_SIZE:usize=4096;//每个页面大小4kb
pub const  PAGE_SIZE_BITS:usize=12;//2^12=4096 4kb
pub const MEMORY_SIZE:usize=1024*1024*16;//总可用空闲物理内存大小16MB
