#include <stdint.h>
#include <stddef.h>

#define outb(port, data) asm volatile ("outb %%al,%%dx" : : "a" (data), "d" (port))

#define COM1 0x3F8

void setup_com1(void) {
    outb(COM1 + 1, 0x00);    // Disable all interrupts
    outb(COM1 + 3, 0x80);    // Enable DLAB (set baud rate divisor)
    outb(COM1 + 0, 0x03);    // Set divisor to 3 (lo byte) 38400 baud
    outb(COM1 + 1, 0x00);    //                  (hi byte)
    outb(COM1 + 3, 0x03);    // 8 bits, no parity, one stop bit
    outb(COM1 + 2, 0xC7);    // Enable FIFO, clear them, with 14-byte threshold
    outb(COM1 + 4, 0x0B);    // IRQs enabled, RTS/DSR set
    outb(COM1 + 4, 0x1E);    // Set in loopback mode, test the serial chip
    outb(COM1 + 0, 0xAE);    // Test serial chip (send byte 0xAE and check if serial returns same byte)

    // Check if serial is faulty (i.e: not same byte as sent)
    /*if(inb(COM1 + 0) != 0xAE) {
        return 1;
    }*/

    // If serial is not faulty set it in normal operation mode
    // (not-loopback with IRQs enabled and OUT#1 and OUT#2 bits enabled)
    outb(COM1 + 4, 0x0F);
    return;
}

void putchar(char c) {
    if (c == '\n') {
        outb(COM1, '\r');
    }
    outb(COM1, c);
}

void putstr(const char* str) {
    for (; *str; str++) {
        putchar(*str);
    }
}

void putint_with_base(int64_t n, uint8_t base) {
    if (n < 0) {
        n = -n;
        putchar('-');
    }

    char digits[30];
    int i = 30;
    if (n == 0) {
        digits[--i] = '0';
    }
    while (n > 0) {
        digits[--i] = "0123456789abcdef"[n % base];
        n /= base;
    }
    while (i < 30) {
        putchar(digits[i++]);
    }
}

void putuint_with_base(uint64_t n, int base) {
    char digits[30];
    int i = 30;
    if (n == 0) {
        digits[--i] = '0';
    }
    while (n > 0) {
        digits[--i] = "0123456789abcdef"[n % base];
        n /= base;
    }
    while (i < 30) {
        putchar(digits[i++]);
    }
}

void putint(int64_t n) {
    putint_with_base(n, 10);
}

void putuint(uint64_t n) {
    putuint_with_base(n, 10);
}