use super::page::PAGE_SHIFT;
use super::page_table::PTE_INDEX_SIZE;
use core::fmt::Display;

const ID_MAP_SIZE: usize = 1 << 30;
// must equal HIGH_ID_MAP_VMA from linker
const HIGH_ID_MAP_ADDR: usize = 0xFFFF_FFFF_C000_0000;
const VIRT_ADDR_BITS: usize = 48;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Alignment(usize);

impl Alignment {
    #[inline(always)]
    pub fn mask(&self) -> usize {
        (1 << self.0) - 1
    }

    #[inline(always)]
    pub fn high_mask(&self) -> usize {
        !self.mask()
    }

    pub fn size(&self) -> usize {
        1 << self.0
    }
}

const PAGE_ALIGNMENT: Alignment = Alignment(PAGE_SHIFT);
const PTL2_ALIGNMENT: Alignment = Alignment(PAGE_SHIFT + PTE_INDEX_SIZE);
const PTL3_ALIGNMENT: Alignment = Alignment(PAGE_SHIFT + PTE_INDEX_SIZE * 2);
const PTL4_ALIGNMENT: Alignment = Alignment(PAGE_SHIFT + PTE_INDEX_SIZE * 3);

pub trait HasPhysAddr {
    fn usize(&self) -> usize;

    #[inline(always)]
    fn phys_addr(&self) -> PhysAddr {
        PhysAddr(self.usize())
    }

    #[inline(always)]
    fn try_as_aligned<A: AlignedPhys>(&self) -> Option<A> {
        if !self.is_aligned(A::ALIGNMENT) {
            return None;
        }
        Some(A::from_usize(self.usize()))
    }

    #[inline(always)]
    fn as_aligned<A: AlignedPhys>(&self) -> A {
        self.try_as_aligned()
            .unwrap_or_else(|| panic!("unaligned address: {}", self.phys_addr()))
    }

    #[inline(always)]
    fn align<A: AlignedPhys>(&self) -> A {
        A::from_usize(self.usize() & A::ALIGNMENT.high_mask())
    }

    fn to_virt(&self) -> VirtAddr {
        // only the bottom 1GiB phys is id-mapped
        let addr = self.usize();
        assert!(
            addr < ID_MAP_SIZE,
            "tried to use the 1GiB id-map on an address outside its range: {}",
            self.phys_addr()
        );
        VirtAddr(addr + HIGH_ID_MAP_ADDR)
    }

    #[inline(always)]
    fn align_offset(&self, alignment: Alignment) -> usize {
        self.usize() & alignment.mask()
    }

    #[inline(always)]
    fn is_aligned(&self, alignment: Alignment) -> bool {
        self.align_offset(alignment) == 0
    }
}

pub trait AlignedPhys: HasPhysAddr {
    const ALIGNMENT: Alignment;

    fn from_usize(n: usize) -> Self;
}

pub trait HasVirtAddr {
    fn usize(&self) -> usize;

    #[inline(always)]
    fn ptr<T>(&self) -> *mut T {
        self.usize() as *mut T
    }

    #[inline(always)]
    fn virt_addr(&self) -> VirtAddr {
        VirtAddr(self.usize())
    }

    #[inline(always)]
    fn try_as_aligned<A: AlignedVirt>(&self) -> Option<A> {
        if !self.is_aligned(A::ALIGNMENT) {
            return None;
        }
        Some(A::from_usize(self.usize()))
    }

    #[inline(always)]
    fn as_aligned<A: AlignedVirt>(&self) -> A {
        self.try_as_aligned()
            .unwrap_or_else(|| panic!("unaligned address: {}", self.virt_addr()))
    }

    #[inline(always)]
    fn align<A: AlignedVirt>(&self) -> A {
        A::from_usize(self.usize() & A::ALIGNMENT.high_mask())
    }

    fn to_phys(&self) -> PhysAddr {
        let addr = self.usize();
        // only the top 1GB virt is id-mapped
        assert!(
            HIGH_ID_MAP_ADDR <= addr,
            "tried to use the 1GiB id-map on an address outside its range: {}",
            self.virt_addr()
        );
        (addr - HIGH_ID_MAP_ADDR).phys_addr()
    }

    #[inline(always)]
    fn align_offset(&self, alignment: Alignment) -> usize {
        self.usize() & alignment.mask()
    }

    #[inline(always)]
    fn is_aligned(&self, alignment: Alignment) -> bool {
        self.align_offset(alignment) == 0
    }
}

pub trait AlignedVirt: HasVirtAddr {
    const ALIGNMENT: Alignment;

    fn from_usize(n: usize) -> Self;
}

macro_rules! make_addr_struct {
    ($name:ident) => {
        #[repr(C)]
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(usize);
    };
}

macro_rules! impl_addr_trait {
    ($name:ident, $trait:ident) => {
        impl $trait for $name {
            #[inline(always)]
            fn usize(&self) -> usize {
                self.0
            }
        }
    };
}

macro_rules! impl_aligned {
    ($name:ident, $trait:ident, $alignment:ident) => {
        impl $trait for $name {
            const ALIGNMENT: Alignment = $alignment;

            #[inline(always)]
            fn from_usize(n: usize) -> Self {
                Self(n & $alignment.high_mask())
            }
        }

        impl $name {
            pub fn next(self, off: usize) -> Self {
                Self(self.0 + $alignment.size() * off)
            }

            pub fn prev(self, off: usize) -> Self {
                Self(self.0 - $alignment.size() * off)
            }

            pub fn offset(self, off: isize) -> Self {
                Self((self.0 as isize + $alignment.size() as isize * off) as usize)
            }
        }
    };
}

macro_rules! impl_display {
    ($name:ident, $cast_method:ident) => {
        impl Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.$cast_method().fmt(f)
            }
        }
    };
}

make_addr_struct!(PhysAddr);
impl_addr_trait!(PhysAddr, HasPhysAddr);

make_addr_struct!(VirtAddr);
impl_addr_trait!(VirtAddr, HasVirtAddr);

make_addr_struct!(FrameAddr);
make_addr_struct!(PTL2FrameAddr);
make_addr_struct!(PTL3FrameAddr);
make_addr_struct!(PTL4FrameAddr);
impl_display!(FrameAddr, phys_addr);
impl_display!(PTL2FrameAddr, phys_addr);
impl_display!(PTL3FrameAddr, phys_addr);
impl_display!(PTL4FrameAddr, phys_addr);
impl_addr_trait!(FrameAddr, HasPhysAddr);
impl_addr_trait!(PTL2FrameAddr, HasPhysAddr);
impl_addr_trait!(PTL3FrameAddr, HasPhysAddr);
impl_addr_trait!(PTL4FrameAddr, HasPhysAddr);
impl_aligned!(FrameAddr, AlignedPhys, PAGE_ALIGNMENT);
impl_aligned!(PTL2FrameAddr, AlignedPhys, PTL2_ALIGNMENT);
impl_aligned!(PTL3FrameAddr, AlignedPhys, PTL3_ALIGNMENT);
impl_aligned!(PTL4FrameAddr, AlignedPhys, PTL3_ALIGNMENT);

make_addr_struct!(PageAddr);
make_addr_struct!(PTL2PageAddr);
make_addr_struct!(PTL3PageAddr);
make_addr_struct!(PTL4PageAddr);
impl_display!(PageAddr, virt_addr);
impl_display!(PTL2PageAddr, virt_addr);
impl_display!(PTL3PageAddr, virt_addr);
impl_display!(PTL4PageAddr, virt_addr);
impl_addr_trait!(PageAddr, HasVirtAddr);
impl_addr_trait!(PTL2PageAddr, HasVirtAddr);
impl_addr_trait!(PTL3PageAddr, HasVirtAddr);
impl_addr_trait!(PTL4PageAddr, HasVirtAddr);
impl_aligned!(PageAddr, AlignedVirt, PAGE_ALIGNMENT);
impl_aligned!(PTL2PageAddr, AlignedVirt, PTL2_ALIGNMENT);
impl_aligned!(PTL3PageAddr, AlignedVirt, PTL3_ALIGNMENT);
impl_aligned!(PTL4PageAddr, AlignedVirt, PTL3_ALIGNMENT);

impl HasVirtAddr for usize {
    #[inline(always)]
    fn usize(&self) -> usize {
        let mut addr = *self;
        // sign extend to upper bits
        if addr & (1 << (VIRT_ADDR_BITS - 1)) != 0 {
            addr |= usize::MAX << VIRT_ADDR_BITS;
        }
        addr
    }
}

impl<T: ?Sized> HasVirtAddr for *const T {
    #[inline(always)]
    fn usize(&self) -> usize {
        *self as *const () as usize
    }
}

impl<T: ?Sized> HasVirtAddr for *mut T {
    #[inline(always)]
    fn usize(&self) -> usize {
        *self as *mut () as usize
    }
}

impl<T> HasVirtAddr for &T {
    #[inline(always)]
    fn usize(&self) -> usize {
        *self as *const T as usize
    }
}

impl<T> HasVirtAddr for &mut T {
    #[inline(always)]
    fn usize(&self) -> usize {
        *self as *const T as usize
    }
}

impl HasPhysAddr for usize {
    #[inline(always)]
    fn usize(&self) -> usize {
        *self
    }
}

impl Display for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PA-{:016x}", self.0)
    }
}

impl Display for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VA-{:016x}", self.0)
    }
}
