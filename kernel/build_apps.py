#!/usr/bin/env python3
"""
BlueStarOS 应用程序构建脚本
自动扫描 user/src/bin/ 目录，生成 app.asm 文件
"""

import os
import sys
from pathlib import Path

# 配置
USER_DIR = Path("../user")
USER_BIN_DIR = USER_DIR / "src" / "bin"
USER_TARGET_DIR = USER_DIR / "target" / "riscv64gc-unknown-none-elf" / "release"
OUTPUT_ASM = Path("src/app.asm")

def find_user_apps():
    """
    查找所有用户程序
    返回: [(app_name, elf_path), ...]
    注意: init.rs 固定为索引 0，idle.rs 固定为索引 1，其他应用按名称排序
    """
    if not USER_BIN_DIR.exists():
        print(f"Error: {USER_BIN_DIR} not found!", file=sys.stderr)
        return []
    
    # 特殊应用：固定位置
    init_app = None
    idle_app = None
    other_apps = []
    
    for rs_file in USER_BIN_DIR.glob("*.rs"):
        app_name = rs_file.stem
        # 跳过一些特殊文件
        if app_name.startswith("_") or app_name.startswith("."):
            continue
        
        elf_path = USER_TARGET_DIR / app_name
        
        # 检查是否是特殊应用
        if app_name == "init":
            init_app = (app_name, elf_path)
        elif app_name == "idle":
            idle_app = (app_name, elf_path)
        else:
            other_apps.append((app_name, elf_path))
    
    # 其他应用按名称排序
    other_apps.sort(key=lambda x: x[0])
    
    # 组装最终列表：init 固定为 0，idle 固定为 1，其他应用从 2 开始
    apps = []
    if init_app:
        apps.append(init_app)
    if idle_app:
        apps.append(idle_app)
    apps.extend(other_apps)
    
    return apps

def generate_app_asm(apps):
    """
    生成 app.asm 文件
    """
    if not apps:
        print("Warning: No user applications found!", file=sys.stderr)
        # 生成空的 app.asm
        asm_content = """
#被链接到data段
.section .data.app
.global app_list_start
.global app_list_end
app_list_start:
app_list_end:
"""
        return asm_content
    
    lines = []
    lines.append("")
    lines.append("")
    lines.append("#被链接到data段")
    lines.append(".section .data.app")
    lines.append(".global app_list_start")
    lines.append(".global app_list_end")
    
    # 生成应用列表
    lines.append("app_list_start:")
    for i, (app_name, _) in enumerate(apps, 1):
        lines.append(f"    .quad app_{i}_start")
        lines.append(f"    .quad app_{i}_end")
    lines.append("app_list_end:")
    lines.append("")
    
    # 生成应用数据段
    for i, (app_name, elf_path) in enumerate(apps, 1):
        lines.append(f"app_{i}_start:")
        # 使用相对于 kernel 目录的路径
        relative_path = f"../{elf_path.relative_to(Path('..'))}"
        lines.append(f'.incbin "{relative_path}"')
        lines.append(f"app_{i}_end:")
    
    return "\n".join(lines)

def write_app_asm(content):
    """
    写入 app.asm 文件
    """
    OUTPUT_ASM.parent.mkdir(parents=True, exist_ok=True)
    
    with open(OUTPUT_ASM, "w", encoding="utf-8") as f:
        f.write(content)
    
    # Windows 兼容的输出（避免 Unicode 错误）
    try:
        print(f"✓ Generated {OUTPUT_ASM}")
    except UnicodeEncodeError:
        print(f"[OK] Generated {OUTPUT_ASM}")

def main():
    print("=" * 60)
    print("  BlueStarOS Application Builder")
    print("=" * 60)
    
    # 查找应用
    print(f"\n[1/3] Scanning {USER_BIN_DIR}...")
    apps = find_user_apps()
    
    if apps:
        print(f"      Found {len(apps)} application(s):")
        for i, (app_name, elf_path) in enumerate(apps):
            # 显示索引（从 0 开始）和应用名称
            if app_name == "init":
                print(f"      [{i}] {app_name} (fixed at index 0)")
            elif app_name == "idle":
                print(f"      [{i}] {app_name} (fixed at index 1)")
            else:
                print(f"      [{i}] {app_name}")
    else:
        print("      No applications found!")
    
    # 生成 app.asm
    print(f"\n[2/3] Generating {OUTPUT_ASM}...")
    asm_content = generate_app_asm(apps)
    
    # 写入文件
    print(f"\n[3/3] Writing to disk...")
    write_app_asm(asm_content)
    
    print("\n" + "=" * 60)
    try:
        print(f"  ✓ Build configuration complete!")
    except UnicodeEncodeError:
        print(f"  [OK] Build configuration complete!")
    print(f"  Total applications: {len(apps)}")
    print("=" * 60)
    print()
    
    return 0

if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\n\nAborted by user.", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"\nError: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        sys.exit(1)

