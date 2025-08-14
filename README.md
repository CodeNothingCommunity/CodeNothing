# CodeNothing

CodeNothing是世界上最好的语言。

## 🎉 v0.9.2 重大更新

### 🚀 完整的面向对象编程支持

CodeNothing 现在支持完整的面向对象编程功能，包括：

#### ✨ 字段赋值功能（全新！）
```codenothing
class Student {
    public name : string;
    public age : int;
    public score : int;

    constructor(name : string, age : int, score : int) {
        this.name = name;
        this.age = age;
        this.score = score;
    };

    public fn getInfo() : string {
        return this.name + " (年龄: " + this.age + ", 分数: " + this.score + ")";
    };
};

fn main() : int {
    student : Student = new Student("小明", 15, 85);

    // 🎯 新功能：字段赋值
    student.name = "小红";      // 修改姓名
    student.age = 16;          // 修改年龄
    student.score = 92;        // 修改分数

    // 支持复杂表达式
    old_score : int = student.score;
    student.score = old_score + 5;  // 分数加5

    std::println(student.getInfo());
    return 0;
};
```

#### 🏗️ 完整的OOP特性
- ✅ **类定义和实例化**：`class` 关键字定义类，`new` 创建实例
- ✅ **字段访问**：`object.field` 读取字段值
- ✅ **字段修改**：`object.field = value` 修改字段值 🆕
- ✅ **方法调用**：`object.method()` 调用对象方法
- ✅ **构造函数**：`constructor` 初始化对象状态
- ✅ **访问控制**：`public`/`private` 访问修饰符

### 🧠 高级特性

#### 🔧 泛型系统基础设施
- 泛型类型管理和实例化
- 类型推断和缓存机制
- 为未来的泛型编程奠定基础

#### 🚀 性能优化
- 本地内存管理器：线程本地内存池
- 循环变量优化：专门的循环内存管理
- 模式匹配JIT编译：实验性JIT优化
- 生命周期分析：智能内存安全检查

### 📚 示例程序

查看 `example/` 目录获取更多示例：
- `oop_simple_test.cn` - 基础面向对象编程
- `basic_field_test.cn` - 字段赋值功能演示
- `comprehensive_field_test.cn` - 复杂字段操作示例


## 动态库开发

CodeNothing 支持通过动态库扩展功能。动态库必须遵循以下规则：

1. 必须导出一个名为 `cn_init` 的函数，该函数返回一个包含库函数的 HashMap 指针。
2. 库函数必须接受 `Vec<String>` 类型的参数，并返回 `String` 类型的结果。

详细信息请参阅 `library_example` 目录中的示例库和说明文档。
