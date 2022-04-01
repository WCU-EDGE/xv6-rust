//! # Page Allocator
//! A physical page allocator.
//! The free memory is allocated from the end of the kernel to the end of physical memory.
//! A linked list of free pages is used to keep track of allocations.

use core::ops::Deref;
use console::print;
use memory_layout;
use mmu::{page_round_up, PAGE_SIZE};

/// The default allocator used by the kernel.
pub static mut FREE_PAGE_LIST: AllocationList = AllocationList::new();

#[repr(C)]
struct AllocationNode {
  next: Option<&'static mut AllocationNode>
}

/// Stores a list of free pages.
/// Removing a page allocates it.
/// Adding a page deallocates it.
/// Pages are stored using their virtual addresses.
/// The AllocationNode is stored at the begging of the free page.
#[repr(C)]
pub struct AllocationList {
  head: AllocationNode
}

impl AllocationNode {

  const fn new() -> Self {
    Self {next: None}
  }

  /// Returns the address where this page is stored.
  fn address(&self) -> usize {
    self as *const Self as usize
  }

}

extern "C" {
  static END_SYMBOL: usize;
}

/// Initialize the allocator using [page_round_up(end), 4MB].
/// This is a maximum of 1024 4096 byte pages. As the code size grows, end also grows leaving less free pages.
pub fn init() {
  unsafe {
    FREE_PAGE_LIST.dealloc_range(&END_SYMBOL as *const usize as usize, memory_layout::map_physical_virtual(0x400000));
  }
}

impl AllocationList {

  const fn new() -> Self {
    Self {
      head: AllocationNode::new()
    }
  }

  /// Frees pages in the range of [page_round_up(start),end].
  /// Returns an option containing the address of the allocated page.
  /// # Arguments
  /// * 'start' - The start address of the page range. If this is not aligned to a 4096 byte address, then the address will be rounded up to the next page.
  /// * 'end' - The end address of the page range. This value does not need to be aligned.
  pub unsafe fn dealloc_range(&mut self, start: usize, end: usize) {
    assert!(start <= end);
    let start_page = page_round_up(start);

    let mut page = start_page;
    while page + PAGE_SIZE <= end {
      self.dealloc_page(page);
      page += PAGE_SIZE;
    }
  }

  /// Allocates a page.
  /// Returns an option containing the address of the allocated page.
  /// # Arguments
  /// * 'address' - A page address returned from alloc_page.
  pub fn alloc_page(&mut self) -> Option<usize> {
    unsafe {
      let cur = self.head.next.take();
      match cur {
        None => {
          None
        }
        Some(page) => {
          let ret = page.address();
          self.head.next = page.next.take();

          Some(ret)
        }
      }
    }
  }

  /// Deallocates a page.
  /// # Arguments
  /// * 'address' - A page address returned from alloc_page.
  pub unsafe fn dealloc_page(&mut self, address: usize) {
    // Create the new node.
    let mut page_allocation_node = AllocationNode::new();
    // The new node will point to the current head's next.
    page_allocation_node.next = self.head.next.take();

    // Create a pointer to the start of the next free block.
    let page_allocation_node_pointer = address as *mut AllocationNode;
    // Write the new node into that memory.
    page_allocation_node_pointer.write(page_allocation_node);

    // Set the current head to the new node.
    self.head.next = Some(&mut *page_allocation_node_pointer);
  }

}