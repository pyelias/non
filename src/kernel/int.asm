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

%macro PUSH_REGS 0
    ; flags are automatically saved
    push rdi
    push rsi
    push rdx
    push rcx
    push rax
    push r8
    push r9
    push r10
    push r11
%endmacro 

%macro POP_REGS 0
    pop r11
    pop r10
    pop r9
    pop r8
    pop rax
    pop rcx
    pop rdx
    pop rsi
    pop rdi
%endmacro

extern c_handle_generic
%macro GENERIC_IRQ 1
generic_irq_%1:
    cld
    mov rdi, %1
    call c_handle_generic
%endmacro

%assign i 0
%rep 22
    GENERIC_IRQ i
    %assign i i+1
%endrep

global generic_irq_table
generic_irq_table:
    %assign i 0
    %rep 22
        dq generic_irq_%+i
        %assign i i+1
    %endrep
    %undef i

global asm_handle_double_fault
asm_handle_double_fault:
    ; sysv abi requires direction flag is clear
    cld
    pop rdi
    extern c_handle_double_fault
    call c_handle_double_fault

global asm_handle_test
asm_handle_test:
    PUSH_REGS
    cld
    extern c_handle_test
    call c_handle_test
    POP_REGS
    iretq

global asm_fire_test
asm_fire_test:
    int 50
    ret