use core::{ptr, mem::forget};

/// Self's size is <= T's,  Self's alignment divides T's, and any valid value for T is valid for Self
// so a pointer to a From can be converted to a pointer to a Self
pub unsafe trait TransmuteFrom<T>: Sized {
    fn from_ref(from: &T) -> &Self {
        unsafe { &*(from as *const T as *const Self) }
    }

    fn from_mut(from: &mut T) -> &mut Self {
        unsafe { &mut *(from as *mut T as *mut Self) }
    }

    fn transmute_from(from: T) -> Self {
        let res = &from as *const T as *const Self;
        forget(from);
        unsafe { ptr::read(res) }
    }
}
pub unsafe trait TransmuteInto<T>: Sized {
    fn into_ref(&self) -> &T;
    fn into_mut(&mut self) -> &mut T;
    fn transmute_into(self) -> T;
}

unsafe impl<T, U> TransmuteInto<T> for U where T: TransmuteFrom<U> {
    fn into_ref(&self) -> &T {
        T::from_ref(self)
    }

    fn into_mut(&mut self) -> &mut T {
        T::from_mut(self)
    }

    fn transmute_into(self) -> T {
        T::transmute_from(self)
    }
}

pub unsafe trait TransmuteBetween<T>: Sized + TransmuteFrom<T> + TransmuteInto<T> {}

unsafe impl<T, U> TransmuteBetween<T> for U where U: TransmuteFrom<T>, U: TransmuteInto<T> {}