use core::marker::PhantomData;

use super::StoresIn;

pub struct IntrusiveLinkedList<Item, NextSlot> {
    head: Option<Item>,
    _slot: PhantomData<NextSlot>,
}

impl<'item, Item: 'item, NextSlot: StoresIn<Item, Option<Item>>>
    IntrusiveLinkedList<Item, NextSlot>
{
    pub fn empty() -> Self {
        Self {
            head: None,
            _slot: PhantomData,
        }
    }

    pub fn push(&mut self, mut item: Item) {
        let next = self.head.take();
        NextSlot::set(&mut item, next);
        self.head = Some(item);
    }

    pub fn pop(&mut self) -> Option<Item> {
        let mut head = self.head.take()?;
        let rest = NextSlot::take(&mut head);
        self.head = rest;
        Some(head)
    }
}
