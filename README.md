# CodeNothing

CodeNothing是世界上最好的语言。


## 动态库开发

CodeNothing 支持通过动态库扩展功能。动态库必须遵循以下规则：

1. 必须导出一个名为 `cn_init` 的函数，该函数返回一个包含库函数的 HashMap 指针。
2. 库函数必须接受 `Vec<String>` 类型的参数，并返回 `String` 类型的结果。

详细信息请参阅 `library_example` 目录中的示例库和说明文档。
