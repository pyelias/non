pub unsafe trait Zeroable: Sized {}

pub const fn zeroed<T: Zeroable>() -> T {
    unsafe { core::mem::zeroed() }
}

pub unsafe fn zero_ptr<'a, T: Zeroable>(ptr: *mut T) -> &'a mut T {
    core::ptr::write_bytes(ptr, 0, 1);
    unsafe { &mut *ptr }
}
