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
USER_BIN_DIR := $(USER_DIR)/src/bin
USER_TARGET_DIR := $(USER_DIR)/target/$(USER_TARGET)/$(USER_MODE)

# 自动发现所有用户程序
USER_APPS := $(basename $(notdir $(wildcard $(USER_BIN_DIR)/*.rs)))
USER_ELFS := $(foreach app,$(USER_APPS),$(USER_TARGET_DIR)/$(app))

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

# Python script to generate app.asm
APP_ASM_SCRIPT := build_apps.py
APP_ASM := src/app.asm

# Generate app.asm from user programs
$(APP_ASM): $(USER_BIN_DIR)/*.rs
	@echo "==> Generating app.asm from user programs..."
	@python3 $(APP_ASM_SCRIPT)

# Build all user programs
user-apps: $(USER_ELFS)

$(USER_ELFS): $(USER_BIN_DIR)/*.rs
	@echo "==> Building all user programs..."
	@cd $(USER_DIR) && cargo build --release --target $(USER_TARGET)
	@echo "✓ Built $(words $(USER_APPS)) user program(s): $(USER_APPS)"

# Build kernel binary
$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

# Build kernel (depends on user apps and app.asm)
kernel: $(APP_ASM) user-apps
	@echo "==> Building kernel..."
	@echo "    Platform: $(BOARD)"
	@echo "    Mode: $(MODE)"
	@cargo build $(MODE_ARG)
	@echo "✓ Kernel built successfully"

clean:
	@echo "==> Cleaning build artifacts..."
	@cargo clean
	@cd $(USER_DIR) && cargo clean
	@rm -f $(APP_ASM)
	@echo "✓ Clean complete"

# Convenience targets
user: user-apps
	@echo "✓ All user programs built"

# List all discovered user applications
list-apps:
	@echo "Discovered user applications:"
	@echo "$(USER_APPS)" | tr ' ' '\n' | nl

# Force regenerate app.asm
regen-asm:
	@rm -f $(APP_ASM)
	@$(MAKE) $(APP_ASM)

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

.PHONY: build env kernel clean user user-apps disasm disasm-vim run-inner gdbserver gdbclient list-apps regen-asm
