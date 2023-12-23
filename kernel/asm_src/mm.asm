global flush_tlb
flush_tlb:
    mov rax, cr3
    mov cr3, rax
    retq