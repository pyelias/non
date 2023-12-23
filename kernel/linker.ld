ENTRY(_start)

BOOTSTRAP_LMA = 1M;
HIGH_ID_MAP_VMA = 0xFFFFFFFFC0000000;

SECTIONS
{
    . = BOOTSTRAP_LMA;
    .bootstrap_text ALIGN(4K) :
    {
        KEEP(*(.multiboot))
        *(.bootstrap_text)
    }

    .bootstrap_data ALIGN(4K) :
    {
        *(.bootstrap_data)
    }

    .bootstrap_rodata ALIGN(4K) :
    {
        *(.bootstrap_rodata)
    }

    .bootstrap_bss ALIGN(4K) :
    {
        *(.bootstrap_bss)
    }

    . = ALIGN(4K);
    KERNEL_LMA = .;

    /* relocate kernel code so that
       the top 2GB of virtual memory can map to the bottom 2GB of physical
    */
    . = HIGH_ID_MAP_VMA + KERNEL_LMA;

    .text ALIGN(4K) : AT(KERNEL_LMA)
    {
        *(.text*)
        *(.got)
        *(.eh_frame*)
    }

    .data ALIGN(4K) :
    {
        *(.data*)
    }

    .rodata ALIGN(4K) :
    {
        *(.rodata*)
    }

    .bss ALIGN(4K) :
    {
        *(.bss*)
    }

    KERNEL_END_VMA = .;
}