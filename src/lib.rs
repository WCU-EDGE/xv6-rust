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
mod local_interrupt_controller;
mod acpi;

use core::arch::asm;
use core::panic::PanicInfo;
use x86::bits32::paging::{PAGE_SIZE_ENTRIES, PD, PDEntry};
use acpi::ACPI2;
use memory_layout::KERNEL_BASE;
use mmu::PAGE_DIRECTORY_INDEX_SHIFT;
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

    ACPI2.lock().populate_cpu_info();

    page_allocator::init();

    unsafe {
        kmalloc();
        //multi_processor::init();
        //ACPI2.lock().populate_cpu_info();
        //println!("MP configured.");

        // Todo: fix.
        local_interrupt_controller::init();

        // Todo: fix.
        interrupt_controller::init();
    }

    //user_init();

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[repr(align(4096))]
pub struct PD1([PDEntry; PAGE_SIZE_ENTRIES]);

const fn default_page_directory() -> PD1 {
    const PDE_P   : u32 = 0x001;   // Present
    const PDE_RW   : u32 = 0x002;   // Readable/Writeable
    const PDE_PS  : u32 = 0x080;   // Page Size
    const ADDRESS_MASK_PSE: u32 = !0x3fffff;

    let mut default_page_directory: PD1 = PD1([PDEntry(0); PAGE_SIZE_ENTRIES]);

    let mut i: u32 = 0;
    while i < 256 {
        default_page_directory.0[i as usize] = PDEntry(((i * 4194304) & ADDRESS_MASK_PSE) | (PDE_RW | PDE_P | PDE_PS));
        i += 1;
    }
    default_page_directory.0[KERNEL_BASE >> PAGE_DIRECTORY_INDEX_SHIFT] = PDEntry((0 & ADDRESS_MASK_PSE) | (PDE_RW | PDE_P | PDE_PS));

    default_page_directory
}

#[no_mangle]
pub static DEFAULT_PAGE_DIRECTORY : PD1 = default_page_directory();