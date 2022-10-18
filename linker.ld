ENTRY(_start)

BOOTSTRAP_LMA = 1M;
KERNEL_VMA = 0xFFFFFFFFC0000000;

SECTIONS
{
    . = BOOTSTRAP_LMA;
    .bootstrap_text ALIGN(4K) :
    {
        *(.multiboot)
        *(.bootstrap_text)
    }

    .bootstrap_bss ALIGN(4K) :
    {
        *(.bootstrap_bss)
    }

    .bootstrap_rodata ALIGN(4K) :
    {
        *(.bootstrap_rodata)
    }

    . = ALIGN(4K);
    KERNEL_LMA = .;

    /* relocate kernel code so that
       the top 2GB of virtual memory can map to the bottom 2GB of physical
    */
    . = KERNEL_VMA + KERNEL_LMA;

    .text ALIGN(4K) : AT(KERNEL_LMA)
    {
        *(.text)
    }

    .data ALIGN(4K) :
    {
        *(.data)
    }

    .rodata ALIGN(4K) :
    {
        *(.rodata)
    }

    .bss ALIGN(4K) :
    {
        *(.bss)
    }

    KERNEL_END_VMA = .;
}