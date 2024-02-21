use super::{IsSlotOf, Store, Stores};

pub trait ListLink: Sized {
    type Slot: IsSlotOf<Self, Value = LinkStore<Self>>;
    type Store: Stores<Option<Self>>;
}

type LinkStore<Link: ListLink> = Store<Link::Store, Option<Link>>;

pub struct List<ItemLink> {
    head: Option<ItemLink>,
}

impl<'item, Link: 'item + ListLink> List<Link> {
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
