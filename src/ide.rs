use x86::io::{inb, outb};
use acpi::MAX_CPUS;
use interrupt_controller;
use traps::IRQ_IDE;

pub const SECTOR_SIZE: u16 = 512;
pub const IDE_BUSY: u8 = 0x80;
pub const IDE_DRIVE_READY: u8 = 0x40;
pub const IDE_DRIVE_WRITE_FAULT: u8 = 0x20;
pub const IDE_ERROR: u8 = 0x01;

static mut HAS_DISK_1: bool = false;

pub fn init() {
  let i: usize;

  unsafe {
    interrupt_controller::enable(IRQ_IDE, (MAX_CPUS - 1) as u32);
  }
  wait(false);

  unsafe {
    outb(0x1f6u16, 0xe0 | (1u8 << 4));

    for i in 0..1000 {
      if inb(0x1f7) != 0 {
        HAS_DISK_1 = true;
        break;
      }
    }

    // Switch back to disk 0.
    outb(0x1f6, 0xe0 | (0 << 4));
  }
}

fn wait(check_error: bool) -> bool {
  let mut result: u8;
  unsafe {
    loop {
      result = inb(0x01f7u16);
      if result & (IDE_BUSY | IDE_DRIVE_READY) != IDE_DRIVE_READY {
        break;
      }
    }

    if check_error && (result & (IDE_DRIVE_WRITE_FAULT | IDE_ERROR)) != 0 {
      return true;
    }
    false
  }

}