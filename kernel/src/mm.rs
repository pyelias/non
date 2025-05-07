pub mod address_space;
pub mod bitmap_frame_alloc;
pub mod bump_alloc;
pub mod page_alloc;
pub mod page_table;
pub mod slab_alloc;

// i could switch this out for something easier to debug in the future
pub use bitmap_frame_alloc as frame_alloc;

pub use frame_alloc::{
    alloc_frame, alloc_frame_with_order, free_frame, free_frame_with_order, FrameOrder,
};
pub use page_alloc::{alloc_l1_entry, alloc_page};
pub use slab_alloc::{Segment, Slab};
