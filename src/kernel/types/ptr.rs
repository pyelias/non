use core::{sync::atomic::AtomicPtr, marker::PhantomData};

pub fn ptr_from_option_ref<T>(x: Option<&T>) -> *const T {
    x.map_or(core::ptr::null(), |x| x as *const T)
}

pub fn ptr_from_option_mut<T>(x: Option<&mut T>) -> *mut T {
    x.map_or(core::ptr::null_mut(), |x| x as *mut T)
}

pub unsafe fn ptr_to_option_ref<'a, T>(x: *const T) -> Option<&'a T> {
    if x.is_null() {
        None
    } else {
        Some(&*x)
    }
}

pub unsafe fn ptr_to_option_mut<'a, T>(x: *mut T) -> Option<&'a mut T> {
    if x.is_null() {
        None
    } else {
        Some(&mut *x)
    }
}