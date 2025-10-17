
use log::{debug, error, trace};
use riscv::register::{sstatus::Sstatus, stval, stvec, utvec::TrapMode};

#[repr(C)]
pub struct TrapContext{
    pub x:[usize;32],
    pub sstatus:Sstatus,
    pub spec:usize
}

pub fn set_kernel_trap_handler(){
    unsafe {
        stvec::write(kernel_trap_handler as usize, TrapMode::Direct);
    }
}

#[no_mangle]
pub extern "C" fn trap_handler(){
    debug!("Traper")
}


#[no_mangle]
pub extern "C" fn kernel_trap_handler()->!{
    use riscv::register::sepc;
    error!("stval ={:#x},spec={:#x} ",stval::read(),sepc::read());
    loop {
        
    }
    panic!("Kernel Traped.... ")
}

