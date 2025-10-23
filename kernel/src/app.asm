

#被链接到data段
.global app_start
.global app_end
.section .data.app
app_start:
.incbin "./te.elf"
app_end: