use core::fmt::{self,Write};
use core::cell::{SyncUnsafeCell, UnsafeCell};
/// UART 地址 (QEMU virt 机器)
const URAT_ADDR:*mut u8=0x1000_0000 as *mut u8;
pub static  GLOBAL_UART:SyncUnsafeCell<Option<UART>>=SyncUnsafeCell::new(None);
pub struct UART{}
pub struct Stdout;
impl UART {
    pub fn init_uart(){
        unsafe {
            //测试串口是否存在
            core::ptr::write_volatile(URAT_ADDR,'\n' as u8);
            if *URAT_ADDR != '\n' as u8{
                return;
            }
        }
        unsafe {
            GLOBAL_UART.get().write(Some(UART {  }));
        }
        
    }
    pub fn putc(&self,ca:u8)->bool{
        unsafe{
                core::ptr::write_volatile(URAT_ADDR, ca);
                return true;

        } 
        false
    }
    pub fn write_usize_hex(num:usize){
        let hex="0123456789ABCDEF".as_bytes();
        for i in (0..(core::mem::size_of::<usize>()*2)).rev(){
            let v=(num>>(i*4))&0xf;
            unsafe {
                core::ptr::write_volatile(URAT_ADDR, hex[v] );
            }
        }
        unsafe {
                core::ptr::write_volatile(URAT_ADDR, b'\n' );
        }
    }
}
impl fmt::Write for UART {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let byte=s.as_bytes();
        for i in byte{
            self.putc(*i);
        }
        
            Ok(())
    }
}

impl Stdout {
    pub fn putc(&self,ca:u8){
        unsafe{
                core::ptr::write_volatile(URAT_ADDR, ca);
        } 
    }
}

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let byte = s.as_bytes();
        for i in byte {
            self.putc(*i);
        }

        Ok(())
    }
}



#[macro_export]
macro_rules! print {
    ($($li:tt)*)=>{
        unsafe {
            use core::fmt::Write;
            use crate::GLOBAL_UART;
           if let Some(uart)=&mut *$crate::GLOBAL_UART.get(){
                uart.write_fmt(format_args!($($li)*)).unwrap();
           }
        }   
    };
}
#[macro_export]
macro_rules! print_hex {
    ($num:expr) => {
        unsafe {
            use crate::UART;
           if let Some(uart)=&mut *$crate::GLOBAL_UART.get(){
                UART::write_usize_hex($num);
           }
        }   
    };
    () => {
        
    };
}
#[macro_export]
macro_rules! println {
    ()=>{
        print!("\n");
    };
    ($li:literal)=>{
        print!($li);
        print!("\n");
    }
}










