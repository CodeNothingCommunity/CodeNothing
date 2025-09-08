#!/bin/bash

echo "=== 硬编码修复验证脚本 ==="

echo "1. 编译项目..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ 编译成功！"
else
    echo "❌ 编译失败！"
    exit 1
fi

echo ""
echo "2. 测试现有功能..."

# 测试现有的演示文件
echo "测试 complete_loops_demo.cn:"
./target/release/CodeNothing.exe complete_loops_demo.cn > test_output.txt 2>&1

if [ $? -eq 0 ]; then
    echo "✅ complete_loops_demo.cn 运行成功！"
    echo "输出行数: $(wc -l < test_output.txt)"
else
    echo "❌ complete_loops_demo.cn 运行失败！"
    cat test_output.txt
fi

echo ""
echo "3. 检查硬编码修复..."

# 检查源代码中是否还有硬编码
echo "检查 codegen.rs 中的硬编码使用情况:"

# 检查命名空间分隔符
namespace_count=$(grep -c '"::"' src/lang/codegen.rs || echo "0")
echo "  剩余硬编码 '::' 数量: $namespace_count"

# 检查主函数名
main_count=$(grep -c '"main"' src/lang/codegen.rs || echo "0")
echo "  剩余硬编码 'main' 数量: $main_count"

# 检查常量使用
constants_count=$(grep -c 'constants::' src/lang/codegen.rs || echo "0")
echo "  使用 constants:: 的地方: $constants_count"

echo ""
echo "4. 验证常量定义..."

# 检查常量模块是否存在
if grep -q "mod constants" src/lang/codegen.rs; then
    echo "✅ 常量模块已定义"
    
    # 列出定义的常量
    echo "定义的常量:"
    grep -A 20 "mod constants" src/lang/codegen.rs | grep "pub const" | sed 's/^/  /'
else
    echo "❌ 常量模块未找到"
fi

echo ""
echo "=== 验证完成 ==="

# 清理临时文件
rm -f test_output.txt
