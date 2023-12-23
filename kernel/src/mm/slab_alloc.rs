use core::{
    alloc::Layout,
    cell::UnsafeCell,
    marker::PhantomPinned,
    mem::{align_of, size_of, MaybeUninit},
    pin::Pin,
    ptr::NonNull,
};

use super::bump_alloc::BumpAllocator;

const SMALL_ALLOC_SEG_SIZE: usize = 1 << 21;
const SMALL_ALLOC_SLAB_SHIFT: usize = 16;
const SMALL_ALLOC_SLAB_SIZE: usize = 1 << SMALL_ALLOC_SLAB_SHIFT;
const SMALL_ALLOC_SLAB_COUNT: usize = SMALL_ALLOC_SEG_SIZE / SMALL_ALLOC_SLAB_SIZE;

struct ObjList(Option<NonNull<ObjList>>);

impl ObjList {
    fn new() -> Self {
        Self(None)
    }

    // Safety: obj must be valid to hold an ObjList
    unsafe fn push(&mut self, obj: *mut ()) {
        let obj = obj as *mut ObjList;
        obj.write(Self(self.0));
        *self = Self(Some(NonNull::new(obj).unwrap()));
    }

    fn pop(&mut self) -> Option<*mut ()> {
        let res = self.0?.as_ptr();
        *self = unsafe { res.read() };
        Some(res as *mut ())
    }
}

pub struct Slab {
    avail: UnsafeCell<ObjList>,
    to_free: UnsafeCell<ObjList>,
}

impl Slab {
    pub fn new(layout: Layout, buffer: &'static mut [MaybeUninit<u8>]) -> Slab {
        assert!(layout.size() >= size_of::<usize>());
        assert!(layout.align() >= align_of::<usize>());

        let mut alloc = BumpAllocator::new_from_slice(buffer);
        let obj_count = alloc.max_allocs_of_layout(layout);
        let obj_buffer = alloc.alloc_slice_layout(layout, obj_count);
        let mut avail_list = ObjList::new();
        for i in 0..obj_count {
            let obj_ptr = (obj_buffer as *mut u8).wrapping_add(layout.size() * i) as *mut ();
            unsafe { avail_list.push(obj_ptr) };
        }
        Slab {
            avail: UnsafeCell::new(avail_list),
            to_free: UnsafeCell::new(ObjList::new()),
        }
    }

    // Safety: must be called from the CPU that owns this Slab
    pub unsafe fn alloc_fast(&self) -> Option<*mut ()> {
        (&mut *self.avail.get()).pop()
    }

    // Safety: obj must have been allocated from this slab & must be called from the CPU that owns this Slab
    pub unsafe fn dealloc(&self, obj: *mut ()) {
        let obj = obj as *mut ObjList;
        let new_to_free = ObjList(Some(NonNull::new(obj).unwrap()));
        let old_to_free = core::mem::replace(&mut *self.avail.get(), new_to_free);
        obj.write(old_to_free);
    }
}

struct SlabSlot {
    buffer: *mut [u8],
    slab: UnsafeCell<Option<Slab>>,
}

pub struct Segment {
    slabs: &'static mut [SlabSlot],
    // needs to be a pointer because we hand out a mutable reference to the Slabs
    // i think this is safe
    slab_size_shift: usize,
    pin: PhantomPinned,
}

impl Segment {
    // Safety: buffer's start and end must be aligned to something something
    pub unsafe fn make_small_alloc(buffer: &'static mut [MaybeUninit<u8>]) -> Pin<&'static Self> {
        assert!(buffer.len() == SMALL_ALLOC_SEG_SIZE);
        let mut alloc = BumpAllocator::new_from_slice(&mut buffer[..SMALL_ALLOC_SEG_SIZE]);

        let seg = alloc.alloc_uninit::<Segment>();
        let slabs = alloc.alloc_slice_uninit::<SlabSlot>(SMALL_ALLOC_SLAB_COUNT);

        let mut init_slabs = 0;
        for entry in slabs.iter_mut() {
            let slab_size =
                SMALL_ALLOC_SLAB_SIZE - ((alloc.curr_ptr() as usize) & (SMALL_ALLOC_SLAB_SIZE - 1));
            if slab_size == 0 {
                break;
            }
            *entry = MaybeUninit::new(SlabSlot {
                buffer: alloc.alloc_slice_ptr::<u8>(slab_size),
                slab: UnsafeCell::new(None),
            });
            init_slabs += 1;
        }
        let slabs = MaybeUninit::slice_assume_init_mut(&mut slabs[..init_slabs]);
        Pin::new_unchecked(seg.write(Segment {
            slabs,
            slab_size_shift: SMALL_ALLOC_SLAB_SHIFT,
            pin: PhantomPinned,
        }))
    }

    // Safety: must be called from the CPU that owns this segment
    pub unsafe fn alloc_slab(self: Pin<&Self>, layout: Layout) -> Option<&'static Slab> {
        for slot in self.slabs.iter() {
            // Safety: we never have a mutable reference or mutate this except later in this function
            if unsafe { &*slot.slab.get() }.is_none() {
                let new_slab = Slab::new(layout, unsafe {
                    &mut *(slot.buffer as *mut [MaybeUninit<u8>])
                });

                // Safety: if the slab is None, nobody except us has a reference to it, so we can mutate it
                unsafe { *slot.slab.get() = Some(new_slab) };

                // Safety: see above
                return unsafe { &*slot.slab.get() }.as_ref();
            }
        }
        None
    }
}

pub struct Heap {}
