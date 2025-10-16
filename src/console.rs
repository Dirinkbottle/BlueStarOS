use core::fmt::{self, Write};
use crate::sbi::putc;
struct Stdout;


impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for cha in s.chars() {
            putc(cha as usize);
        }
        Ok(())
    }
}


pub fn print(fmt:fmt::Arguments){
    Stdout.write_fmt(fmt).unwrap()
}


///print string
#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?))
    }
}

/// Println! to the host console using the format string and arguments.
#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?))
    }
}