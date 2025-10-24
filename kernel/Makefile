# Building
TARGET := riscv64gc-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin
DISASM_TMP := target/$(TARGET)/$(MODE)/asm

# User program settings
USER_DIR := ../user
USER_TARGET := $(TARGET)
USER_MODE := release
USER_BIN ?= printf  # 可以通过 make USER_BIN=test_buffer 切换
USER_ELF := $(USER_DIR)/target/$(USER_TARGET)/$(USER_MODE)/$(USER_BIN)
KERNEL_USER_ELF := te.elf

# Building mode argument
ifeq ($(MODE), release)
	MODE_ARG := --release
endif

# BOARD
BOARD := qemu
SBI ?= rustsbi
BOOTLOADER := ./bootloader/$(SBI)-$(BOARD).bin

# KERNEL ENTRY
KERNEL_ENTRY_PA := 0x80200000

# Binutils
OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

# Disassembly
DISASM ?= -x

build: env $(KERNEL_BIN)

env:
	(rustup target list | grep "riscv64gc-unknown-none-elf (installed)") || rustup target add $(TARGET)
	cargo install cargo-binutils
	rustup component add rust-src
	rustup component add llvm-tools-preview

# Build user program and copy to kernel directory
$(KERNEL_USER_ELF): $(USER_ELF)
	@echo "Copying user program $(USER_BIN) to $(KERNEL_USER_ELF)"
	@cp $(USER_ELF) $(KERNEL_USER_ELF)

$(USER_ELF):
	@echo "Building user program: $(USER_BIN)"
	@cd $(USER_DIR) && cargo build --bin $(USER_BIN) --release --target $(USER_TARGET)

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

kernel: $(KERNEL_USER_ELF)
	@echo Platform: $(BOARD)
	@cargo build $(MODE_ARG)

clean:
	@cargo clean
	@cd $(USER_DIR) && cargo clean
	@rm -f $(KERNEL_USER_ELF)

# Convenience target to build only user program
user: $(KERNEL_USER_ELF)

disasm: kernel
	@$(OBJDUMP) $(DISASM) $(KERNEL_ELF) | less
	
disasm-vim: kernel
	@$(OBJDUMP) $(DISASM) $(KERNEL_ELF) > $(DISASM_TMP)
	@vim $(DISASM_TMP)
	@rm $(DISASM_TMP)

run: run-inner

run-inner: build
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios $(BOOTLOADER) \
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) -s  

debug: build
	@tmux new-session -d \
		"qemu-system-riscv64 -machine virt -nographic -bios $(BOOTLOADER) -device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) -s -S" && \
		tmux split-window -h "riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'" && \
		tmux -2 attach-session -d

gdbserver: build
	@qemu-system-riscv64 -machine virt -nographic -bios $(BOOTLOADER) -device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) -s 

gdbclient:
	@riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'

.PHONY: build env kernel clean user disasm disasm-vim run-inner gdbserver gdbclient $(USER_ELF)
