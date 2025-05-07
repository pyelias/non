use super::{IsSlotOf, Store, Stores};

pub trait ListLink: Sized {
    type Slot: IsSlotOf<Self, Value = Store<Self::Store, Option<Self>>>;
    type Store: Stores<Option<Self>>;
}

pub struct IntrusiveList<ItemLink> {
    head: Option<ItemLink>,
}

impl<Link: ListLink> IntrusiveList<Link> {
    pub fn empty() -> Self {
        Self { head: None }
    }

    pub fn push(&mut self, mut item: Link) {
        let next = self.head.take();
        Link::Slot::get_mut(&mut item).set(next);
        self.head = Some(item);
    }

    pub fn pop(&mut self) -> Option<Link> {
        let mut head = self.head.take()?;
        let rest = Link::Slot::get_mut(&mut head).take();
        self.head = rest;
        Some(head)
    }
}
