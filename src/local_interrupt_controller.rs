use memory_layout::map_physical_virtual;
use traps::{IRQ_ERROR, IRQ_SPURIOUS, IRQ_TIMER, T_IRQ0};

pub static mut LOCAL_INTERRUPT_CONTROLLER: *mut u32 = 0 as *mut u32;

// Spurious Interrupt Vector
const ID: u32 = 0x0020;
const VERSION: u32 = 0x0030;
const SVR: u32 = 0x00F0;
const ENABLE: u32 = 0x00000100;
const TDCR: u32 = 0x03E0;
const X1: u32 = 0x0000000B;
const TIMER: u32 = 0x0320;
const PERIODIC: u32 = 0x00020000;
const TICR: u32 = 0x0380;
const LINT0: u32 = 0x0350;
const LINT1: u32 = 0x0360;
const MASKED: u32 = 0x00010000;
const PCINT: u32 = 0x0340;
const ERROR: u32 = 0x0370;
const ESR: u32 = 0x0280;
const EOI: u32 = 0x00B0;
const ICRHI: u32 = 0x0310;
const ICRLO: u32 = 0x0300;
const DELIVS: u32 = 0x00001000;
const BCAST: u32 = 0x00080000;
const TPR: u32 = 0x0080;
const INIT: u32 = 0x00000500;
const LEVEL: u32 = 0x00008000;

pub unsafe fn init() {
/*  if LOCAL_INTERRUPT_CONTROLLER.is_null() {
    println!("Local interrupt controller is undefined.");
    return;
  }*/
  //LOCAL_INTERRUPT_CONTROLLER = map_physical_virtual(LOCAL_INTERRUPT_CONTROLLER as usize) as *mut u32;
  // Enable local APIC; set spurious interrupt vector.

  write(SVR, ENABLE | (T_IRQ0 + IRQ_SPURIOUS));

  // The timer repeatedly counts down at bus frequency
  // from lapic[TICR] and then issues an interrupt.
  // If xv6 cared more about precise timekeeping,
  // TICR would be calibrated using an external time source.
  write(TDCR, X1);
  write(TIMER, PERIODIC | (T_IRQ0 + IRQ_TIMER));
  write(TICR, 1000000);

  // Disable logical interrupt lines.
  write(LINT0, MASKED);
  write(LINT1, MASKED);

  // Disable performance counter overflow interrupts
  // on machines that provide that interrupt entry.
  // TODO: Fix offsets.
  let version = LOCAL_INTERRUPT_CONTROLLER.offset(VERSION as isize).read_volatile();
  if ((version >> 16) & 0xFF) >= 4 {
    write(PCINT, MASKED);
  }

  // Map error interrupt to IRQ_ERROR.
  write(ERROR, T_IRQ0 + IRQ_ERROR);

  // Clear error status register (requires back-to-back writes).
  write(ESR, 0);
  write(ESR, 0);

  // Ack any outstanding interrupts.
  write(EOI, 0);

  // Send an Init Level De-Assert to synchronise arbitration ID's.
  write(ICRHI, 0);
  write(ICRLO, BCAST | INIT | LEVEL);
  while LOCAL_INTERRUPT_CONTROLLER.offset(ICRLO as isize).read_volatile() & DELIVS != 0 {
  }

  // Enable interrupts on the APIC (but not on the processor).
  write(TPR, 0);

  println!("local interrupt controller enabled.");
}

unsafe fn write(register: u32, value: u32) {
  (((LOCAL_INTERRUPT_CONTROLLER as usize).overflowing_add(register as usize).0) as *mut u32).write_volatile(value);
  // Wait for write to finish, by reading
  (((LOCAL_INTERRUPT_CONTROLLER as usize).overflowing_add(ID as usize).0) as *mut u32).read_volatile();
}