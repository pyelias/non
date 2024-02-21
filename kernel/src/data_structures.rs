pub mod list;
mod slot;
mod storage;

pub use slot::{IsSlotOf, ObjectSlot};
pub use storage::{AtomicStore, Store, Stores};
