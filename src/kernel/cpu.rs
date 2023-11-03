use core::cell::Cell;

use crate::{task::TCB, sync::SpinLockGuard};

#[repr(C)]
struct CPUInfo {
    curr_task: SpinLockGuard<'static, TCB>
}