use core::{marker::PhantomData, sync::atomic::{AtomicPtr, Ordering}, ptr};

use crate::types::{StoresIn, ptr_from_option_mut};

pub struct IntrinsicLinkedList<Item, NextSlot> {
    head: Option<Item>,
    _slot: PhantomData<NextSlot>
}

impl<'item, Item: 'item, NextSlot: StoresIn<Item, Option<Item>>> IntrinsicLinkedList<Item, NextSlot> {
    pub fn empty() -> Self {
        Self {
            head: None,
            _slot: PhantomData
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

/* TODO figure this out
do i even need this?
mimalloc uses it, but i might not use mimalloc (b/c not real-time)
whatever i replace it with also might use it, and there are other things i could use it for
but it could be difficult to implement (ABA problem?)

pub type AtomicNextPtr<'item, Item> = AtomicPtr<Item>;

pub struct AtomicIntrinsicLinkedList<'item, Item, NextSlot> {
    head: AtomicNextPtr<'item, Item>,
    _slot: PhantomData<NextSlot>,
    _item: PhantomData<&'item mut Item>
}

impl<'item, Item, NextSlot: StoresIn<Item, AtomicNextPtr<'item, Item>>> AtomicIntrinsicLinkedList<'item, Item, NextSlot> {
    pub fn empty() -> Self {
        Self {
            head: AtomicPtr::new(ptr::null_mut()),
            _slot: PhantomData,
            _item: PhantomData
        }
    }

    pub fn push(&self, item: &'item mut Item) {
        let item_ptr = ptr_from_option_mut(Some(item));
        let curr_head = self.head.load(Ordering::Relaxed);
        while self.head.compare_exchange_weak(curr_head, item_ptr, Ordering::Release, Ordering::Relaxed)
    }
}*/