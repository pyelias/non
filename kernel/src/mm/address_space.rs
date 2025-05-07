use crate::{
    data_structures::{
        intrusive_list::{IntrusiveList, ListLink},
        IsSlotOf, Store, Stores,
    },
    types::PageAddr,
};

// right now, linked list of memory regions
// later, rb-tree or something

struct Region {
    start: PageAddr,
    size: usize,
    next: Store<Option<RawRegionLink>, Option<RegionLink>>,
}

type RawRegionLink = *mut Region;
type RegionLink = &'static mut Region;

unsafe impl Stores<Option<RegionLink>> for Option<RawRegionLink> {
    unsafe fn store(val: &Option<RegionLink>) -> Self {
        val.as_ref().map(|p| *p as *const Region as *mut Region)
    }

    unsafe fn extract(store: Self) -> Option<RegionLink> {
        store.map(|p| &mut *p)
    }
}

struct NextLinkSlot;
unsafe impl IsSlotOf<RegionLink> for NextLinkSlot {
    type Value = Store<Option<RawRegionLink>, Option<RegionLink>>;

    fn get(st: &RegionLink) -> &Self::Value {
        &st.next
    }

    fn get_mut(st: &mut RegionLink) -> &mut Self::Value {
        &mut st.next
    }
}

impl ListLink for RegionLink {
    type Slot = NextLinkSlot;
    type Store = Option<RawRegionLink>;
}

struct AddressSpace {
    regions: IntrusiveList<RegionLink>,
}
