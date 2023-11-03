use core::marker::PhantomData;

use crate::types::{HasPhysAddr, HasVirtAddr, PhysAddr};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct AOutSymbolTable {
    pub tabsize: u32,
    pub strsize: u32,
    pub addr: u32,
    pub reserved: u32
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ElfSectionHeaderTable {
    pub num: u32,
    pub size: u32,
    pub addr: u32,
    pub shndx: u32
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union SymbolTable {
    pub aout_sym: AOutSymbolTable,
    pub elf_sec: ElfSectionHeaderTable
}

const HAS_MMAP_FLAG: u32 = 1 << 6;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Info {
    pub flags: u32,
    pub mem_lower: u32,
    pub mem_upper: u32,
    pub boot_device: u32,
    pub cmdline: u32,
    pub mods_count: u32,
    pub mods_addr: u32,
    pub syms: SymbolTable,
    pub mmap_length: u32,
    pub mmap_addr: u32,
    pub drives_length: u32,
    pub drives_addr: u32,
    pub config_table: u32,
    pub boot_loader_name: u32,
    // more, but i don't need them right now
}

impl Info {
    pub fn mmap_entries(&self) -> MMapIterator {
        if self.flags & HAS_MMAP_FLAG == 0 {
            panic!("multiboot info did not have a mmap");
        }
        let head = (self.mmap_addr as usize).to_virt().ptr::<MMapEntry>();
        MMapIterator {
            curr: head,
            end: head.wrapping_byte_add(self.mmap_length as usize),
            _marker: PhantomData
        }
    }
}


#[derive(Copy, Clone, Debug)]
pub enum MMapEntryType {
    Available,
    Reserved,
    ACPIReclaimable,
    NVS,
    BadRam
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct MMapEntry {
    pub size: u32,
    pub addr: PhysAddr,
    pub len: usize,
    type_: u32
}

impl MMapEntry {
    pub fn type_(&self) -> MMapEntryType {
        match self.type_ {
            1 => MMapEntryType::Available,
            3 => MMapEntryType::ACPIReclaimable,
            4 => MMapEntryType::NVS,
            5 => MMapEntryType::BadRam,
            // 2 or anything else that might come up
            _ => MMapEntryType::Reserved
        }
    }
}

pub struct MMapIterator<'a> {
    curr: *const MMapEntry,
    end: *const MMapEntry,
    _marker: PhantomData<&'a MMapEntry>
}

impl<'a> Iterator for MMapIterator<'a> {
    type Item = &'a MMapEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr >= self.end {
            return None;
        }
        // safe because constructing an MMapIterator requires an Info
        // which can only be obtained from C
        // properly formatted multiboot info will have valid entry pointers
        unsafe { 
            let res = &*self.curr;
            self.curr = self.curr.byte_add(res.size as usize + 4);
            Some(res)
        }
    }
}