use core::marker::PhantomData;

use crate::types::{PageAddr, PTL2PageAddr, PTL3PageAddr, PTL4PageAddr, HasVirtAddr, page_table::ENTRY_COUNT};
use super::{EntryValue, PTL1EntrySlot, PTL2EntrySlot, PTL3EntrySlot, PTL4EntrySlot};

#[repr(C)]
pub struct PTL1 {
    entries: [usize; ENTRY_COUNT],
}

// a higher level page table, then an array of virtual addresses of the tables it points to
#[repr(C)]
struct PageTableWithSubtables {
    entries: [usize; ENTRY_COUNT],
    subtables: [usize; ENTRY_COUNT]
}

pub struct PTL2(PageTableWithSubtables);
pub struct PTL3(PageTableWithSubtables);
pub struct PTL4(PageTableWithSubtables);

// these need to be pointers, not references because of some annoying provenance thing probably
// i want to be able to store an entry slot with one pointer, but an entry slot is non-contiguous (entry and subtable are separate)
// references can only be to contiguous objects, so if i use them i would need separate pointers for entry and subtable
pub struct PTL1Handle<'a> {
    entries: *mut usize,
    len: usize,
    addr: PageAddr,
    _marker: PhantomData<&'a mut usize>
}


pub struct PTL2Handle<'a> {
    entries: *mut usize,
    len: usize,
    addr: PTL2PageAddr,
    _marker: PhantomData<&'a mut usize>
}

pub struct PTL3Handle<'a> {
    entries: *mut usize,
    len: usize,
    addr: PTL3PageAddr,
    _marker: PhantomData<&'a mut usize>
}

pub struct PTL4Handle<'a> {
    entries: *mut usize,
    len: usize,
    addr: PTL4PageAddr,
    _marker: PhantomData<&'a mut usize>
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
            pub fn new(table: &'a mut $table, addr: $addr) -> Self {
                Self { 
                    entries: table as *mut $table as *mut usize,
                    len: ENTRY_COUNT,
                    addr,
                    _marker: PhantomData
                }
            }
        
            pub fn addr(&self) -> $addr {
                self.addr
            }
        
            pub fn len(&self) -> usize {
                self.len
            }

            fn get_entry(&self, ind: usize) -> *mut usize {
                assert!(ind < self.len);
                unsafe { self.entries.add(ind) }
            }
        
            pub fn entry(&mut self, ind: usize) -> $entry_slot {
                $entry_slot {
                    entry: self.get_entry(ind),
                    _marker: PhantomData
                }
            }
        
            pub fn take_entry(mut self, ind: usize) -> $entry_slot<'a> {
                $entry_slot {
                    entry: self.get_entry(ind),
                    _marker: PhantomData
                }
            }

            pub fn split_at(self, mid: usize) -> ($table_handle<'a>, $table_handle<'a>) {
                assert!(mid < self.len);
                let a2 = self.addr.next(mid);
                ($table_handle {
                    entries: self.entries,
                    len: mid,
                    addr: self.addr,
                    _marker: PhantomData
                }, $table_handle {
                    entries: unsafe { self.entries.add(mid) },
                    len: self.len - mid,
                    addr: a2,
                    _marker: PhantomData
                })
            }
        
            pub fn take_first_n(&mut self, n: usize) -> $table_handle<'a>{
                assert!(n < self.len);
                let first_entries = self.entries;
                self.entries = unsafe { self.entries.add(n) };
                self.len -= n;
                let res = $table_handle {
                    entries: first_entries,
                    len: n,
                    addr: self.addr,
                    _marker: PhantomData
                };
                self.addr = self.addr.next(n);
                res
            }
        
            pub fn take_first(&mut self) -> $entry_slot<'a> {
                self.take_first_n(1).take_entry(0)
            }
        
            pub fn get(&self, ind: usize) -> EntryValue {
                EntryValue::from_usize(unsafe { *self.get_entry(ind) })
            }
        }
    };
}

impl_handle_methods!(PTL1Handle, PTL1EntrySlot, PTL1, PageAddr);
impl_handle_methods!(PTL2Handle, PTL2EntrySlot, PTL2, PTL2PageAddr);
impl_handle_methods!(PTL3Handle, PTL3EntrySlot, PTL3, PTL3PageAddr);
impl_handle_methods!(PTL4Handle, PTL4EntrySlot, PTL4, PTL4PageAddr);