#!/bin/bash

# AI 文档搜索引擎 - 多线程全栈服务演示脚本
# 这个脚本会启动搜索服务，然后自动添加一些测试文档并演示搜索功能

echo "🚀 AI 文档搜索引擎 - 多线程全栈服务演示"
echo "=============================================="

# 检查是否在正确的目录
if [ ! -f "Cargo.toml" ]; then
    echo "❌ 请在项目根目录运行此脚本"
    exit 1
fi

# 创建必要的目录
echo "📁 创建目录..."
mkdir -p docs storage

# 清理旧的索引（可选）
read -p "是否清理旧的索引数据？(y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "🗑️ 清理旧索引..."
    rm -rf storage/*
fi

# 启动搜索服务（后台运行）
echo "🔄 启动多线程全栈服务..."
cargo run &
SERVICE_PID=$!

# 等待服务启动
sleep 3

echo ""
echo "📝 开始添加测试文档..."
echo "=============================="

# 添加一些测试文档
echo "人工智能是计算机科学的一个重要分支，涉及机器学习、深度学习等技术。" > docs/ai_intro.txt
echo "✅ 已添加: ai_intro.txt"
sleep 1

echo "机器学习是人工智能的核心，通过算法让计算机从数据中学习规律。" > docs/ml_basics.txt
echo "✅ 已添加: ml_basics.txt"
sleep 1

echo "深度学习使用神经网络模拟人脑处理信息的方式，取得了突破性进展。" > docs/deep_learning.txt
echo "✅ 已添加: deep_learning.txt"
sleep 1

echo "自然语言处理让计算机能够理解和生成人类语言，是AI的重要应用领域。" > docs/nlp_intro.txt
echo "✅ 已添加: nlp_intro.txt"
sleep 1

echo ""
echo "🔍 演示搜索功能..."
echo "=================="

# 等待所有文档被索引
sleep 2

echo "✅ 演示完成！"
echo "=============="
echo "服务正在后台运行 (PID: $SERVICE_PID)"
echo "你可以继续手动测试搜索功能"
echo ""
echo "💡 使用提示:"
echo "   - 在服务提示符下输入关键词进行搜索"
echo "   - 输入 'quit' 退出服务"
echo "   - 后台会自动监听和索引新文档"
echo ""
echo "停止服务请运行: kill $SERVICE_PID"
echo "或按 Ctrl+C 停止当前脚本"

# 等待用户输入
read -p "按 Enter 键停止演示..."

# 停止服务
echo "🛑 停止服务..."
kill $SERVICE_PID 2>/dev/null

echo "✅ 演示结束"
echo "   - 支持 .txt, .md, .pdf 等格式"
echo ""
echo "🔄 如需完整功能，请运行: cargo run"