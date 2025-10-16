#!/bin/bash

rustc \
    --target riscv64imac-unknown-none-elf \
    -C panic=abort \
    -C link-arg=-Tlinker.ld \
    -C link-arg=-nostdlib \
    -C opt-level=3 \
    --emit obj=build/main.o \
    src/main.rs

riscv64-unknown-elf-gcc -nostdlib -march=rv64imac -mabi=lp64 -c src/boot.S  -o build/boot.o

riscv64-unknown-elf-ld \
    -T linker.ld \
    -nostdlib \
    -o build/kernel.elf \
    build/boot.o \
    build/main.o

    riscv64-unknown-elf-objcopy -O binary build/kernel.elf build/kernel.bin

    qemu-system-riscv64 \
    -machine virt \
    -bios none \
    -kernel build/kernel.bin \
    -nographic