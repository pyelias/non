[section .data]
align 8
global IDT
IDT:
    ; 256 entries with 16 bytes each
    dq 256 dup (0, 0),
    .desc:
    dw $ - IDT - 1
    dq IDT

[section .text]
global asm_set_idt
asm_set_idt:
    lidt [IDT.desc]
    ret

global asm_handle_double_fault
asm_handle_double_fault:
    cld
    extern c_handle_double_fault
    call c_handle_double_fault
    iretq

global asm_fire_double_fault
asm_fire_double_fault:
    ret