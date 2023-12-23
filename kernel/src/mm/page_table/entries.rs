use super::{EntryValue, NonPresentUsize, PTL1Entries, PTL2Entries, PTL3Entries, PTL1, PTL2, PTL3};
use crate::types::{
    page_table::Entry, zeroable, HasVirtAddr, PTL2PageAddr, PTL3PageAddr, PTL4PageAddr, PageAddr,
};

pub struct EntrySlot<'a>(&'a mut usize);

pub struct PTL1EntrySlot<'a> {
    pub(super) entry: &'a mut usize,
    pub(super) addr: PageAddr,
}

pub struct PTL2EntrySlot<'a> {
    pub(super) entry: &'a mut usize,
    pub(super) addr: PTL2PageAddr,
}

pub struct PTL3EntrySlot<'a> {
    pub(super) entry: &'a mut usize,
    pub(super) addr: PTL3PageAddr,
}

pub struct PTL4EntrySlot<'a> {
    pub(super) entry: &'a mut usize,
    pub(super) addr: PTL4PageAddr,
}

impl EntrySlot<'_> {
    pub fn get(&self) -> EntryValue {
        EntryValue::from_usize(*self.0)
    }

    pub fn set(&mut self, val: EntryValue) {
        *self.0 = val.to_usize();
    }

    pub fn replace(&mut self, val: EntryValue) -> EntryValue {
        let old = core::mem::replace(self.0, val.to_usize());
        EntryValue::from_usize(old)
    }

    pub fn take(&mut self) -> EntryValue {
        self.replace(EntryValue::Raw(NonPresentUsize::new(0)))
    }

    pub fn get_raw(&self) -> NonPresentUsize {
        let EntryValue::Raw(v) = self.get() else {
            panic!("entry not raw value")
        };
        v
    }

    pub fn get_entry(&self) -> Entry {
        let EntryValue::Entry(e) = self.get() else {
            panic!("entry not mapped")
        };
        e
    }

    pub fn set_raw(&mut self, raw: NonPresentUsize) {
        self.set(EntryValue::Raw(raw));
    }

    pub fn set_entry(&mut self, entry: Entry) {
        self.set(EntryValue::Entry(entry));
    }
}

macro_rules! impl_entry_methods {
    ($lifetime:lifetime, $addr:ident) => {
        pub unsafe fn from_entry_addr(entry: &$lifetime mut usize, addr: $addr) -> Self {
            Self {
                entry,
                addr
            }
        }

        pub fn entry(&mut self) -> EntrySlot {
            EntrySlot(self.entry)
        }

        pub fn get_entry_value(&self) -> EntryValue {
            EntryValue::from_usize(*self.entry)
        }
    };
}

macro_rules! impl_map_page_methods {
    ($addr:ident) => {
        pub unsafe fn map_page(mut self, entry: Entry) -> $addr {
            self.entry().set_entry(entry);
            self.addr
        }
    };
}

macro_rules! impl_map_subtable_methods {
    ($lifetime:lifetime, $subtable:ident, $subtable_handle:ident) => {
        pub unsafe fn map_subtable(
            mut self,
            entry: Entry,
            subtable_ptr: PageAddr,
        ) -> $subtable_handle<$lifetime> {
            self.entry().set_entry(entry);
            let subtable_ptr = subtable_ptr.ptr();
            let subtable: &mut $subtable = zeroable::zero_ptr::<$lifetime>(subtable_ptr);
            $subtable_handle {
                entries: &mut subtable.entries,
                addr: self.addr.as_aligned(),
            }
        }
    };
}

// these are all the same type, but they're semantically different
impl<'a> PTL1EntrySlot<'a> {
    impl_entry_methods!('a, PageAddr);

    impl_map_page_methods!(PageAddr);
}

impl<'a> PTL2EntrySlot<'a> {
    impl_entry_methods!('a, PTL2PageAddr);

    impl_map_subtable_methods!('a, PTL1, PTL1Entries);
}

impl<'a> PTL3EntrySlot<'a> {
    impl_entry_methods!('a, PTL3PageAddr);

    impl_map_subtable_methods!('a, PTL2, PTL2Entries);
}

impl<'a> PTL4EntrySlot<'a> {
    impl_entry_methods!('a, PTL4PageAddr);

    impl_map_subtable_methods!('a, PTL3, PTL3Entries);
}
