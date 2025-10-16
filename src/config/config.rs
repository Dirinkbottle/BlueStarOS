unsafe extern "C"{
    pub fn _stack_top();
    pub fn _stack_bottom();
    pub fn _kernel_start();
    pub fn _kernel_end();
}


pub const PAGESIZE:usize=4096;
