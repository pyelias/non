struc TCB
    .rsp resw 1
endstruc

global switch_to_task
switch_to_task:
    ; rdi = pointer to current TCB register
    ; rsi = pointer to target task TCB
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15

    mov rax, [rdi] ; rax = pointer to current TCB
    mov [rax + TCB.rsp], rsp
    mov [rdi], rsi ; current TCB register = new TCB
    mov rsp, [rsi + TCB.rsp]

    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    ret