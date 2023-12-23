KERNEL_BUILD_OUT_DIR := $(dir $(realpath $(lastword $(MAKEFILE_LIST))))kernel/target/x86_64-unknown-kernel/debug
KERNEL_DEPS := $(KERNEL_BUILD_OUT_DIR)/sparkle.d
KERNEL_OBJ := $(KERNEL_BUILD_OUT_DIR)/sparkle

$(KERNEL_OBJ) $(KERNEL_DEPS): kernel/build.rs
	cd kernel && cargo build -q -p sparkle --profile dev

include $(KERNEL_DEPS)

isodir/boot/nonos.bin: $(KERNEL_OBJ)
	cp $< $@

nonos.iso: isodir/boot/nonos.bin
	grub-mkrescue -o nonos.iso --xorriso=../xorriso/xorriso isodir

.PHONY: build run debug

build: nonos.iso

run: nonos.iso
	qemu-system-x86_64 -cdrom nonos.iso

debug: nonos.iso
	qemu-system-x86_64 -s -S -d int -D qemu-log.txt -cdrom nonos.iso