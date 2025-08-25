# 🍭 CodeNothing 语法糖提案列表

为了让 CodeNothing 语言更加简洁、优雅和易用，以下是建议添加的语法糖功能：

## 🔥 高优先级语法糖

### 1. 字符串插值 (String Interpolation)
**当前语法:**
```codenothing
var name = "Alice";
var age = 25;
var message = "Hello, " + name + "! You are " + lang::to_string(age) + " years old.";
```

**语法糖:**
```codenothing
var message = "Hello, ${name}! You are ${age} years old.";
var calculation = "Result: ${x + y} (${(x + y) * 100}%)";
```

### 2. 自增/自减操作符 (Increment/Decrement)
**当前语法:**
```codenothing
counter = counter + 1;
index = index - 1;
total = total + 5;
```

**语法糖:**
```codenothing
counter++;     // 后置自增
++counter;     // 前置自增
index--;       // 后置自减
--index;       // 前置自减
total += 5;    // 复合赋值
score *= 2;    // 乘法赋值
remainder %= 10; // 取模赋值
```

### 3. 三元条件操作符 (Ternary Operator)
**当前语法:**
```codenothing
var result = match condition {
    true => "yes",
    false => "no",
};
```

**语法糖:**
```codenothing
var result = condition ? "yes" : "no";
var max = a > b ? a : b;
var status = score >= 60 ? "pass" : "fail";
```

## ⭐ 中优先级语法糖

### 4. 数组字面量 (Array Literals)
**当前语法:**
```codenothing
var arr = Array::new();
arr.push(1);
arr.push(2);
arr.push(3);
```

**语法糖:**
```codenothing
var numbers = [1, 2, 3, 4, 5];
var names = ["Alice", "Bob", "Charlie"];
var mixed = [1, "hello", true, 3.14];
var empty = [];
```

### 5. 范围表达式 (Range Expressions)
**当前语法:**
```codenothing
var i = 1;
while (i <= 10) {
    std::println(lang::to_string(i));
    i = i + 1;
}
```

**语法糖:**
```codenothing
for i in 1..10 {
    std::println(i);
}

for i in 1..=10 {  // 包含结束值
    std::println(i);
}

for item in array {
    std::println(item);
}
```

### 6. 空值合并操作符 (Null Coalescing)
**当前语法:**
```codenothing
var result = match value {
    null => default_value,
    _ => value,
};
```

**语法糖:**
```codenothing
var result = value ?? default_value;
var name = user.name ?? "Unknown";
var config = user_config ?? default_config ?? {};
```

### 7. 解构赋值 (Destructuring Assignment)
**当前语法:**
```codenothing
var point = get_point();
var x = point.x;
var y = point.y;
```

**语法糖:**
```codenothing
var {x, y} = get_point();
var [first, second, third] = get_array();
var {name, age, ...rest} = get_person();
```

## 💡 低优先级语法糖

### 8. 管道操作符 (Pipe Operator)
**当前语法:**
```codenothing
var result = process_c(process_b(process_a(data)));
```

**语法糖:**
```codenothing
var result = data |> process_a |> process_b |> process_c;

var final_result = input_data
    |> validate
    |> transform
    |> filter
    |> sort;
```

### 9. 箭头函数 (Arrow Functions)
**当前语法:**
```codenothing
fn add(a: int, b: int) : int {
    return a + b;
}
```

**语法糖:**
```codenothing
var add = (a: int, b: int) => a + b;
var multiply = (x, y) => x * y;  // 类型推断
var greet = name => "Hello, " + name;  // 单参数
```

### 10. 链式调用 (Method Chaining)
**当前语法:**
```codenothing
var result = process_step1(data);
result = process_step2(result);
result = process_step3(result);
```

**语法糖:**
```codenothing
var result = data
    .process_step1()
    .process_step2()
    .process_step3();
```

### 11. when 表达式 (When Expression)
**当前语法:**
```codenothing
var grade = match score {
    _ => if (score >= 90) {
        "A"
    } else if (score >= 80) {
        "B"
    } else {
        "C"
    },
};
```

**语法糖:**
```codenothing
var grade = when {
    score >= 90 => "A",
    score >= 80 => "B",
    score >= 70 => "C",
    else => "F"
};
```

### 12. 展开操作符 (Spread Operator)
**当前语法:**
```codenothing
var new_array = Array::new();
for item in old_array {
    new_array.push(item);
}
new_array.push(new_item);
```

**语法糖:**
```codenothing
var new_array = [...old_array, new_item];
var combined = [...array1, ...array2, ...array3];
```

## 🎯 实现优先级建议

### 第一阶段 (立即实现)
1. **字符串插值** - 大幅提升字符串处理体验
2. **自增/自减操作符** - 基础但重要的语法糖
3. **三元条件操作符** - 简化条件表达式

### 第二阶段 (短期实现)
4. **数组字面量** - 提升数据结构使用体验
5. **范围表达式** - 简化循环语法
6. **空值合并操作符** - 提升空值处理安全性

### 第三阶段 (中期实现)
7. **解构赋值** - 提升数据提取效率
8. **when 表达式** - 增强条件匹配能力
9. **箭头函数** - 简化函数定义

### 第四阶段 (长期实现)
10. **管道操作符** - 提升函数组合体验
11. **链式调用** - 增强面向对象编程体验
12. **展开操作符** - 简化数组/对象操作

## 🔧 技术实现考虑

### 词法分析器 (Lexer) 需要添加的 Token
- `PlusPlus` (`++`)
- `MinusMinus` (`--`)
- `PlusEqual` (`+=`)
- `Question` (`?`)
- `DoubleQuestion` (`??`)
- `Arrow` (`=>`)
- `Pipe` (`|>`)
- `Spread` (`...`)

### 解析器 (Parser) 需要添加的语法规则
- 字符串插值解析
- 三元表达式解析
- 数组字面量解析
- 范围表达式解析
- 解构赋值解析

### 代码生成器 (CodeGen) 需要的新功能
- 字符串插值代码生成
- 自增/自减操作码生成
- 数组字面量初始化
- 范围迭代器实现

## 🎉 预期效果

实现这些语法糖后，CodeNothing 代码将变得更加：
- **简洁** - 减少样板代码
- **可读** - 提高代码可读性
- **现代** - 符合现代编程语言趋势
- **高效** - 提升开发效率
- **优雅** - 让代码更加优雅美观

这些语法糖将使 CodeNothing 成为一个更加现代化和用户友好的编程语言！
