pub mod page;
mod addr;
pub mod page_table;
mod storage;
mod ptr;

pub use page::Page;
pub use addr::{PhysAddr, FrameAddr, PTL2FrameAddr, PTL3FrameAddr, PTL4FrameAddr, VirtAddr, PageAddr, PTL2PageAddr, PTL3PageAddr, PTL4PageAddr, HasPhysAddr, AlignedPhys, HasVirtAddr, AlignedVirt};
pub use storage::{GetsField, Stores, StoresIn, DefaultSlot, FieldSlot};
pub use ptr::{ptr_from_option_ref, ptr_from_option_mut};