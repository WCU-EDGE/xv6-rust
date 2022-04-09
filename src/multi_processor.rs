use core::mem;
use core::mem::{MaybeUninit, size_of};
use core::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};
use x86::apic::ioapic::IoApic;
use x86::io::{inb, outb};
use local_interrupt_controller::LOCAL_INTERRUPT_CONTROLLER;
use memory_layout::map_physical_virtual;
use mmu::TaskState;
use process;
use process::Cpu;

pub const MAX_CPUS: usize = 8;
pub static mut CPUS: [Cpu; MAX_CPUS] = [Cpu::new(); MAX_CPUS];
pub static mut INTERRUPT_CONTROLLER_ID: u8 = 0;

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

static mut IS_MULTI_PROCESSOR: bool = false;

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

  p = ((0u8) | *bda.add(0x0E)) as u32;

  if p << 4 != 0 {
    let res = search1(p as usize, 1024);
    if res.is_some() {
      return Some(res.unwrap());
    }
  } else {
    p = ((0u8)|*bda.add(0x13)) as u32 * 1024;
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
unsafe fn config(pmp: *mut *mut MultiProcessor) -> Option<*mut MultiProcessorConfig>  {
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

const MPPROC: u8 = 0;  // One per processor
const MPBUS: u8 = 1;  // One per processor
const MPIOAPIC: u8 = 2;  // One per processor
const MPIOINTR: u8 = 3;  // One per processor
const MPLINTR: u8 = 4;  // One per processor

static mut ncpus: u8 = 0;

pub(crate) unsafe fn init() {
  let mut p: *mut u8;
  let e: *mut u8;
  let mut multi_processor: *mut MultiProcessor = 0 as * mut MultiProcessor;
  let configuration: *mut MultiProcessorConfig;
  let mut processor_table_entry: *mut ProcessorTableEntry;
  let mut io_apic: *mut ApicTableEntry;

  let configuration_result= config(&mut multi_processor);
  if configuration_result.is_none() {
    return;
  }

  configuration = configuration_result.unwrap();
  IS_MULTI_PROCESSOR = true;
  LOCAL_INTERRUPT_CONTROLLER = (*configuration).lapicaddr;

  p = (configuration as usize + 1) as *mut u8;
  e = ((configuration as usize) + (*configuration).length as usize) as *mut u8;
  while p < e {

    match *p {
      MPPROC => {
        processor_table_entry = p as *mut ProcessorTableEntry;
        if ncpus < MAX_CPUS as u8 {
          CPUS[ncpus as usize].apicid = (*processor_table_entry).apic_id;
        }
      },
      MPBUS => {
        p = (p as usize + 8) as *mut u8;
      },
      MPIOAPIC => {
        io_apic = p as *mut ApicTableEntry;
        INTERRUPT_CONTROLLER_ID = (*io_apic).apic_id;
        p = p.add(size_of::<ApicTableEntry>());
      },
      MPIOINTR => {
        p = (p as usize + 8) as *mut u8;
      },
      MPLINTR => {
        p = (p as usize + 8) as *mut u8;
      },
      _ => {
        IS_MULTI_PROCESSOR = false;
      }
    }

    p = p.add(size_of::<IoApic>())
  }

  if !IS_MULTI_PROCESSOR {
    panic!("Didn't find a suitable machine");
  }

  if (*multi_processor).imcrp != 0 {
    // Bochs doesn't support IMCR, so this doesn't run on Bochs.
    // But it would on real hardware.
    outb(0x22, 0x70);   // Select IMCR
    outb(0x23, inb(0x23) | 1);  // Mask external interrupts.
  }

}

