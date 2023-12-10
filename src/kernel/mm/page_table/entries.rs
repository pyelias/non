use core::marker::PhantomData;

use crate::types::{page_table::Entry, page::PAGE_SIZE};
use super::{NonPresentUsize, EntryValue};

pub struct EntrySlot<'a>(&'a mut usize);

pub struct PTL1EntrySlot<'a> {
    pub(super) entry: *mut usize,
    pub(super) _marker: PhantomData<&'a mut usize>
}

pub struct PTL2EntrySlot<'a> {
    pub(super) entry: *mut usize,
    pub(super) _marker: PhantomData<&'a mut usize>
}

pub struct PTL3EntrySlot<'a> {
    pub(super) entry: *mut usize,
    pub(super) _marker: PhantomData<&'a mut usize>
}

pub struct PTL4EntrySlot<'a> {
    pub(super) entry: *mut usize,
    pub(super) _marker: PhantomData<&'a mut usize>
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
        let EntryValue::Raw(v) = self.get() else { panic!("entry not raw value") };
        v
    }

    pub fn get_entry(&self) -> Entry {
        let EntryValue::Entry(e) = self.get() else { panic!("entry not mapped") };
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
    () => {
        pub fn entry(&mut self) -> EntrySlot {
            EntrySlot(unsafe { &mut *self.entry })
        }
    
        pub fn get_entry_value(&self) -> EntryValue {
            EntryValue::from_usize(unsafe { *self.entry })
        }

        pub fn into_ptr(self) -> *mut usize {
            self.entry
        }

        pub unsafe fn from_ptr(ptr: *mut usize) -> Self {
            Self {
                entry: ptr,
                _marker: PhantomData
            }
        }
    };
}

macro_rules! impl_subtable_methods {
    () => {
        pub fn subtable(&mut self) -> &mut usize {
            unsafe { &mut *entry_ptr_to_subtable_ptr(self.entry) }
        }
    
        pub fn get_subtable(&self) -> usize {
            unsafe { *entry_ptr_to_subtable_ptr(self.entry) }
        }
    
        pub fn entry_subtable(&mut self) -> (EntrySlot, &mut usize) {
            let entry = unsafe { &mut *self.entry };
            let subtable = unsafe { &mut *entry_ptr_to_subtable_ptr(self.entry) };
            (EntrySlot(entry), subtable)
        }
    };
}

impl PTL1EntrySlot<'_> {
    impl_entry_methods!();
}

// these are all the same types, but they're semantically different

impl<'a> PTL2EntrySlot<'a> {
    impl_entry_methods!();

    // impl_subtable_methods!();
}

impl<'a> PTL3EntrySlot<'a> {
    impl_entry_methods!();

    // impl_subtable_methods!();
}

impl<'a> PTL4EntrySlot<'a> {
    impl_entry_methods!();

    // impl_subtable_methods!();
}