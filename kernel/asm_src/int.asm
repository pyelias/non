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

extern handle_generic_interrupt
%macro GENERIC_IRQ 1
generic_irq_%1:
    cld
    mov rdi, %1
    call handle_generic_interrupt
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
    extern handle_double_fault_interrupt
    call handle_double_fault_interrupt

global asm_handle_test
asm_handle_test:
    PUSH_REGS
    cld
    extern handle_test_interrupt
    call handle_test_interrupt
    POP_REGS
    iretq