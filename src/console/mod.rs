extern crate spin;
use spin::Mutex;
use core::fmt;

mod uart;
mod vga;

use self::vga::VgaWriter;
use self::uart::UartWriter;

const BACKSPACE: i32 = 0x100;
const BACKSCHAR: u8 = b'\x08';

lazy_static! {
    static ref LOCK: Mutex<i32> = Mutex::new(0);
    pub static ref UART_CONSOLE: spin::Mutex<UartWriter> = spin::Mutex::new(UartWriter::new());
    pub static ref VGA_CONSOLE: spin::Mutex<VgaWriter> = spin::Mutex::new(VgaWriter::new());
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::print(format_args!($($arg)*)));
    ($($arg:tt)*) => ($crate::vga::print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ({
        $crate::console::print(format_args_nl!($($arg)*));
    })
}

pub fn clear_screen() {
  VGA_CONSOLE.lock().clear_screen();
  UART_CONSOLE.lock().clear_screen();
}

pub fn print(args: fmt::Arguments) {
  use core::fmt::Write;
  LOCK.lock();
  UART_CONSOLE.lock().write_fmt(args).unwrap();
  VGA_CONSOLE.lock().write_fmt(args).unwrap();
}

pub fn console_interrupt(c: fn() -> i32) {}