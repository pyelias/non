#include <stddef.h>
#include <stdint.h>

typedef struct vga_entry {
    uint8_t chara;
    uint8_t color;
} vga_entry;

static const size_t WIDTH = 80;

uint8_t color;
size_t row;
size_t col;
vga_entry* terminal_buffer;

void putchar(char c) {
    if (c == '\n') {
        col = 0;
        row++;
        return;
    }
    terminal_buffer[WIDTH * row + col] = (vga_entry){c, color};
    col++;
    if (col == WIDTH) {
        col = 0;
        row++;
    }
}

void write(char* str) {
    for (; *str; str++) {
        putchar(*str);
    }
}

void kernel_main(void) {
    row = 8;
    col = 0;
    color = 7;
    terminal_buffer = (vga_entry*)0xB8000;

    write("in kernel now\nnewline");

    return;
}