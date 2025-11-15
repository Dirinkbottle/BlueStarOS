mod virtioblk;
mod blockdevice;
mod blk_api;

pub use self::virtioblk::{VirtioBlockDevice,BLOCK_DEVICE};
pub use self::blockdevice::BlockDevice;
pub use self::blk_api::test_block_write_read;