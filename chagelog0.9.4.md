# CodeNothing v0.9.4 及其子版本 更新日志

## [v0.9.4 Pre1] - 2025-08-15

### 核心变更

#### VM库支持系统
- 移除硬编码的函数索引限制，实现动态函数查找机制
- 添加对`using lib <>`语句的完整支持
- VM与解释器现在使用相同的库加载接口
- VM功能覆盖范围显著扩展

#### 字节码指令集扩展
- 新增指令：
  - `CallLibrary(String, String, u8)` - 库函数调用
  - `Mul` - 乘法运算
  - `Div` - 除法运算
  - `Greater`, `GreaterEqual`, `Equal`, `NotEqual`, `Less` - 比较运算
- 数据结构变更：
  - `CompiledProgram`增加`imported_libraries`字段
  - `Compiler`增加库管理相关字段

#### 编译器模块变更 (`src/vm/compiler.rs`)
- 添加库导入语句的解析和处理逻辑
- 实现动态函数索引分配机制
- 支持库函数调用的字节码生成
- 改进库加载失败时的错误信息

#### 执行引擎模块变更 (`src/vm/vm.rs`)
- 实现`CallLibrary`指令的执行逻辑
- 添加基于函数名的动态函数查找
- 实现算术运算方法：
  - `mul_values()` - 乘法运算
  - `div_values()` - 除法运算（含除零检查）
  - 各类比较运算方法
- 修复递归函数调用的栈管理问题
