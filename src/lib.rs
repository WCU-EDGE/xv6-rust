#![no_std]

#![feature(abi_x86_interrupt)]
#![feature(format_args_nl)]
#![feature(const_mut_refs)]

#[macro_use]
extern crate bitfield;
#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate x86;

pub mod arch;
#[macro_use]
pub mod console;
pub mod file;
pub mod fs;
pub mod ioapic;
pub mod interrupt_controller;
pub mod kbd;
pub mod string;
pub mod syscall;
pub mod sysproc;
pub mod mmu;
pub mod param;
pub mod pipe;
pub mod process;
pub mod trap;
pub mod traps;
pub mod types;

pub mod page_allocator;
pub mod interrupts;
mod memory_layout;
mod virtual_memory;
mod ide;
mod multi_processor;
mod local_interrupt_controller;

use core::arch::asm;
use core::panic::PanicInfo;
use process::user_init;
use virtual_memory::kmalloc;

/// The entry point into the xv6 rust kernel.
/// Called from main.c.
/// End is where the kernel ends, which c gets from the linker.
#[no_mangle]
pub extern "C" fn rust_main() {
    interrupts::init();

    console::clear_screen();
    println!("Welcome to Rust xV6!");
    page_allocator::init();

    unsafe {
        kmalloc();
        multi_processor::init();

        // Todo: fix.
        // interrupt_controller::init();
    }

    user_init();

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
