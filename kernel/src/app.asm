

#被链接到data段
.section .data.app
.global app_list_start
.global app_list_end
app_list_start:
    .quad app_1_start
    .quad app_1_end
    .quad app_2_start
    .quad app_2_end
app_list_end:

app_1_start:
.incbin "../user/target/riscv64gc-unknown-none-elf/release/printf"
app_1_end:
app_2_start:
.incbin "../user/target/riscv64gc-unknown-none-elf/release/switch"
app_2_end: