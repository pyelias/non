// marker types; never constructed
pub struct ObjectSlot;

pub unsafe trait IsSlotOf<Struct> {
    type Value;

    fn get(st: &Struct) -> &Self::Value;
    fn get_mut(st: &mut Struct) -> &mut Self::Value;
}

unsafe impl<T> IsSlotOf<T> for ObjectSlot {
    type Value = T;

    fn get(st: &T) -> &T {
        st
    }

    fn get_mut(st: &mut T) -> &mut T {
        st
    }
}
