use core::fmt::Display;
use super::page::{PAGE_SIZE, PAGE_LOW_MASK, PAGE_HIGH_MASK};

const ID_MAP_SIZE: usize = 1 << 30;
// must equal HIGH_ID_MAP_VMA from linker
const HIGH_ID_MAP_ADDR: usize = 0xFFFF_FFFF_C000_0000;
const VIRT_ADDR_BITS: usize = 48;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PhysAddr(usize);

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct VirtAddr(usize);

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct FrameAddr(usize);

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PageAddr(usize);

impl FrameAddr {
    pub fn next_frame(self) -> Self {
        Self(self.0 + PAGE_SIZE)
    }
}

impl PageAddr {
    pub fn next_page(self) -> Self {
        Self(self.0 + PAGE_SIZE)
    }
}

pub trait HasPhysAddr {
    fn usize(&self) -> usize;

    // if we're using a usize as phys addr, we want to be able to Display it like a PhysAddr
    // but a FrameAddr should still be displayed like a FrameAddr
    fn display(&self) -> impl Display {
        self.phys_addr()
    }

    fn phys_addr(&self) -> PhysAddr {
        PhysAddr(self.usize())
    }

    fn try_frame_addr(&self) -> Option<FrameAddr> {
        self.is_frame_aligned().then_some(FrameAddr(self.usize()))
    }

    fn frame_addr(&self) -> FrameAddr {
        assert!(self.is_frame_aligned(), "tried to make a FrameAddr with an unaligned address: {}", self.phys_addr());
        FrameAddr(self.usize())
    }

    fn to_virt(&self) -> VirtAddr {
        // only the bottom 1GiB phys is id-mapped
        let addr = self.usize();
        assert!(addr < ID_MAP_SIZE, "tried to use the 1GiB id-map on an address outside its range: {}", self.phys_addr());
        VirtAddr(addr + HIGH_ID_MAP_ADDR)
    }

    fn frame_offset(&self) -> usize {
        self.usize() & PAGE_LOW_MASK
    }

    fn is_frame_aligned(&self) -> bool {
        self.frame_offset() == 0
    }

    fn containing_frame(&self) -> FrameAddr {
        FrameAddr(self.usize() & PAGE_HIGH_MASK)
    }
}

impl HasPhysAddr for usize {
    fn usize(&self) -> usize { *self }
}

impl HasPhysAddr for PhysAddr {
    fn usize(&self) -> usize { self.0 }
}

impl HasPhysAddr for FrameAddr {
    fn usize(&self) -> usize { self.0 }

    fn display(&self) -> Self { *self }
}

pub trait HasVirtAddr {
    fn usize(&self) -> usize;

    fn display(&self) -> impl Display {
        self.virt_addr()
    }

    fn ptr<T>(&self) -> *mut T {
        self.usize() as *mut T
    }

    fn virt_addr(&self) -> VirtAddr {
        VirtAddr(self.usize())
    }

    fn try_page_addr(&self) -> Option<PageAddr> {
        self.is_page_aligned().then_some(PageAddr(self.usize()))
    }

    fn page_addr(&self) -> PageAddr {
        assert!(self.is_page_aligned(), "tried to make a PageAddr with an unaligned address: {}", self.virt_addr());
        PageAddr(self.usize())
    }

    fn to_phys(&self) -> PhysAddr {
        let addr = self.usize();
        // only the top 1GB virt is id-mapped
        assert!(HIGH_ID_MAP_ADDR <= addr, "tried to use the 1GiB id-map on an address outside its range: {}", self.virt_addr());
        (addr - HIGH_ID_MAP_ADDR).phys_addr()
    }

    fn page_offset(&self) -> usize {
        self.usize() & PAGE_LOW_MASK
    }

    fn is_page_aligned(&self) -> bool {
        self.page_offset() == 0
    }

    fn containing_page(&self) -> PageAddr {
        PageAddr(self.usize() & PAGE_HIGH_MASK)
    }
}

impl HasVirtAddr for usize {
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
    fn usize(&self) -> usize {
        *self as *const () as usize
    }
}

impl<T: ?Sized> HasVirtAddr for *mut T {
    fn usize(&self) -> usize {
        *self as *mut () as usize
    }
}

impl<T> HasVirtAddr for &T {
    fn usize(&self) -> usize {
        *self as *const T as usize
    }
}

impl<T> HasVirtAddr for &mut T {
    fn usize(&self) -> usize {
        *self as *const T as usize
    }
}

impl HasVirtAddr for VirtAddr {
    fn usize(&self) -> usize {
        self.0
    }
}

impl HasVirtAddr for PageAddr {
    fn usize(&self) -> usize {
        self.0
    }

    fn display(&self) -> Self { *self }
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

impl Display for FrameAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PF-{:016x}", self.0)
    }
}

impl Display for PageAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VP-{:016x}", self.0)
    }
}