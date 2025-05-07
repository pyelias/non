use core::ptr::NonNull;

use alloc::boxed::Box;

pub struct Node<T> {
    pub val: T,
    next: Option<NonNull<Node<T>>>,
}

impl<T> Node<T> {
    pub fn new_box(val: T) -> Box<Self> {
        Box::new(Self { val, next: None })
    }

    pub fn to_ptr(self: Box<Self>) -> NonNull<Self> {
        unsafe { NonNull::new_unchecked(Box::into_raw(self)) }
    }

    pub unsafe fn from_ptr(ptr: NonNull<Self>) -> Box<Self> {
        Box::from_raw(ptr.as_ptr())
    }
}

pub struct List<T> {
    head: Option<Box<Node<T>>>,
}

pub struct TailList<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
}

impl<T> TailList<T> {
    pub fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    pub fn push_node(&mut self, mut node: Box<Node<T>>) {
        debug_assert!(node.next.is_none());
        node.next = self.head.take();
        let new_head = node.to_ptr();
        self.head = Some(new_head);
        self.tail.get_or_insert(new_head); // if empty, set the tail too
    }

    pub fn push(&mut self, val: T) {
        self.push_node(Node::new_box(val));
    }

    pub fn pop_node(&mut self) -> Option<Box<Node<T>>> {
        let mut node = unsafe { Node::from_ptr(self.head.take()?) };
        self.head = node.next;
        if node.next.is_none() {
            self.tail = None;
        }
        node.next = None; // prefer not to have next pointers set while user has node
        Some(node)
    }

    pub fn pop(&mut self) -> Option<T> {
        self.pop_node().map(|node| node.val)
    }

    pub fn push_back_node(&mut self, node: Box<Node<T>>) {
        debug_assert!(node.next.is_none());
        let node = node.to_ptr();
        if let Some(tail) = self.tail {
            unsafe { (*tail.as_ptr()).next = Some(node) };
        } else {
            // if empty, set the head too
            self.head = Some(node);
        }
        self.tail = Some(node);
    }

    pub fn push_back(&mut self, val: T) {
        self.push_back_node(Node::new_box(val))
    }
}
