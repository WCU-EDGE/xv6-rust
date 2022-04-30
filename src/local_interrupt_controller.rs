//! See chapter 10, "Advanced Programmable Interrupt Controller (APIC)" of the "Intel® 64 and IA-32 Architectures Software Developer’s Manual"
//! See also, https://wiki.osdev.org/APIC#Local_APIC_and_IO-APIC.

use traps::{IRQ_ERROR, IRQ_SPURIOUS, IRQ_TIMER, T_IRQ0};

pub static mut LOCAL_INTERRUPT_CONTROLLER: *mut u32 = 0 as *mut u32;

// Registers
// The offsets are divided by 4 because each register is 4 bytes allowing us to index into
// LOCAL_INTERRUPT_CONTROLLER like an array.
const ID: u32 = 0x0020 / 4;
const VERSION: u32 = 0x0030 / 4;
const SPURIOUS_INTERRUPT_VECTOR: u32 = 0x00F0 / 4;
const TIMER_INITIAL_COUNT: u32 = 0x0380 / 4;
const TIMER_DIVIDE_CONFIGURATION: u32 = 0x03E0 / 4;
const TIMER_LOCAL_VECTOR_TABLE: u32 = 0x0320 / 4;
const LOCAL_INTERRUPT_0_PIN: u32 = 0x0350 / 4;
const LOCAL_INTERRUPT_1_PIN: u32 = 0x0360 / 4;
const PERFORMANCE_MONITORING_COUNTERS_LOCAL_VECTOR_TABLE: u32 = 0x0340 / 4;
const ERROR_LOCAL_VECTOR_TABLE: u32 = 0x0370 / 4;
const ERROR_STATUS: u32 = 0x0280 / 4;
const END_OF_INTERRUPT: u32 = 0x00B0 / 4;
const INTERRUPT_COMMAND_HIGH: u32 = 0x0310 / 4;
const INTERRUPT_COMMAND_LOW: u32 = 0x0300 / 4;
const TASK_PRIORITY: u32 = 0x0080 / 4;

// Constants
const ENABLE: u32 = 0x00000100;
// Divide counts by 1.
const TIMER_DIVIDE_VALUE: u32 = 0x0000000B;
const PERIODIC: u32 = 0x00020000;
const MASKED: u32 = 0x00010000;
const DELIVERY_STATUS: u32 = 0x00001000;
// Send to all APICs, including self.
const BROADCAST: u32 = 0x00080000;
const INIT: u32 = 0x00000500;
const LEVEL_TRIGGERED: u32 = 0x00008000;

/// Initialize the local interrupt controller.
pub unsafe fn init() {
  if LOCAL_INTERRUPT_CONTROLLER.is_null() {
    println!("Local interrupt controller is undefined.");
    return;
  }

  // Enable local APIC; set spurious interrupt vector.
  write(SPURIOUS_INTERRUPT_VECTOR, ENABLE | (T_IRQ0 + IRQ_SPURIOUS));

  // The timer repeatedly counts down at bus frequency.
  // from LOCAL_INTERRUPT_CONTROLLER[TIMER_INITIAL_COUNT] and then issues an interrupt.
  // If xv6 cared more about precise timekeeping, TIMER_INITIAL_COUNT would be calibrated using an external time source.
  write(TIMER_DIVIDE_CONFIGURATION, TIMER_DIVIDE_VALUE);
  write(TIMER_LOCAL_VECTOR_TABLE, PERIODIC | (T_IRQ0 + IRQ_TIMER));
  write(TIMER_INITIAL_COUNT, 1000000);

  // Disable logical interrupt lines.
  write(LOCAL_INTERRUPT_0_PIN, MASKED);
  write(LOCAL_INTERRUPT_1_PIN, MASKED);

  // Disable performance counter overflow interrupts on machines that provide that interrupt entry.
  let version = read(VERSION);
  if ((version >> 16) & 0xFF) >= 4 {
    write(PERFORMANCE_MONITORING_COUNTERS_LOCAL_VECTOR_TABLE, MASKED);
  }

  // Map error interrupt to IRQ_ERROR.
  write(ERROR_LOCAL_VECTOR_TABLE, T_IRQ0 + IRQ_ERROR);

  // Clear error status register (requires back-to-back writes).
  write(ERROR_STATUS, 0);
  write(ERROR_STATUS, 0);

  // Ack any outstanding interrupts.
  write(END_OF_INTERRUPT, 0);

  // Send an Init Level De-Assert to synchronise arbitration ID's.
  write(INTERRUPT_COMMAND_HIGH, 0);
  write(INTERRUPT_COMMAND_LOW, BROADCAST | INIT | LEVEL_TRIGGERED);
  while read(INTERRUPT_COMMAND_LOW) & DELIVERY_STATUS != 0 {
  }

  // Enable interrupts on the APIC (but not on the processor).
  write(TASK_PRIORITY, 0);

  println!("local interrupt controller enabled.");
}

/// Write the local interrupt controller register with the provided value and wait for the write to finish.
unsafe fn write(register: u32, value: u32) {
  LOCAL_INTERRUPT_CONTROLLER.offset(register as isize).write_volatile(value);
  // Wait for write to finish, by reading
  LOCAL_INTERRUPT_CONTROLLER.offset(ID as isize).read_volatile();
}

/// Read the local interrupt controller register with the provided value.
unsafe fn read(register: u32) -> u32 {
  LOCAL_INTERRUPT_CONTROLLER.offset(register as isize).read_volatile()
}


pub fn get_id() -> u8 {
  unsafe {
    if LOCAL_INTERRUPT_CONTROLLER.is_null() {
      return 0;
    }

    (read(ID) >> 24) as u8
  }
}