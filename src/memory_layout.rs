/// # Memory layout

/// Start of extended memory
pub const EXTENDED_MEMORY: usize = 0x100000;

/// Top physical memory
pub const PHYSICAL_TOP: usize = 0xE000000;
/// Other devices are at high addresses
pub const DEVICE_SPACE: usize = 0xFE000000;

/// Key addresses for address space layout (see kmap in vm.c for layout)
/// First kernel virtual address
pub const KERNEL_BASE: usize = 0x80000000;
/// Address where kernel is linked
pub const KERNEL_LINK: usize = KERNEL_BASE + EXTENDED_MEMORY;

/// Maps a virtual address to a physical address.
pub const fn map_virtual_to_physical(address: usize) -> usize {
  address - KERNEL_BASE
}

/// Maps a physical address to a virtual address.
pub const fn map_physical_virtual(address: usize) -> usize {
  address.overflowing_add(KERNEL_BASE).0
}