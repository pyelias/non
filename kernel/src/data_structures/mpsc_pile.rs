use super::{StoresAtomic, StoresInAtomic};
use core::{marker::PhantomData, sync::atomic::Ordering};

#[derive(Clone, Copy)]
pub enum NextPtr<Item> {
    End,
    Next(Item),
    Soon,
}

impl<Item> NextPtr<Item> {
    fn is_ready(&self) -> bool {
        match self {
            Self::Soon => false,
            _ => true,
        }
    }

    fn try_to_option(self) -> Option<Option<Item>> {
        match self {
            Self::End => Some(None),
            Self::Next(item) => Some(Some(item)),
            Self::Soon => None,
        }
    }
}

pub struct Pile<Item, NextStore, NextSlot> {
    head: NextStore,
    _phantom: PhantomData<(Item, NextSlot)>,
}

impl<'item, Item, NextStore, NextSlot> Pile<Item, NextStore, NextSlot>
where
    Item: Copy + 'item,
    NextStore: StoresAtomic<NextPtr<Item>>,
    NextSlot: StoresInAtomic<Item, NextPtr<Item>>,
{
    pub fn empty() -> Self {
        Self {
            head: NextStore::new(NextPtr::End),
            _phantom: PhantomData,
        }
    }

    pub fn push(&self, item: Item) {
        NextSlot::store(&item, NextPtr::Soon, Ordering::Relaxed);
        let rest_of_list = self.head.swap(NextPtr::Next(item), Ordering::AcqRel);
        NextSlot::store(&item, rest_of_list, Ordering::Release);
    }

    pub fn take(&self) -> TakenPile<Item, NextStore, NextSlot> {
        let list = self.head.swap(NextPtr::End, Ordering::AcqRel);
        TakenPile {
            head: NextStore::new(list),
            _phantom: PhantomData,
        }
    }
}

pub struct TakenPile<Item, NextStore, NextSlot> {
    head: NextStore,
    _phantom: PhantomData<(Item, NextSlot)>,
}

impl<'item, Item, NextStore, NextSlot> TakenPile<Item, NextStore, NextSlot>
where
    Item: Copy + 'item,
    NextStore: StoresAtomic<NextPtr<Item>>,
    NextSlot: StoresInAtomic<Item, NextPtr<Item>>,
{
    pub fn pop(&mut self) -> Option<Item> {
        let first = self
            .head
            .load(Ordering::Relaxed)
            .try_to_option()
            .expect("Soon should never be in TakenPile head")?;
        let next = loop {
            // wait for the next ptr to stop being Soon before putting it in self.head
            // this won't take long, there's only one operation between setting to Soon and setting to Next
            let next = NextSlot::load(&first, Ordering::Acquire);
            if next.is_ready() {
                break next;
            }
        };
        self.head.store(next, Ordering::Relaxed);
        Some(first)
    }
}
