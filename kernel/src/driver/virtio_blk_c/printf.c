#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdarg.h>
#include "rust_print.h"

// 辅助函数：将整数转换为字符串（十进制）
static void uint_to_str(uint64_t value, char* buffer, size_t* len) {
    if (value == 0) {
        buffer[0] = '0';
        *len = 1;
        return;
    }
    
    char temp[32];
    size_t idx = 0;
    
    while (value > 0) {
        temp[idx++] = '0' + (value % 10);
        value /= 10;
    }
    
    // 反转字符串
    for (size_t i = 0; i < idx; i++) {
        buffer[i] = temp[idx - 1 - i];
    }
    *len = idx;
}

// 辅助函数：将有符号整数转换为字符串
static void int_to_str(int64_t value, char* buffer, size_t* len) {
    if (value < 0) {
        buffer[0] = '-';
        // 处理最小负数的情况，避免溢出 (-9223372036854775808)
        if (value == ((int64_t)1 << 63)) {
            // 直接处理最小负数：9223372036854775808
            uint_to_str(9223372036854775808ULL, buffer + 1, len);
        } else {
            uint_to_str((uint64_t)(-value), buffer + 1, len);
        }
        (*len)++;
    } else {
        uint_to_str((uint64_t)value, buffer, len);
    }
}

// 辅助函数：将整数转换为十六进制字符串
static void uint_to_hex(uint64_t value, char* buffer, size_t* len, bool uppercase) {
    if (value == 0) {
        buffer[0] = '0';
        *len = 1;
        return;
    }
    
    char temp[32];
    size_t idx = 0;
    const char* hex_chars = uppercase ? "0123456789ABCDEF" : "0123456789abcdef";
    
    while (value > 0) {
        temp[idx++] = hex_chars[value & 0xF];
        value >>= 4;
    }
    
    // 反转字符串
    for (size_t i = 0; i < idx; i++) {
        buffer[i] = temp[idx - 1 - i];
    }
    *len = idx;
}

// 辅助函数：将整数转换为八进制字符串
static void uint_to_oct(uint64_t value, char* buffer, size_t* len) {
    if (value == 0) {
        buffer[0] = '0';
        *len = 1;
        return;
    }
    
    char temp[32];
    size_t idx = 0;
    
    while (value > 0) {
        temp[idx++] = '0' + (value & 0x7);
        value >>= 3;
    }
    
    // 反转字符串
    for (size_t i = 0; i < idx; i++) {
        buffer[i] = temp[idx - 1 - i];
    }
    *len = idx;
}

// 辅助函数：将指针转换为十六进制字符串
static void ptr_to_hex(const void* ptr, char* buffer, size_t* len) {
    uintptr_t addr = (uintptr_t)ptr;
    buffer[0] = '0';
    buffer[1] = 'x';
    uint_to_hex(addr, buffer + 2, len, false);
    *len += 2;
}

// 解析格式说明符并打印
static void print_format_spec(const char** fmt, va_list* args) {
    const char* p = *fmt;
    
    // 跳过 '%'
    p++;
    
    // 解析标志和宽度（简化版，只处理基本功能）
    int width = 0;
    bool left_align = false;
    bool zero_pad = false;
    
    // 解析标志
    while (*p == '-' || *p == '+' || *p == ' ' || *p == '0' || *p == '#') {
        if (*p == '-') left_align = true;
        if (*p == '0') zero_pad = true;
        p++;
    }
    
    // 解析宽度（简化版，只支持数字）
    while (*p >= '0' && *p <= '9') {
        width = width * 10 + (*p - '0');
        p++;
    }
    
    // 解析精度（简化版，跳过）
    if (*p == '.') {
        p++;
        while (*p >= '0' && *p <= '9') {
            p++;
        }
    }
    
    // 解析长度修饰符（简化版，只处理基本类型）
    bool is_long = false;
    bool is_long_long = false;
    if (*p == 'l') {
        p++;
        if (*p == 'l') {
            is_long_long = true;
            p++;
        } else {
            is_long = true;
        }
    }
    
    // 解析格式字符
    char spec = *p;
    p++;
    *fmt = p;
    
    char buffer[128];
    size_t len = 0;
    const char* str = NULL;
    size_t str_len = 0;
    
    switch (spec) {
        case 'd':
        case 'i': {
            int64_t value;
            if (is_long_long) {
                value = va_arg(*args, int64_t);
            } else if (is_long) {
                value = (int64_t)va_arg(*args, long);
            } else {
                value = (int64_t)va_arg(*args, int);
            }
            int_to_str(value, buffer, &len);
            rust_print_formatted(buffer, len);
            break;
        }
        
        case 'u': {
            uint64_t value;
            if (is_long_long) {
                value = va_arg(*args, uint64_t);
            } else if (is_long) {
                value = (uint64_t)va_arg(*args, unsigned long);
            } else {
                value = (uint64_t)va_arg(*args, unsigned int);
            }
            uint_to_str(value, buffer, &len);
            rust_print_formatted(buffer, len);
            break;
        }
        
        case 'x': {
            uint64_t value;
            if (is_long_long) {
                value = va_arg(*args, uint64_t);
            } else if (is_long) {
                value = (uint64_t)va_arg(*args, unsigned long);
            } else {
                value = (uint64_t)va_arg(*args, unsigned int);
            }
            uint_to_hex(value, buffer, &len, false);
            rust_print_formatted(buffer, len);
            break;
        }
        
        case 'X': {
            uint64_t value;
            if (is_long_long) {
                value = va_arg(*args, uint64_t);
            } else if (is_long) {
                value = (uint64_t)va_arg(*args, unsigned long);
            } else {
                value = (uint64_t)va_arg(*args, unsigned int);
            }
            uint_to_hex(value, buffer, &len, true);
            rust_print_formatted(buffer, len);
            break;
        }
        
        case 'o': {
            uint64_t value;
            if (is_long_long) {
                value = va_arg(*args, uint64_t);
            } else if (is_long) {
                value = (uint64_t)va_arg(*args, unsigned long);
            } else {
                value = (uint64_t)va_arg(*args, unsigned int);
            }
            uint_to_oct(value, buffer, &len);
            rust_print_formatted(buffer, len);
            break;
        }
        
        case 's': {
            str = va_arg(*args, const char*);
            if (str == NULL) {
                str = "(null)";
            }
            // 计算字符串长度
            str_len = 0;
            while (str[str_len] != '\0') {
                str_len++;
            }
            rust_print_formatted(str, str_len);
            break;
        }
        
        case 'c': {
            char c = (char)va_arg(*args, int);
            rust_print_char(c);
            break;
        }
        
        case 'p': {
            const void* ptr = va_arg(*args, const void*);
            ptr_to_hex(ptr, buffer, &len);
            rust_print_formatted(buffer, len);
            break;
        }
        
        case '%': {
            rust_print_char('%');
            break;
        }
        
        default: {
            // 未知格式说明符，打印 '%' 和字符本身
            rust_print_char('%');
            rust_print_char(spec);
            break;
        }
    }
}

// 主要的 printf 函数
int printf(const char* format, ...) {
    va_list args;
    va_start(args, format);
    
    const char* p = format;
    
    while (*p != '\0') {
        if (*p == '%') {
            print_format_spec(&p, &args);
        } else {
            rust_print_char(*p);
            p++;
        }
    }
    
    va_end(args);
    return 0; // 简化版，不返回实际打印的字符数
}

