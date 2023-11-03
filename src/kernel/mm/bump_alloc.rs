use core::{mem::{size_of, align_of, MaybeUninit}, marker::PhantomData, ptr::slice_from_raw_parts_mut, slice::from_raw_parts_mut, alloc::Layout};

use crate::types::HasVirtAddr;

// the lifetime of buffer is the same as the lifetime of the allocator
pub struct BumpAllocator<'a> {
    buffer: *mut u8,
    len: usize,
    marker: PhantomData<&'a mut [MaybeUninit<u8>]>
}

impl<'a> BumpAllocator<'a> {
    // Safety: buffer must not be aliased while this allocator exists
    pub unsafe fn new(buffer: *mut u8, len: usize) -> Self {
        Self {
            buffer,
            len,
            marker: PhantomData
        }
    }

    pub fn new_from_slice(buffer: &'a mut [MaybeUninit<u8>]) -> Self {
        unsafe { Self::new(buffer.as_mut_ptr() as *mut u8, buffer.len()) }
    }

    pub fn remaining(&self) -> usize { self.len }

    fn advance(&mut self, amt: usize) {
        if amt > self.len {
            panic!("requested more than available space from BumpAllocator buffer: {}", amt);
        }
        self.buffer = self.buffer.wrapping_add(amt);
        self.len -= amt;
    }

    pub fn alloc_bytes(&mut self, size: usize) -> *mut u8 {
        println!("bump allocating at {}", self.buffer.virt_addr());
        let res = self.buffer;
        self.advance(size);
        res
    }

    pub fn align(&mut self, align: usize) {
        assert!(align.is_power_of_two());
        let addr_mod_align = (self.buffer as usize - 1) & (align - 1);
        let off = align - addr_mod_align - 1;
        self.advance(off);
    }

    pub fn align_to<T>(&mut self) {
        self.align(align_of::<T>())
    }

    pub fn alloc_layout(&mut self, layout: Layout) -> *mut () {
        self.align(layout.align());
        self.alloc_bytes(layout.size()) as *mut ()
    }

    pub fn alloc_ptr<T>(&mut self) -> *mut T {
        self.alloc_layout(Layout::new::<T>()) as *mut T
    }

    pub fn alloc_uninit<T>(&mut self) -> &'a mut MaybeUninit<T> {
        unsafe { &mut *self.alloc_ptr::<MaybeUninit<T>>() }
    }

    pub fn alloc_default<T: Default>(&mut self) -> &'a mut T {
        self.alloc_uninit::<T>().write(T::default())
    }

    pub fn alloc_slice_layout(&mut self, layout: Layout, len: usize) -> *mut () {
        let layout = layout.repeat(len).unwrap().0;
        self.alloc_layout(layout)
    }

    pub fn alloc_slice_ptr<T>(&mut self, len: usize) -> *mut [T] {
        self.align_to::<T>();
        slice_from_raw_parts_mut(self.alloc_bytes(size_of::<T>() * len) as *mut T, len)
    }

    pub fn alloc_slice_uninit<T>(&mut self, len: usize) -> &'a mut [MaybeUninit<T>] {
        let ptr = self.alloc_slice_ptr::<MaybeUninit<T>>(len);
        unsafe { &mut *ptr }
    }

    pub fn alloc_slice_default<T: Default>(&mut self, len: usize) -> &'a mut [T] {
        let res = self.alloc_slice_uninit::<T>(len);
        for elem in res.iter_mut() {
            elem.write(T::default());
        }
        unsafe { MaybeUninit::slice_assume_init_mut(res) }
    }

    pub fn max_allocs_of_layout(&self, layout: Layout) -> usize {
        assert_ne!(layout.size(), 0, "max_allocs_of zero-sized-type");
        // no need to think about alignment
        // size is >= align, so the padding needed to align the first one isn't enough to fit an extra
        self.remaining() / layout.size()
    }

    pub fn max_allocs_of<T>(&self) -> usize {
        self.max_allocs_of_layout(Layout::new::<T>())
    }

    pub fn curr_ptr(&self) -> *mut u8 {
        self.buffer
    }

    pub fn done(mut self) -> &'a mut [MaybeUninit<u8>] {
        self.alloc_slice_uninit::<u8>(self.len)
    }

    pub fn done_ptr(self) -> *mut u8 {
        self.buffer
    }
}