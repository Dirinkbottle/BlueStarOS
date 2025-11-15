mod stdio;
mod normal_externel_interrupt;
mod virtio;
mod virtio_blk;
pub use self::stdio::*;
pub use self::virtio::*;
pub use self::virtio_blk::*;
pub use self::virtio_blk::{init_global_block_device, get_global_block_device};