use crate::types::{HasPhysAddr, HasVirtAddr, FrameAddr};
use crate::multiboot::{self, MMapEntryType};

// 48 bits in a phys addr, but lower 12 are all 0
const PT_FRAME_BITS: usize = 0xFFFF_FFFF_F000;

const PRESENT_FLAG: usize = 1 << 0;
const WRITABLE_FLAG: usize = 1 << 1;
const SUPERVISOR_ONLY_FLAG: usize = 1 << 2;
const WRITE_THROUGH_FLAG: usize = 1 << 3;
const DISABLE_CACHE_FLAG: usize = 1 << 4;
const ACCESSED_FLAG: usize = 1 << 5;
const AVAILABLE_FLAG: usize = 1 << 6; // not used for anything in hardware, free for the os
const BIG_FLAG: usize = 1 << 7;
const EXEC_DISABLE_FLAG: usize = 1 << 63;

const FLAGS_MASK: usize = 0x8000_0000_0000_00ff; 

#[repr(C)]
pub struct NormalPTFlags(usize);

impl NormalPTFlags {
    fn none() -> Self {
        Self(0)
    }

    fn present(self) -> Self {
        Self(self.0 | PRESENT_FLAG)
    }

    fn not_present(self) -> Self {
        Self(self.0 & !PRESENT_FLAG)
    }

    fn writable(self) -> Self {
        Self(self.0 & WRITABLE_FLAG)
    }
}

#[repr(C)]
pub struct NormalPTEntry(usize);

impl NormalPTEntry {
    fn empty() -> Self {
        Self(0)
    }

    fn at_frame(addr: FrameAddr) -> Self {
        Self(addr.usize())
    }

    fn get_frame(self) -> FrameAddr {
        (self.0 & PT_FRAME_BITS).frame_addr()
    }

    fn with_frame(self, addr: FrameAddr) -> Self {
        Self((self.0 & !PT_FRAME_BITS) | (addr.usize() & PT_FRAME_BITS))
    }

    fn flags(self) -> NormalPTFlags {
        NormalPTFlags(self.0)
    }

    fn with_flags(self, flags: NormalPTFlags) -> Self {
        Self((self.0 & !FLAGS_MASK) | (flags.0 & FLAGS_MASK))
    }
}

extern "sysv64" {
    static HIGH_ID_MAP_VMA: ();
    static KERNEL_END_VMA: ();
}

#[no_mangle]
extern "sysv64" fn get_usable_memory(info: &multiboot::Info, low_frame: &mut FrameAddr, high_frame: &mut FrameAddr) {
    // safe because we don't care about the values of those things
    // just their addresses
    let kernel_phys_end = unsafe { core::ptr::addr_of!(KERNEL_END_VMA).usize() - core::ptr::addr_of!(HIGH_ID_MAP_VMA).usize() };
    *low_frame = kernel_phys_end.containing_frame().next_frame();

    let mut high_addr = 0;
    for entry in info.mmap_entries() {
        if let MMapEntryType::Available = entry.type_() {
            high_addr = core::cmp::max(high_addr, entry.addr.usize() + entry.len);
        }
    }
    *high_frame = high_addr.containing_frame();
}