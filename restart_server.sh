#!/bin/bash

# 重新构建并启动 card-server 的脚本

set -e

echo "[RESTART] 正在查找并关闭现有的 card-server 进程..."

# 查找并关闭现有的 card-server 进程
if pgrep -f "card-server" > /dev/null; then
    echo "[RESTART] 发现正在运行的 card-server 进程，正在关闭..."
    pkill -f "card-server" || true
    sleep 2
    # 强制关闭如果还在运行
    if pgrep -f "card-server" > /dev/null; then
        echo "[RESTART] 强制关闭进程中..."
        pkill -9 -f "card-server" || true
        sleep 1
    fi
    echo "[RESTART] 已关闭现有进程"
else
    echo "[RESTART] 没有正在运行的 card-server 进程"
fi

# 也检查 cargo run 进程
if pgrep -f "cargo run.*card-server" > /dev/null; then
    echo "[RESTART] 发现正在运行的 cargo 进程，正在关闭..."
    pkill -f "cargo run.*card-server" || true
    sleep 2
fi

echo "[RESTART] 正在重新构建 card-server..."
cd /Users/zhouzihao/RustroverProjects/card_v3

# 构建 release 版本
cargo build --release -p card-server

echo "[RESTART] 构建完成，正在启动服务..."

# 启动服务并记录日志
nohup ./target/release/card-server > /Users/zhouzihao/RustroverProjects/card_v3/logs/server.log 2>&1 &

# 等待服务启动
sleep 2

# 检查服务是否成功启动
if pgrep -f "card-server" > /dev/null; then
    echo "[RESTART] ✅ card-server 已成功启动！"
    echo "[RESTART] 日志文件: /Users/zhouzihao/RustroverProjects/card_v3/logs/server.log"
    echo "[RESTART] 使用 'tail -f /Users/zhouzihao/RustroverProjects/card_v3/logs/server.log' 查看日志"
else
    echo "[RESTART] ❌ card-server 启动失败，请检查日志"
    exit 1
fi
