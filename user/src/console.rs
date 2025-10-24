use core::fmt::{self, Write};
use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use spin::mutex::Mutex;
use lazy_static::lazy_static;
use crate::{sys_write};

const FD_TYPE_STDIN: usize = 1;
const FD_TYPE_STDOUT: usize = 2;
const BUFFER_SIZE: usize = 256 * 10;

struct STDIN;
struct STDOUT;

// 使用静态数组代替 VecDeque，避免在 lazy_static 初始化时进行堆分配
struct STDBUFFER(VecDeque<u8>);
// 使用 static 和 Mutex，避免 lazy_static 的初始化死锁
lazy_static!{
 static ref STD_BUFFER: Arc<Mutex<STDBUFFER>> =Arc::new(Mutex::new(STDBUFFER(VecDeque::new())));

}
impl STDBUFFER {
    fn flush(&mut self) -> isize {
        let buffer = self.0.make_contiguous();
        let resultcode =sys_write(FD_TYPE_STDOUT, buffer.as_ptr() as usize, buffer.len());
        self.0.clear();
        resultcode as isize
    }
}




impl Write for STDBUFFER {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.as_bytes().iter() {
            self.0.push_back(*byte);
            if self.0.len() == BUFFER_SIZE || *byte==b'\n'{
                if self.flush()==-1{
                    return Err(core::fmt::Error);
                }
            }
        }
        Ok(())
    }
}

pub fn print(fmt: fmt::Arguments) {
    // 不使用 expect()，避免在 panic handler 中再次获取锁导致死锁
    let _ = STD_BUFFER.lock().write_fmt(fmt);
}

#[macro_export]
macro_rules! print {
    ($lit:literal $(,$($arg:tt)+)?) => {
        crate::print(format_args!($lit $(,$($arg)+)?))
    };
}


#[macro_export]
macro_rules! println {
    ($lit:literal $(,$($arg:tt)+)?) => {
        crate::print(format_args!(concat!($lit,'\n') $(,$($arg)+)?))
    };
}