use super::{page::PAGE_SHIFT, AlignedPhys, FrameAddr, HasPhysAddr, HasVirtAddr, PageAddr};
use core::mem::MaybeUninit;
use paste::paste;

// 52 bits in a phys addr, but lower 12 are all 0
pub const PT_FRAME_BITS: usize = 0x000F_FFFF_FFFF_F000;

pub const PTE_INDEX_SIZE: usize = 9;
const PTE_INDEX_MASK: usize = (1 << PTE_INDEX_SIZE) - 1;

const L4_TABLE_ADDR: usize = 0x1000;

// TODO should the present flag always be set?
// something without it doesn't function as an entry
pub const PRESENT_FLAG: usize = 1 << 0;
const WRITABLE_FLAG: usize = 1 << 1;
const SUPERVISOR_ONLY_FLAG: usize = 1 << 2;
const WRITE_THROUGH_FLAG: usize = 1 << 3;
const DISABLE_CACHE_FLAG: usize = 1 << 4;
const ACCESSED_FLAG: usize = 1 << 5;
const AVAILABLE_FLAG: usize = 1 << 6; // not used for anything in hardware, free for the os
const LARGE_PAGE_FLAG: usize = 1 << 7;
const EXEC_DISABLE_FLAG: usize = 1 << 63;

macro_rules! add_flag_methods {
    ($name:ident, $mask:ident) => {
        paste! {
            fn [<set_ $name>](&mut self) -> Self { self.set($mask) }
            fn [<unset_ $name>](&mut self) -> Self { self.clear($mask) }
            fn [<is_ $name>](self) -> bool {
                self.to_usize() & $mask != 0
            }
        }
    };
}

mod flags_supertrait_seal {
    pub trait Seal {}
}

pub trait Flags: flags_supertrait_seal::Seal + Sized {
    const MASK: usize;
    const ALWAYS_SET: usize;

    fn from_usize(flags: usize) -> Self;
    fn to_usize(&self) -> usize;
    fn set(&mut self, flags: usize) -> Self;
    fn clear(&mut self, flags: usize) -> Self;

    fn none() -> Self {
        Self::from_usize(0)
    }

    fn get(self, flag: usize) -> bool {
        self.to_usize() & flag != 0
    }

    fn get_all(self, flags: usize) -> usize {
        self.to_usize() & flags
    }

    add_flag_methods!(large_page, LARGE_PAGE_FLAG);
}

macro_rules! make_flags_type {
    ($name:ident, $mask:literal, $always_set:literal) => {
        #[derive(Copy, Clone)]
        #[repr(transparent)]
        pub struct $name(usize);

        impl flags_supertrait_seal::Seal for $name {}

        impl Flags for $name {
            const MASK: usize = $mask;
            const ALWAYS_SET: usize = $always_set | PRESENT_FLAG;

            fn from_usize(flags: usize) -> Self {
                Self(flags & Self::MASK | Self::ALWAYS_SET)
            }

            fn to_usize(&self) -> usize {
                self.0
            }

            fn set(&mut self, flags: usize) -> Self {
                self.0 |= flags & Self::MASK;
                *self
            }

            fn clear(&mut self, flags: usize) -> Self {
                self.0 &= !(flags & Self::MASK);
                *self
            }
        }
    };
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct Entry(usize);

impl Entry {
    pub fn to_usize(self) -> usize {
        self.0
    }

    pub fn from_usize(raw: usize) -> Self {
        Self(raw | PRESENT_FLAG)
    }

    pub fn empty() -> Self {
        Self::from_usize(0)
    }

    pub fn at_frame(addr: FrameAddr) -> Self {
        Self::from_usize(addr.usize())
    }

    pub fn frame(&self) -> FrameAddr {
        self.frame_with_align()
    }

    pub fn frame_with_align<A: AlignedPhys>(&self) -> A {
        (self.0 & A::ALIGNMENT.mask() & PT_FRAME_BITS)
            .phys_addr()
            .as_aligned()
    }

    pub fn set_frame_with_size(&mut self, addr: FrameAddr, size_bits: usize) -> Self {
        self.0 &= !size_bits;
        self.0 |= addr.usize() & size_bits | PRESENT_FLAG;
        *self
    }

    pub fn set_frame(&mut self, addr: FrameAddr) -> Self {
        self.set_frame_with_size(addr, PT_FRAME_BITS)
    }

    pub fn flags<F: Flags>(self) -> F {
        assert!(self.to_usize() & F::ALWAYS_SET == F::ALWAYS_SET);
        F::from_usize(self.to_usize())
    }

    pub fn set_flags<F: Flags>(&mut self, flags: F) -> Self {
        self.0 &= !F::MASK;
        self.0 |= flags.to_usize();
        *self
    }

    pub fn add_flags<F: Flags>(&mut self, flags: F) -> Self {
        self.0 |= flags.to_usize();
        *self
    }
}

make_flags_type!(
    GenericEntryFlags,
    0x8000_0000_0000_003F,
    0x0000_0000_0000_0000
);

make_flags_type!(PageFlags, 0xf800_0000_0000_01FF, 0x0000_0000_0000_0000);

make_flags_type!(
    HighLevelEntryFlags,
    0xf800_0000_0000_00FF,
    0x0000_0000_0000_0000
);
make_flags_type!(SubtableFlags, 0x8000_0000_0000_007F, 0x0000_0000_0000_0000);
make_flags_type!(LargePageFlags, 0xf800_0000_0000_117F, 0x0000_0000_0000_0080);

pub const ENTRY_COUNT: usize = super::page::PAGE_SIZE / core::mem::size_of::<usize>();

// ENTRY_COUNT * sizeof(GenericEntry) = PAGE_SIZE
pub type PageTable = [MaybeUninit<Entry>; ENTRY_COUNT];

pub fn pte_indices_to_addr(l4: usize, l3: usize, l2: usize, l1: usize) -> PageAddr {
    let addr = ((l4 << PTE_INDEX_SIZE * 3)
        + (l3 << PTE_INDEX_SIZE * 2)
        + (l2 << PTE_INDEX_SIZE * 1)
        + (l1 << PTE_INDEX_SIZE * 0))
        << PAGE_SHIFT;
    // sign extend to the top 16 bits
    addr.virt_addr().as_aligned()
}

pub fn addr_to_pte_indices(addr: PageAddr) -> (usize, usize, usize, usize) {
    let page_number = addr.usize() >> PAGE_SHIFT;
    (
        (page_number >> PTE_INDEX_SIZE * 3) & PTE_INDEX_MASK,
        (page_number >> PTE_INDEX_SIZE * 2) & PTE_INDEX_MASK,
        (page_number >> PTE_INDEX_SIZE * 1) & PTE_INDEX_MASK,
        (page_number >> PTE_INDEX_SIZE * 0) & PTE_INDEX_MASK,
    )
}
