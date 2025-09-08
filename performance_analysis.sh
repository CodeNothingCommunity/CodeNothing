#!/bin/bash

echo "=== CodeNothing 性能优化分析 ==="
echo

# 1. 分析当前性能问题
echo "1. 分析当前性能问题..."

# 检查clone()使用情况
clone_count=$(grep -c "\.clone()" src/lang/codegen.rs)
echo "  clone()调用次数: $clone_count"

# 检查unwrap()使用情况
unwrap_count=$(grep -c "\.unwrap()" src/lang/codegen.rs)
echo "  unwrap()调用次数: $unwrap_count"

# 检查to_string()使用情况
to_string_count=$(grep -c "\.to_string()" src/lang/codegen.rs)
echo "  to_string()调用次数: $to_string_count"

# 检查Vec::new()使用情况
vec_new_count=$(grep -c "Vec::new()" src/lang/codegen.rs)
echo "  Vec::new()调用次数: $vec_new_count"

# 检查Vec::with_capacity()使用情况
vec_capacity_count=$(grep -c "Vec::with_capacity" src/lang/codegen.rs)
echo "  Vec::with_capacity()调用次数: $vec_capacity_count"

echo

# 2. 找出最频繁的clone()调用
echo "2. 分析clone()使用模式..."
echo "  最频繁的clone()调用:"
grep -n "\.clone()" src/lang/codegen.rs | head -10 | sed 's/^/    /'

echo

# 3. 找出unwrap()调用位置
echo "3. 分析unwrap()使用位置..."
if [ $unwrap_count -gt 0 ]; then
    echo "  unwrap()调用位置:"
    grep -n "\.unwrap()" src/lang/codegen.rs | sed 's/^/    /'
else
    echo "  ✅ 未发现unwrap()调用"
fi

echo

# 4. 分析字符串分配模式
echo "4. 分析字符串分配模式..."

# 检查String::from使用
string_from_count=$(grep -c "String::from" src/lang/codegen.rs)
echo "  String::from()调用次数: $string_from_count"

# 检查format!宏使用
format_count=$(grep -c "format!" src/lang/codegen.rs)
echo "  format!宏调用次数: $format_count"

# 检查字符串连接
string_concat_count=$(grep -c " + " src/lang/codegen.rs | head -1)
echo "  字符串连接操作估计: $string_concat_count"

echo

# 5. 创建性能测试用例
echo "5. 创建性能测试用例..."

cat > performance_test.cn << 'EOF'
// 性能测试用例 - 大量表达式和操作
fn main() {
    // 大量变量声明
    var a1 : int = 1;
    var a2 : int = 2;
    var a3 : int = 3;
    var a4 : int = 4;
    var a5 : int = 5;
    var a6 : int = 6;
    var a7 : int = 7;
    var a8 : int = 8;
    var a9 : int = 9;
    var a10 : int = 10;
    
    // 大量计算表达式
    var result1 : int = a1 + a2 * a3 - a4 / a5;
    var result2 : int = a6 + a7 * a8 - a9 / a10;
    var result3 : int = result1 + result2;
    var result4 : int = result3 * 2;
    var result5 : int = result4 + 100;
    
    // 大量字符串操作
    var str1 : string = "Hello";
    var str2 : string = "World";
    var str3 : string = "Test";
    var str4 : string = "Performance";
    
    // 大量数组操作
    var arr1 : array = [1, 2, 3, 4, 5];
    var arr2 : array = [6, 7, 8, 9, 10];
    var arr3 : array = [11, 12, 13, 14, 15];
    
    // 大量循环
    for (var i : int = 0; i < 10; i++) {
        var temp : int = i * 2;
    }
    
    for (var j : int = 0; j < 5; j++) {
        var temp2 : int = j + 1;
    }
}
EOF

echo "✅ 性能测试用例已创建"

echo

# 6. 基准性能测试
echo "6. 基准性能测试..."

echo "编译项目..."
if cargo build --quiet; then
    echo "✅ 编译成功"
else
    echo "❌ 编译失败"
    exit 1
fi

echo "运行基准测试（10次）:"
total_time=0
for i in {1..10}; do
    start_time=$(date +%s%N)
    ./target/debug/CodeNothing.exe performance_test.cn > /dev/null 2>&1
    end_time=$(date +%s%N)
    duration=$((($end_time - $start_time) / 1000000))
    echo "  第${i}次: ${duration}ms"
    total_time=$((total_time + duration))
done

average_time=$((total_time / 10))
echo "  平均时间: ${average_time}ms"

echo

# 7. 内存使用分析
echo "7. 内存使用分析..."

# 检查大的数据结构
echo "  检查可能的内存热点:"
echo "    - HashMap使用次数: $(grep -c "HashMap" src/lang/codegen.rs)"
echo "    - Vec使用次数: $(grep -c "Vec<" src/lang/codegen.rs)"
echo "    - String使用次数: $(grep -c "String" src/lang/codegen.rs)"

echo

# 8. 清理
rm -f performance_test.cn

echo "=== 性能分析完成 ==="
echo
echo "📊 **性能问题总结**:"
echo "  - clone()调用: $clone_count 次 (需要优化)"
echo "  - unwrap()调用: $unwrap_count 次 (需要改进错误处理)"
echo "  - to_string()调用: $to_string_count 次 (可以优化)"
echo "  - Vec预分配: $vec_capacity_count/$vec_new_count (需要改进)"
echo "  - 平均执行时间: ${average_time}ms"
echo
echo "🎯 **优化建议**:"
echo "  1. 减少不必要的clone()调用，使用引用"
echo "  2. 将unwrap()替换为适当的错误处理"
echo "  3. 使用Vec::with_capacity()预分配容量"
echo "  4. 减少字符串分配，使用&str或Cow<str>"
echo "  5. 缓存频繁使用的字符串"
