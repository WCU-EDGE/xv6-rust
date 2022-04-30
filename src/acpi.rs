//! Advanced Configuration and Power Interface (ACPI)
//! In xv6-rust ACPi is used to detect number of cpus and their local interrupt controller id's.
//! This interface must not be used after the initial page table is changed.
//! Xv6's memory map overlaps with many ACPI structures.

use core::mem;
use core::ptr::slice_from_raw_parts;
use console::print;
use local_interrupt_controller::LOCAL_INTERRUPT_CONTROLLER;
use memory_layout::map_physical_virtual;
use process::Cpu;

pub const MAX_CPUS: usize = 8;
pub static mut NUM_CPUS: usize = 0;
pub static mut CPUS: [Cpu; MAX_CPUS] = [Cpu::new(); MAX_CPUS];
pub static mut INTERRUPT_CONTROLLER_ID: u8 = 0;

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct RootSystemDescription {
  // 1.0
  signature: [u8; 8],
  checksum: u8,
  oem_id: [u8; 6],
  revision: u8,
  rsdt_32: u32,
  // 2.0
  length: u32,
  rsdt_64_address: u64,
  extended_checksum: u8,
  reserved: [u8; 3],
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct SystemDescriptionHeader {
  signature: [u8; 4],
  length: u32,
  revision: u8,
  checksum: u8,
  oem_id: [u8; 6],
  oem_table_id: [u8; 8],
  oem_revision: u32,
  creator_id: u32,
  creator_revision: u32
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct MADTEntryHeader {
  entry_type: u8,
  length: u8,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct ProcessorLocalAPIC {
  header: MADTEntryHeader,
  apic_id: u8,
  flags: u32
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct RootSystemDescriptionTable32 {
  header: SystemDescriptionHeader
  // Followed by a variable length array of pointers to other system descriptor tables.
}

/// Multiple APIC Description Table
#[repr(C, packed)]
#[derive(Copy, Clone)]
struct MADT {
  header: SystemDescriptionHeader,
  local_apic_address: usize,
  flags: u32
  // Followed by a variable length array of Entries
}

const APIC_SIGNATURE: [u8; 4] = [b'A', b'P', b'I', b'C'];

lazy_static! {
    pub static ref ACPI2: spin::Mutex<ACPI> = spin::Mutex::new(ACPI::new());
}

pub struct ACPI {
  root_system_description: &'static RootSystemDescription
}

/// Sum a region of bytes.
/// Used for checksums.
fn sum(region: &[u8]) -> u8 {
  let mut sum: u8 = 0;

  for byte in region {
    sum = sum.overflowing_add(*byte).0;
  }

  sum
}

/// Search a block of memory for a block of memory.
/// Returns the start address of the found block of memory or None.
fn search(region: &[u8], data: &[u8]) -> Option<usize> {
  let mut current_data: &[u8] = &region[0..0];

  if data.len() > region.len() {
    return None;
  }

  loop {

    if (current_data.as_ptr() as usize + 1) <= (data.as_ptr() as usize + data.len() - 1) {

      if current_data == &data[0..current_data.len()] {

        if current_data.len() == data.len() {
          return Some(current_data.as_ptr() as usize);
        } else {
          unsafe {
            current_data = &*slice_from_raw_parts(current_data.as_ptr(), current_data.len() + 1);
          }
        }
      } else {
        unsafe {
          current_data = &*slice_from_raw_parts((current_data.as_ptr() as usize + 1) as *const u8, 1);
        }
      }

    } else {
      break;
    }

  }

  None
}

impl ACPI {

  pub fn new() -> Self {

    let rsd_address: usize = unsafe {
      let res = search(& *slice_from_raw_parts(map_physical_virtual(0x40E) as *const u8, 1024), HEADER);
      //let res2 = search(& *slice_from_raw_parts(0x000E0000 as *const u8, 0x20000), HEADER);

      if res.is_some() {
        res.unwrap()
      } else {
          panic!("Could not find Root System Description.");
      }
    };

    let rsdp: &RootSystemDescription = unsafe {& *(rsd_address as *const RootSystemDescription)};

    let is_valid = (*rsdp).is_checksum_valid();
    if !is_valid {
      panic!("ACPI - Root system description checksum is invalid!");
    }

    println!("ACPI - init");
    Self {
      root_system_description: rsdp
    }

  }

  pub fn populate_cpu_info(&self) {
    self.root_system_description.rsdt_32().print_rsdt();
    let res = self.root_system_description.rsdt_32().search_entry(&APIC_SIGNATURE);
    println!("APIC entry found: {}", res.is_some());

    if res.is_none() {
      return;
    }

    unsafe {
      let madt = res.unwrap() as *const MADT;
      LOCAL_INTERRUPT_CONTROLLER = (*madt).local_apic_address as *mut u32;

      let mut entry = madt.offset(1) as *const MADTEntryHeader;
      let end = madt as usize + (*madt).header.length as usize;

      while (entry as usize) < end {
        match (*entry).entry_type {
          0 => {
            if NUM_CPUS < CPUS.len() {
              let processor_local_apic = entry as *const ProcessorLocalAPIC;
              CPUS[NUM_CPUS].apicid = (*processor_local_apic).apic_id;
              NUM_CPUS += 1;
            }
          },
          _ => {
          }
        }
        entry = ((entry as usize) + (*entry).length as usize) as *const MADTEntryHeader;
      }
      println!("CPUS: {}", NUM_CPUS);

    }

  }

}

const HEADER: &[u8] = "RSD PTR ".as_bytes();

impl RootSystemDescription {

  fn rsdt_32(&self) -> &'static RootSystemDescriptionTable32 {
    unsafe {
      &*((self.rsdt_32 as usize) as *const RootSystemDescriptionTable32)
    }
  }

  /// Checks if the structure contains a valid checksum.
  fn is_checksum_valid(&self) -> bool {

    // 20 Is the size of a version 1 struct.
    // 14 Is the size of the added fields in version 2.
    let version_one_data = unsafe {& *slice_from_raw_parts(self as *const RootSystemDescription as usize as *const u8, 20)};
    let version_two_data = unsafe {& *slice_from_raw_parts((self as *const RootSystemDescription as usize + 20) as *const u8, 14)};

    let mut res = sum(version_one_data) == 0;

    if self.revision == 2 {
      res = res && (sum(version_two_data) == 0);
    }

    res
  }

}

impl RootSystemDescriptionTable32 {

  pub fn search_entry(&self, signature: &[u8; 4]) -> Option<usize> {
    let table_entries = (self.header.length - mem::size_of::<SystemDescriptionHeader>() as u32) / 4;
    let entries: &[*const SystemDescriptionHeader];
    unsafe {
      entries = &*slice_from_raw_parts(((self as *const RootSystemDescriptionTable32 as usize) + mem::size_of::<SystemDescriptionHeader>()) as *const *const SystemDescriptionHeader, table_entries as usize);

      for entry in entries {
        if (*(*entry)).signature == *signature {
          return Some(*entry as usize);
        }
      }
    }
    None
  }

  pub fn print_rsdt(&self) {
    println!("RSDT Entries:");
    let table_entries = self.get_entries();
    let entries: &[*const SystemDescriptionHeader];
    unsafe {
      entries = &*slice_from_raw_parts(((self as *const RootSystemDescriptionTable32 as usize) + mem::size_of::<SystemDescriptionHeader>()) as *const *const SystemDescriptionHeader, table_entries as usize);

      for entry in entries {
        println!("  {}{}{}{}", (**entry).signature[0] as char, (**entry).signature[1] as char, (**entry).signature[2] as char, (**entry).signature[3] as char);
      }

    }
  }

  fn get_entries(&self) -> u32 {
    (self.header.length - mem::size_of::<SystemDescriptionHeader>() as u32) / 4
  }

}