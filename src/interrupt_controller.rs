// The I/O APIC manages hardware interrupts for an SMP system.
// https://pdos.csail.mit.edu/6.828/2018/readings/ia32/ioapic.pdf
// See also picirq.c.

use local_interrupt_controller::LOCAL_INTERRUPT_CONTROLLER;
use multi_processor::INTERRUPT_CONTROLLER_ID;
use traps::T_IRQ0;

// Default physical address of IO APIC
pub const IOAPIC: usize = 0xFEC00000;

// Register index: ID
pub const ID_REGISTER: u32 = 0x00;
// Register index: version
pub const VERSION_REGISTER: u32 = 0x01;
// Redirection table base
pub const TABLE_REGISTER: u32 = 0x10;

// The redirection table starts at REG_TABLE and uses
// two registers to configure each interrupt.
// The first (low) register in a pair contains configuration bits.
// The second (high) register contains a bitmask telling which
// CPUs can serve that interrupt.

// Interrupt disabled
pub const INTERRUPT_DISABLED: u32 = 0x00010000;
// Level-triggered (vs edge-)
pub const INTERRUPT_LEVEL: u32 = 0x00008000;
// Active low (vs high)
pub const INT_ACTIVE_LOW: u32 = 0x00002000;
// Destination is CPU id (vs APIC ID)
pub const INT_LOGICAL: i32 = 0x00000800;

// InputOutputAdvancedInterruptController


const INTERRUPT_CONTROLLER_REGISTER: *mut u32 = 0xFE_C0_00_00 as *mut u32;
const INTERRUPT_CONTROLLER_DATA: *mut u32 = 0xFE_C0_00_10 as *mut u32;

unsafe fn write(register: u32, data: u32) {
  INTERRUPT_CONTROLLER_REGISTER.write_volatile(register);
  INTERRUPT_CONTROLLER_DATA.write_volatile(data);
}

unsafe fn read(register: u32) -> u32 {
  INTERRUPT_CONTROLLER_REGISTER.write_volatile(register);
  INTERRUPT_CONTROLLER_DATA.read_volatile()
}

pub unsafe fn init() {
  let max_interrupt = (read(VERSION_REGISTER) >> 16) & 0xFF;
  let id = read(ID_REGISTER) >> 24;
  if id != INTERRUPT_CONTROLLER_ID as u32 { // look here
    println!("interrupt_controller::init: id isn't equal to INTERRUPT_CONTROLLER_REGISTER; not a MP");
  }

  // Mark all interrupts edge-triggered, active high, disabled,
  // and not routed to any CPUs.
  for i in 0..=max_interrupt {
    write(TABLE_REGISTER + 2 * i, INTERRUPT_DISABLED | (T_IRQ0 + i));
    write(TABLE_REGISTER + 2 * i + 1, 0);
  }
}

pub unsafe fn enable(irq: u32, cpu_number: u32) {
  // Mark interrupt edge-triggered, active high,
  // enabled, and routed to the given cpu_number,
  // which happens to be that cpu's APIC ID.
  write(TABLE_REGISTER+2*irq, T_IRQ0 + irq);
  write(TABLE_REGISTER+2*irq+1, cpu_number << 24);
}