#pragma once

#include "types.h"

typedef struct __attribute__((__packed__)) {
    uint64_t lo;
    uint64_t hi;
} idt_entry;

void idt_init(void);