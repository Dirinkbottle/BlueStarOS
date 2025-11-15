#ifndef RUST_ALLOC_H
#define RUST_ALLOC_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// ==================== 标准 C 内存分配函数 ====================

/// 分配指定大小的内存
/// @param size 要分配的内存大小（字节数）
/// @return 成功返回指向分配内存的指针，失败返回 NULL
/// 
/// @note 内存连续性保证：
///   - 单个 malloc 调用保证返回的内存块是连续的
///   - 使用 Buddy System 分配器，从连续堆区域分配连续块
///   - 多次 malloc 调用之间分配的内存块可能不连续
/// 
/// @example
///   void* ptr = malloc(1024);  // 这 1024 字节在内存中是连续的
///   // ptr[0] 到 ptr[1023] 在内存地址上是连续的
void* malloc(size_t size);

/// 分配并清零内存
/// @param num 元素数量
/// @param size 每个元素的大小（字节数）
/// @return 成功返回指向分配并清零的内存指针，失败返回 NULL
void* calloc(size_t num, size_t size);

/// 重新分配内存
/// @param ptr 之前分配的内存指针（可以为 NULL）
/// @param new_size 新的内存大小（字节数）
/// @return 成功返回指向新内存的指针，失败返回 NULL
/// @note 如果 ptr 为 NULL，行为等同于 malloc(new_size)
/// @note 如果 new_size 为 0，行为等同于 free(ptr)
void* realloc(void* ptr, size_t new_size);

/// 释放之前分配的内存
/// @param ptr 要释放的内存指针（可以为 NULL）
/// @note 如果 ptr 为 NULL，函数不执行任何操作
void free(void* ptr);

// ==================== 调试和统计函数 ====================

/// 获取分配统计信息
/// @param alloc_count 输出参数：总分配次数（可以为 NULL）
/// @param free_count 输出参数：总释放次数（可以为 NULL）
/// @param active_count 输出参数：当前活跃分配数（可以为 NULL）
void get_alloc_stats(size_t* alloc_count, size_t* free_count, size_t* active_count);

#ifdef __cplusplus
}
#endif

#endif // RUST_ALLOC_H

