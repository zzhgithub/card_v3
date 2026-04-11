#!/bin/bash
# 构建 card-effect-cli 工具并复制到 Godot 项目

set -e

echo "Building card-effect-cli..."

# 构建 release 版本
cd "$(dirname "$0")"
cargo build --release -p card-effect-cli

# 确保目标目录存在
mkdir -p godot-client/tools

# 复制二进制文件到 Godot 项目
echo "Copying binary to godot-client/tools/..."

# 根据操作系统复制不同文件
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    cp target/release/card-effect-tool godot-client/tools/
    chmod +x godot-client/tools/card-effect-tool
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Linux
    cp target/release/card-effect-tool godot-client/tools/
    chmod +x godot-client/tools/card-effect-tool
else
    # Windows (假设在 Windows 上运行)
    cp target/release/card-effect-tool.exe godot-client/tools/ 2>/dev/null || echo "Windows binary not found"
fi

echo "Done! Binary location: godot-client/tools/card-effect-tool"
