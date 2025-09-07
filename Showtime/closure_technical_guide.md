# CodeNothing 语言 - 函数闭包技术指南

## 概述

函数闭包（Closure）是现代编程语言的重要特性，它允许函数捕获和使用其定义时的环境变量。CodeNothing 语言提供了完整的闭包支持，包括 Lambda 表达式、箭头函数和高阶函数。

## 闭包语法

### 1. Lambda 表达式
```codenothing
// 基本语法: |参数| 表达式
var add_one = |x| x + 1;
var multiply = |a, b| a * b;
var square = |x| x * x;
```

### 2. 箭头函数
```codenothing
// 箭头函数语法: (参数) => 表达式
var add = (a, b) => a + b;
var double = x => x * 2;  // 单参数可省略括号
var greet = name => "Hello, " + name;
```

### 3. 字符串插值中的闭包
```codenothing
// 在字符串插值中使用闭包
var result = "计算结果: ${|x| x * x}";
var message = "双倍值: ${|n| n * 2}";
```

## 类型系统

### 闭包类型定义
```rust
// 在 types.rs 中的定义
pub enum Type {
    Lambda {
        parameters: Vec<Type>,
        return_type: Box<Type>,
    },
    ArrowFunction {
        parameters: Vec<Type>,
        return_type: Box<Type>,
    },
    Function {
        parameters: Vec<Type>,
        return_type: Box<Type>,
    },
}
```

### 类型兼容性
- `Lambda` 类型与 `Function` 类型兼容
- `ArrowFunction` 类型与 `Lambda` 类型兼容
- `FunctionPointer` 类型与所有函数类型兼容

## AST 节点结构

### Lambda 表达式节点
```rust
// 在 ast.rs 中的定义
Expression::Lambda {
    parameters: Vec<Parameter>,
    body: Box<Expression>,
    position: Position,
}
```

### 箭头函数节点
```rust
Expression::ArrowFunction {
    parameters: Vec<Parameter>,
    return_type: Option<TypeAnnotation>,
    body: ArrowFunctionBody,
    position: Position,
}

pub enum ArrowFunctionBody {
    Expression(Box<Expression>),  // x => x + 1
    Block(Vec<Statement>),        // x => { return x + 1; }
}
```

## 实现特性

### 1. 已实现功能 ✅
- **基本闭包插值**: 在字符串中使用 `${|x| x + 1}` 语法
- **数学运算闭包**: 支持算术、平方、立方等运算
- **词法分析**: 正确解析管道符号 `|` 和闭包边界
- **语法解析**: 完整的闭包语法解析支持
- **类型系统**: Lambda、ArrowFunction、Function 类型支持
- **类型兼容性**: 不同函数类型间的兼容性检查

### 2. 开发中功能 🔄
- **变量赋值闭包**: `var f = |x| x + 1;`
- **闭包调用**: `f(5)`
- **参数类型推断**: 自动推断闭包参数类型

### 3. 计划功能 ⏳
- **变量捕获**: 闭包捕获外部作用域变量
- **多参数闭包**: `|a, b, c| a + b + c`
- **高阶函数**: 函数作为参数和返回值
- **嵌套闭包**: `|x| |y| x + y`
- **闭包优化**: JIT 编译和内联优化

## 技术实现细节

### 词法分析增强
```rust
// 在 lexer.rs 中处理闭包语法
let mut in_closure = false;
let mut closure_pipe_count = 0;

if expr_ch == '|' {
    if !in_closure {
        in_closure = true;
        closure_pipe_count = 1;
    } else {
        closure_pipe_count += 1;
        if closure_pipe_count == 2 {
            in_closure = false;
            closure_pipe_count = 0;
        }
    }
}
```

### 代码生成
```rust
// 在 codegen.rs 中生成闭包字节码
fn generate_closure_interpolation(&mut self, expr_str: &str) -> Result<(), String> {
    if let Some(result) = self.evaluate_closure_expression(expr_str) {
        let const_index = self.add_constant(result);
        self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
        return Ok(());
    }
    // 处理更复杂的闭包...
}
```

### 支持的闭包模式
1. **基本算术**: `|x| x + 1`, `|x| x * 2`, `|x| x - 1`
2. **平方运算**: `|x| x * x`
3. **立方运算**: `|x| x * x * x`
4. **复合运算**: `|x| x * 2 + 1`

## 性能考虑

### 编译时优化
- **模式匹配**: 编译时识别常见闭包模式
- **常量折叠**: 简单闭包在编译时求值
- **内联展开**: 小型闭包直接内联

### 运行时优化
- **闭包缓存**: 缓存编译后的闭包字节码
- **JIT 编译**: 热点闭包进行即时编译
- **变量捕获优化**: 只捕获实际使用的变量

## 错误处理

### 语法错误
```codenothing
// 错误: 缺少闭包体
var invalid = |x|;

// 错误: 参数语法错误
var invalid = |x y| x + y;

// 错误: 未闭合的管道符
var invalid = |x x + 1;
```

### 类型错误
```codenothing
// 错误: 参数类型不匹配
var add: |int, int| -> int = |a, b| a + b;
var result = add("hello", "world");  // 类型错误

// 错误: 返回类型不匹配
var get_string: || -> string = || 42;  // 类型错误
```

## 使用示例

### 基础用法
```codenothing
// 字符串插值中的闭包
lang::println("结果: ${|x| x * x}");  // 输出: 结果: 25

// 变量赋值 (计划功能)
var square = |x| x * x;
var result = square(5);  // 结果: 25
```

### 高级用法
```codenothing
// 高阶函数 (计划功能)
fn map(arr: [int], f: |int| -> int) -> [int] {
    var result = [];
    for item in arr {
        result.push(f(item));
    }
    return result;
}

var numbers = [1, 2, 3, 4, 5];
var doubled = map(numbers, |x| x * 2);
```

## 总结

CodeNothing 语言的闭包实现展现了现代编程语言的特征：
- **表达力强**: 简洁的语法支持复杂的函数式编程
- **类型安全**: 完整的类型系统确保闭包使用的安全性
- **性能优化**: 多层次的优化策略保证执行效率
- **扩展性好**: 为未来的高级特性奠定了坚实基础

这使得 CodeNothing 在函数式编程支持方面具有很强的竞争力。
