#include "multiboot.h"
#include "io.h"
#include "misc.h"
#include "types.h"
#include "mm.h"

#define FRAME_SHIFT 12
#define FRAME_SIZE ((size_t)1 << FRAME_SHIFT)
#define FRAME_LOW_MASK (FRAME_SIZE - 1)
#define FRAME_HIGH_MASK (~FRAME_LOW_MASK)
// can't be more than 14
#define MAX_PAGE_ORDER 10

#define PT_ENTRY_PRESENT (1 << 0)
#define PT_ENTRY_RW (1 << 1)

group_map_entry *(group_maps[16]);
group_size max_group_size;
size_t max_size_group_count;
frame_ptr low_frame, high_frame;

single_page_allocator virt_page_alloc;

void verify_frame_ptr(phys_ptr frame) {
    if (frame & ((1 << 12) - 1)) {
        putstr("invalid page frame ");
        putint_with_base(frame, 16);
        hang();
    }
}

void get_usable_memory(multiboot_info_t *multiboot_info) {
    extern char KERNEL_END_VMA, KERNEL_VMA;
    low_frame = (size_t)(&KERNEL_END_VMA - &KERNEL_VMA);

    high_frame = 0;
    size_t length = multiboot_info->mmap_length;
    multiboot_memory_map_t* curr = PHYS_TO_VIRT(multiboot_info->mmap_addr);
    while (length > 0) {
        if (curr->type == MULTIBOOT_MEMORY_AVAILABLE) {
            phys_ptr avail_end = curr->addr + curr->len;
            if (avail_end > high_frame) {
                high_frame = avail_end;
            }
        }

        length -= curr->size + 4;
        curr = (multiboot_memory_map_t*)((size_t)curr + curr->size + 4);
    }

    low_frame = (low_frame + FRAME_LOW_MASK) & FRAME_HIGH_MASK; // round up to the first fully usable page
    high_frame = high_frame & FRAME_HIGH_MASK; // round down to the last fully usable page
}

// a group avail is either 0 if completely full, or order+1 if a sub-page is available with that order
void set_group_avail(group_num group, group_size size, int avail) {
    /*putstr("setting group ");
    putint_with_base(group, 4);
    putstr(" with size ");
    putint(size);
    putstr(" to avail ");
    putint(avail);
    putchar('\n');*/
    if (size == 0) {
        size_t group_entry = group / 64;
        int entry_pos = group % 64;
        if (avail) {
            group_maps[0][group_entry] |= (group_map_entry)1 << entry_pos;
        } else {
            group_maps[0][group_entry] &= ~((group_map_entry)1 << entry_pos);
        }
    } else if (size == 1) {
        size_t group_entry = group / 32;
        int entry_pos = 2 * (group % 32);
        group_maps[1][group_entry] &= ~((group_map_entry)3 << entry_pos);
        group_maps[1][group_entry] |= (uint64_t)avail << entry_pos;
    } else {
        size_t group_entry = group / 16;
        int entry_pos = 4 * (group % 16);
        group_maps[size][group_entry] &= ~((group_map_entry)15 << entry_pos);
        group_maps[size][group_entry] |= (uint64_t)avail << entry_pos;
    }
    /*putstr("is now ");
    putint(get_group_avail(group, size));
    putchar('\n');*/
}

int get_group_avail(group_num group, group_size size) {
    if (size == 0) {
        size_t group_entry = group / 64;
        int entry_pos = group % 64;
        return (group_maps[0][group_entry] >> entry_pos) & 1;
    } else if (size == 1) {
        size_t group_entry = group / 32;
        int entry_pos = 2 * (group % 32);
        return (group_maps[1][group_entry] >> entry_pos) & 3;
    } else {
        size_t group_entry = group / 16;
        int entry_pos = 4 * (group % 16);
        return (group_maps[size][group_entry] >> entry_pos) & 15;
    }
}

// returns whether it actually changed or not
bool update_group_avail(group_num group, group_size size) {
    int old_avail = get_group_avail(group, size);

    int sub_group_max_order = 1 + 2 * (size - 1);
    int max_sub_avail = 0;
    bool all_empty = true;
    for (int i = 0; i < 4; i++) {
        int sub_avail = get_group_avail(4 * group + i, size - 1);
        if (sub_avail > max_sub_avail) {
            max_sub_avail = sub_avail;
        }
        all_empty &= sub_avail == sub_group_max_order;
    }

    int new_avail;
    if (all_empty) {
        new_avail = 1 + 2 * size;
        if (new_avail > MAX_PAGE_ORDER + 1) {
            new_avail = MAX_PAGE_ORDER + 1;
        }
    } else {
        new_avail = max_sub_avail;
    }
    set_group_avail(group, size, new_avail);

    return old_avail != new_avail;
}

void update_all_parents(group_num group, group_size size) {
    for (; size < max_group_size;)  {
        size++;
        group >>= 2;
        if (!update_group_avail(group, size)) {
            // if nothing changes, early exit
            return;
        }
    }
}

void free_phys_page(frame_ptr page, page_order order) {
    /*putstr("freeing ");
    putuint_with_base(page, 16);
    putstr(" with order ");
    putint(order);
    putchar('\n');*/
    // order must be multiple of two for now, todo improve
    group_size size = order / 2;

    // update the avail of the page's group
    group_num group = page >> (FRAME_SHIFT + order);
    set_group_avail(group, size, order + 1);

    // update the avail of the groups containing it
    update_all_parents(group, size);
}

// returns 0 if not available
frame_ptr alloc_phys_page(page_order order) {
    int req_avail = order + 1;

    group_num curr_group_pos;
    for (curr_group_pos = 0; curr_group_pos < max_size_group_count; curr_group_pos++) {
        if (get_group_avail(curr_group_pos, max_group_size) >= req_avail) {
            break;
        }
    }
    if (curr_group_pos == max_size_group_count) {
        return 0;
    }
    int min_size = order / 2;
    int curr_size = max_group_size - 1;
    while (curr_size >= min_size) {
        for (int i = 0; i < 4; i++) {
            group_num sub_group_pos = 4 * curr_group_pos + i;
            if (get_group_avail(sub_group_pos, curr_size) >= req_avail) {
                curr_group_pos = sub_group_pos;
                break;
            }
        }
        curr_size -= 1;
    }

    set_group_avail(curr_group_pos, min_size, 0);
    update_all_parents(curr_group_pos, min_size);

    return curr_group_pos << (FRAME_SHIFT + order);
}

int advance_page_table_cursor(page_table_cursor *c, pt_entry e) {
    c->table[c->pos++] = e;
    // if i don't have this, it doesn't actually put the changes in the table
    // dunno what kind of optimization it's trying to do but this stops it
    return c->pos;
}

void reset_page_table_cursor(page_table_cursor *c, pt_entry *new_table) {
    c->table = new_table;
    c->pos = 0;
}

void* assign_next_virt_addr() {
    void* res = virt_page_alloc.virt_addr;
    virt_page_alloc.virt_addr += FRAME_SIZE;
    return res;
}

void* alloc_virt_page() {
    phys_ptr phys_frame = alloc_phys_page(0);
    if (phys_frame == 0) {
        return 0;
    }

    void* res = assign_next_virt_addr();
    pt_entry entry = phys_frame | PT_ENTRY_PRESENT | PT_ENTRY_RW;
    // map the virt addr to the newly alloced frame
    if (advance_page_table_cursor(&virt_page_alloc.pml1_cursor, entry) == 512) {
        // if we just filled the last slot of this l1 table
        // use the newly alloced frame as a new l1 table
        reset_page_table_cursor(&virt_page_alloc.pml1_cursor, res);
        // make the l2 table point to it
        if (advance_page_table_cursor(&virt_page_alloc.pml2_cursor, entry) == 511) {
            // we've exhausted the whole l2 table we were allocated
            // just die, i guess
            putstr("single page allocator hit memory limit\n");
            hang();
        }
        // now that we have a fresh l1 table, try again at allocating a page
        return alloc_virt_page();
    }
    return res;
}

void print_alloc_state(group_size start_size) {
    size_t group_count = high_frame >> 12;
    for (group_size size = 0; size <= max_group_size; size++) {
        if (size >= start_size) {
            putstr("printing map for size: ");
            putint(size);
            putchar('\n');
            putint(group_count);
            putstr(" groups\n");

            for (size_t i = 0; i < group_count; i += 16) {
                for (size_t j = i; j < i + 16 && j < group_count; j++) {
                    putint_with_base(get_group_avail(j, size), 16);
                }
                putchar('\n');
            }
        }
        group_count = (group_count + 3) / 4;
    }
}

void frame_alloc_init(void) {
    size_t frame_count = high_frame >> FRAME_SHIFT;

    size_t group_count = frame_count;
    group_size group_size = 0;
    group_map_entry* curr_frame_map = PHYS_TO_VIRT(low_frame);
    while (group_count >= 4) {
        putint(group_count);
        putstr(" groups of order ");
        putint(group_size);
        putchar('\n');

        int bits_per_group;
        if (group_size == 0) {
            bits_per_group = 1;
        } else if (group_size == 1) {
            bits_per_group = 2;
        } else {
            bits_per_group = 4;
        }

        size_t space_req = (bits_per_group * group_count + 63) / 64;
        putstr("uses ");
        putint(space_req);
        putstr(" entries\n");
        group_maps[group_size] = curr_frame_map;
        for (size_t i = 0; i < space_req; i++) {
            // 0 signifies unavailable
            // available pages will be marked later
            curr_frame_map[i] = 0;
        }
        curr_frame_map += space_req;
        low_frame += space_req;
        putuint_with_base((phys_ptr)group_maps[group_size], 16);
        putchar('\n');

        max_group_size = group_size;
        max_size_group_count = group_count;
        group_size += 1;
        group_count = (group_count + 3) / 4;
    }

    low_frame = (low_frame + FRAME_LOW_MASK) & FRAME_HIGH_MASK; // round up to the first fully usable page

    for (frame_ptr frame = low_frame; frame < high_frame; frame += FRAME_SIZE) {
        free_phys_page(frame, 0);
    }
    putstr("total free frames: ");
    putint((high_frame - low_frame) / FRAME_SIZE);
    putchar('\n');
}

pt_entry virt_page_allocator_first_l2[512] __attribute__((aligned(4096)));
pt_entry virt_page_allocator_first_l1[512] __attribute__((aligned(4096)));

void mm_init(multiboot_info_t *multiboot_info) {
    // clear low-address identity mapping setup during boot
    // it's probably fine to just leave it but i dont want to
    volatile pt_entry* ptl4 = PHYS_TO_VIRT(0x1000);
    ptl4[0] = 0;
    volatile pt_entry* ptl3 = PHYS_TO_VIRT(0x2000);
    ptl3[0] = 0;

    get_usable_memory(multiboot_info);
    putint_with_base(low_frame, 16);
    putchar('\n');
    putint_with_base(high_frame, 16);
    putchar('\n');
    
    // for allocator debugging, make the memory really small i guess
    // high_frame = 0x128000;

    frame_alloc_init();

    // start the allocator at the 510th entry of the l3 page table, which is the 511 entry of the l4 table
    virt_page_alloc.virt_addr = (void*)(((size_t)0xffff << 48) | ((size_t)511 << 39) | ((size_t)510 << 30)); 

    // get 2 more unused pages for the single-page-allocator
    ptl3[510] = VIRT_TO_PHYS(&virt_page_allocator_first_l2) | PT_ENTRY_PRESENT | PT_ENTRY_RW;
    reset_page_table_cursor(&virt_page_alloc.pml2_cursor, virt_page_allocator_first_l2);
    // add an entry for the l1 table
    advance_page_table_cursor(&virt_page_alloc.pml2_cursor, VIRT_TO_PHYS(&virt_page_allocator_first_l1) | PT_ENTRY_PRESENT | PT_ENTRY_RW);
    reset_page_table_cursor(&virt_page_alloc.pml1_cursor, virt_page_allocator_first_l1);
}