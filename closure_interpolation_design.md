# CodeNothing 字符串插值中的闭包支持设计

## 概述

在 CodeNothing 的字符串插值功能基础上，添加对闭包（Lambda 表达式）的支持，使字符串插值更加强大和灵活。

## 语法设计

### 基础闭包插值
```codenothing
var name = "World";
var greeting = "Hello ${|x| x + " " + name}("Beautiful")!";
// 输出: "Hello Beautiful World!"
```

### 立即执行闭包
```codenothing
var x = 10;
var y = 20;
var result = "Sum: ${|a, b| a + b}(x, y)";
// 输出: "Sum: 30"
```

### 条件闭包
```codenothing
var isActive = true;
var status = "Status: ${|flag| flag ? "Active" : "Inactive"}(isActive)";
// 输出: "Status: Active"
```

### 数组处理闭包
```codenothing
var numbers = [1, 2, 3, 4, 5];
var sum = "Total: ${|arr| arr.reduce(|a, b| a + b, 0)}(numbers)";
// 输出: "Total: 15"
```

## 技术实现

### 1. 词法分析增强

在 `read_interpolated_string` 方法中识别闭包语法：

```rust
// 检测闭包开始标记 |
if expr_ch == '|' && !in_string {
    // 进入闭包参数解析模式
    in_closure_params = true;
    closure_depth += 1;
}
```

### 2. 语法分析增强

在插值表达式解析中支持闭包：

```rust
fn parse_interpolation_expression(&mut self, expr_str: &str) -> Result<Expression, ParseError> {
    // 检查是否包含闭包语法
    if expr_str.contains('|') {
        return self.parse_closure_interpolation(expr_str);
    }
    // 原有逻辑...
}

fn parse_closure_interpolation(&mut self, expr_str: &str) -> Result<Expression, ParseError> {
    // 解析闭包定义和调用
    // 支持 |params| body 和 |params| body(args) 语法
}
```

### 3. 语义分析增强

```rust
fn analyze_closure_interpolation(&mut self, closure_expr: &Expression) -> Type {
    match closure_expr {
        Expression::Lambda { parameters, body, .. } => {
            // 分析闭包参数和返回类型
            // 处理变量捕获
            self.analyze_closure_capture(parameters, body)
        }
        _ => Type::String
    }
}
```

### 4. 代码生成增强

```rust
fn generate_closure_interpolation(&mut self, closure_expr: &Expression) -> Result<(), String> {
    match closure_expr {
        Expression::Lambda { parameters, body, .. } => {
            // 生成闭包字节码
            let closure_index = self.generate_closure_chunk(parameters, body)?;
            
            // 生成闭包调用指令
            self.emit_instruction(Instruction::with_u16(OpCode::CreateClosure, closure_index, 0));
            
            // 如果有立即调用，生成调用指令
            self.emit_instruction(Instruction::new(OpCode::CallIndirect, 0));
            
            // 转换结果为字符串
            self.emit_instruction(Instruction::new(OpCode::ToString, 0));
        }
        _ => return Err("Invalid closure expression".to_string())
    }
    Ok(())
}
```

### 5. 虚拟机增强

添加新的操作码支持闭包：

```rust
// 新增操作码
pub enum OpCode {
    // ... 现有操作码
    
    /// 创建闭包对象
    /// Operand: closure_index (u16), capture_count (u8)
    CreateClosure = 0x78,
    
    /// 捕获变量到闭包
    /// Operand: var_index (u16)
    CaptureVariable = 0x79,
    
    /// 调用闭包
    /// Stack: [closure, arg1, arg2, ..., argN, param_count]
    CallClosure = 0x7A,
}
```

虚拟机执行逻辑：

```rust
impl VirtualMachine {
    fn execute_create_closure(&mut self, instruction: &Instruction) -> VMResult<ExecutionResult> {
        let closure_index = instruction.get_u16(0)?;
        let capture_count = instruction.get_u8(2)?;
        
        // 创建闭包对象
        let mut captured_vars = Vec::new();
        for _ in 0..capture_count {
            captured_vars.push(self.stack.pop()?);
        }
        
        let closure = Value::Closure {
            chunk_index: closure_index as usize,
            captured_variables: captured_vars,
        };
        
        self.stack.push(closure)?;
        Ok(ExecutionResult::Continue)
    }
    
    fn execute_call_closure(&mut self) -> VMResult<ExecutionResult> {
        let param_count = self.stack.pop()?.as_int()? as usize;
        let mut args = Vec::new();
        
        for _ in 0..param_count {
            args.push(self.stack.pop()?);
        }
        
        let closure = self.stack.pop()?;
        
        if let Value::Closure { chunk_index, captured_variables } = closure {
            // 设置闭包执行环境
            self.setup_closure_environment(chunk_index, args, captured_variables)?;
        }
        
        Ok(ExecutionResult::Continue)
    }
}
```

## 实现优先级

### Phase 1: 基础闭包支持
1. ✅ 词法分析器增强 - 识别闭包语法
2. ✅ 语法分析器增强 - 解析闭包表达式
3. ✅ 代码生成器增强 - 生成闭包字节码
4. ✅ 虚拟机增强 - 执行闭包指令

### Phase 2: 变量捕获
1. 🔄 实现词法作用域分析
2. 🔄 添加变量捕获机制
3. 🔄 优化闭包性能

### Phase 3: 高级特性
1. ⏳ 支持嵌套闭包
2. ⏳ 支持闭包作为返回值
3. ⏳ 支持高阶函数

## 示例用例

### 数据处理
```codenothing
var data = [1, 2, 3, 4, 5];
var processed = "Doubled: ${|arr| arr.map(|x| x * 2)}(data)";
```

### 条件逻辑
```codenothing
var score = 85;
var grade = "Grade: ${|s| s >= 90 ? "A" : s >= 80 ? "B" : "C"}(score)";
```

### 函数式编程
```codenothing
var numbers = [1, 2, 3, 4, 5];
var result = "Result: ${|arr| arr.filter(|x| x % 2 == 0).reduce(|a, b| a + b, 0)}(numbers)";
```

## 性能考虑

1. **闭包缓存**: 缓存编译后的闭包字节码
2. **内联优化**: 简单闭包可以内联执行
3. **变量捕获优化**: 只捕获实际使用的变量
4. **JIT 编译**: 热点闭包可以 JIT 编译

## 错误处理

1. **语法错误**: 闭包语法不正确
2. **类型错误**: 参数类型不匹配
3. **运行时错误**: 闭包执行异常
4. **捕获错误**: 变量捕获失败

## 总结

在字符串插值中加入闭包逻辑是完全可行的，这将使 CodeNothing 的字符串处理能力达到一个新的高度。通过分阶段实现，我们可以逐步构建一个强大而高效的闭包插值系统。
