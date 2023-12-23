use crate::types::page_table::{Entry, PRESENT_FLAG};

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct NonPresentUsize(usize);

impl NonPresentUsize {
    pub fn try_new(val: usize) -> Option<Self> {
        if val & PRESENT_FLAG == 0 {
            Some(Self(val))
        } else {
            None
        }
    }

    pub fn new(val: usize) -> Self {
        Self::try_new(val).unwrap()
    }
}

impl From<NonPresentUsize> for usize {
    fn from(value: NonPresentUsize) -> Self {
        value.0
    }
}

#[derive(Clone, Copy)]
pub enum EntryValue {
    Raw(NonPresentUsize),
    Entry(Entry),
}

impl EntryValue {
    pub fn from_usize(v: usize) -> Self {
        match NonPresentUsize::try_new(v) {
            Some(v) => Self::Raw(v),
            None => Self::Entry(Entry::from_usize(v)),
        }
    }

    pub fn to_usize(&self) -> usize {
        match self {
            Self::Raw(v) => v.0,
            Self::Entry(e) => e.to_usize(),
        }
    }
}
