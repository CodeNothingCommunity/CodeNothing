#!/bin/bash

echo "=== CodeNothing codegen.rs 代码质量分析 ==="
echo

# 1. 分析函数长度
echo "1. 分析函数长度..."
echo "查找可能过长的函数："

# 创建临时文件来分析函数长度
temp_file="temp_function_analysis.txt"
awk '
/^[[:space:]]*fn / { 
    func_name = $0; 
    func_line = NR; 
    brace_count = 0; 
    in_function = 1;
    lines_in_func = 0;
}
in_function {
    lines_in_func++;
    # 计算大括号
    for(i=1; i<=length($0); i++) {
        char = substr($0, i, 1);
        if(char == "{") brace_count++;
        if(char == "}") brace_count--;
    }
    if(brace_count == 0 && in_function && lines_in_func > 1) {
        if(lines_in_func > 50) {
            print "⚠️  长函数: " func_name " (行数: " lines_in_func ", 起始行: " func_line ")";
        } else if(lines_in_func > 30) {
            print "📝 中等函数: " func_name " (行数: " lines_in_func ", 起始行: " func_line ")";
        }
        in_function = 0;
    }
}' src/lang/codegen.rs > $temp_file

if [ -s $temp_file ]; then
    cat $temp_file
else
    echo "✅ 未发现明显过长的函数"
fi
echo

# 2. 分析重复代码模式
echo "2. 分析重复代码模式..."

# 查找重复的错误处理模式
error_patterns=$(grep -c "self.errors.push" src/lang/codegen.rs)
echo "  错误处理调用次数: $error_patterns"

# 查找重复的调试输出模式
debug_patterns=$(grep -c "if self.debug_mode" src/lang/codegen.rs)
echo "  调试模式检查次数: $debug_patterns"

# 查找重复的指令生成模式
emit_patterns=$(grep -c "self.emit_instruction" src/lang/codegen.rs)
echo "  指令生成调用次数: $emit_patterns"

# 查找重复的常量添加模式
const_patterns=$(grep -c "self.add_constant" src/lang/codegen.rs)
echo "  常量添加调用次数: $const_patterns"

echo

# 3. 分析复杂的条件逻辑
echo "3. 分析复杂条件逻辑..."

# 查找深层嵌套的if语句
nested_if_count=$(grep -E "^[[:space:]]{12,}if " src/lang/codegen.rs | wc -l)
echo "  深层嵌套if语句 (缩进12+): $nested_if_count"

# 查找复杂的match语句
match_arms=$(grep -c "=>" src/lang/codegen.rs)
echo "  match分支总数: $match_arms"

# 查找长的条件表达式
long_conditions=$(grep -E "if.*&&.*&&" src/lang/codegen.rs | wc -l)
echo "  复杂条件表达式 (多个&&): $long_conditions"

echo

# 4. 分析错误处理
echo "4. 分析错误处理..."

# 查找unwrap()调用
unwrap_count=$(grep -c "\.unwrap()" src/lang/codegen.rs)
echo "  unwrap()调用次数: $unwrap_count"
if [ $unwrap_count -gt 0 ]; then
    echo "  ⚠️  建议减少unwrap()使用，改用适当的错误处理"
fi

# 查找expect()调用
expect_count=$(grep -c "\.expect(" src/lang/codegen.rs)
echo "  expect()调用次数: $expect_count"

# 查找panic!调用
panic_count=$(grep -c "panic!" src/lang/codegen.rs)
echo "  panic!调用次数: $panic_count"
if [ $panic_count -gt 0 ]; then
    echo "  ⚠️  发现panic!调用，建议使用Result返回错误"
fi

echo

# 5. 分析命名规范
echo "5. 分析命名规范..."

# 查找可能的命名问题
single_letter_vars=$(grep -oE "\b[a-z]\b" src/lang/codegen.rs | sort | uniq -c | sort -nr | head -5)
echo "  单字母变量使用情况:"
echo "$single_letter_vars" | sed 's/^/    /'

# 查找很长的函数名
long_function_names=$(grep -oE "fn [a-zA-Z_][a-zA-Z0-9_]{25,}" src/lang/codegen.rs)
if [ -n "$long_function_names" ]; then
    echo "  过长的函数名:"
    echo "$long_function_names" | sed 's/^/    /'
else
    echo "  ✅ 函数名长度合理"
fi

echo

# 6. 分析注释覆盖率
echo "6. 分析注释覆盖率..."

total_lines=$(wc -l < src/lang/codegen.rs)
comment_lines=$(grep -c "^[[:space:]]*//\|^[[:space:]]*\*\|^[[:space:]]*\/\*" src/lang/codegen.rs)
doc_comment_lines=$(grep -c "^[[:space:]]*///" src/lang/codegen.rs)

echo "  总行数: $total_lines"
echo "  注释行数: $comment_lines"
echo "  文档注释行数: $doc_comment_lines"

if [ $total_lines -gt 0 ]; then
    comment_ratio=$(echo "scale=2; $comment_lines * 100 / $total_lines" | bc -l 2>/dev/null || echo "计算失败")
    echo "  注释覆盖率: $comment_ratio%"
    
    if [ $(echo "$comment_ratio < 10" | bc -l 2>/dev/null || echo "0") -eq 1 ]; then
        echo "  ⚠️  注释覆盖率较低，建议增加注释"
    fi
fi

echo

# 7. 分析代码复杂度指标
echo "7. 分析代码复杂度指标..."

# 计算圈复杂度相关指标
if_count=$(grep -c "if " src/lang/codegen.rs)
while_count=$(grep -c "while " src/lang/codegen.rs)
for_count=$(grep -c "for " src/lang/codegen.rs)
match_count=$(grep -c "match " src/lang/codegen.rs)

echo "  if语句数量: $if_count"
echo "  while循环数量: $while_count"
echo "  for循环数量: $for_count"
echo "  match语句数量: $match_count"

total_complexity=$((if_count + while_count + for_count + match_count))
echo "  总复杂度指标: $total_complexity"

if [ $total_complexity -gt 200 ]; then
    echo "  ⚠️  代码复杂度较高，建议重构"
elif [ $total_complexity -gt 100 ]; then
    echo "  📝 代码复杂度中等，可考虑适当重构"
else
    echo "  ✅ 代码复杂度合理"
fi

echo

# 8. 分析性能相关问题
echo "8. 分析潜在性能问题..."

# 查找可能的性能问题
clone_count=$(grep -c "\.clone()" src/lang/codegen.rs)
echo "  clone()调用次数: $clone_count"
if [ $clone_count -gt 50 ]; then
    echo "  ⚠️  clone()调用较多，可能影响性能"
fi

# 查找字符串分配
string_alloc=$(grep -c "String::" src/lang/codegen.rs)
to_string_count=$(grep -c "\.to_string()" src/lang/codegen.rs)
echo "  String分配次数: $string_alloc"
echo "  to_string()调用次数: $to_string_count"

# 查找Vec分配
vec_new_count=$(grep -c "Vec::new()" src/lang/codegen.rs)
vec_with_capacity=$(grep -c "Vec::with_capacity" src/lang/codegen.rs)
echo "  Vec::new()调用次数: $vec_new_count"
echo "  Vec::with_capacity()调用次数: $vec_with_capacity"

if [ $vec_new_count -gt $vec_with_capacity ] && [ $vec_new_count -gt 10 ]; then
    echo "  💡 建议在已知容量时使用Vec::with_capacity()"
fi

echo

# 清理临时文件
rm -f $temp_file

echo "=== 分析完成 ==="
echo
echo "📊 **总结建议**:"
echo "1. 关注长函数，考虑拆分为更小的函数"
echo "2. 减少重复代码模式，提取公共函数"
echo "3. 简化复杂的条件逻辑"
echo "4. 改善错误处理，减少unwrap()使用"
echo "5. 增加文档注释，提高代码可读性"
echo "6. 考虑性能优化，减少不必要的内存分配"
