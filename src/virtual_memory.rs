use core::ffi::c_void;
use core::slice;
use x86::bits32::paging::{PAddr, PD, PDEntry, PDFlags, PT, PTEntry, PTFlags};
use x86::task::tr;
use ::{memory_layout, mmu};
use memory_layout::{DEVICE_SPACE, EXTENDED_MEMORY, KERNEL_BASE, KERNEL_LINK, PHYSICAL_TOP};
use mmu::PAGE_SIZE;
use page_allocator::FREE_PAGE_LIST;

struct KernelMap {
  virtual_adress: usize,
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
    KernelMap {virtual_adress: KERNEL_BASE, phys_start: 0, phys_end: EXTENDED_MEMORY, perm: PTFlags::RW},
    KernelMap {virtual_adress: KERNEL_LINK, phys_start: 0, phys_end: EXTENDED_MEMORY, perm: PTFlags::RW},
    KernelMap {virtual_adress: tmp, phys_start: 0, phys_end: EXTENDED_MEMORY, perm: PTFlags::RW},
    KernelMap {virtual_adress: DEVICE_SPACE, phys_start: 0, phys_end: EXTENDED_MEMORY, perm: PTFlags::RW},
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
    let map_result = map_pages(page_directory, mapping.virtual_adress, mapping.phys_end - mapping.phys_start,
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
  let mut address: usize;
  let mut physical_address = physical_address;
  let last: usize;

  address = mmu::page_round_down(virtual_address);
  last = mmu::page_round_down(virtual_address + size - 1);
  loop {
    match walk_page_directory(page_directory, address, true) {
      Some(page_table_entry) => {
        if page_table_entry.is_present() {
          panic!("Remap!");
        }

        *page_table_entry = PTEntry::new(PAddr::from(physical_address), PTFlags::P | permissions);
        if address == last {
          break;
        } else {
          address += PAGE_SIZE;
          physical_address += PAGE_SIZE;
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