/// C语言外部接口 - 模拟 malloc/free 函数
/// 使用 Rust 内核的全局分配器为 C 代码提供内存分配功能

use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;
use core::ptr;

/// 获取全局分配器实例
/// 这里使用 buddy_system_allocator::LockedHeap
/// 它已经在 memory::frame_allocator 中定义为全局分配器
extern crate alloc;
use alloc::alloc::alloc as global_alloc;
use alloc::alloc::dealloc as global_dealloc;

/// C 风格的 malloc 函数
/// 
/// # 参数
/// - `size`: 要分配的内存大小（字节数）
/// 
/// # 返回值
/// - 成功：返回指向分配内存的指针（`void*`）
/// - 失败：返回 `NULL`
/// 
/// # 内存连续性保证
/// - **单个 malloc 调用保证内存连续**：返回的内存块内部是连续的
/// - 使用 Buddy System 分配器，从连续的堆内存区域中分配连续块
/// - 多次 malloc 调用之间分配的内存块可能不连续（中间可能有其他分配）
/// 
/// # 示例
/// ```c
/// void* ptr = malloc(1024);  // 这 1024 字节是连续的
/// // ptr[0] 到 ptr[1023] 在内存中是连续的
/// ```
/// 
/// # 安全性
/// - 这是 unsafe 函数，因为返回原始指针
/// - 调用者负责确保正确释放内存
/// 
/// # 注意
/// - 这个函数使用分配跟踪系统，确保可以正确释放
#[no_mangle]
pub extern "C" fn malloc(size: usize) -> *mut c_void {
    // 默认使用带跟踪的分配函数
    malloc_tracked(size)
}

/// C 风格的 calloc 函数
/// 分配并清零内存
/// 
/// # 参数
/// - `num`: 元素数量
/// - `size`: 每个元素的大小（字节数）
/// 
/// # 返回值
/// - 成功：返回指向分配并清零的内存指针
/// - 失败：返回 `NULL`
/// 
/// # 注意
/// - 使用带跟踪的 malloc，确保可以正确释放
#[no_mangle]
pub extern "C" fn calloc(num: usize, size: usize) -> *mut c_void {
    let total_size = num.saturating_mul(size);
    if total_size == 0 {
        return ptr::null_mut();
    }

    // 使用带跟踪的分配函数
    let ptr = malloc_tracked(total_size);
    if ptr.is_null() {
        return ptr::null_mut();
    }

    // 清零内存
    unsafe {
        ptr::write_bytes(ptr as *mut u8, 0, total_size);
    }

    ptr
}

/// C 风格的 realloc 函数
/// 重新分配内存（扩大或缩小）
/// 
/// # 参数
/// - `ptr`: 之前分配的内存指针（可以为 NULL）
/// - `new_size`: 新的内存大小（字节数）
/// 
/// # 返回值
/// - 成功：返回指向新内存的指针
/// - 失败：返回 `NULL`，原内存保持不变
/// 
/// # 注意
/// - 如果 `ptr` 为 NULL，行为等同于 `malloc(new_size)`
/// - 如果 `new_size` 为 0，行为等同于 `free(ptr)`，返回 NULL
/// - 会复制旧内存的内容到新内存（最多复制 min(旧大小, 新大小) 字节）
#[no_mangle]
pub extern "C" fn realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
    // 如果新大小为 0，释放内存并返回 NULL
    if new_size == 0 {
        if !ptr.is_null() {
            free(ptr);
        }
        return ptr::null_mut();
    }

    // 如果原指针为 NULL，等同于 malloc
    if ptr.is_null() {
        return malloc(new_size);
    }

    // 获取旧内存的大小（从分配跟踪表）
    let old_size = {
        let addr = ptr as usize;
        let table = ALLOCATION_TABLE.lock();
        table.get(&addr).map(|info| info.size).unwrap_or(0)
    };

    // 分配新内存
    let new_ptr = malloc(new_size);
    if new_ptr.is_null() {
        return ptr::null_mut();
    }

    // 复制旧内存的内容（最多复制旧大小和新大小的最小值）
    if old_size > 0 {
        let copy_size = core::cmp::min(old_size, new_size);
        unsafe {
            ptr::copy_nonoverlapping(
                ptr as *const u8,
                new_ptr as *mut u8,
                copy_size
            );
        }
    }

    // 释放旧内存
    free(ptr);

    new_ptr
}

/// C 风格的 free 函数
/// 释放之前通过 malloc/calloc/realloc 分配的内存
/// 
/// # 参数
/// - `ptr`: 要释放的内存指针（可以为 NULL）
/// 
/// # 安全性
/// - 如果 `ptr` 为 NULL，函数不执行任何操作（符合 C 标准）
/// - 如果 `ptr` 不是通过 malloc/calloc/realloc 分配的，行为未定义
/// - 双重释放会导致未定义行为
/// 
/// # 注意
/// - 这个函数使用分配跟踪系统来正确释放内存
/// - 如果使用不带跟踪的 malloc，应该使用 free_tracked
#[no_mangle]
pub extern "C" fn free(ptr: *mut c_void) {
    // 默认使用带跟踪的释放函数
    free_tracked(ptr);
}

// ==================== 改进版本：带分配跟踪的 malloc/free ====================

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicUsize, Ordering};
use lazy_static::lazy_static;
use crate::sync::UPSafeCell;

/// 分配元数据：记录分配的大小和对齐
#[derive(Clone, Copy)]
struct AllocationInfo {
    size: usize,
    align: usize,
}

/// 全局分配跟踪表
/// 使用 BTreeMap 来跟踪所有活跃的分配
lazy_static! {
    static ref ALLOCATION_TABLE: UPSafeCell<BTreeMap<usize, AllocationInfo>> = 
        unsafe { UPSafeCell::new(BTreeMap::new()) };
}

/// 分配计数器（用于调试）
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static FREE_COUNT: AtomicUsize = AtomicUsize::new(0);

/// 改进的 malloc 函数（带分配跟踪）
/// 
/// # 内存连续性保证
/// - **保证单个分配的内存块是连续的**
/// - 使用 Buddy System 分配器（buddy_system_allocator::LockedHeap）
/// - Buddy System 从连续的堆内存区域中分配连续的内存块
/// - 分配器管理的堆内存区域本身是连续的（KERNEL_HEADP 到 KERNEL_HEADP + KERNEL_HEAP_SIZE）
/// 
/// # 技术细节
/// - Buddy System 分配器会将堆内存分成不同大小的块（2的幂次）
/// - 分配时找到合适大小的连续块并返回
/// - 返回的指针指向一个连续的内存区域，大小为请求的 size 字节
#[no_mangle]
pub extern "C" fn malloc_tracked(size: usize) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut();
    }

    // 使用合理的默认对齐（通常是 8 字节，适合大多数类型）
    let align = core::mem::align_of::<usize>();
    let layout = match Layout::from_size_align(size, align) {
        Ok(layout) => layout,
        Err(_) => return ptr::null_mut(),
    };

    unsafe {
        // 使用全局分配器分配内存
        // 底层使用 Buddy System，保证返回的内存块是连续的
        let ptr = global_alloc(layout);
        if ptr.is_null() {
            return ptr::null_mut();
        }

        let addr = ptr as usize;
        
        // 记录分配信息
        ALLOCATION_TABLE.lock().insert(addr, AllocationInfo {
            size,
            align,
        });

        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        ptr as *mut c_void
    }
}

/// 改进的 free 函数（带分配跟踪）
#[no_mangle]
pub extern "C" fn free_tracked(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }

    let addr = ptr as usize;
    let mut table = ALLOCATION_TABLE.lock();
    
    if let Some(info) = table.remove(&addr) {
        // 找到了分配记录，使用正确的布局释放
        let layout = Layout::from_size_align(info.size, info.align)
            .expect("Invalid layout in allocation table");
        
        unsafe {
            global_dealloc(ptr as *mut u8, layout);
        }
        
        FREE_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        // 没有找到分配记录，可能是：
        // 1. 双重释放
        // 2. 使用未跟踪的分配函数分配的内存
        // 3. 无效指针
        // 
        // 为了安全，我们不做任何操作
        // 在实际应用中，可以记录警告或错误
    }
}

/// 获取分配统计信息（用于调试）
/// 
/// # 参数
/// - `alloc_count`: 输出参数，总分配次数（可以为 NULL）
/// - `free_count`: 输出参数，总释放次数（可以为 NULL）
/// - `active_count`: 输出参数，当前活跃分配数（可以为 NULL）
#[no_mangle]
pub extern "C" fn get_alloc_stats(
    alloc_count: *mut usize, 
    free_count: *mut usize, 
    active_count: *mut usize
) {
    unsafe {
        if !alloc_count.is_null() {
            *alloc_count = ALLOC_COUNT.load(Ordering::Relaxed);
        }
        if !free_count.is_null() {
            *free_count = FREE_COUNT.load(Ordering::Relaxed);
        }
        if !active_count.is_null() {
            *active_count = ALLOCATION_TABLE.lock().len();
        }
    }
}
