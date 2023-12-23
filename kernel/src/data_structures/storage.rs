use core::sync::atomic;

// these traits will be used by unsafe code to store pointers, so they must work correctly
// therefore, they are unsafe to implement
pub unsafe trait GetsField<Struct> {
    type FieldType;

    fn get(st: &Struct) -> &Self::FieldType;
    fn get_mut(st: &mut Struct) -> &mut Self::FieldType;
}

pub unsafe trait Stores<T> {
    fn new(val: T) -> Self;
    fn get(&self) -> T
    where
        T: Copy;
    fn replace(&mut self, val: T) -> T;

    #[inline]
    fn set(&mut self, val: T) {
        self.replace(val);
    }

    #[inline]
    fn take(&mut self) -> T
    where
        T: Default,
    {
        self.replace(T::default())
    }
}

unsafe impl<T> Stores<T> for T {
    #[inline]
    fn new(val: T) -> T {
        val
    }

    #[inline]
    fn get(&self) -> T
    where
        T: Copy,
    {
        *self
    }

    #[inline]
    fn replace(&mut self, val: Self) -> Self {
        core::mem::replace(self, val)
    }

    #[inline]
    fn set(&mut self, val: Self) {
        *self = val;
    }
}

pub unsafe trait StoresIn<HasSlot, Value> {
    fn get(has_slot: &HasSlot) -> Value
    where
        Value: Copy;
    fn replace(has_slot: &mut HasSlot, val: Value) -> Value;

    #[inline]
    fn set(has_slot: &mut HasSlot, val: Value) {
        Self::replace(has_slot, val);
    }

    #[inline]
    fn take(has_slot: &mut HasSlot) -> Value
    where
        Value: Default,
    {
        Self::replace(has_slot, Value::default())
    }
}

// marker type; never constructed
pub enum DefaultSlot {}

unsafe impl<HasSlot, Value> StoresIn<HasSlot, Value> for DefaultSlot
where
    HasSlot: Stores<Value>,
{
    #[inline]
    fn get(has_slot: &HasSlot) -> Value
    where
        Value: Copy,
    {
        has_slot.get()
    }

    #[inline]
    fn replace(has_slot: &mut HasSlot, val: Value) -> Value {
        has_slot.replace(val)
    }

    #[inline]
    fn set(has_slot: &mut HasSlot, val: Value) {
        has_slot.set(val);
    }

    #[inline]
    fn take(has_slot: &mut HasSlot) -> Value
    where
        Value: Default,
    {
        has_slot.take()
    }
}

// marker type; never constructed
pub struct FieldSlot<Field, Subslot>(Field, Subslot);

unsafe impl<Struct, Value, Field: GetsField<Struct>, Subslot: StoresIn<Field::FieldType, Value>>
    StoresIn<Struct, Value> for FieldSlot<Field, Subslot>
{
    #[inline]
    fn get(has_field: &Struct) -> Value
    where
        Value: Copy,
    {
        let has_slot = Field::get(has_field);
        Subslot::get(has_slot)
    }

    #[inline]
    fn replace(has_field: &mut Struct, val: Value) -> Value {
        let has_slot = Field::get_mut(has_field);
        Subslot::replace(has_slot, val)
    }

    #[inline]
    fn set(has_field: &mut Struct, val: Value) {
        let has_slot = Field::get_mut(has_field);
        Subslot::set(has_slot, val);
    }

    #[inline]
    fn take(has_field: &mut Struct) -> Value
    where
        Value: Default,
    {
        let has_slot = Field::get_mut(has_field);
        Subslot::take(has_slot)
    }
}

pub unsafe trait StoresAtomic<T> {
    fn new(val: T) -> Self;
    fn load(&self, ordering: atomic::Ordering) -> T
    where
        T: Copy;
    fn store(&self, val: T, ordering: atomic::Ordering);
    fn swap(&self, val: T, ordering: atomic::Ordering) -> T;
    fn compare_exchange(
        &self,
        current: T,
        new: T,
        success: atomic::Ordering,
        failure: atomic::Ordering,
    ) -> Result<T, T>;
    fn compare_exchange_weak(
        &self,
        current: T,
        new: T,
        success: atomic::Ordering,
        failure: atomic::Ordering,
    ) -> Result<T, T>;
}

pub unsafe trait StoresInAtomic<HasSlot, Value> {
    fn load(has_slot: &HasSlot, ordering: atomic::Ordering) -> Value
    where
        Value: Copy;
    fn store(has_slot: &HasSlot, val: Value, ordering: atomic::Ordering);
    fn swap(has_slot: &HasSlot, val: Value, ordering: atomic::Ordering) -> Value;
    fn compare_exchange(
        has_slot: &HasSlot,
        current: Value,
        new: Value,
        success: atomic::Ordering,
        failure: atomic::Ordering,
    ) -> Result<Value, Value>
    where
        Value: Copy;
    fn compare_exchange_weak(
        has_slot: &HasSlot,
        current: Value,
        new: Value,
        success: atomic::Ordering,
        failure: atomic::Ordering,
    ) -> Result<Value, Value>
    where
        Value: Copy;
}

unsafe impl<Struct, Value, Field: GetsField<Struct>, Subslot: StoresInAtomic<Field::FieldType, Value>>
    StoresInAtomic<Struct, Value> for FieldSlot<Field, Subslot>
{
    fn load(has_field: &Struct, ordering: atomic::Ordering) -> Value
    where
        Value: Copy,
    {
        let has_slot = Field::get(has_field);
        Subslot::load(has_slot, ordering)
    }

    fn store(has_field: &Struct, val: Value, ordering: atomic::Ordering) {
        let has_slot = Field::get(has_field);
        Subslot::store(has_slot, val, ordering);
    }

    fn swap(has_field: &Struct, val: Value, ordering: atomic::Ordering) -> Value {
        let has_slot = Field::get(has_field);
        Subslot::swap(has_slot, val, ordering)
    }

    fn compare_exchange(
        has_field: &Struct,
        current: Value,
        new: Value,
        success: atomic::Ordering,
        failure: atomic::Ordering,
    ) -> Result<Value, Value>
    where
        Value: Copy,
    {
        let has_slot = Field::get(has_field);
        Subslot::compare_exchange(has_slot, current, new, success, failure)
    }

    fn compare_exchange_weak(
        has_field: &Struct,
        current: Value,
        new: Value,
        success: atomic::Ordering,
        failure: atomic::Ordering,
    ) -> Result<Value, Value>
    where
        Value: Copy,
    {
        let has_slot = Field::get(has_field);
        Subslot::compare_exchange_weak(has_slot, current, new, success, failure)
    }
}

unsafe impl<HasSlot, Value> StoresInAtomic<HasSlot, Value> for DefaultSlot
where
    HasSlot: StoresAtomic<Value>,
{
    #[inline]
    fn load(has_slot: &HasSlot, ordering: atomic::Ordering) -> Value
    where
        Value: Copy,
    {
        has_slot.load(ordering)
    }

    #[inline]
    fn store(has_slot: &HasSlot, val: Value, ordering: atomic::Ordering) {
        has_slot.store(val, ordering);
    }

    #[inline]
    fn swap(has_slot: &HasSlot, val: Value, ordering: atomic::Ordering) -> Value {
        has_slot.swap(val, ordering)
    }

    #[inline]
    fn compare_exchange(
        has_slot: &HasSlot,
        current: Value,
        new: Value,
        success: atomic::Ordering,
        failure: atomic::Ordering,
    ) -> Result<Value, Value> {
        has_slot.compare_exchange(current, new, success, failure)
    }

    #[inline]
    fn compare_exchange_weak(
        has_slot: &HasSlot,
        current: Value,
        new: Value,
        success: atomic::Ordering,
        failure: atomic::Ordering,
    ) -> Result<Value, Value> {
        has_slot.compare_exchange_weak(current, new, success, failure)
    }
}
