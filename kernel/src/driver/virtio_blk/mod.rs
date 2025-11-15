mod block;
mod testblock;

pub use self::testblock::test_block_device;
pub use block::*;
pub use block::{init_global_block_device, get_global_block_device, GLOBAL_BLOCK_DEVICE};