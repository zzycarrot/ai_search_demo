#!/bin/bash

# 测试脚本：验证初始文件扫描和监听功能

echo "🧪 测试多线程文档搜索引擎"
echo "=============================="

# 清理环境
rm -rf storage/* docs/*
mkdir -p docs storage

# 创建初始测试文档
echo "人工智能是计算机科学的一个重要分支。" > docs/ai.txt
echo "机器学习是AI的核心技术。" > docs/ml.txt

echo "📄 已创建初始文档："
ls -la docs/

# 启动程序（后台运行5秒）
echo ""
echo "🚀 启动程序..."
timeout 8s cargo run &
PROGRAM_PID=$!

# 等待程序启动并扫描文件
sleep 3

# 在程序运行期间添加新文件
echo ""
echo "📝 添加新文件..."
echo "深度学习使用神经网络。" > docs/dl.txt

# 等待程序处理
sleep 2

# 停止程序
kill $PROGRAM_PID 2>/dev/null
wait $PROGRAM_PID 2>/dev/null

echo ""
echo "✅ 测试完成！"
echo "检查结果："
echo "- 初始文件是否被扫描: $(ls storage/ | wc -l) 个索引文件"
echo "- 监听是否工作: 查看上面的程序输出"