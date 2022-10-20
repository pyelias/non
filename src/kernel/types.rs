use core::fmt::Debug;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PhysAddr(usize);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VirtAddr(usize);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FrameAddr(usize);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PageAddr(usize);

// must equal HIGH_ID_MAP_VMA from linker
const HIGH_ID_MAP_ADDR: usize = 0xFFFF_FFFF_C000_0000;
const PAGE_SHIFT: u8 = 12;
const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
const PAGE_LOW_MASK: usize = PAGE_SIZE - 1;
const PAGE_HIGH_MASK: usize = !PAGE_LOW_MASK;

impl FrameAddr {
    pub fn next_frame(self) -> Self {
        Self(self.0 + PAGE_SIZE)
    }
}

pub trait HasPhysAddr : Sized + Copy {
    fn usize(self) -> usize;

    // if we're using a usize as phys addr, we want to be able to Debug it like a PhysAddr
    // but a FrameAddr should still be debugged like a FrameAddr
    type DebugType: Debug;
    fn debug(self) -> Self::DebugType;

    fn phys_addr(self) -> PhysAddr {
        PhysAddr(self.usize())
    }

    fn try_frame_addr(self) -> Option<FrameAddr> {
        self.is_frame_aligned().then_some(FrameAddr(self.usize()))
    }

    fn frame_addr(self) -> FrameAddr {
        assert!(self.is_frame_aligned(), "tried to make a FrameAddr with an unaligned address: {:?}", self.phys_addr());
        FrameAddr(self.usize())
    }

    fn to_virt(self) -> VirtAddr {
        // only the bottom 1GiB phys is id-mapped
        let addr = self.usize();
        assert!(addr < 0x100000, "tried to use the 1GiB id-map on an address outside its range: {:?}", self.phys_addr());
        VirtAddr(addr + HIGH_ID_MAP_ADDR)
    }

    fn frame_offset(self) -> usize {
        self.usize() & PAGE_LOW_MASK
    }

    fn is_frame_aligned(self) -> bool {
        self.frame_offset() == 0
    }

    fn containing_frame(self) -> FrameAddr {
        FrameAddr(self.usize() & PAGE_HIGH_MASK)
    }
}

impl HasPhysAddr for usize {
    fn usize(self) -> usize { self }

    type DebugType = PhysAddr;
    fn debug(self) -> PhysAddr { self.phys_addr() }
}

impl HasPhysAddr for PhysAddr {
    fn usize(self) -> usize { self.0 }

    type DebugType = PhysAddr;
    fn debug(self) -> PhysAddr { self }
}

impl HasPhysAddr for FrameAddr {
    fn usize(self) -> usize { self.0 }

    type DebugType = FrameAddr;
    fn debug(self) -> FrameAddr { self }
}

pub trait HasVirtAddr : Sized + Copy {
    fn ptr<T>(self) -> *mut T;

    type DebugType: Debug;
    fn debug(self) -> Self::DebugType;

    fn usize(self) -> usize {
        self.ptr::<()>() as usize
    }

    fn virt_addr(self) -> VirtAddr {
        VirtAddr(self.usize())
    }

    fn try_page_addr(self) -> Option<PageAddr> {
        self.is_page_aligned().then_some(PageAddr(self.usize()))
    }

    fn page_addr(self) -> PageAddr {
        assert!(self.is_page_aligned(), "tried to make a PageAddr with an unaligned address: {:?}", self.virt_addr());
        PageAddr(self.usize())
    }

    fn to_phys(self) -> PhysAddr {
        let addr = self.usize();
        // only the top 1GB virt is id-mapped
        assert!(HIGH_ID_MAP_ADDR <= addr, "tried to use the 1GiB id-map on an address outside its range: {:?}", self.virt_addr());
        (addr - HIGH_ID_MAP_ADDR).phys_addr()
    }

    fn page_offset(self) -> usize {
        self.usize() & PAGE_LOW_MASK
    }

    fn is_page_aligned(self) -> bool {
        self.page_offset() == 0
    }

    fn containing_page(self) -> PageAddr {
        PageAddr(self.usize() & PAGE_HIGH_MASK)
    }
}

impl<T> HasVirtAddr for *const T {
    fn ptr<U>(self) -> *mut U {
        self as *mut U
    }

    type DebugType = VirtAddr;
    fn debug(self) -> VirtAddr { self.virt_addr() }
}

impl<T> HasVirtAddr for *mut T {
    fn ptr<U>(self) -> *mut U {
        self as *mut U
    }

    type DebugType = VirtAddr;
    fn debug(self) -> VirtAddr { self.virt_addr() }
}

impl HasVirtAddr for VirtAddr {
    fn ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    type DebugType = Self;
    fn debug(self) -> Self { self }
}

impl HasVirtAddr for PageAddr {
    fn ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    type DebugType = Self;
    fn debug(self) -> Self { self }
}

impl Debug for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PA-{:16x}", self.0)
    }
}

impl Debug for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VA-{:16x}", self.0)
    }
}

impl Debug for FrameAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PF-{:16x}", self.0)
    }
}

impl Debug for PageAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VF-{:16x}", self.0)
    }
}