use super::{EntryValue, PTL1EntrySlot, PTL2EntrySlot, PTL3EntrySlot, PTL4EntrySlot};
use crate::types::{
    page_table::ENTRY_COUNT, PTL2PageAddr, PTL3PageAddr, PTL4PageAddr, PageAddr, Zeroable,
};

#[repr(C, align(4096))]
pub struct PTL1 {
    pub(super) entries: [usize; ENTRY_COUNT],
}

#[repr(C, align(4096))]
pub struct PTL2 {
    pub(super) entries: [usize; ENTRY_COUNT],
}

#[repr(C, align(4096))]
pub struct PTL3 {
    pub(super) entries: [usize; ENTRY_COUNT],
}

#[repr(C, align(4096))]
pub struct PTL4 {
    pub(super) entries: [usize; ENTRY_COUNT],
}

unsafe impl Zeroable for PTL1 {}
unsafe impl Zeroable for PTL2 {}
unsafe impl Zeroable for PTL3 {}
unsafe impl Zeroable for PTL4 {}

pub struct PTL1Entries<'a> {
    pub(super) entries: &'a mut [usize],
    pub(super) addr: PageAddr,
}

pub struct PTL2Entries<'a> {
    pub(super) entries: &'a mut [usize],
    pub(super) addr: PTL2PageAddr,
}

pub struct PTL3Entries<'a> {
    pub(super) entries: &'a mut [usize],
    pub(super) addr: PTL3PageAddr,
}

pub struct PTL4Entries<'a> {
    pub(super) entries: &'a mut [usize],
    pub(super) addr: PTL4PageAddr,
}

fn take_first_mut<'a, T>(s: &mut &'a mut [T]) -> &'a mut T {
    let slice = core::mem::take(s);
    let (s1, s2) = slice.split_first_mut().unwrap();
    *s = s2;
    s1
}

fn take_first_n_mut<'a, T>(s: &mut &'a mut [T], n: usize) -> &'a mut [T] {
    let slice = core::mem::take(s);
    let (s1, s2) = slice.split_at_mut(n);
    *s = s2;
    s1
}

macro_rules! impl_handle_methods {
    ($table_handle:ident, $entry_slot:ident, $table:ident, $addr:ident) => {
        impl<'a> $table_handle<'a> {
            pub unsafe fn from_entries_addr(entries: &'a mut [usize], addr: $addr) -> Self {
                Self { entries, addr }
            }

            pub fn addr(&self) -> $addr {
                self.addr
            }

            pub fn table_ptr(&self) -> *const usize {
                self.entries.as_ptr() as *const usize
            }

            pub fn len(&self) -> usize {
                self.entries.len()
            }

            pub fn entry(&mut self, ind: usize) -> $entry_slot {
                $entry_slot {
                    entry: &mut self.entries[ind],
                    addr: self.addr.next(ind),
                }
            }

            pub fn take_entry(self, ind: usize) -> $entry_slot<'a> {
                $entry_slot {
                    entry: &mut self.entries[ind],
                    addr: self.addr.next(ind),
                }
            }

            pub fn split_at(self, mid: usize) -> ($table_handle<'a>, $table_handle<'a>) {
                let (e1, e2) = self.entries.split_at_mut(mid);
                let a2 = self.addr.next(mid);
                (
                    $table_handle {
                        entries: e1,
                        addr: self.addr,
                    },
                    $table_handle {
                        entries: e2,
                        addr: a2,
                    },
                )
            }

            pub fn take_first_n(&mut self, n: usize) -> $table_handle<'a> {
                let first_entries = take_first_n_mut(&mut self.entries, n);
                let res = $table_handle {
                    entries: first_entries,
                    addr: self.addr,
                };
                self.addr = self.addr.next(n);
                res
            }

            pub fn take_first(&mut self) -> $entry_slot<'a> {
                let first_entry = take_first_mut(&mut self.entries);
                let old_addr = self.addr;
                self.addr = self.addr.next(1);
                $entry_slot {
                    entry: first_entry,
                    addr: old_addr,
                }
            }

            pub fn get(&self, ind: usize) -> EntryValue {
                EntryValue::from_usize(self.entries[ind])
            }
        }
    };
}

impl_handle_methods!(PTL1Entries, PTL1EntrySlot, PTL1, PageAddr);
impl_handle_methods!(PTL2Entries, PTL2EntrySlot, PTL2, PTL2PageAddr);
impl_handle_methods!(PTL3Entries, PTL3EntrySlot, PTL3, PTL3PageAddr);
impl_handle_methods!(PTL4Entries, PTL4EntrySlot, PTL4, PTL4PageAddr);
