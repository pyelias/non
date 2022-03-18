CC = x86_64-elf-gcc
OBJ_DIR = build/objs

c_src = $(wildcard src/*.c)
asm_src = $(wildcard src/*.asm)
obj =  $(c_src:src/%.c=$(OBJ_DIR)/%.o)
obj += $(asm_src:src/%.asm=$(OBJ_DIR)/%.o)
dep = $(src:src/%.c=$(OBJ_DIR)/%.d)

test.bin: build/full.o
	objcopy -O elf32-i386 build/full.o test.bin

build/full.o: $(obj) src/linker.ld
	ld -m elf_x86_64 -T src/linker.ld -o $@ $(obj)

$(OBJ_DIR)/%.o: src/%.c $(OBJ_DIR) 
	$(CC) -MMD -m64 -c -o $@ -ffreestanding -mcmodel=kernel -O2 -Wall -Wextra $<

$(OBJ_DIR)/%.o: src/%.asm $(OBJ_DIR) 
	nasm -f elf64 -w+orphan-labels -o $@ $<

$(OBJ_DIR):
	mkdir -p $@

-include $(dep)

.PHONY: clean
clean:
	rm -rf build

.PHONY: cleandep
cleandep:
	rm build/*.d
