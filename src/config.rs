extern "C"{
    pub fn kernel_stack_lower_bound();
        pub fn kernel_stack_top();
        pub fn ekernel();
        pub fn skernel();
}

pub const KERNEL_HEAP_SIZE:usize=1024*1024*16;//16MB
pub static mut KERNEL_HEADP:[u8;KERNEL_HEAP_SIZE]=[0;KERNEL_HEAP_SIZE];
pub const  PAGE_SIZE:usize=4096;//4kB