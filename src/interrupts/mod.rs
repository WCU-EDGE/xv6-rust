use interrupts::idt::InterruptStackFrame;

mod idt;

lazy_static! {
  static ref IDT: idt::Idt = {
    let mut idt = idt::Idt::new();
    idt.set_handler(0, divide_by_zero_handler);
    idt
  };
}

extern "x86-interrupt" fn divide_by_zero_handler(exception_stack_frame: InterruptStackFrame) {
  println!("Division by zero exception!");
  println!("Interrupt Stack Frame {:#?}", exception_stack_frame);

  unsafe {
    loop{}
  }

}

pub fn init() {
  IDT.load();
}