use log::debug;

use crate::driver::{BlockDevice, virtio_blk::block::VirtBlk};


pub fn test_block_device(){
 let mut blk =VirtBlk::new();    
 let mut user_buffer = [0;512];
 let mut new_buffer = [0u8;512];
 blk.write_blk(0, &mut new_buffer);
 debug!("buffer:{:?}",new_buffer);
}