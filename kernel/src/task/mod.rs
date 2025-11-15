mod task;
mod process;
use crate::config::*;
use alloc::vec::Vec;
use alloc::format;
use log::debug;

/// 文件加载器，根据 app_id 从文件系统 /test 目录加载对应的 ELF 文件
/// app_id 从 0 开始
pub fn file_loader(app_id: usize) -> Vec<u8> {
    use BlueosFS::read_file;
    
    // 应用文件名列表（对应 app.asm 中的顺序）
    let app_names = [
        "init",
        "idle",
        "create_and_read_file",
        "for_read",
        "i_can_yield",
        "loop",
        "loop2",
        "printf",
        "switch",
        "sys_map",
        "unmap",
    ];
    
    let app_name = if app_id < app_names.len() {
        app_names[app_id]
    } else {
        panic!("App id {} out of range", app_id);
    };
    
    let file_path = format!("/test/{}", app_name);
    
    match read_file(&file_path) {
        Ok(data) => {
            debug!("Loading app {} ({}) from {} with size {} bytes", app_id, app_name, file_path, data.len());
            data
        }
        Err(e) => {
            panic!("Failed to load app {} from {}: {:?}", app_id, file_path, e);
        }
    }
}

/// 获取应用程序总数
pub fn get_app_count() -> usize {
    get_app_num()
}

pub use task::*;