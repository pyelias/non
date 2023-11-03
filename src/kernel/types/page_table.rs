use core::mem::MaybeUninit;
use super::{Page, PageAddr, FrameAddr, HasPhysAddr, HasVirtAddr, transmute::{TransmuteFrom, TransmuteBetween, TransmuteInto}, page::PAGE_SHIFT};
use paste::paste;

// 52 bits in a phys addr, but lower 12 are all 0
const PT_FRAME_BITS: usize = 0x000F_FFFF_FFFF_F000;
pub const PTE_INDEX_SIZE: usize = 9;
const PTE_INDEX_MASK: usize = (1 << PTE_INDEX_SIZE) - 1;

const L4_TABLE_ADDR: usize = 0x1000;

const PRESENT_FLAG: usize = 1 << 0;
const WRITABLE_FLAG: usize = 1 << 1;
const SUPERVISOR_ONLY_FLAG: usize = 1 << 2;
const WRITE_THROUGH_FLAG: usize = 1 << 3;
const DISABLE_CACHE_FLAG: usize = 1 << 4;
const ACCESSED_FLAG: usize = 1 << 5;
const AVAILABLE_FLAG: usize = 1 << 6; // not used for anything in hardware, free for the os
const BIG_FLAG: usize = 1 << 7;
const EXEC_DISABLE_FLAG: usize = 1 << 63;

macro_rules! add_flag_methods {
    ($name:ident, $mask:ident) => {
        paste! {
            fn $name(self) -> Self { self.set(PRESENT_FLAG) }
            fn [<not_ $name>](self) -> Self { self.clear(PRESENT_FLAG) }
            fn [<is_ $name>](self) -> bool {
                self.to_usize() & PRESENT_FLAG != 0
            }
        }
    };
}

pub unsafe trait Flags: Copy + TransmuteBetween<usize> {
    type Entry: Entry;
    
    const MASK: usize;
    const ALWAYS_SET: usize;

    fn from_usize(flags: usize) -> Self {
        Self::transmute_from(flags & Self::MASK | Self::ALWAYS_SET)
    }

    fn none() -> Self {
        Self::ALWAYS_SET.transmute_into()
    }

    fn to_usize(self) -> usize {
        self.transmute_into()
    }
    
    fn set(self, flags: usize) -> Self {
        Self::from_usize(self.to_usize() | flags)
    }
    
    fn clear(self, flags: usize) -> Self {
        Self::from_usize(self.to_usize() & !flags)
    }
    
    fn get(self, flag: usize) -> bool {
        self.to_usize() & flag != 0
    }
    
    fn get_all(self, flags: usize) -> usize {
        self.to_usize() & flags
    }
    add_flag_methods!(present, PRESENT_FLAG);
}

macro_rules! make_flags_type {
    ($entry:ty, $name:ident, $mask:literal, $always_set:literal) => {
        #[derive(Copy, Clone)]
        #[repr(transparent)]
        pub struct $name(usize);

        // Safety: it's repr(transparent) over a usize
        unsafe impl TransmuteFrom<usize> for $name {}
        unsafe impl TransmuteFrom<$name> for usize {}
        unsafe impl Flags for $name {
            type Entry = $entry;

            const MASK: usize = $mask;
            const ALWAYS_SET: usize = $always_set;
        }   
    };
}


// Safety: must be repr(transparent) over a usize
pub unsafe trait Entry: Copy + TransmuteBetween<usize> {
    fn to_usize(self) -> usize {
        self.transmute_into()
    }

    fn from_usize(flags: usize) -> Self {
        flags.transmute_into()
    }

    fn empty() -> Self {
        Self::from_usize(0)
    }

    fn to_generic(self) -> GenericEntry {
        GenericEntry::from_usize(self.to_usize())
    }

    fn at_frame(addr: FrameAddr) -> Self {
        Self::from_usize(addr.usize())
    }

    fn get_frame(self) -> FrameAddr {
        (self.to_usize() & PT_FRAME_BITS).frame_addr()
    }

    fn with_frame(self, addr: FrameAddr) -> Self {
        Self::from_usize((self.to_usize() & !PT_FRAME_BITS) | (addr.usize() & PT_FRAME_BITS))
    }

    fn flags<F: Flags<Entry=Self>>(self) -> F {
        F::from_usize(self.to_usize())
    }

    fn with_flags<F: Flags<Entry=Self>>(self, flags: F) -> Self {
        Self::from_usize((self.to_usize() & !F::MASK) | (flags.to_usize()))
    }
}

macro_rules! make_entry_type {
    ($name:ident) => {
        #[derive(Copy, Clone)]
        #[repr(transparent)]
        pub struct $name(usize);

        unsafe impl TransmuteFrom<$name> for usize {}
        unsafe impl TransmuteFrom<usize> for $name {}

        unsafe impl Entry for $name {}
    };
}

make_entry_type!(GenericEntry);
make_flags_type!(GenericEntry, GenericEntryFlags, 0x8000_0000_0000_003F, 0x0000_0000_0000_0000);

make_entry_type!(PageEntry);
make_flags_type!(PageEntry, PageEntryFlags, 0xf800_0000_0000_017F, 0x0000_0000_0000_0000);
make_flags_type!(PageEntry, SmallPageEntryFlags, 0xf800_0000_0000_01FF, 0x0000_0000_0000_0000);
make_flags_type!(PageEntry, LargePageEntryFlags, 0xf800_0000_0000_117F, 0x0000_0000_0000_0080);

make_entry_type!(PageTableEntry);
make_flags_type!(PageTableEntry, PageTableEntryFlags, 0x8000_0000_0000_003F, 0x0000_0000_0000_0000);

pub const ENTRY_COUNT: usize = super::page::PAGE_SIZE / core::mem::size_of::<usize>();

// ENTRY_COUNT * sizeof(GenericEntry) = PAGE_SIZE
pub type PageTable = [MaybeUninit<GenericEntry>; ENTRY_COUNT];

// Safety: PageTable and Page are the same size
// PageTable has an alignment of 8, and Page has an alignment of 4096
// Anything is a valid value for MaybeUninit
unsafe impl TransmuteFrom<Page> for PageTable {}

pub fn pte_indices_to_addr(l4: usize, l3: usize, l2: usize, l1: usize) -> PageAddr {
    let addr = (
        (l4 << PTE_INDEX_SIZE * 3) +
        (l3 << PTE_INDEX_SIZE * 2) +
        (l2 << PTE_INDEX_SIZE * 1) +
        (l1 << PTE_INDEX_SIZE * 0)
    ) << PAGE_SHIFT;
    // sign extend to the top 16 bits
    addr.page_addr()
}

pub fn addr_to_pte_indices(addr: PageAddr) -> (usize, usize, usize, usize) {
    let page_number = addr.usize() >> PAGE_SHIFT;
    (
        (page_number >> PTE_INDEX_SIZE * 3) & PTE_INDEX_MASK,
        (page_number >> PTE_INDEX_SIZE * 2) & PTE_INDEX_MASK,
        (page_number >> PTE_INDEX_SIZE * 1) & PTE_INDEX_MASK,
        (page_number >> PTE_INDEX_SIZE * 0) & PTE_INDEX_MASK
    )
}