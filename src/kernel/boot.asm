bits 32
    
MBALIGN     equ 1 << 0 ; page-aligned
MEMINFO     equ 1 << 1 ; provide memory map
FLAGS       equ MBALIGN | MEMINFO
MAGIC       equ 0x1BADB002
CHECKSUM    equ -(MAGIC + FLAGS)

; multiboot header
[section .multiboot alloc noexec nowrite progbits align=4]
align 4
    dd MAGIC
    dd FLAGS
    dd CHECKSUM

; space for a small stack
[section .bootstrap_bss alloc noexec write nobits align=4]
align 16
stack_bottom:
resb 1 << 16
stack_top:

[section .bootstrap_text alloc exec nowrite progbits align=16]
global _start:function (_start.end - _start)
_start:
    mov esp, stack_top ; setup stack

    ; save multiboot stuff for call_kernel_main
    push eax
    push ebx

    mov byte [0xB8500], 'a'
    mov byte [0xB8501], 7 ; light grey on black

    call is_cpuid_supported
    mov esi, err_no_cpuid
    jz error

    call is_long_mode_supported
    mov esi, err_no_long_mode
    jz error

    ; todo make this work
    ; call load_tss

    call set_up_paging

    call prepare_for_long_mode

    ; load the GDT with a 32 bit physical address
    lgdt [GDT.desc]
    ; use the GDT to jump to 64-bit mode
    jmp GDT.long_mode_code:.in_long_mode
.in_long_mode:
    bits 64

    ; reload the GDT, now with a 64 bit virtual address
    extern HIGH_ID_MAP_VMA
    add qword [GDT.desc_base_addr], HIGH_ID_MAP_VMA
    lgdt [GDT.desc]

    add rsp, HIGH_ID_MAP_VMA
    mov rax, call_kernel_main
    jmp rax
    bits 32
.end:

error:
    call puts
    jmp hang

; takes string address in esi
puts:
    mov edi, 0xB8500
.loop:
    lodsb
    test al, al
    jz .end
    mov ah, 7
    stosw
    jmp .loop
.end:
    ret

hang:
    cli
    hlt
    jmp hang

; eax is zero and zero flag set if not
is_cpuid_supported:
    ; push EFLAGS twice
    pushfd
    pushfd

    xor dword [esp], 1 << 21 ; flip bit 21 in TOS

    ; set EFLAGS with flipped bit, then put the new value in eax
    popfd
    pushfd
    pop eax

    ; make eax non-zero on bits where storing and loading it changed it
    xor eax, [esp]
    and eax, 1 << 21 ; just check if bit 21 changed

    popfd ; restore EFLAGS from first pushed copy

    ret

; zero flag set if not
is_long_mode_supported:
    ; check whether cpuid leaf 0x80000001 is supported
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb .no

    ; check whether long mode is supported
    mov eax, 0x80000001
    cpuid
    test edx, 1 << 29
    ret

.no:
    xor eax, eax
    ret

load_tss:
    ; lower half of TSS virt addr in eax
    mov eax, TSS
    or eax, 0x80000000

    mov edi, GDT + GDT.TSS
    mov [edi], word TSS.size-1 ; 16 bits limit
    mov [edi + 2], word ax     ; 16 bits base
    shr eax, 16                ;
    mov [edi + 4], byte al     ; 8 bits base
    mov [edi + 5], byte 0x89   ; 8 bits base
    mov [edi + 6], byte 0      ; 4 bit limit + 4 bit flags
    mov [edi + 7], byte ah     ; 8 bits base
    mov [edi + 8], dword 0     ; 16 bits base
    mov [edi + 12], dword 0    ; 16 bits base


    mov eax, GDT.TSS
    ; ltr ax

    ret

set_up_paging:
    ; clear paging bit in cr0
    mov eax, cr0
    and eax, ~(1 << 31)
    mov cr0, eax

    ; now, make page tables
    ; 0x1000: level 4 PML4T (page map level-4 table)
    ; 0x2000: level 3 PDPT (page directory pointer table)
    ; 0x3000: level 2 PDT (page directory table)

    ; 0x1000 will point to 0x2000 at 0 and 511
    ; 0x2000 will point to 0x3000 at 0 and 511
    ; 0x3000 will identity map the bottom 1gb

    ; first, zero all 3 pages
    mov edi, 0x1000
    mov cr3, edi
    xor eax, eax
    mov ecx, 3 * 1024
    ; 3 * 1024 times, set 4 bytes at [edi] to 0 and add 4 to edi
    ; so zero 3 * 1024 * 4 bytes or 3 pages
    rep stosd
    mov edi, cr3

    .ENTRY_PRESENT  equ 1 << 0
    .ENTRY_RW       equ 1 << 1
    .ENTRY_PAGESIZE equ 1 << 7

    ; now, add entries for 1-4
    ; 0x1000 (L4) points to:
    ; 0x2000 at 0b000000000 and 0b111111111
    mov dword [edi], 0x2000 | .ENTRY_PRESENT | .ENTRY_RW
    mov dword [edi + 511 * 8], 0x2000 | .ENTRY_PRESENT | .ENTRY_RW
    add edi, 0x1000
    ; 0x2000 (L3) points to:
    ; 0x3000 at 0b000000000 and 0b111111111
    mov dword [edi], 0x3000 | .ENTRY_PRESENT | .ENTRY_RW
    mov dword [edi + 511 * 8], 0x3000 | .ENTRY_PRESENT | .ENTRY_RW
    add edi, 0x1000
    ; 0x3000 (L2) points to:
    ; 2MB at 0MB at 0b000000000
    ; 2MB at 2MB at 0b000000001
    ; etc.
    ; identity map 1GB total
    mov ecx, 512
    mov eax, .ENTRY_PRESENT | .ENTRY_RW | .ENTRY_PAGESIZE ; pdt entry
.id_map_loop:
    mov dword [edi], eax
    add eax, 0x200000 ; point next entry to next 2MB
    add edi, 8        ; move to location of next entry
    loop .id_map_loop

    ret

prepare_for_long_mode:
    mov eax, cr4
    or eax, 1 << 5 ; set PAE bit
    mov cr4, eax

    mov ecx, 0xC0000080
    rdmsr ; reads the EFER MSR
    or eax, 1 << 8 ; set long mode bit
    wrmsr

    mov eax, cr0
    or eax, 1 << 31 ; set paging bit
    mov cr0, eax

    ret

[section .bootstrap_data alloc noexec write progbits align=4]
GDT:
    ; access bits
    .PRESENT    equ 1 << 7
    .NOT_SYS    equ 1 << 4
    .EXEC       equ 1 << 3
    .RW         equ 1 << 1

    ; flag bits
    .PAGE_GRAN  equ 1 << 7
    .SIZE_32    equ 1 << 6
    .LONG_MODE  equ 1 << 5

    .64_bit_TSS equ 0x9

    ; args are access and flags
    %macro gdt_entry 2
        dw 0xFFFF     ; 16 bits of limit
        dw 0          ; 16 bits of base
        db 0          ; 8 bits of base
        db %1         ; access byte
        db (%2) | 0xF ; flags and 4 bits of limit
        db 0          ; 8 bits of base
    %endmacro

    .null: equ $ - GDT
        dq 0
    .long_mode_code: equ $ - GDT
        gdt_entry (.PRESENT | .NOT_SYS | .EXEC | .RW), (.PAGE_GRAN | .LONG_MODE)
    .data: equ $ - GDT
        gdt_entry (.PRESENT | .NOT_SYS | .RW), (.PAGE_GRAN | .SIZE_32)
    .TSS: equ $ - GDT
        dq 0, 0

    ; GDT descriptor
    .desc:
        dw $ - GDT - 1 ; size - 1
    .desc_base_addr:
        dq GDT         ; location
; this needs to be accessed from C
global GDT_long_mode_code_offset
GDT_long_mode_code_offset: equ GDT.long_mode_code

TSS:
    dq 13 dup 0
    .size equ $ - TSS

section .text
call_kernel_main:
    bits 64

    ; eax and ebx from multiboot got pushed earlier, and they're still there
    mov edi, [rsp] ; pop ebx for info
    mov esi, [rsp + 4] ; pop eax for magic
    add rsp, 8

    extern kernel_main
    call kernel_main

.halt_loop:
    cli
    hlt
    jmp .halt_loop

    bits 32

[section .bootstrap_rodata progbits alloc noexec nowrite align=4]
err_no_cpuid: db "cpuid not available", 0
err_no_long_mode: db "long mode not available", 0