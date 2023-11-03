use core::cell::Cell;

#[repr(C)]
pub struct TCB {
    rsp: u64 // stack pointer
}

impl TCB {
    pub fn new_empty() -> Self {
        Self {
            rsp: 0
        }
    }

    pub fn new_with_raw_stack_ptr(stack: *mut u8) -> Self {
        Self {
            rsp: stack as u64
        }
    }

    pub fn new_with_stack(stack: &'static mut [u8], entry_point: fn()) -> Self {
        assert!(stack.len() >= 8 * 7); // room for 6 registers + return address
        let len = stack.len();
        let stack_addr = (stack as *mut [u8] as *mut u8 as u64) + len as u64; // rust why?

        stack[len -  8..len    ].copy_from_slice(&(entry_point as u64).to_ne_bytes());
        stack[len - 16..len - 8].copy_from_slice(&stack_addr.to_ne_bytes());
        Self::new_with_raw_stack_ptr(&mut stack[len - 8 * 7])
    }
}

extern "sysv64" { 
    pub fn switch_to_task(curr_tcb_reg: &mut &mut Cell<TCB>, next_tcb: &mut Cell<TCB>);
}