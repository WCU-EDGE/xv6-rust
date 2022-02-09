
#[repr(C)]
struct AllocationNode {
  size: usize,
  next: Option<&'static mut AllocationNode>
}

#[repr(C)]
struct AllocationList {
  head: AllocationNode
}

impl AllocationNode {

  fn new(size: usize) -> Self {
    Self {size, next: None}
  }

}