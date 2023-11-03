#include "multiboot.h"
#include "io.h"
#include "types.h"
#include "mm.h"
#include "int.h"

extern void print_all_mmap_elements(multiboot_info_t *info);
extern void kernel_main_rust(multiboot_info_t *info);

void kernel_main(uint32_t multiboot_info_phys, uint32_t magic) {
    multiboot_info_t *multiboot_info = PHYS_TO_VIRT(multiboot_info_phys);

    setup_com1();
    if (magic != MULTIBOOT_BOOTLOADER_MAGIC) {
        putstr("bad magic number, dying now");
        return;
    }

    putstr("in kernel now\n");

    idt_init();

    //mm_init(multiboot_info);
    kernel_main_rust(multiboot_info);
    return;
    // old C mm
    /* mm_init(multiboot_info);
    putstr("all init done\n");

    print_alloc_state(4);

    // 1024 page-frames consecutively (unmapped)
    frame_ptr page0 = alloc_phys_page(10);

    print_alloc_state(4);

    putstr("page0: ");
    putuint_with_base(page0, 16);
    putchar('\n');

    // 1 page-frame + a virtual address mapping
    void* page1 = alloc_virt_page();

    print_alloc_state(4);

    putstr("page1: ");
    putuint_with_base((size_t)page1, 16);
    putchar('\n');*/

    putstr("causing a page fault\n");
    putstr("error message next, hopefully\n");
    *(volatile uint64_t*)(0x1000) = 0x123456789ABCDEF0;

    return;
}