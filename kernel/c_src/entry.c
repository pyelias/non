#include "multiboot.h"
#include "io.h"
#include "types.h"
#include "mm.h"
#include "int.h"

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

    kernel_main_rust(multiboot_info);
    
    return;
}