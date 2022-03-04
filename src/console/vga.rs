use core::cmp::min;
use core::fmt;
use core::ptr::{read_volatile, write_volatile};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
  Black = 0,
  Blue = 1,
  Green = 2,
  Cyan = 3,
  Red = 4,
  Magenta = 5,
  Brown = 6,
  LightGray = 7,
  DarkGray = 8,
  LightBlue = 9,
  LightGreen = 10,
  LightCyan = 11,
  LightRed = 12,
  Pink = 13,
  Yellow = 14,
  White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
  fn new(foreground: Color, background: Color) -> Self {
    Self((background as u8) << 4 | (foreground as u8))
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
  ascii_character: u8,
  color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
  chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT]
}

pub struct VgaWriter {
  row_position: usize,
  column_position: usize,
  color_code: ColorCode,
  buffer: &'static mut Buffer
}

impl VgaWriter {

  pub fn new() -> Self {
    VgaWriter {
      row_position: 0,
      column_position: 0,
      color_code: ColorCode::new(Color::Green, Color::Black),
      buffer: unsafe { &mut *(0xb8000 as *mut Buffer) }
    }
  }

  pub fn write_byte(&mut self, byte: u8) {
    match byte {
      b'\n' => self.new_line(),
      byte => {
        if self.column_position >= BUFFER_WIDTH {
          self.new_line();
        }

        let row = min(BUFFER_HEIGHT - 1, self.row_position);
        let col = self.column_position;

        let color_code = self.color_code;
        unsafe {
          write_volatile(&mut self.buffer.chars[row][col], ScreenChar {
            ascii_character: byte,
            color_code
          });
        }

        self.column_position += 1;
      }
    }
  }

  fn new_line(&mut self) {
    if min(BUFFER_HEIGHT - 1, self.row_position) == BUFFER_HEIGHT - 1 {
      for row in 1..BUFFER_HEIGHT {
        for col in 0..BUFFER_WIDTH {
          let character = unsafe {
            read_volatile(&self.buffer.chars[row][col])
          };
          unsafe {
            write_volatile(&mut self.buffer.chars[row - 1][col], character)
          }
        }
      }
    }
    self.row_position = min(BUFFER_HEIGHT - 1, self.row_position + 1);
    self.clear_row(min(BUFFER_HEIGHT - 1, self.row_position));
    self.column_position = 0;
  }

  fn clear_row(&mut self, row: usize) {
    let blank = ScreenChar {
      ascii_character: b' ',
      color_code: self.color_code
    };
    for col in 0..BUFFER_WIDTH {
      unsafe {
        write_volatile(&mut self.buffer.chars[row][col], blank)
      }
    }
  }

  pub fn write_string(&mut self, string: &str) {
    for byte in string.bytes() {
      match byte {
        0x20..=0x7e | b'\n' => self.write_byte(byte),
        _ => self.write_byte(0xfe)
      }
    }
  }

  pub fn clear_screen(&mut self) {
    for row in 0..BUFFER_HEIGHT {
      self.clear_row(row);
    }
  }

}

impl fmt::Write for VgaWriter {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    self.write_string(s);
    Ok(())
  }
}