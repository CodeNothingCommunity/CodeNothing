# CodeNothing GitHub Actions

本目录包含用于自动构建和发布CodeNothing解释器和库文件的GitHub Actions工作流。

## 工作流说明

项目包含两个主要工作流：

1. **构建解释器** (`build_interpreter.yml`)
   - 自动构建CodeNothing解释器本体
   - 支持Windows、Linux和macOS平台
   - 在推送到main分支时自动触发（除非更改仅涉及库文件或文档）

2. **构建库文件** (`build_libraries.yml`)
   - 构建CodeNothing的所有库文件
   - 支持Windows、Linux和macOS平台
   - 仅通过手动触发运行
   - 可以选择构建特定的库或所有库

## 如何使用

### 自动构建解释器

每次推送到main分支时，解释器构建工作流将自动运行。如果只想构建解释器，不需要执行任何额外操作。

### 手动构建库文件

要手动构建库文件：

1. 在GitHub仓库页面上，点击"Actions"标签
2. 在左侧列表中找到"构建CodeNothing库文件"工作流
3. 点击"Run workflow"按钮
4. 在"要构建的库列表"输入框中：
   - 输入`all`构建所有库
   - 或输入特定库名称（用逗号分隔），如`io,example`
5. 点击"Run workflow"开始构建

## 添加新库

要添加新的库并确保其可以通过GitHub Actions构建：

1. 创建新的库目录，命名为`library_<库名>`
2. 确保库目录包含有效的Cargo.toml文件
3. 在库目录中添加`library.json`配置文件，格式如下：

```json
{
  "name": "库名",
  "version": "版本号",
  "description": "库描述",
  "output_name": "输出文件名（不含扩展名）",
  "author": "作者",
  "requires_namespace": true|false,
  "namespaces": ["命名空间1", "命名空间2"]
}
```

4. 库结构应遵循CodeNothing库的标准格式，包含必要的`cn_init`函数

添加新库后，它将自动被"构建库文件"工作流检测到，无需修改工作流文件。

## 发布

当在GitHub上创建新标签（tag）时，两个工作流都会自动将构建产物上传到对应的GitHub Release。

## 注意事项

- 库构建工作流仅在手动触发时运行，以避免不必要的构建
- 解释器构建工作流会忽略对库文件和文档的更改
- 两个工作流都支持所有主要操作系统 