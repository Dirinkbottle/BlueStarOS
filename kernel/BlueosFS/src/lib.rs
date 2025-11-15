#![no_std]
#![feature(split_array)]
mod bitmap;
mod blueosfs;
mod root;
mod vfs;
mod api;

extern crate alloc;

// 只导出用户需要的 API
pub use api::{
    // 文件系统初始化
    initial_root_filesystem,
    // 文件操作
    open, create_file, create_dir, remove, read_file, write_file, append_file,
    // 目录操作
    list_dir,
    // 文件信息
    exists, is_file, is_dir,
    // 文件描述符
    FileDescriptor, FileFlags,
};
pub use vfs::{FileAttribute,NodeType, VfsError, BlockDeviceTrait, VfsOps, set_global_block_device,VfsNodeOps};
pub use blueosfs::{BlueosFileSystem, DATABITMAP_COUNT, INODEBITMAP_COUNT};