use core::borrow::{Borrow, BorrowMut};
use file::{File, Inode};
use types::Pde;
use ::{panic, param};
use mmu;
use page_allocator;

use core::{ffi, mem};
use core::arch::asm;
use core::ptr::null_mut;
use x86::bits32::paging::PD;
use arch::TrapFrame;
use console::print;
use mmu::{PAGE_SIZE, SegDesc, TaskState};
use page_allocator::FREE_PAGE_LIST;
use virtual_memory::{setup_kernel_virtual_memory};

static mut PROCESS_ID: usize = 0;


unsafe fn trapret() {
    const x: usize = 0x8;
    asm!("popal",
         "pop gs",
         "pop fs",
         "pop es",
         "pop ds",
         "add {x}, esp",
         "iret",
    x = in(reg) x);
}

lazy_static! {
    pub static ref PROCESS_TABLE : spin::Mutex<[Option<Process>; 24]> = spin::Mutex::new([Option::None; 24]);
}

#[derive(Copy, Clone)]
pub struct Cpu {
    pub(crate) apicid: u8,
    scheduler: *const Context,
    ts: mmu::TaskState,
    gdt: [mmu::SegDesc<u32>; param::NSEGS],
    started: bool,
    ncli: i32,
    intena: i32,
    proc: *const Process,
}

impl Cpu {
    pub const fn new() -> Cpu {
        Cpu {
            apicid: 0,
            scheduler: 0 as *const Context,
            ts: TaskState::new(),
            gdt: [SegDesc(0u32); param::NSEGS],
            started: false,
            ncli: 0,
            intena: 0,
            proc: 0 as *const Process
        }
    }
}

#[derive(Copy, Clone)]
pub struct Context {
    edi: u32,
    esi: u32,
    ebx: u32,
    ebp: u32,
    eip: u32,
}

#[derive(Copy, Clone)]
enum ProcessState {
    UNUSED,
    EMBRYO,
    SLEEPING,
    RUNNABLE,
    RUNNING,
    ZOMBIE
}

/// Allocates memory for a process and adds it to the process_table.
///
/// Returns a process id if successful.
fn alloc_process() -> Option<usize> {
    let mut pt = PROCESS_TABLE.lock();

    let mut process = Option::None;
    for process_entry in &mut *pt {
        match process_entry {
            None => {
                process = Some(process_entry);
                break;
            }
            _ => {
                panic!("Failed to allocate process memory.");
            }
        }
    }

    let mut kernel_stack;

    match process {
        Some(process_entry) => {
            let mut stack_pointer: usize;
            unsafe {
                PROCESS_ID += 1;

                kernel_stack = FREE_PAGE_LIST.alloc_page().expect("Failed to get memory for process.");
            }
            // PAGE_SIZE should probably be KERNEL_STACK_SIZE in the future
            stack_pointer = kernel_stack + PAGE_SIZE;

            stack_pointer -= mem::size_of::<*mut TrapFrame>();
            let trap_frame_pointer: *mut TrapFrame = stack_pointer as *mut TrapFrame;

            stack_pointer -= mem::size_of::<usize>();
            unsafe {
                *(stack_pointer as *mut usize) = trapret as usize; // trapret
            }

            stack_pointer -= mem::size_of::<Context>();
            let context_pointer: *mut Context = stack_pointer as *mut Context;

            process_entry.replace(Process {
                process_state: ProcessState::EMBRYO,
                kernel_stack: kernel_stack,
                id: unsafe {PROCESS_ID},
                page_directory: null_mut(),
                trap_frame: trap_frame_pointer,
                context: context_pointer
            });

            unsafe {
                *process_entry.unwrap().context = Context {
                    edi: 0,
                    esi: 0,
                    ebx: 0,
                    ebp: 0,
                    eip: 13, // forkret
                };
            }
            Some(unsafe {PROCESS_ID - 1})
        },
        None => {
            None
        }
    }
}

#[derive(Copy, Clone)]
pub struct Process {
    process_state: ProcessState,
    kernel_stack: usize,
    id: usize,
    trap_frame: *mut TrapFrame,
    context: *mut Context,
    pub page_directory: *mut PD,
/*  pub sz: u32,
    pub procstate: u32, // Should be enum
    pub parent: *const Process,
    pub chan: *const ffi::c_void,
    pub killed: i32,
    pub ofile: [*const File; param::NOFILE],
    pub cwd: *const Inode,
    pub name: [u8; 16],*/
}

unsafe impl Send for Process {

}

pub fn user_init() {
    println!("user_init");

    let pid = alloc_process().expect("Could not create user process");
    let page_directory = setup_kernel_virtual_memory();
    let mut process = PROCESS_TABLE.lock()[pid].unwrap();

    if page_directory.is_some() {
        process.page_directory = page_directory.unwrap() as *mut PD;
    } else {
        panic!("user_init: out of memory?")
    }
    println!("user_init: Success.");

}