#include "int.h"
#include "io.h"
#include "misc.h"

#define IDT_INTERRUPT_GATE (0b1110 << 8)
#define IDT_TRAP_GATE (0b1111 << 8)

extern idt_entry IDT[256];
extern void asm_set_idt(void);
extern void asm_handle_double_fault(void);
extern void asm_handle_test(void);
extern void asm_fire_test(void);
extern void (*generic_irq_table[22])(void);
extern const char GDT_long_mode_code_offset;
#define LONG_MODE_CODE_SELECTOR ((uint16_t)(size_t)&GDT_long_mode_code_offset)

idt_entry make_idt_entry(void* off, uint16_t sel, uint16_t flags) {
    idt_entry res = {0};
    phys_ptr off_addr = (phys_ptr)off;
    flags |= 1 << 15; // set present bit
    res.lo |= off_addr & 0xFFFF;
    res.lo |= (uint64_t)sel << 16;
    res.lo |= (uint64_t)flags << 32;
    res.lo |= (off_addr & 0xFFFF0000) << 32;
    res.hi |= off_addr >> 32;
    return res;
}

void idt_init(void) {
    for (int i = 0; i < 22; i++) {
        IDT[i] = make_idt_entry(generic_irq_table[i], LONG_MODE_CODE_SELECTOR, IDT_INTERRUPT_GATE);
        
    }
    IDT[8] = make_idt_entry(&asm_handle_double_fault, LONG_MODE_CODE_SELECTOR, IDT_TRAP_GATE);
    IDT[50] = make_idt_entry(&asm_handle_test, LONG_MODE_CODE_SELECTOR, IDT_INTERRUPT_GATE);
    asm_set_idt();
    asm_fire_test();
}

void c_handle_generic(uint64_t vector) {
    putstr("some interrupt happened\n");
    putstr("vector: ");
    putint(vector);
    putchar('\n');
    hang();
}

void c_handle_double_fault(uint64_t error) {
    putstr("double fault occurred, halting\n");
    putstr("error code: ");
    putint(error);
    putchar('\n');
    putstr("aren't you glad i handled this instead of just restarting?\n");
    hang();
}

void c_handle_test() {
    putstr("printing this from an interrupt\n");
}