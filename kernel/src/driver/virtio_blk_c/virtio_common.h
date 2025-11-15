#ifndef VIRTIO_COMMON_H
#define VIRTIO_COMMON_H

#include <stdint.h>
#include <stddef.h>

// ------------------------------
// 1. 设备状态（Section 5.1）
// ------------------------------
#define VIRTIO_STATUS_ACKNOWLEDGE     (1 << 0)  // 驱动已检测到设备
#define VIRTIO_STATUS_DRIVER          (1 << 1)  // 驱动已加载
#define VIRTIO_STATUS_DRIVER_OK       (1 << 2)  // 驱动初始化完成
#define VIRTIO_STATUS_FEATURES_OK     (1 << 3)  // 特征位协商完成
#define VIRTIO_STATUS_DEVICE_NEEDS_RESET (1 << 6)  // 设备需要重置
#define VIRTIO_STATUS_FAILED          (1 << 7)  // 初始化失败


// ------------------------------
// 2. 通用特征位（Section 5.2）
// ------------------------------
// 基础特征（所有设备必选/可选）
#define VIRTIO_FEATURE_RING_INDIRECT_DESC    (1ULL << 2)   // 支持间接描述符
#define VIRTIO_FEATURE_RING_EVENT_IDX        (1ULL << 6)   // 支持事件索引（抑制无效中断）
#define VIRTIO_FEATURE_VERSION_1             (1ULL << 32)  // 支持 Virtio 1.0+ 版本
#define VIRTIO_FEATURE_ACCESS_PLATFORM       (1ULL << 33)  // 平台特定内存访问优化
#define VIRTIO_FEATURE_RING_PACKED           (1ULL << 34)  // 支持打包队列（v1.3 新增）
#define VIRTIO_FEATURE_IN_ORDER              (1ULL << 35)  // 按顺序处理请求
#define VIRTIO_FEATURE_ORDER_PLATFORM        (1ULL << 36)  // 平台特定顺序优化
#define VIRTIO_FEATURE_SR_IOV                (1ULL << 37)  // 支持 SR-IOV 虚拟化
#define VIRTIO_FEATURE_NOTIFICATION_DATA     (1ULL << 38)  // 通知携带额外数据（v1.3 新增）


// ------------------------------
// 3. 描述符标志（Section 5.3.1）
// ------------------------------
#define VIRTQ_DESC_F_NEXT        (1 << 0)  // 存在下一个描述符（构成链）
#define VIRTQ_DESC_F_WRITE       (1 << 1)  // 设备可写入该缓冲区（驱动→设备为读，设备→驱动为写）
#define VIRTQ_DESC_F_INDIRECT    (1 << 2)  // 该描述符指向间接描述符表


// ------------------------------
// 4. 可用环标志（Section 5.4.1）
// ------------------------------
#define VIRTQ_AVAIL_F_NO_INTERRUPT (1 << 0)  // 设备处理后不触发中断


// ------------------------------
// 5. 已用环标志（Section 5.5.1）
// ------------------------------
#define VIRTQ_USED_F_NO_NOTIFY     (1 << 0)  // 驱动无需通知设备（设备主动轮询）


// ------------------------------
// 6. MMIO 传输方式寄存器（Section 4.2）
// ------------------------------
// 基础信息寄存器
#define VIRTIO_MMIO_MAGIC_VALUE    0x000  // 魔数（固定为 0x74726976）
#define VIRTIO_MMIO_VERSION        0x004  // 版本号（1 表示 1.0+）
#define VIRTIO_MMIO_DEVICE_ID      0x008  // 设备类型 ID（块设备为 0x2）
#define VIRTIO_MMIO_VENDOR_ID      0x00C  // 厂商 ID

// 特征位寄存器
#define VIRTIO_MMIO_DEVICE_FEATURES      0x010  // 设备支持的特征位（低 32 位）
#define VIRTIO_MMIO_DEVICE_FEATURES_SEL  0x014  // 特征位选择（0=低 32 位，1=高 32 位）
#define VIRTIO_MMIO_DRIVER_FEATURES      0x020  // 驱动接受的特征位（低 32 位）
#define VIRTIO_MMIO_DRIVER_FEATURES_SEL  0x024  // 驱动特征位选择（0=低 32 位，1=高 32 位）

// 队列配置寄存器
#define VIRTIO_MMIO_QUEUE_SEL        0x030  // 选择要配置的队列索引
#define VIRTIO_MMIO_QUEUE_SIZE       0x034  // 队列大小（设备支持的最大长度）
#define VIRTIO_MMIO_QUEUE_ADDR       0x038  // 队列物理地址（低 32 位）
#define VIRTIO_MMIO_QUEUE_ADDR_HI    0x03C  // 队列物理地址（高 32 位，64 位系统用）
#define VIRTIO_MMIO_QUEUE_ENABLE     0x040  // 队列使能（1=启用，0=禁用）

// 通知与中断寄存器
#define VIRTIO_MMIO_QUEUE_NOTIFY     0x050  // 通知设备处理队列（写入队列索引）
#define VIRTIO_MMIO_INTERRUPT_STATUS 0x060  // 中断状态（bit 0=队列事件，bit 1=配置事件）
#define VIRTIO_MMIO_INTERRUPT_ACK    0x064  // 中断确认（写入状态位清除中断）
#define VIRTIO_MMIO_STATUS           0x070  // 设备状态（见 VIRTIO_STATUS_*）

// 配置空间寄存器（设备专用）
#define VIRTIO_MMIO_CONFIG_GENERATION 0x080  // 配置空间版本（检测配置变化）
#define VIRTIO_MMIO_CONFIG           0x090  // 配置空间起始地址（设备专用结构）


#endif  // VIRTIO_COMMON_H