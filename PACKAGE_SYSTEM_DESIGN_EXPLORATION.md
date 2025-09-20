# CodeNothing包系统设计探讨

## 🎯 项目概述

本文档探讨了基于现有include系统构建CodeNothing包系统的设计方案。包系统将提供更高级的模块化能力，同时保持与现有include系统的完全兼容性。

## 📊 当前Include系统分析

### 现有功能
- ✅ **基本Include**: `include "file.cn";`
- ✅ **通配符匹配**: `include "utils/*.cn";`
- ✅ **选择性导入**: `include "math.cn" { add, multiply };`
- ✅ **别名支持**: `include "utils.cn" { func as alias };`
- ✅ **搜索路径**: `include <std/io>;`
- ✅ **循环依赖检测**: 完整的依赖链跟踪
- ✅ **文件缓存**: 避免重复解析

### 技术架构
```
词法分析 → 语法分析 → Include处理 → 语义分析 → 代码生成
           ↑              ↑
      include关键字    文件解析&符号提取
```

## 🏗️ 包系统设计方案

### 1. 核心概念

#### 包定义 (Package Definition)
```codenothing
// package.cn - 包配置文件
package math {
    version: "1.0.0",
    author: "CodeNothing Team",
    dependencies: ["std/io", "utils"]
}

// 导出符号
export fn add(a: int, b: int) : int { return a + b; }
export var PI : float = 3.14159;

// 私有符号（不导出）
fn internal_helper() : void { }
```

#### 包导入 (Package Import)
```codenothing
// 基本包导入
import math;
import std::io;

// 选择性导入
import math { add, PI };

// 别名导入
import math as m;
import math { add as plus };

// 相对导入
import ./utils;
import ../common/helpers;
```

### 2. 目录结构设计

```
project/
├── package.cn          # 项目配置
├── src/
│   ├── main.cn        # 主程序
│   ├── math/          # math包
│   │   ├── mod.cn     # 包定义
│   │   ├── basic.cn   # 基础数学函数
│   │   └── advanced.cn # 高级数学函数
│   └── utils/
│       ├── mod.cn
│       └── string.cn
├── lib/               # 外部依赖
│   └── external_pkg/
└── tests/
    └── math_test.cn
```

### 3. AST扩展设计

```rust
// 包声明
#[derive(Debug, Clone, PartialEq)]
pub struct PackageDeclaration {
    pub name: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub dependencies: Vec<String>,
    pub position: Position,
}

// 导入声明
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDeclaration {
    pub package_path: PackagePath,
    pub alias: Option<String>,
    pub selective_imports: Option<Vec<ImportSpec>>,
    pub position: Position,
}

// 导出声明
#[derive(Debug, Clone, PartialEq)]
pub struct ExportDeclaration {
    pub declaration: Box<Declaration>,
    pub position: Position,
}

// 包路径类型
#[derive(Debug, Clone, PartialEq)]
pub enum PackagePath {
    Absolute(Vec<String>),  // std::io
    Relative(Vec<String>),  // ./utils, ../common
    Local(String),          // math
}
```

### 4. 编译流程扩展

```
源码 → 词法分析 → 语法分析 → 包解析 → Include处理 → 语义分析 → 代码生成
                              ↑
                         新增包处理阶段
```

## 🔄 与Include系统的集成

### 向后兼容策略
```codenothing
// 传统include方式仍然支持
include "utils.cn";

// 新的包导入方式
import utils;

// 混合使用
import math;
include "legacy_code.cn";
```

### 迁移路径
1. **阶段1**: 保持include系统不变，添加包系统支持
2. **阶段2**: 提供include到import的自动转换工具
3. **阶段3**: 逐步弃用include，推荐使用import

## 📋 实现计划

### Phase 1: 基础包系统
- [ ] 扩展词法分析器（package, import, export关键字）
- [ ] 扩展AST节点
- [ ] 实现基本的包解析器
- [ ] 集成到编译流程

### Phase 2: 包管理
- [ ] 实现包配置文件解析
- [ ] 添加依赖解析逻辑
- [ ] 实现包搜索和加载

### Phase 3: 高级特性
- [ ] 版本管理
- [ ] 包缓存
- [ ] 错误处理优化

## 🎯 设计优势

1. **渐进式升级**: 不破坏现有代码
2. **功能增强**: 提供更强大的模块化能力
3. **标准化**: 遵循现代编程语言的包管理模式
4. **可扩展**: 为未来功能扩展留有空间

## 📝 结论

包系统设计充分利用了现有include系统的技术基础，通过渐进式的方式引入现代化的包管理概念。这种设计既保证了向后兼容性，又为CodeNothing语言提供了更强大的模块化能力。

---
*本文档为设计探讨阶段，具体实现将根据实际需求进行调整。*
