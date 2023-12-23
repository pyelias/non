pub trait GetsField<Struct> {
    type FieldType;

    fn get<'a>(st: &'a Struct) -> &'a Self::FieldType;
    fn get_mut<'a>(st: &'a mut Struct) -> &'a mut Self::FieldType;
}

pub trait Stores<T> {
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

impl<T> Stores<T> for T {
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

pub trait StoresIn<HasSlot, Value> {
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

impl<HasSlot, Value> StoresIn<HasSlot, Value> for DefaultSlot
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

impl<Struct, Value, Field: GetsField<Struct>, Subslot: StoresIn<Field::FieldType, Value>>
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
