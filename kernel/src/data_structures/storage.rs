use atomic_traits::Atomic;
use core::{marker::PhantomData, sync::atomic::Ordering};

// this trait could be used to copy types that are not Copy
// so it is unsafe
// the Store and AtomicStore types provide a safe API
pub unsafe trait Stores<T>: Copy {
    unsafe fn store(val: &T) -> Self;
    unsafe fn extract(store: Self) -> T;
}

// a value of type T stored as type S
#[repr(transparent)]
pub struct Store<S, T>(S, PhantomData<T>);

impl<S: Copy, T: Copy> Clone for Store<S, T> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}
impl<S: Copy, T: Copy> Copy for Store<S, T> {}

impl<S: Stores<T>, T> Store<S, T> {
    pub fn new(val: T) -> Self {
        Self(unsafe { S::store(&val) }, PhantomData)
    }

    unsafe fn new_raw(val: S) -> Self {
        Self(val, PhantomData)
    }

    pub fn into_inner(self) -> T {
        unsafe { S::extract(self.0) }
    }

    pub fn get(&self) -> T
    where
        T: Copy,
    {
        unsafe { S::extract(self.0) }
    }

    pub fn replace(&mut self, val: T) -> T {
        let new = unsafe { S::store(&val) };
        let old = core::mem::replace(&mut self.0, new);
        unsafe { S::extract(old) }
    }

    pub fn set(&mut self, val: T) {
        self.replace(val);
    }

    pub fn take(&mut self) -> T
    where
        T: Default,
    {
        self.replace(T::default())
    }
}

pub struct AtomicStore<S, T>(S, PhantomData<T>);

impl<S: Atomic, T> AtomicStore<S, T>
where
    S::Type: Stores<T> + Copy,
{
    pub fn new(val: T) -> Self {
        Self(unsafe { S::new(S::Type::store(&val)) }, PhantomData)
    }

    pub fn into_inner(self) -> T {
        unsafe { S::Type::extract(self.0.into_inner()) }
    }

    pub fn load_raw(&self, order: Ordering) -> S::Type {
        self.0.load(order)
    }

    pub fn compare_exchange_weak(
        &self,
        old: S::Type,
        new: T,
        success: Ordering,
        failure: Ordering,
    ) -> Result<T, (S::Type, T)> {
        let new_raw = unsafe { S::Type::store(&new) };
        match self.0.compare_exchange_weak(old, new_raw, success, failure) {
            Ok(old) => Ok(unsafe { S::Type::extract(old) }),
            Err(old) => Err((old, new)),
        }
    }
}
