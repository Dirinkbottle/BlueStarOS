use core::panic::PanicInfo;

use crate::print;





#[panic_handler]
pub fn panic(info:&PanicInfo)->!{
    let panic_location = info.location();
    if let Some(location) = panic_location{
        print!("USER APPLICATION panic on file:{} line:{} message:{} \n",location.file(),location.line(),info.message().unwrap())
    }else{
        print!("USER APPLICATION panic Message:{}",info.message().unwrap())
    }
    loop {
        
    }
}