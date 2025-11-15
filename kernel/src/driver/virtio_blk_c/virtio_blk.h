#ifndef VIRTIO_BLK_H
#define VIRTIO_BLK_H

#include "virtio_common.h"
#include "virtio_queue.h"

// ------------------------------
// 1. 块设备类型 ID（Section 9.1）
// ------------------------------
#define VIRTIO_DEVICE_ID_BLOCK  0x2  // 块设备的设备 ID


// ------------------------------
// 2. 块设备特征位（Section 9.2）
// ------------------------------
#define VIRTIO_BLK_F_SIZE              (1ULL << 0)   // 支持获取磁盘大小
#define VIRTIO_BLK_F_SEG_MAX           (1ULL << 1)   // 限制最大段数
#define VIRTIO_BLK_F_GEOMETRY          (1ULL << 2)   // 暴露磁盘几何结构（柱面/磁头/扇区）
#define VIRTIO_BLK_F_RO                (1ULL << 5)   // 设备为只读
#define VIRTIO_BLK_F_BLK_SIZE          (1ULL << 6)   // 支持扇区大小 > 512 字节
#define VIRTIO_BLK_F_FLUSH             (1ULL << 9)   // 支持缓存刷新（数据持久化）
#define VIRTIO_BLK_F_TOPOLOGY          (1ULL << 10)  // 暴露拓扑信息（最优 I/O 粒度）
#define VIRTIO_BLK_F_MQ                (1ULL << 12)  // 支持多队列（并行 I/O）
#define VIRTIO_BLK_F_DISCARD           (1ULL << 13)  // 支持 DISCARD/TRIM 操作
#define VIRTIO_BLK_F_WRITE_ZEROES      (1ULL << 14)  // 支持快速写零（高效填充）
#define VIRTIO_BLK_F_SECURE_ERASE      (1ULL << 16)  // 支持安全擦除（数据不可恢复）


// ------------------------------
// 3. 块设备请求类型（Section 9.3.2）
// ------------------------------
#define VIRTIO_BLK_T_IN         0x00000000  // 读扇区（设备→驱动）
#define VIRTIO_BLK_T_OUT        0x00000001  // 写扇区（驱动→设备）
#define VIRTIO_BLK_T_FLUSH      0x00000004  // 刷新缓存（确保数据写入物理介质）
#define VIRTIO_BLK_T_GET_SIZE   0x00000008  // 获取磁盘总扇区数
#define VIRTIO_BLK_T_DISCARD    0x0000000c  // 释放扇区（TRIM）
#define VIRTIO_BLK_T_WRITE_ZEROES 0x00000014  // 快速写零


// ------------------------------
// 4. 块设备请求头（Section 9.3.2）
// ------------------------------
struct virtio_blk_req {
    uint32_t type;     // 请求类型（见 VIRTIO_BLK_T_*）
    uint32_t reserved; // 保留字段（必须为 0）
    uint64_t sector;   // 起始扇区号（512 字节为单位，除非协商 VIRTIO_BLK_F_BLK_SIZE）
} __attribute__((packed));


// ------------------------------
// 5. 块设备响应状态（Section 9.3.3）
// ------------------------------
#define VIRTIO_BLK_S_OK         0x0  // 操作成功
#define VIRTIO_BLK_S_IOERR      0x1  // I/O 错误
#define VIRTIO_BLK_S_UNSUPP     0x2  // 不支持的操作

struct virtio_blk_resp {
    uint8_t status;  // 响应状态（见 VIRTIO_BLK_S_*）
} __attribute__((packed));


// ------------------------------
// 6. 块设备配置空间（Section 9.4）
// 注：通过 VIRTIO_MMIO_CONFIG 寄存器访问
// ------------------------------
struct virtio_blk_config {
    uint64_t capacity;     // 总扇区数（512 字节/扇区，除非协商 VIRTIO_BLK_F_BLK_SIZE）
    uint32_t size_max;     // 最大段大小（若协商 VIRTIO_BLK_F_SEG_MAX）
    uint32_t seg_max;      // 最大段数（若协商 VIRTIO_BLK_F_SEG_MAX）
    uint16_t cylinders;    // 柱面数（若协商 VIRTIO_BLK_F_GEOMETRY）
    uint8_t heads;         // 磁头数（若协商 VIRTIO_BLK_F_GEOMETRY）
    uint8_t sectors;       // 每磁道扇区数（若协商 VIRTIO_BLK_F_GEOMETRY）
    uint32_t blk_size;     // 扇区大小（字节，若协商 VIRTIO_BLK_F_BLK_SIZE）
    // 拓扑信息（若协商 VIRTIO_BLK_F_TOPOLOGY）
    struct {
        uint32_t physical_block_exp;  // 物理块大小 = 2^physical_block_exp
        uint32_t alignment_offset;    // 对齐偏移（字节）
        uint16_t min_io_size;         // 最小 I/O 大小（物理块数）
        uint16_t opt_io_size;         // 最优 I/O 大小（物理块数）
    } topology;
    uint8_t writeback;     // 缓存策略（1=写回，0=直写，若协商 VIRTIO_BLK_F_FLUSH）
    uint16_t num_queues;   // 队列数量（若协商 VIRTIO_BLK_F_MQ）
    uint32_t max_discard_sectors;  // 最大 discard 扇区数（若协商 VIRTIO_BLK_F_DISCARD）
    uint32_t max_discard_seg;      // 最大 discard 段数（若协商 VIRTIO_BLK_F_DISCARD）
    uint32_t discard_sector_alignment;  // discard 对齐扇区数（若协商 VIRTIO_BLK_F_DISCARD）
    uint32_t max_write_zeroes_sectors;  // 最大写零扇区数（若协商 VIRTIO_BLK_F_WRITE_ZEROES）
    uint8_t write_zeroes_may_unmap;     // 写零可释放空间（1=允许，若协商 VIRTIO_BLK_F_WRITE_ZEROES）
} __attribute__((packed));

/* 定义块设备结构体 */
struct virtio_block_device{
    uint64_t desctable_addr;
    uint64_t available_queue_addr;
    uint64_t used_queue_addr;
    uint32_t magic;
    uint32_t block_id;
    uint32_t block_version;
    uint16_t queue_size;
};


#endif 