#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <virtio_blk_func.h>
#include <virtio_blk.h>
#include <virtio_common.h>
#include <rust_print.h>
#include <printf.h>
#include <rust_alloc.h>

#define VIRTIO_BLK_BASE_ADDR 0x10001000
#define VIRTIO_BLK_MAGIC     0x74726976

typedef struct virtq_used virtq_used ;
typedef struct virtq_desc virtq_desc ; 
typedef struct virtq_avail virtq_avail;
typedef struct virtq_used_elem virtq_used_elem;

static volatile uint32_t* virtio_blk_base_addr = (volatile uint32_t*)VIRTIO_BLK_BASE_ADDR;
static virtq_desc* desc_table_addr=NULL;
static virtq_used* virtq_used_addr=NULL;
static virtq_avail* virtq_avail_addr=NULL;
static uint32_t device_support_max=0;


uint32_t mmio_read(uint32_t offset){
    volatile uint32_t* target_addr  = (volatile uint32_t*)((uintptr_t)virtio_blk_base_addr + offset);
    uint32_t value = *target_addr;
    return value;
}

/*  返回验证数值 */
uint32_t mmio_write(uint32_t offset, uint32_t value){
    volatile uint32_t* target_addr  = (volatile uint32_t*)((uintptr_t)virtio_blk_base_addr + offset);
    *target_addr = value;
    uint32_t verify =  mmio_read(offset);
    return verify;
}

uint32_t read_status(){
    uint32_t value =  mmio_read(VIRTIO_MMIO_STATUS);
    return value;
}

/* 自动验证写入 */
void write_status(uint32_t value){
    uint32_t verify =  mmio_write(VIRTIO_MMIO_STATUS,value);
    if(verify!=value){
        printf("status wirte failed write:%d but read:%d",value,verify);
    }
    return;
}

/* 验证设备id和魔数*/
bool virtio_find_blk_device(void){
    uint32_t magic = mmio_read(VIRTIO_MMIO_MAGIC_VALUE);
    uint32_t stand_magic = VIRTIO_BLK_MAGIC;
    uint32_t device_id = mmio_read(VIRTIO_MMIO_DEVICE_ID);
    
    printf("Virtio BLK Device Check:\n");
    printf("  Magic: 0x%x (expected: 0x%x)\n", magic, stand_magic);
    printf("  Device ID: 0x%x\n", device_id);
    
    if (magic != stand_magic){
        printf("ERROR: Magic value mismatch! Device not found.\n");
        return false;
    }
    
    if (device_id != VIRTIO_DEVICE_ID_BLOCK) {
        printf("WARNING: Device ID mismatch (got 0x%x, expected 0x%x)\n", 
               device_id, VIRTIO_DEVICE_ID_BLOCK);
    } else {
        printf("SUCCESS: Virtio BLK device found!\n");
    }
    
    
    return true;
}

/* 协商特性，握手初始化 */
bool ackonowledge(){
    /* 记得叠加 */
    write_status(VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_DRIVER_OK);
    /* 特性读出协商 */
    
}
/* 队列准备 */
void ready_queue(void){
    /* 描述符表准备 */
     device_support_max = mmio_read(VIRTIO_MMIO_QUEUE_SIZE);
     desc_table_addr= malloc(sizeof(virtq_desc)*device_support_max);

    if(!desc_table_addr){
        printf("desc_table alloc failed");
    }

    for (size_t i = 0; i < device_support_max; i++)
    {
        virtq_desc new_desc;
        uint32_t buffer_size =SIGNAL_BUFFER_SIZE;
        uint64_t buffer_addr = (uint64_t)malloc(buffer_size);

        if(!buffer_addr){
            printf("buffer alloc failed!");
        }

        new_desc.addr = buffer_addr;
        new_desc.flags = 0;
        new_desc.len=buffer_size;
        new_desc.next=0;
        desc_table_addr[i] = new_desc;
    }

    /* 可用环初始化 */
    uint32_t mini_size = sizeof(virtq_avail);
    uint32_t ring_size = sizeof(uint16_t)*device_support_max;
    uint32_t total_size =mini_size + ring_size;
    virtq_avail_addr=(virtq_avail*)malloc(total_size);
    virtq_avail_addr->flags=0;
    virtq_avail_addr->idx=0;
    for (uint32_t i = (uint32_t)(virtq_avail_addr)+mini_size; i < (uint32_t)(virtq_avail_addr)+(mini_size)+ring_size; i+=sizeof(uint16_t))
    {
        *(uint16_t*)i =0;
    }
    
    /* 已用环初始化 */
    mini_size = sizeof(virtq_used);
    ring_size = sizeof(virtq_used_elem)*device_support_max;
    total_size =mini_size + ring_size;
    virtq_used_addr=(virtq_used*)malloc(total_size);
    virtq_used_addr->flags=0;
    virtq_used_addr->idx=0;
    for (uint32_t i = (uint32_t)(virtq_used_addr)+mini_size; i < (uint32_t)(virtq_used_addr)+(mini_size)+ring_size; i+=sizeof(virtq_used_elem))
    {
        virtq_used_elem elem;
        elem.id=0;
        elem.len=0;
        *(virtq_used_elem*)i =elem;
    }
    

}

/* 配置块设备参数 */

bool virtio_blk_init(void){
    virtio_find_blk_device();
    ready_queue();
    return true;
}
