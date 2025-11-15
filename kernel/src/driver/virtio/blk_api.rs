use crate::driver::{BLOCK_DEVICE, BlockDevice, virtio::virtioblk::BLOCK_SIZE_SIGNAL};

///块设备外部api暴漏


///外部c驱动
extern "C" {
    fn virtio_blk_init()-> bool;
}

pub fn test_block_write_read()->bool{
    unsafe {
        //virtio_blk_init();
    }
    let mut user_buffer = [0u8;BLOCK_SIZE_SIGNAL];
    BLOCK_DEVICE.lock().write_blk(0, &user_buffer);
    BLOCK_DEVICE.lock().read_blk(0, &mut user_buffer);
    return false;
}