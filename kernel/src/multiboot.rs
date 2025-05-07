use core::{mem::size_of, ops::Range};

use acpi::AcpiTables;
use bytemuck::AnyBitPattern;

use crate::types::{HasPhysAddr, HasVirtAddr, PhysAddr, VirtAddr};

#[repr(C)]
#[derive(Copy, Clone)]
struct InfoHeader {
    total_size: u32,
    _reserved: u32,
}

pub struct Info(&'static [u8]);

impl Info {
    pub unsafe fn new(info: VirtAddr) -> Self {
        let header: InfoHeader = unsafe { *info.ptr() };
        let data: &[u8] = core::slice::from_raw_parts(info.ptr(), header.total_size as usize);
        Self(&data[core::mem::size_of::<InfoHeader>()..])
    }

    pub fn tags_iter(&self) -> TagsIter {
        TagsIter(self.0)
    }

    pub fn get_tag<T: Tag>(&self) -> Option<T> {
        for (kind, data) in self.tags_iter() {
            if let Some(res) = T::try_make(kind, data) {
                return Some(res);
            }
        }
        return None;
    }

    pub fn mem_range(&self) -> Range<PhysAddr> {
        let start = self.0.as_ptr().to_phys();
        let end = (start.usize() + self.0.len()).phys_addr();
        start..end
    }
}

#[repr(C)]
#[derive(Clone, Copy, AnyBitPattern)]
struct TagHeader {
    kind: u32,
    size: u32,
}

pub trait Tag: Sized {
    fn try_make(kind: u32, data: &'static [u8]) -> Option<Self>;
}

pub struct BootloaderName(&'static [u8]);

impl BootloaderName {
    pub fn as_str(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(self.0)
    }

    pub fn bytes(&self) -> &'static [u8] {
        self.0
    }
}

impl Tag for BootloaderName {
    fn try_make(kind: u32, data: &'static [u8]) -> Option<Self> {
        if kind == 2 {
            Some(Self(data))
        } else {
            None
        }
    }
}

pub struct Rsdp(acpi::rsdp::Rsdp);

impl Rsdp {
    pub fn get_tables(&self) -> AcpiTables<crate::acpi::IdMapper> {
        // hopefully theres no way to pass a bad rsdp here
        unsafe { crate::acpi::acpi_tables_from_rsdp(self.0).unwrap() }
    }
}

impl Tag for Rsdp {
    fn try_make(kind: u32, data: &'static [u8]) -> Option<Self> {
        match kind {
            14 | 15 => Some(Self(crate::acpi::rsdp_from_buf(data))),
            _ => None,
        }
    }
}

pub struct TagsIter(&'static [u8]);

impl Iterator for TagsIter {
    type Item = (u32, &'static [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        let align_offset = (&self.0[0] as *const u8).align_offset(8);
        self.0 = &self.0[align_offset..];

        let header: TagHeader = *bytemuck::from_bytes(&self.0[..size_of::<TagHeader>()]);
        let data = &self.0[size_of::<TagHeader>()..header.size as usize];

        self.0 = &self.0[header.size as usize..];

        if header.kind == 0 {
            return None;
        }

        Some((header.kind, data))
    }
}

#[repr(C)]
#[derive(Copy, Clone, AnyBitPattern)]
struct MMapTagHeader {
    entry_size: u32,
    _entry_version: u32,
}

#[derive(Copy, Clone)]
pub struct MMapEntryIterator {
    entry_size: u32,
    entries: &'static [u8],
}

impl MMapEntryIterator {
    fn new(mut data: &'static [u8]) -> Self {
        let header: MMapTagHeader = *bytemuck::from_bytes(&data[..size_of::<MMapTagHeader>()]);
        assert!(header.entry_size % 8 == 0);

        data = &data[size_of::<MMapTagHeader>()..];
        Self {
            entry_size: header.entry_size,
            entries: data,
        }
    }
}

impl Iterator for MMapEntryIterator {
    type Item = MMapEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.entries.is_empty() {
            return None;
        }

        let entry = *bytemuck::from_bytes(&self.entries[..self.entry_size as usize]);
        self.entries = &self.entries[self.entry_size as usize..];
        Some(entry)
    }
}

impl Tag for MMapEntryIterator {
    fn try_make(kind: u32, data: &'static [u8]) -> Option<Self> {
        if kind == 6 {
            Some(Self::new(data))
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum MMapEntryKind {
    Available,
    Reserved,
    ACPIReclaimable,
    NVS,
    BadRam,
}

#[repr(C)]
#[derive(Copy, Clone, AnyBitPattern)]
pub struct MMapEntry {
    pub addr: PhysAddr,
    pub len: usize,
    kind: u32,
    _reserved: u32,
}

impl MMapEntry {
    pub fn kind(&self) -> MMapEntryKind {
        match self.kind {
            1 => MMapEntryKind::Available,
            3 => MMapEntryKind::ACPIReclaimable,
            4 => MMapEntryKind::NVS,
            5 => MMapEntryKind::BadRam,
            // 2 or anything else that might come up
            _ => MMapEntryKind::Reserved,
        }
    }
}
