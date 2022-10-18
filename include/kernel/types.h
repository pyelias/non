#pragma once

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>


// i'd like these not to implicitly cast into each other
// but not sure how to do that easily
typedef size_t phys_ptr;
typedef phys_ptr frame_ptr;

extern const phys_ptr PHYS_ZERO_POINTER;
#define PHYS_TO_VIRT(addr) ((void*)((phys_ptr)(addr) + PHYS_ZERO_POINTER))
// only works for addresses > 0xFFFFFFFFC0000000 (those are mapped trivially)
#define VIRT_TO_PHYS(addr) ((phys_ptr)((void*)(addr) - PHYS_ZERO_POINTER))