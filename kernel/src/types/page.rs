use core::mem::MaybeUninit;

pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const PAGE_LOW_MASK: usize = PAGE_SIZE - 1;
pub const PAGE_HIGH_MASK: usize = !PAGE_LOW_MASK;

#[repr(align(4096))] // PAGE_SIZE = 4096, but i can't use a const here i guess
#[derive(Clone, Copy)]
pub struct Page([MaybeUninit<u8>; PAGE_SIZE]);

impl Page {
    pub const fn zeroed() -> Self {
        Self([MaybeUninit::zeroed(); PAGE_SIZE])
    }
}
