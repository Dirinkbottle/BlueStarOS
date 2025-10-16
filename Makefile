# 工具链配置
RUSTC := rustc
CC := riscv64-unknown-elf-gcc
LD := riscv64-unknown-elf-ld
OBJCOPY := riscv64-unknown-elf-objcopy
QEMU := qemu-system-riscv64

# 目标架构
TARGET := riscv64imac-unknown-none-elf

# 构建目录
BUILD_DIR := build

# 源文件
RUST_SRC := src/main.rs
ASM_SRC := src/boot.S
LINKER_SCRIPT := linker.ld

# 输出文件
RUST_OBJ := $(BUILD_DIR)/main.o
BOOT_OBJ := $(BUILD_DIR)/boot.o
KERNEL_ELF := $(BUILD_DIR)/kernel.elf
KERNEL_BIN := $(BUILD_DIR)/kernel.bin

# QEMU 参数
QEMU_OPTS := -machine virt -bios none -kernel $(KERNEL_BIN) -nographic -serial mon:stdio

.PHONY: all clean run

all: $(KERNEL_BIN)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

# 编译 Rust 代码
$(RUST_OBJ): $(RUST_SRC) | $(BUILD_DIR)
	$(RUSTC) \
		--target $(TARGET) \
		-C panic=abort \
		-C link-arg=-T$(LINKER_SCRIPT) \
		-C link-arg=-nostdlib \
		-C opt-level=3 \
		--emit obj=$@ \
		$<

# 编译汇编启动代码
$(BOOT_OBJ): $(ASM_SRC) | $(BUILD_DIR)
	$(CC) -nostdlib -march=rv64imac -mabi=lp64 -c $< -o $@

# 链接内核 - 修复链接脚本多次出现的问题
$(KERNEL_ELF): $(BOOT_OBJ) $(RUST_OBJ) $(LINKER_SCRIPT)
	$(LD) -T $(LINKER_SCRIPT) -nostdlib -o $@ $(BOOT_OBJ) $(RUST_OBJ)

# 生成二进制镜像
$(KERNEL_BIN): $(KERNEL_ELF)
	$(OBJCOPY) -O binary $< $@

# 运行 QEMU
run: $(KERNEL_BIN)
	$(QEMU) $(QEMU_OPTS)

# 清理构建
clean:
	rm -rf $(BUILD_DIR)