void hang(void) {
    while (1) {
        asm volatile (
            "cli\n"
            "hlt\n"
        );
    }
}