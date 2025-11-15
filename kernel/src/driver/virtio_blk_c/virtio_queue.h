#ifndef VIRTIO_QUEUE_H
#define VIRTIO_QUEUE_H

#include "virtio_common.h"



#define SIGNAL_BUFFER_SIZE 512;

// ------------------------------
// 1. 描述符（Descriptor）：单个缓冲区描述（Section 5.3）
// ------------------------------
struct virtq_desc {
    uint64_t addr;   // 缓冲区物理地址（64 位）
    uint32_t len;    // 缓冲区长度（字节）
    uint16_t flags;  // 标志（见 VIRTQ_DESC_F_*）
    uint16_t next;   // 下一个描述符的索引（构成链）
} __attribute__((packed));  // 紧凑布局，无填充


// ------------------------------
// 2. 可用环（Available Ring）：驱动向设备提交请求（Section 5.4）
// ------------------------------
struct virtq_avail {
    uint16_t flags;          // 标志（见 VIRTQ_AVAIL_F_*）
    uint16_t idx;            // 下一个可用描述符索引（驱动写入位置）
    uint16_t ring[];         // 可用描述符链起始索引数组（长度 = 队列大小）
    // 可选：事件索引（若协商 VIRTIO_FEATURE_RING_EVENT_IDX）
    // uint16_t used_event;
} __attribute__((packed));


// ------------------------------
// 3. 已用环元素（Used Element）：设备通知驱动请求完成（Section 5.5）
// ------------------------------
struct virtq_used_elem {
    uint32_t id;    // 已处理的描述符链起始索引
    uint32_t len;   // 设备实际写入的字节数（仅对 WRITE 缓冲区有效）
} __attribute__((packed));


// ------------------------------
// 4. 已用环（Used Ring）：设备向驱动反馈完成状态（Section 5.5）
// ------------------------------
struct virtq_used {
    uint16_t flags;            // 标志（见 VIRTQ_USED_F_*）
    uint16_t idx;              // 下一个已用元素索引（设备写入位置）
    struct virtq_used_elem ring[];  // 已用元素数组（长度 = 队列大小）
    // 可选：事件索引（若协商 VIRTIO_FEATURE_RING_EVENT_IDX）
    // uint16_t avail_event;
} __attribute__((packed));


// ------------------------------
// 5. 完整虚拟队列（Virtqueue）布局（Section 5.6）
// 注：描述符表 + 可用环 + 已用环在物理内存中连续
// ------------------------------
struct virtq {
    uint16_t queue_size;               // 队列大小（描述符数量）
    struct virtq_desc *desc;           // 描述符表（数组，长度 = queue_size）
    struct virtq_avail *avail;         // 可用环（紧跟描述符表）
    struct virtq_used *used;           // 已用环（紧跟可用环，内存对齐可能有填充）
};


#endif  // VIRTIO_QUEUE_H