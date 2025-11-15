#ifndef PRINTF_H
#define PRINTF_H

#include <stdarg.h>

#ifdef __cplusplus
extern "C" {
#endif

// 标准 printf 函数
int printf(const char* format, ...);

#ifdef __cplusplus
}
#endif

#endif // PRINTF_H

