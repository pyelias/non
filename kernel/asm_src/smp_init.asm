[section .smp_init_low alloc exec nowrite progbits align=16]
smp_init:
mov byte [0xB8500], 'g'

hang:
    cli
    hlt
    jmp hang