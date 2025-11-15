use crate::config::{get_app_data, get_app_num};
use BlueosFS::{create_dir, create_file, write_file};
use log::info;
use alloc::format;

/// 将 app.asm 中的测试文件加载到文件系统的 /test 目录
/// 文件命名规则：init, idle, create_and_read_file, for_read, i_can_yield, 
///              loop, loop2, printf, switch, sys_map, unmap
pub fn make_testfile() {
    // 创建 /test 目录
    if let Err(e) = create_dir("/test") {
        // 如果目录已存在，忽略错误
        info!("Directory /test may already exist: {:?}", e);
    }
    
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
    
    let app_count = get_app_num();
    info!("Loading {} test files into /test directory", app_count);
    
    // 将每个应用写入文件系统
    for app_id in 0..app_count {
        let app_name = if app_id < app_names.len() {
            app_names[app_id]
        } else {
            // 如果应用数量超过预定义名称，使用数字命名
            panic!("App count exceeds predefined names");
        };
        
        let file_path = format!("/test/{}", app_name);
        
        // 获取应用数据
        let app_data = get_app_data(app_id);
        
        // 创建并写入文件
        if let Err(e) = create_file(&file_path) {
            info!("File {} may already exist: {:?}", file_path, e);
        }
        
        if let Err(e) = write_file(&file_path, app_data) {
            panic!("Failed to write {}: {:?}", file_path, e);
        }
        
        info!("Loaded {} ({} bytes) to {}", app_name, app_data.len(), file_path);
    }
    
    info!("All test files loaded successfully");
}
