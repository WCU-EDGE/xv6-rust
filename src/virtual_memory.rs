use core::arch::asm;
use core::slice;
use x86::bits32::paging::{PAddr, PD, PDEntry, PDFlags, PT, PTEntry, PTFlags};
use x86::controlregs::cr3_write;
use x86::dtables::{DescriptorTablePointer, lgdt};
use x86::Ring;
use x86::segmentation::{CodeSegmentType, DataSegmentType, Descriptor};
use ::{memory_layout, mmu};
use ::{console, process};
use console::print;
use memory_layout::{DEVICE_SPACE, EXTENDED_MEMORY, KERNEL_BASE, KERNEL_LINK, map_virtual_to_physical, PHYSICAL_TOP};
use mmu::{page_round_up, PAGE_SIZE, SEGMENT_KERNEL_CODE, SEGMENT_KERNEL_DATA, SEGMENT_USER_CODE, SEGMENT_USER_DATA};
use page_allocator::FREE_PAGE_LIST;
use process::Cpu;

struct KernelMap {
  virtual_address: usize,
  phys_start: usize,
  phys_end: usize,
  perm: PTFlags
}

extern {
  static data: usize;
}

/// Creates a page directory and the corresponding page tables need for the kernel's virtual memory mappings.
pub fn setup_kernel_virtual_memory() -> Option<&'static mut PD> {
  let tmp: usize = unsafe { &data as *const usize as usize };
  let map: [KernelMap; 4] = [
    KernelMap {virtual_address: KERNEL_BASE, phys_start: 0, phys_end: EXTENDED_MEMORY, perm: PTFlags::RW},
    KernelMap {virtual_address: KERNEL_LINK, phys_start: map_virtual_to_physical(KERNEL_LINK), phys_end: map_virtual_to_physical(tmp), perm: PTFlags::empty()},
    KernelMap {virtual_address: tmp, phys_start: map_virtual_to_physical(tmp), phys_end: PHYSICAL_TOP, perm: PTFlags::RW},
    KernelMap {virtual_address: DEVICE_SPACE, phys_start: DEVICE_SPACE, phys_end: 0, perm: PTFlags::RW},
  ];

  let page_location = unsafe {FREE_PAGE_LIST.alloc_page()};
  if page_location.is_none() {
    return None;
  }

  if memory_layout::map_physical_virtual(PHYSICAL_TOP) > DEVICE_SPACE {
    panic!("PHYSICAL_TOP too high");
  }

  let page_directory = unsafe {
    &mut *(page_location.unwrap() as *mut PD)
  };

  for page_directory_entry in page_directory.into_iter() {
    *page_directory_entry = PDEntry::new(PAddr::zero(), PDFlags::empty());
  }

  for mapping in map {
    //println!("[{:x},{:x}]", mapping.phys_start, mapping.phys_end);
    let sz = mapping.phys_end.overflowing_sub(mapping.phys_start).0;
    let map_result = map_pages(page_directory, mapping.virtual_address, sz,
                               mapping.phys_start, mapping.perm);
    if map_result == false {
      free_virtual_memory(page_directory);
      return None;
    }

  }

  Some(page_directory)
}

/// Creates page table entries for a virtual_address.
fn map_pages(page_directory: &mut PD, virtual_address: usize, size: usize, physical_address: usize, permissions: PTFlags) -> bool {
  let mut physical_address = physical_address;
  let mut address = mmu::page_round_down(virtual_address);
  let last = mmu::page_round_down(virtual_address.overflowing_add(size).0.overflowing_sub(1).0);
  //println!("({:x}, {:x})", address, last);
  loop {
    match walk_page_directory(page_directory, address, true) {
      Some(page_table_entry) => {
        if page_table_entry.is_present() {
          panic!("Remap!");
        }
          *page_table_entry = PTEntry::new(PAddr::from(physical_address), PTFlags::P | permissions);
        if address == last {
          return true;
        } else {
          address = address.overflowing_add(PAGE_SIZE).0;
          physical_address = physical_address.overflowing_add(PAGE_SIZE).0;
        }
      }
      None => {
        return false;
      }
    }
  }

  return true;
}


/// Checks if there is an entry in the page directory for page table.
/// If the page table already exists, then return a reference to the page table entry for virtual_address.
/// If there is no entry and allocate is false, then None is returned.
/// If there is no entry and allocate is true, then allocate memory for the page table and return the reference to the page table entry for virtual_address.
fn walk_page_directory(page_table: &mut PD, virtual_address: usize, allocate: bool) -> Option<&mut PTEntry> {
  let page_directory_entry = &mut page_table[mmu::page_directory_index(virtual_address)];
  let page_table: &mut PT;

  if page_directory_entry.is_present() {
    unsafe {
      page_table = &mut *(memory_layout::map_physical_virtual(page_directory_entry.address().as_usize()) as *mut PT);
    }
  } else {
    if !allocate {
      return None;
    }

    let page_location = unsafe {FREE_PAGE_LIST.alloc_page()};
    if page_location.is_none() {
      return None;
    }

    unsafe {
      page_table = &mut *(page_location.unwrap() as *mut PT);
    }

    for page_table_entry in page_table.into_iter() {
      *page_table_entry = PTEntry::new(PAddr::zero(), PTFlags::empty());
    }

    *page_directory_entry = PDEntry::new(PAddr::from(memory_layout::map_virtual_to_physical(page_location.unwrap())),
                                         PDFlags::P | PDFlags::RW | PDFlags::US);
  }


  Some(&mut page_table[mmu::page_table_index(virtual_address)])
}

/// Free a page table and all the physical memory pages.
fn free_virtual_memory(page_directory: &mut PD) {
  for page_directory_entry in page_directory.into_iter() {
    if page_directory_entry.is_present() {
      unsafe {
        FREE_PAGE_LIST.dealloc_page(memory_layout::map_physical_virtual(page_directory_entry.address().as_usize()))
      }
    }
  }
  unsafe {
    FREE_PAGE_LIST.dealloc_page(memory_layout::map_physical_virtual(page_directory as *mut PD as usize));
  }
}


// Load the init_code into address 0 of page_directory.
// size must be less than a page.
fn init_user_virtual_memory(page_directory: &mut PD, init_code: usize, size: usize) {
  let mem: *mut u8;
  if size >= PAGE_SIZE {
    panic!("init_uuser_virtual_memory: more than a page");
  }

  let page_location = unsafe {FREE_PAGE_LIST.alloc_page()};
  if page_location.is_none() {
    panic!("No more memory.");
  }

  mem = page_location.unwrap() as *mut u8;
  unsafe {
    mem.write_bytes(0, PAGE_SIZE);
  }
  map_pages(page_directory, 0, PAGE_SIZE, memory_layout::map_virtual_to_physical(page_location.unwrap()), PTFlags::RW | PTFlags::US);

  unsafe {
    let mem = slice::from_raw_parts_mut(mem, size);
    let init = slice::from_raw_parts_mut(init_code as *mut u8, size);
    mem.copy_from_slice(init);
  }
}

static mut kernel_page_directory: *mut PD = 0 as *mut PD;

/// Allocate one page table for the machine for the kernel address space for scheduler processes.
/// After this call all kernel code and peripherals will be mapped to higher memory.
pub unsafe fn kmalloc() {
  kernel_page_directory = setup_kernel_virtual_memory().expect("No kernel page table");
  switchkvm();
  console::switch_to_virtual_memory();
  println!("Kernel memory allocated. Mapped to higher address space.")
}


// Switch h/w page table register to the kernel-only page table, for when no process is running.
unsafe fn switchkvm()  {
  let page_directory_address = map_virtual_to_physical(kernel_page_directory as usize);
  asm!("mov {0}, %cr3", in(reg) page_directory_address as usize, options(att_syntax));
}

/// Sets up segmentation for a cpu core. Called once for each cpu.
/// The primary reason for using segmentation is for per cpu variables.
/// On the pentium segmentation happens before paging.
pub(crate) unsafe fn setup_segmentation() {
  let cpu: &mut Cpu = process::get_current_cpu();

  cpu.gdt[SEGMENT_KERNEL_CODE] = Descriptor::default();
  cpu.gdt[SEGMENT_KERNEL_CODE].set_type(CodeSegmentType::ExecuteRead as u8);
  cpu.gdt[SEGMENT_KERNEL_CODE].set_base_limit(0, 0xffffffff);
  cpu.gdt[SEGMENT_KERNEL_CODE].set_dpl(Ring::Ring0);

  cpu.gdt[SEGMENT_KERNEL_DATA] = Descriptor::default();
  cpu.gdt[SEGMENT_KERNEL_DATA].set_type(DataSegmentType::ReadWrite as u8);
  cpu.gdt[SEGMENT_KERNEL_DATA].set_base_limit(0, 0xffffffff);
  cpu.gdt[SEGMENT_KERNEL_DATA].set_dpl(Ring::Ring0);

  cpu.gdt[SEGMENT_USER_CODE] = Descriptor::default();
  cpu.gdt[SEGMENT_USER_CODE].set_type(CodeSegmentType::ExecuteRead as u8);
  cpu.gdt[SEGMENT_USER_CODE].set_base_limit(0, 0xffffffff);
  cpu.gdt[SEGMENT_USER_CODE].set_dpl(Ring::Ring3);

  cpu.gdt[SEGMENT_USER_DATA] = Descriptor::default();
  cpu.gdt[SEGMENT_USER_DATA].set_type(DataSegmentType::ReadWrite as u8);
  cpu.gdt[SEGMENT_USER_DATA].set_base_limit(0, 0xffffffff);
  cpu.gdt[SEGMENT_USER_DATA].set_dpl(Ring::Ring3);

  let gdt_pointer = DescriptorTablePointer::new(&cpu.gdt);

  lgdt(&gdt_pointer);

  println!("Segmentation setup.");
}