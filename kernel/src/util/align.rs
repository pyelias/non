use core::{mem::align_of, ops::Range};

#[derive(Copy, Clone, Debug)]
pub struct Alignment(usize);

impl Alignment {
    pub const fn new(n: usize) -> Self {
        assert!(n.is_power_of_two());
        Self(n)
    }

    pub const fn new_from_shift(shift: usize) -> Self {
        Self(1 << shift)
    }

    pub const fn of<T>() -> Self {
        Self(align_of::<T>())
    }

    pub fn as_usize(self) -> usize {
        self.0
    }

    pub fn mask(self) -> usize {
        self.0 - 1
    }

    pub fn shift(self) -> usize {
        self.0.ilog2() as usize
    }

    pub fn is_aligned(self, addr: usize) -> bool {
        addr & self.mask() == 0
    }

    pub fn align_down_offset(self, addr: usize) -> usize {
        addr & self.mask()
    }

    pub fn align_down(self, addr: usize) -> usize {
        addr & !self.mask()
    }

    pub fn align_up_offset(self, addr: usize) -> usize {
        (addr.wrapping_add(self.mask())) & self.mask()
    }

    pub fn checked_align_up(self, addr: usize) -> Option<usize> {
        Some((addr.checked_add(self.mask())?) & !self.mask())
    }

    pub fn align_up(self, addr: usize) -> usize {
        (addr + self.mask()) & !self.mask()
    }

    pub fn split_range(self, range: Range<usize>) -> (usize, Range<usize>, usize) {
        (
            self.align_up_offset(range.start),
            self.align_up(range.start)..self.align_down(range.end),
            self.align_down_offset(range.end),
        )
    }
}
