use core::{
    arch::asm,
    ptr::addr_of,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::hang::hang;

pub struct IdtEntryBuilder {
    pub offset: usize,
    pub segment_selector: u16,
    pub ist: u8,
    pub gate_type: GateType,
    pub dpl: Ring,
}

impl IdtEntryBuilder {
    pub fn new(offset: usize) -> Self {
        extern "sysv64" {
            // these are linker variables; their addresses matter, but they have no values
            static GDT_long_mode_code_offset: u8;
        }

        Self {
            offset,
            segment_selector: addr_of!(GDT_long_mode_code_offset) as u16,
            ist: 0,
            gate_type: GateType::Interrupt,
            dpl: Ring::Kernel,
        }
    }

    pub fn with_gate_type(mut self, gate_type: GateType) -> Self {
        self.gate_type = gate_type;
        self
    }
}

#[repr(C)]
pub struct IdtEntry {
    lo: AtomicU64,
    hi: AtomicU64,
}

impl IdtEntry {
    pub unsafe fn clear(&self) {
        self.lo.store(0, Ordering::Relaxed);
        // hi doesn't matter, the present flag is in lo
    }

    pub unsafe fn set(&self, builder: IdtEntryBuilder) {
        let offset_hi = (builder.offset as u64) >> 32;
        let offset_mid = (builder.offset as u64) >> 16 & ((1 << 16) - 1);
        let offset_lo = (builder.offset as u64) & ((1 << 16) - 1);

        let hi = offset_hi;
        self.hi.store(hi, Ordering::Relaxed);
        let lo = (offset_lo)
            | ((builder.segment_selector as u64) << 16)
            | (((builder.ist & 7) as u64) << 32)
            | ((builder.gate_type as u64) << 40)
            | ((builder.dpl as u64) << 45)
            | (1 << 47) // set present
            | (offset_mid << 48);
        self.lo.store(lo, Ordering::Relaxed);
    }
}

pub enum Ring {
    Kernel = 0,
    User = 3,
}

pub enum GateType {
    Interrupt = 14,
    Trap = 15,
}

pub static IDT: [IdtEntry; 256] = [const {
    IdtEntry {
        lo: AtomicU64::new(0),
        hi: AtomicU64::new(0),
    }
}; 256];

#[no_mangle]
fn handle_generic_interrupt(vec: u64) {
    println!("some interrupt happened");
    println!("vector: {}", vec);
    hang();
}

#[no_mangle]
fn handle_double_fault_interrupt(error: u64) {
    println!("double fault occurred, halting");
    println!("error code: {}", error);
    println!("aren't you glad i handled this instead of just restarting?\n");
    hang();
}

#[no_mangle]
fn handle_test_interrupt() {
    println!("printing this from an interrupt");
}

fn set_idt() {
    #[repr(C, packed)]
    struct IdtDescriptor {
        size: u16,
        addr: *const IdtEntry,
    }

    let desc = IdtDescriptor {
        size: (size_of_val(&IDT) - 1) as u16,
        addr: addr_of!(IDT) as *const _,
    };

    unsafe {
        asm!(
            "lidt [{desc}]",
            desc = in(reg) &desc
        )
    };
}

pub fn init() {
    extern "sysv64" {
        // these are linker variables; their addresses matter, but they have no values
        static generic_irq_table: [usize; 22];
        fn asm_handle_double_fault();
        fn asm_handle_test();
    }

    for i in 0..22 {
        unsafe { IDT[i].set(IdtEntryBuilder::new(generic_irq_table[i])) };
    }

    unsafe {
        IDT[8].set(
            IdtEntryBuilder::new(asm_handle_double_fault as usize).with_gate_type(GateType::Trap),
        )
    };

    unsafe {
        IDT[50].set(IdtEntryBuilder::new(asm_handle_test as usize).with_gate_type(GateType::Trap))
    };

    set_idt();

    unsafe {
        asm!("int 50");
    }
}
