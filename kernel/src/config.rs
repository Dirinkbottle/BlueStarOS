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
        pub fn __kernel_trap();//内核陷阱地址
        pub fn __kernel_refume();//内核陷阱恢复地址
        pub fn straper();//内核陷阱的物理起始地址
        pub fn app_start();//测试应用地址
        pub fn app_end();//测试应用地址
}
pub const  MB:usize=1024*1024;
pub const  PAGE_SIZE:usize=4096;//每个页面大小4kb
pub const KERNEL_HEAP_SIZE:usize=1*MB;//内核堆大小
pub static mut KERNEL_HEADP:[u8;KERNEL_HEAP_SIZE]=[0;KERNEL_HEAP_SIZE];//内核堆实例
pub const  PAGE_SIZE_BITS:usize=12;//2^12=4096 4kb
pub const MEMORY_SIZE:usize=10*MB;//总可用空闲物理内存大小100个页
pub const CPU_CIRCLE:usize=12_500_000;
//使用虚拟高地址并且刚好留够一个页面
pub const TRAP_BOTTOM_ADDR:usize=usize::MAX-PAGE_SIZE+1;
pub const HIGNADDRESS_MASK:usize=0xFFFFFFE000000000;


///每秒多少次时钟中断
pub const TIME_FREQUENT:usize=100;