# CodeNothing 更新日志
## [v0.2.5] - 2025-07-16

### 新增
- 添加了const关键字，支持常量定义和使用
- 增加了fs库，支持文件操作
  - 根命名空间函数
    - exists(path) - 判断路径是否存在
    - is_file(path) - 判断是否为文件
    - is_dir(path) - 判断是否为目录
  - 文件操作命名空间 (file::)
    - file::read(path) - 读取文件内容
    - file::read_bytes(path) - 读取文件内容为二进制（返回十六进制字符串）
    - file::write(path, content) - 写入文件
    - file::append(path, content) - 追加内容到文件
    - file::delete(path) - 删除文件
    - file::copy(src, dst) - 复制文件
    - file::rename(old_path, new_path) - 重命名文件
    - file::size(path) - 获取文件大小
  - 目录操作命名空间 (dir::)
    - dir::create(path) - 创建目录
    - dir::delete(path) - 删除目录
    - dir::delete_all(path) - 递归删除目录
    - dir::list(path) - 列出目录内容
    - dir::current() - 获取当前工作目录
  - 路径操作命名空间 (path::)
    - path::join(part1, part2, ...) - 连接路径
    - path::parent(path) - 获取父目录
    - path::filename(path) - 获取文件名
    - path::extension(path) - 获取文件扩展名
    - path::stem(path) - 获取不带扩展名的文件名
    - path::is_absolute(path) - 判断路径是否为绝对路径
- 增加了LICENSE文件。


### 修复
- 修复了常量定义和使用的问题
- 修复了常量在函数内部使用的问题



## [v0.2.4] - 2025-07-16
### 新增
- 添加了json库，支持JSON字符串解析和处理。
 - 解析JSON字符串 (json::parse)
 - 格式化JSON (json::format)
 - 创建JSON对象 (json::create_object)
 - 创建JSON数组 (json::create_array)
 - 从JSON中提取值 (json::get_value)
 - 检查JSON是否有效 (json::is_valid)
 - 合并JSON对象 (json::merge)

### 修复
#### 问题1：JSON字符串解析错误
##### 症状：尝试解析JSON字符串时出现"key must be a string at line 1 column 2"错误。
##### 解决方案：
- 添加了preprocess_json_string函数预处理JSON字符串，处理转义字符问题
- 添加了fix_json_string函数修复常见的JSON格式问题，如为没有引号的键添加引号
- 添加了从HTTP响应中提取JSON部分的功能
- 现在可以正确解析和处理JSON字符串，包括从HTTP响应中提取JSON数据。
#### 问题2：数值类型处理问题
##### 症状：数字字符串被当作普通字符串处理，而不是数值类型。
##### 解决方案：
- 在cn_create_object和cn_create_array函数中增加了数字类型检测
- 尝试将字符串解析为整数或浮点数，如果成功则创建数字类型的JSON值
- 现在数字字符串能够被正确识别为JSON数值类型。
#### 问题3：库命名空间处理问题
##### 症状：解析器中硬编码了特定库的命名空间（如http, std, json），导致无法自动识别新库。
##### 解决方案：
- 移除了表达式解析器中的硬编码命名空间判断
- 改进了解释器中的命名空间函数调用处理逻辑
- 完全移除了特殊处理std命名空间的硬编码逻辑，使用统一的命名空间处理方式
- 现在可以自动识别所有库的命名空间，无需修改源码
- 统一了命名空间函数调用的接口，提高了扩展性

## [v0.2.3] - 2025-07-16

### 新增
- 添加http库，支持HTTP请求和URL编码/解码
- 添加foreach循环语法，支持遍历数组、映射和字符串
  ```
  foreach (item in collection) {
      // 对集合中每个元素执行操作
  };
  ```
- 添加条件表达式（三元运算符）
  ```
  result = condition ? value_if_true : value_if_false;
  ```

### 修复
- 修复了关键性问题
  - 修复了库namespace无法正常使用的问题
  - 修复了命名空间函数调用的解析逻辑
  - 改进了库命名空间管理机制，从库配置文件(library.json)中自动加载命名空间信息
  - 简化了命名空间函数调用方式，无需添加库名前缀，支持多个库共享同一命名空间

#### 优化
- 减少了library.json的强依赖

## [v0.2.2] - 2025-07-15

### 修复
- 修复了`using ns`语法无法工作的问题
- 修复了库定义的namespace错误的问题

### 改进
- 改进了库namespace的注册
- 优化了库

## [v0.2.1] - 2025-07-15

### 错误报告改进
- 现在解释器能够一次性显示所有语法错误，而不是只显示第一个错误后停止
- 重新设计的错误报告系统，更清晰地显示错误信息
- 添加了错误恢复机制，即使存在多个错误也能够尽可能多地检查代码
- 添加了 `--cn-parser` 参数用于显示更详细的解析调试信息
- 错误消息格式统一化，提高可读性

### 健壮性提升
- 改进了错误处理流程，防止因单个错误导致整个解析过程终止
- 添加了跳过错误点的恢复策略，使解析器能够继续处理后续代码
- 优化了内部错误信息收集机制，减少内存占用

## [v0.2.0] - 2025-07-15

### 新增功能

#### 文件导入系统
- 添加了`using file`语法，支持从其他文件导入函数、变量和命名空间
- 实现了嵌套导入支持，允许一个文件通过另一个文件间接导入其他内容
- 添加了循环导入检测机制，防止因循环依赖导致的栈溢出

#### void返回类型
- 添加了`void`返回类型，用于不需要返回值的函数
- 规范化了函数返回行为，void类型函数必须使用不带值的`return;`语句

### 示例文件
- 添加了`examples/import_example.cn`演示基本文件导入功能
- 添加了`examples/nested_import.cn`演示嵌套导入功能
- 添加了`examples/circular_import_test.cn`测试循环导入检测
- 添加了多个工具函数文件，展示模块化代码组织

### 修复
- 修复了解析器在处理多文件项目时的内存泄漏问题
- 修复了命名空间函数调用时的作用域解析错误
- 改进了错误报告系统，现在能更准确地显示导入相关错误的位置和原因
- 修复了在Windows系统上文件路径处理的兼容性问题

### 性能优化
- 优化了文件缓存机制，避免重复加载已导入的文件
- 改进了符号表查找算法，提高了多文件项目的解释速度

## [v0.1.1] - 2025-07-15

### 修复
- 修复了 Linux 系统下无法导入 library 的问题

## [v0.1.0] - 2025-07-14

### 初始版本
- 基本语法支持
- 整数和字符串类型
- 基本算术运算
- 条件语句和循环
- 函数定义和调用
- 命名空间支持 