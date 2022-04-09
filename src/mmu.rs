use core::ffi;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TaskState {
    link: u32,
    esp0: u32,
    ss0: u16,
    padding1: u16,
    esp1: *const u32,
    ss1: u16,
    padding2: u16,
    esp2: *const u32,
    ss2: u16,
    padding3: u16,
    cr3: *const ffi::c_void,
    eip: *const u32,
    eflags: u32,
    eax: u32,
    ecx: u32,
    edx: u32,
    ebx: u32,
    esp: *const u32,
    ebp: *const u32,
    esi: u32,
    edi: u32,
    es: u16,
    padding4: u16,
    cs: u16,
    padding5: u16,
    ss: u16,
    padding6: u16,
    ds: u16,
    padding7: u16,
    fs: u16,
    padding8: u16,
    gs: u16,
    padding9: u16,
    ldt: u16,
    padding10: u16,
    t: u16,
    iomb: u16,
}

impl TaskState {
    pub const fn new() -> TaskState {
        TaskState {
            link: 0,
            esp0: 0,
            ss0: 0,
            padding1: 0,
            esp1: 0 as *const u32,
            ss1: 0,
            padding2: 0,
            esp2: 0 as *const u32,
            ss2: 0,
            padding3: 0,
            cr3: 0 as *const ffi::c_void,
            eip: 0 as *const u32,
            eflags: 0,
            eax: 0,
            ecx: 0,
            edx: 0,
            ebx: 0,
            esp: 0 as *const u32,
            ebp: 0 as *const u32,
            esi: 0,
            edi: 0,
            es: 0,
            padding4: 0,
            cs: 0,
            padding5: 0,
            ss: 0,
            padding6: 0,
            ds: 0,
            padding7: 0,
            fs: 0,
            padding8: 0,
            gs: 0,
            padding9: 0,
            ldt: 0,
            padding10: 0,
            t: 0,
            iomb: 0
        }
    }
}

bitfield!{
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SegDesc(MSB0 [u8]);
    u32;
    get_lim_15_0, _: 15, 0;
    get_base_15_0, _: 31, 16;
    base_23_16, _: 39, 32;
    segtype, _: 43, 40;
    s, _: 44;
    dpl, _: 46, 45;
    p, _: 47;
    lim_19_16, _: 51, 48;
    avl, _: 52;
    rsv1, _: 53;
    db, _: 54;
    g, _: 55;
    base_31_24, _: 63, 56;
}

pub const PAGE_SIZE: usize = 4096;

/// Rounds up to the nearest page.
pub const fn page_round_up(address: usize) -> usize {
    return (address + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

/// Rounds down to the nearest page.
pub const fn page_round_down(address: usize) -> usize {
    return address & !(PAGE_SIZE - 1)
}

/// Get the index of a virtual address's page directory entry in a page directory.
pub const fn page_directory_index(virtual_address: usize) -> usize {
    (virtual_address >> PAGE_DIRECTORY_INDEX_SHIFT) & 0x3FFusize
}

/// Get the index of a virtual address's page table entry in a page table.
pub const fn page_table_index(virtual_address: usize) -> usize {
    ((virtual_address >> PAGE_TABLE_INDEX_SHIFT) & 0x3FFusize)
}

pub const PAGE_DIRECTORY_INDEX_SHIFT: usize = 22; // offset of PDX in a linear address
pub const PAGE_TABLE_INDEX_SHIFT: usize = 12; // offset of PDX in a linear address