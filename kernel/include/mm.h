#pragma once

#include "types.h"
#include "multiboot.h"

typedef uint64_t pt_entry;

typedef uint64_t group_map_entry;

typedef unsigned char page_order;
typedef unsigned char group_size;
typedef size_t group_num;

typedef struct {
    volatile pt_entry* table;
    int pos;
} page_table_cursor;

typedef struct {
    void* virt_addr;
    page_table_cursor pml2_cursor, pml1_cursor;
} single_page_allocator;

extern group_map_entry *(group_maps[16]);
extern group_size max_group_size;
extern frame_ptr low_frame, high_frame;

void mm_init(multiboot_info_t *multiboot_info);
void free_phys_page(frame_ptr page, page_order order);
frame_ptr alloc_phys_page(page_order order);
int get_group_avail(frame_ptr group, group_size size);
void* alloc_virt_page();
void print_alloc_state(group_size start_size);