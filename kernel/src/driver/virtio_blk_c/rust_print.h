// c_driver/rust_print.h
#ifndef RUST_PRINT_H
#define RUST_PRINT_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// 简单字符串打印
void rust_print_str(const char* s);

// 格式化打印函数 - 打印格式化后的字符串片段
void rust_print_formatted(const char* str, size_t len);

// 打印单个字符
void rust_print_char(char c);

// 打印整数（十进制）
void rust_print_int(int64_t value);

// 打印无符号整数（十进制）
void rust_print_uint(uint64_t value);

// 打印十六进制（小写）
void rust_print_hex_lower(uint64_t value);

// 打印十六进制（大写）
void rust_print_hex_upper(uint64_t value);

// 打印八进制
void rust_print_oct(uint64_t value);

// 打印指针
void rust_print_ptr(const void* ptr);

#ifdef __cplusplus
}
#endif

#endif 