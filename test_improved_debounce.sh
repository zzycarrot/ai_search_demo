#!/bin/bash

# 测试改进后的防抖和去重功能
echo "🧪 测试改进后的防抖和去重功能"
echo "=================================="

# 清理环境
rm -rf storage/* docs/*
mkdir -p docs storage

# 创建测试文件
echo "人工智能是计算机科学的一个重要分支。" > docs/test1.txt
echo "机器学习是AI的核心技术。" > docs/test2.txt

echo "📄 已创建测试文件"

# 启动程序
echo ""
echo "🚀 启动程序..."
cargo run &
PROGRAM_PID=$!

# 等待程序扫描现有文件
sleep 4

echo ""
echo "📝 快速连续修改文件，测试防抖..."
for i in {1..5}; do
    echo "修改 $i: $(date +%H:%M:%S.%3N)" >> docs/test1.txt
    sleep 0.2  # 200ms间隔，应该被防抖掉
done

echo "✅ 文件修改完成"

# 等待观察
sleep 3

echo ""
echo "🔍 测试搜索功能..."
echo "人工智能" > /tmp/search_test
echo "quit" >> /tmp/search_test

# 这里可以手动测试搜索
echo "请手动输入搜索命令测试，或查看上面的输出"
echo "如果只看到一次 '新文件已索引'，说明防抖成功"

echo ""
read -p "按 Enter 键停止测试..."

# 停止程序
kill $PROGRAM_PID 2>/dev/null
wait $PROGRAM_PID 2>/dev/null

echo ""
echo "✅ 测试完成"