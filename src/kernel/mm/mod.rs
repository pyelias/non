pub mod bitmap_frame_alloc;
pub mod page_alloc;
pub mod slab_alloc;
pub mod bump_alloc;

// i could switch this out for something easier to debug in the future
pub use bitmap_frame_alloc as frame_alloc;

pub use frame_alloc::{alloc_frame_with_order, alloc_frame, free_frame_with_order, free_frame, FrameOrder};
pub use page_alloc::{alloc_page, alloc_l1_entry, alloc_l2_entry};
pub use slab_alloc::{Segment, Slab};