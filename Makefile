CC = x86_64-elf-gcc
CFLAGS = -MMD -m64 -c -ffreestanding -mcmodel=kernel -mno-red-zone -O2 -Wall -Wextra
AS = nasm
ASFLAGS = -f elf64 -w+orphan-labels

KERNEL_SRC_DIR = src/kernel
KERNEL_OBJ_DIR = build/objs/kernel
KERNEL_DEP_DIR = build/deps/kernel

kernel_c_src = $(wildcard $(KERNEL_SRC_DIR)/*.c)
kernel_asm_src = $(wildcard $(KERNEL_SRC_DIR)/*.asm)
kernel_obj =  $(kernel_c_src:$(KERNEL_SRC_DIR)/%.c=$(KERNEL_OBJ_DIR)/%.c.o)
kernel_obj += $(kernel_asm_src:$(KERNEL_SRC_DIR)/%.asm=$(KERNEL_OBJ_DIR)/%.asm.o)
kernel_dep =  $(src:src/%.c=$(KERNEL_DEP_DIR)/%.c.d)
kernel_dep += $(src:src/%.asm=$(KERNEL_DEP_DIR)/%.asm.d)

test.bin: build/kernel.o
	objcopy -O elf32-i386 build/kernel.o test.bin

build/kernel.o: $(kernel_obj) linker.ld
	ld -m elf_x86_64 -T linker.ld -o $@ $(kernel_obj)

$(KERNEL_OBJ_DIR)/%.c.o: $(KERNEL_SRC_DIR)/%.c $(KERNEL_OBJ_DIR) 
	$(CC) $(CFLAGS) -Iinclude/kernel -o $@ $<

$(KERNEL_OBJ_DIR)/%.asm.o: $(KERNEL_SRC_DIR)/%.asm $(KERNEL_OBJ_DIR) $(KERNEL_DEP_DIR)
	$(AS) $(ASFLAGS) -MD $(KERNEL_DEP_DIR)/$*.asm.d -o $@ $<

$(KERNEL_OBJ_DIR) $(KERNEL_DEP_DIR):
	mkdir -p $@

-include $(kernel_dep)

.PHONY: clean
clean:
	rm -rf build

.PHONY: cleandep
cleandep:
	rm build/*.d
