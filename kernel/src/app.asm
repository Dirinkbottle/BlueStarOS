

#被链接到data段
.section .data.app
.global app_list_start
.global app_list_end
app_list_start:
    .quad app_1_start
    .quad app_1_end
    .quad app_2_start
    .quad app_2_end
    .quad app_3_start
    .quad app_3_end
    .quad app_4_start
    .quad app_4_end
    .quad app_5_start
    .quad app_5_end
    .quad app_6_start
    .quad app_6_end
    .quad app_7_start
    .quad app_7_end
    .quad app_8_start
    .quad app_8_end
    .quad app_9_start
    .quad app_9_end
    .quad app_10_start
    .quad app_10_end
    .quad app_11_start
    .quad app_11_end
app_list_end:

app_1_start:
.incbin "../user/target/riscv64gc-unknown-none-elf/release/init"
app_1_end:
app_2_start:
.incbin "../user/target/riscv64gc-unknown-none-elf/release/idle"
app_2_end:
app_3_start:
.incbin "../user/target/riscv64gc-unknown-none-elf/release/create_and_read_file"
app_3_end:
app_4_start:
.incbin "../user/target/riscv64gc-unknown-none-elf/release/for_read"
app_4_end:
app_5_start:
.incbin "../user/target/riscv64gc-unknown-none-elf/release/i_can_yield"
app_5_end:
app_6_start:
.incbin "../user/target/riscv64gc-unknown-none-elf/release/loop"
app_6_end:
app_7_start:
.incbin "../user/target/riscv64gc-unknown-none-elf/release/loop2"
app_7_end:
app_8_start:
.incbin "../user/target/riscv64gc-unknown-none-elf/release/printf"
app_8_end:
app_9_start:
.incbin "../user/target/riscv64gc-unknown-none-elf/release/switch"
app_9_end:
app_10_start:
.incbin "../user/target/riscv64gc-unknown-none-elf/release/sys_map"
app_10_end:
app_11_start:
.incbin "../user/target/riscv64gc-unknown-none-elf/release/unmap"
app_11_end: