use x86::dtables::{DescriptorTablePointer, lidt};
use x86::{Ring, segmentation};
use x86::segmentation::{SegmentSelector, SystemDescriptorTypes32};
use x86::segmentation::SystemDescriptorTypes32::InterruptGate32;

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct GateDescriptor {
  pointer_low: u16,
  gdt_selector: SegmentSelector,
  reserved: u8,
  options: GateDescriptorOptions,
  pointer_high: u16,
}

impl GateDescriptor {

  pub fn missing(gate_type: SystemDescriptorTypes32) -> Self {
    Self {
      pointer_low: 0,
      gdt_selector: SegmentSelector::new(0, Ring::Ring0),
      reserved: 0,
      options: GateDescriptorOptions::new(gate_type),
      pointer_high: 0
    }
  }

}

#[derive(Debug, Clone, Copy)]
pub struct GateDescriptorOptions {
  bits: u8
}

pub struct Idt([GateDescriptor; 256]);

impl Idt {

  pub fn new() -> Self {
    Idt([GateDescriptor::missing(InterruptGate32); 256])
  }

  pub fn set_handler(&mut self, entry: u8, handler: HandlerFunc) -> &mut GateDescriptorOptions {
    self.0[entry as usize] = GateDescriptor::new(InterruptGate32, segmentation::cs(), handler);
    &mut self.0[entry as usize].options
  }

  pub fn load(&self) {
    let idt_pointer = DescriptorTablePointer::new(self);

    unsafe {
      lidt(&idt_pointer);
    }
  }

}

pub type HandlerFunc = extern "x86-interrupt" fn(_: InterruptStackFrame);

#[derive(Debug)]
#[repr(C)]
pub struct InterruptStackFrame {
  pub instruction_pointer: u32,
  pub code_segment: u32,
  pub cpu_flags: u32,
  pub stack_pointer: u32,
  pub stack_segment: u32
}

impl GateDescriptor {

  fn new(gate_type: SystemDescriptorTypes32, gdt_selector: SegmentSelector, handler: HandlerFunc) -> Self {
    let ptr = handler as u32;
    Self {
      pointer_low: ptr as u16,
      gdt_selector,
      reserved: 0,
      options: GateDescriptorOptions::new(gate_type),
      pointer_high: (ptr >> 16) as u16,
    }
  }

}

impl GateDescriptorOptions {
  fn new(gate_type: SystemDescriptorTypes32) -> Self {
    let mut date_descriptor_options = Self{bits: 0b1000_0000};
    date_descriptor_options.set_present(true).set_gate_type(gate_type);
    date_descriptor_options
  }

  pub fn set_present(&mut self, present: bool) -> &mut Self {
    if present {
      self.bits |= 0x8F;
    } else {
      self.bits &= 0x7F;
    }
    self
  }

  pub fn set_privilege_level(&mut self, descriptor_privilege_level: u8) -> &mut Self {
    self.bits &= 0x9F;
    self.bits |= (descriptor_privilege_level & 0x02) << 4;
    self
  }

  pub fn set_gate_type(&mut self, gate_type: SystemDescriptorTypes32) -> &mut Self {
    self.bits &= 0xF0;
    self.bits |= gate_type as u8;
    self
  }
}

