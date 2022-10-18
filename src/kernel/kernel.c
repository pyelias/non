#include "multiboot.h"
#include "io.h"
#include "types.h"
#include "mm.h"
#include "int.h"

void print_mmap_element(multiboot_memory_map_t *entry) {
    switch (entry->type) {
        case MULTIBOOT_MEMORY_AVAILABLE:
            putstr("available\n");
            break;
        case MULTIBOOT_MEMORY_ACPI_RECLAIMABLE:
            putstr("ACPI\n");
            break;
        case MULTIBOOT_MEMORY_NVS:
            putstr("NVS\n");
            break;
        case MULTIBOOT_MEMORY_BADRAM:
            putstr("defective RAM\n");
            break;
        default:
            putstr("reserved\n");
    }
    putstr("  addr: ");
    putint_with_base(entry->addr, 16);
    putstr("\n  len:  ");
    putint_with_base(entry->len, 16);
    putstr("\n  size: ");
    putint_with_base(entry->size, 10);
    putstr("\n");
}

void print_all_mmap_elements(multiboot_info_t *multiboot_info) {
    size_t length = multiboot_info->mmap_length;
    multiboot_memory_map_t* curr = PHYS_TO_VIRT(multiboot_info->mmap_addr);
    while (length > 0) {
        print_mmap_element(curr);
        length -= curr->size + 4;
        curr = (multiboot_memory_map_t*)((char*)curr + curr->size + 4);
    }
}

void kernel_main(uint32_t multiboot_info_phys, uint32_t magic) {
    multiboot_info_t *multiboot_info = PHYS_TO_VIRT(multiboot_info_phys);

    setup_com1();
    if (magic != MULTIBOOT_BOOTLOADER_MAGIC) {
        putstr("bad magic number, dying now");
        return;
    }

    putstr("in kernel now\n");
    putint_with_base(multiboot_info->flags, 2);
    putchar('\n');
    putint(multiboot_info->mmap_length);
    putchar('\n');
    putstr(PHYS_TO_VIRT(multiboot_info->boot_loader_name));
    putchar('\n');
    print_all_mmap_elements(multiboot_info);
    putchar('\n');

    idt_init();
    mm_init(multiboot_info);

    putstr("low_frame: ");

    putint_with_base(low_frame, 16);
    putchar('\n');

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
    putchar('\n');

    // verify in gdb or whatever
    *(uint64_t*)page1 = 0x123456789ABCDEF0;

    return;
}