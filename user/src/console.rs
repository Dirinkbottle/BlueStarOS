use core::fmt::{self, Write};
use spin::mutex::Mutex;

use crate::sys_write;

const FD_TYPE_STDIN: usize = 1;
const FD_TYPE_STDOUT: usize = 2;
const BUFFER_SIZE: usize = 256 * 10;

struct STDIN;
struct STDOUT;

// 使用静态数组代替 VecDeque，避免在 lazy_static 初始化时进行堆分配
struct STDBUFFER {
    buffer: [u8; BUFFER_SIZE],
    write_pos: usize,
}

impl STDBUFFER {
    const fn new() -> Self {
        STDBUFFER {
            buffer: [0u8; BUFFER_SIZE],
            write_pos: 0,
        }
    }

    fn flush(&mut self) -> isize {
        if self.write_pos == 0 {
            return 0;
        }
        let result = sys_write(
            FD_TYPE_STDOUT,
            self.buffer.as_ptr() as usize,
            self.write_pos,
        );
        if result >= 0 {
            self.write_pos = 0;
        }
        result
    }
}

// 使用 static 和 Mutex，避免 lazy_static 的初始化死锁
static STD_BUFFER: Mutex<STDBUFFER> = Mutex::new(STDBUFFER::new());


impl Write for STDBUFFER {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for &byte in s.as_bytes() {
            // 如果缓冲区满了，先 flush
            if self.write_pos >= BUFFER_SIZE {
                if self.flush() == -1 {
                    return Err(core::fmt::Error);
                }
            }
            
            self.buffer[self.write_pos] = byte;
            self.write_pos += 1;
            
            // 遇到换行符就 flush
            if byte == b'\n' {
                if self.flush() == -1 {
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