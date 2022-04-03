use core::mem;
use core::mem::{MaybeUninit, size_of};
use core::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};
use x86::apic::ioapic::IoApic;
use memory_layout::map_physical_virtual;
use mmu::TaskState;
use process;
use process::Cpu;

pub const MAX_CPUS: usize = 8;
pub static mut CPUS: [u32; MAX_CPUS] = [0; MAX_CPUS];

struct MultiProcessor {
  // floating pointer
  signature: [u8; 4],
  // "_MP_"
  physical_address: usize,
  // phys addr of MP config table
  length: u8,
  // 1
  spec_revision: u8,
  // [14]
  checksum: u8,
  // all bytes must add up to 0
  mp_type: u8,
  // MP system config type
  imcrp: u8,
  reserved: [u8; 3],
}

struct MultiProcessorConfig {
  // configuration table header
  signature: [u8; 4],
  // "PCMP"
  length: u8,
  // total table length
  version: u8,
  // [14]
  checksum: u8,
  // all bytes must add up to 0
  product: [u8; 20],
  // product id
  oemtable: *mut u32,
  // OEM table pointer
  oemlength: u8,
  // OEM table length
  entry: u8,
  // entry count
  lapicaddr: *mut u32,
  // address of local APIC
  xlength: u8,
  // extended table length
  xchecksum: u8,
  // extended table checksum
  reserved: u8,
}

// processor table entry
struct ProcessorTableEntry {
  // entry type (0)
  entry_type: u8,
  // local APIC id
  apic_id: u8,
  // local APIC version
  version: u8,
  // CPU flags
  flags: u8,
  // CPU signature
  signature: [u8; 4],
  // feature flags from CPUID instruction
  feature: u32,
  reserved: [u8; 8],
}

// I/O APIC table entry
struct ApicTableEntry {
  // entry type (2)
  entry_type: u8,
  // I/O APIC ids
  apic_id: u8,
  // I/O APIC version
  version: u8,
  // I/O APIC flags
  flags: u8,
  address: *const u32,                  // I/O APIC address
}

fn sum(address: &[u8]) -> u8 {
  let mut sum: u8 = 0;

  for i in address {
    sum += i;
  }

  sum
}

const HEADER: [u8; 4] = [b'_', b'M', b'P', b'_'];
const HEADER_2: [u8; 4] = [b'P', b'C', b'M', b'P'];

unsafe fn search1(address: usize, length: usize) -> Option<*mut MultiProcessor> {
  let address: *mut u8 = map_physical_virtual(address) as *mut u8;
  let end: *mut u8 = address.add(length);
  let mut p = address;
  loop {
    if p < end {
      let e= slice_from_raw_parts_mut(p as *mut u8, 4);
      (*e).copy_from_slice(&HEADER);
      if p as usize == 0 && sum(&*slice_from_raw_parts_mut(p as *mut u8, mem::size_of::<MultiProcessor>())) == 0 {
        return Some(p as *mut MultiProcessor);
      }

      p = p.add(size_of::<MultiProcessor>());
    } else {
      break;
    }
  }
  None
}

// Search for the MP Floating Pointer Structure, which according to the
// spec is in one of the following three locations:
// 1) in the first KB of the EBDA;
// 2) in the last KB of system base memory;
// 3) in the BIOS ROM between 0xE0000 and 0xFFFFF.
unsafe fn search() -> Option<*mut MultiProcessor> {
 let bda: *mut u8;
  let mut p: u32;
  let mp: *mut MultiProcessor;

  bda = map_physical_virtual(0x400) as *mut u8;
  p = ((*bda.add(0x0F) << 8) | (*bda.add(0x0E) << 4)) as u32;
  if p != 0 {
    let res = search1(p as usize, 1024);
    if res.is_some() {
      return Some(res.unwrap());
    }
  } else {
    p = ((*bda.add(0x14) << 8) | *bda.add(0x13)) as u32 * 1024;
    let res = search1(p as usize - 1024, 1024);
    if res.is_some() {
      return Some(res.unwrap());
    }
  }

  return search1(0xF0000, 0x10000);
}

// Search for an MP configuration table.  For now,
// don't accept the default configurations (physaddr == 0).
// Check for correct signature, calculate the checksum and,
// if correct, check the version.
// To do: check extended table checksum.
unsafe fn mpconfig(pmp: *mut *mut MultiProcessor) -> Option<*mut MultiProcessorConfig>  {
  let configuration: *mut MultiProcessorConfig;
  let multi_processor: *mut MultiProcessor;
  let result = search();
  if result.is_none() {
    return None;
  }
  multi_processor = result.unwrap();
  if (*multi_processor).physical_address == 0 {
    return None;
  }

  configuration = map_physical_virtual((*multi_processor).physical_address) as *mut MultiProcessorConfig;

  let dest = slice_from_raw_parts_mut(configuration as *mut u8, 4);

  (*dest).copy_from_slice(&HEADER_2);

  if configuration as usize == 0 {
    return None;
  }

  if (*configuration).version != 1 && (*configuration).version != 4 {
    return None;
  }

  let x = &*slice_from_raw_parts_mut(configuration as *mut u8, (*configuration).length as usize);
  if sum(&*x) != 0 {
    return None;
  }
  *pmp = multi_processor;
  return Some(configuration);
}

fn init() {
  let p: *mut u8;
  let e: *mut u8;
  let configuration: *mut MultiProcessorConfig;
  let processor_table_entry: *mut ProcessorTableEntry;
  let io_apic: *mut IoApic;

  //configuration = mpconfig(&mp);



}

