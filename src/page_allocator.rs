//! # Page Allocator
//! A physical page allocator.
//! The free memory is allocated from the end of the kernel to the end of physical memory.
//! A linked list of free pages is used to keep track of allocations.

use memory_layout;
use mmu::{page_round_up, PAGE_SIZE};

/// Stores a list of free pages.
/// Removing a page from the FREE_PAGE_LIST allocates it.
/// Adding a page to the FREE_PAGE_LIST deallocates it.
/// Pages are stored using their virtual addresses.
/// The AllocationNode is stored at the begging of the free page.
static mut FREE_PAGE_LIST: AllocationList = AllocationList::new();

#[repr(C)]
struct AllocationNode {
  next: Option<&'static mut AllocationNode>
}

#[repr(C)]
struct AllocationList {
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

/// Initialize the allocator using [page_round_up(end), 4MB].
pub fn init(end: usize) {
  unsafe {
    FREE_PAGE_LIST.dealloc_range(end, memory_layout::map_physical_virtual(0x400000));
  }
}

impl AllocationList {

  const fn new() -> Self {
    Self {
      head: AllocationNode::new()
    }
  }

  // Add the 4MB page(really 256 4k pages) to the free list.
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
  pub unsafe fn alloc_page(&mut self) -> Option<usize> {
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