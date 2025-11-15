#ifndef VIRTIO_BLK_FUNC_H
#define VIRTIO_BLK_FUNC_H
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

/* 初始化设备 */
bool virtio_blk_init(void);

/* 读设备 */
int virtio_blk_read(uint64_t sector,void* buffer, size_t size);

/* 写设备 */
int virtio_blk_write(uint64_t sector,void* buffer,size_t size);

/* 获取设备信息 */
int virtio_blk_get_info(uint64_t sector_count, uint32_t* sector_size);

/* 清理资源 */



#endif