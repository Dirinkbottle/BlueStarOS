mod task;
use crate::config::*;
use log::debug;

/// 文件加载器，根据 app_id 加载对应的 ELF 文件
/// app_id 从 0 开始
pub fn file_loader(app_id: usize) -> &'static [u8] {
    let app_data = get_app_data(app_id);
    debug!("Loading app {} with size {} bytes", app_id, app_data.len());
    app_data
}

/// 获取应用程序总数
pub fn get_app_count() -> usize {
    get_app_num()
}

pub use task::*;