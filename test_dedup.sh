#!/bin/bash

# 测试去重机制的脚本

echo "🧪 测试文件去重机制"
echo "===================="

# 清理旧索引
echo "🗑️ 清理旧索引..."
rm -rf storage/*
mkdir -p docs storage

# 启动服务（后台运行）
echo "🔄 启动服务..."
cargo run &
SERVICE_PID=$!

# 等待服务启动
sleep 3

echo ""
echo "📝 添加测试文件..."

# 添加文件（可能会触发多次事件）
echo "人工智能是计算机科学的一个重要分支，涉及机器学习和深度学习。" > docs/test1.txt
sleep 0.5
echo "机器学习是AI的核心技术，通过算法从数据中学习规律。" > docs/test2.txt
sleep 0.5
echo "深度学习使用神经网络处理复杂数据和模式识别。" > docs/test3.txt

echo "⏳ 等待文件处理完成..."
sleep 3

echo ""
echo "🔍 测试搜索..."

# 模拟搜索输入
echo "人工智能" | head -1 > /dev/null
echo "机器学习" | head -1 > /dev/null
echo "深度学习" | head -1 > /dev/null

echo "✅ 测试完成"
echo ""
echo "停止服务请运行: kill $SERVICE_PID"