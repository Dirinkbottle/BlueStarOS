use std::env;
use std::path::PathBuf;

fn main() {
    // 获取 C 驱动源码目录
    let c_driver_dir = PathBuf::from("src/driver/virtio_blk_c");
    
    // 设置头文件搜索路径
    let include_path = c_driver_dir.clone();
    
    // 获取目标架构（从 Cargo 环境变量）
    let target = env::var("TARGET").unwrap();
    
    // 编译 C 代码
    let mut build = cc::Build::new();
    build
        .file(c_driver_dir.join("virtio_blk.c"))
        .file(c_driver_dir.join("printf.c"))
        .include(&include_path)
        .flag("-fPIC") // 添加位置无关代码选项
        .flag("-ffreestanding")  // 独立环境，不依赖标准库
        .flag("-fno-common")     // 不将未初始化的全局变量放在 common section
        .flag("-nostdlib")       // 不使用标准库
        .flag("-mno-relax");     // 禁用 RISC-V 链接器松弛优化
    
    // 如果是 RISC-V 目标，添加特定标志
    if target.contains("riscv") {
        build.flag("-march=rv64gc").flag("-mabi=lp64d");
    }
    
    build.compile("virtio_blk_c");
    
    // 重新编译如果 C 文件改变
    println!("cargo:rerun-if-changed={}", c_driver_dir.join("virtio_blk.c").display());
    println!("cargo:rerun-if-changed={}", c_driver_dir.join("printf.c").display());
    println!("cargo:rerun-if-changed={}", c_driver_dir.join("virtio_blk.h").display());
    println!("cargo:rerun-if-changed={}", c_driver_dir.join("virtio_common.h").display());
    println!("cargo:rerun-if-changed={}", c_driver_dir.join("virtio_queue.h").display());
    println!("cargo:rerun-if-changed={}", c_driver_dir.join("virtio_blk_func.h").display());
    println!("cargo:rerun-if-changed={}", c_driver_dir.join("rust_print.h").display());
    println!("cargo:rerun-if-changed={}", c_driver_dir.join("printf.h").display());
}

