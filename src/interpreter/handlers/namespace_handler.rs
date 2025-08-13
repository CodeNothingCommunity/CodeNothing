use crate::ast::{Expression, NamespaceType};
use crate::interpreter::executor::ExecutionResult;
use crate::interpreter::interpreter_core::{Interpreter, debug_println};
use crate::interpreter::expression_evaluator::ExpressionEvaluator;
use crate::interpreter::library_loader::{load_library, call_library_function, convert_values_to_string_args};
use crate::interpreter::value::Value;

pub fn handle_import_namespace(interpreter: &mut Interpreter, ns_type: NamespaceType, path: Vec<String>) -> ExecutionResult {
    match ns_type {
        NamespaceType::Code => {
            // 导入代码命名空间
            let namespace_path = path.join("::");
            debug_println(&format!("导入代码命名空间: {}", namespace_path));

            let mut found = false;

            // 首先检查是否是库命名空间（如 std）
            if path.len() == 1 {
                let ns_name = &path[0];

                // 检查是否有对应的库命名空间
                for (lib_name, lib_functions) in &interpreter.imported_libraries {
                    debug_println(&format!("检查库 '{}' 中的命名空间函数", lib_name));

                    // 查找以 namespace:: 开头的函数
                    let ns_prefix = format!("{}::", ns_name);
                    for (func_full_path, _) in lib_functions.iter() {
                        if func_full_path.starts_with(&ns_prefix) {
                            // 获取函数名（路径的最后一部分）
                            let parts: Vec<&str> = func_full_path.split("::").collect();
                            if let Some(func_name) = parts.last() {
                                // 将函数添加到导入的命名空间列表
                                interpreter.imported_namespaces
                                    .entry(func_name.to_string())
                                    .or_insert_with(Vec::new)
                                    .push(func_full_path.clone());

                                found = true;
                                debug_println(&format!("  导入库函数: {}", func_full_path));
                            }
                        }
                    }
                }
            }

            // 然后检查代码命名空间中的函数
            for (full_path, _) in &interpreter.namespaced_functions {
                // 检查函数是否属于指定的命名空间
                if full_path.starts_with(&namespace_path) {
                    // 获取函数名（路径的最后一部分）
                    let parts: Vec<&str> = full_path.split("::").collect();
                    if let Some(func_name) = parts.last() {
                        // 将函数添加到导入的命名空间列表
                        interpreter.imported_namespaces
                            .entry(func_name.to_string())
                            .or_insert_with(Vec::new)
                            .push(full_path.clone());

                        found = true;
                        debug_println(&format!("  导入代码函数: {}", full_path));
                    }
                }
            }

            if !found {
                debug_println(&format!("警告: 命名空间 '{}' 中没有找到函数", namespace_path));
            }
        },
        NamespaceType::Library => {
            // 导入库命名空间
            if path.len() != 1 {
                panic!("库名称应该是单个标识符");
            }
            
            let lib_name = &path[0];
            debug_println(&format!("导入动态库: {}", lib_name));
            
            // 尝试加载库
            match load_library(lib_name) {
                Ok(functions) => {
                    // 库加载成功，将其添加到已导入库列表
                    interpreter.imported_libraries.insert(lib_name.clone(), functions);
                    debug_println(&format!("库 '{}' 加载成功", lib_name));
                    
                    // 将库中的所有函数添加到全局函数列表
                    if let Some(lib_functions) = interpreter.imported_libraries.get(lib_name) {
                        debug_println(&format!("库 '{}' 中的函数:", lib_name));
                        for (func_name, _) in lib_functions.iter() {
                            debug_println(&format!("  - {}", func_name));
                            
                            // 检查是否是命名空间函数（包含::）
                            if func_name.contains("::") {
                                let parts: Vec<&str> = func_name.split("::").collect();
                                if parts.len() >= 2 {
                                    let ns_name = parts[0];
                                    debug_println(&format!("  检测到命名空间: {} 在库 {}", ns_name, lib_name));
                                    // 记录命名空间与库的映射关系
                                    interpreter.library_namespaces.insert(ns_name.to_string(), lib_name.to_string());
                                }
                            }
                            
                            // 直接将库函数注册为全局函数，这样可以直接调用
                            interpreter.library_functions.insert(func_name.to_string(), (lib_name.clone(), func_name.to_string()));
                        }
                    }
                },
                Err(err) => {
                    // 尝试查找常见的库命名约定变体
                    let lib_variants = [
                        format!("{}", lib_name),        // 原始名称
                        format!("cn_{}", lib_name),     // cn_前缀
                        format!("library_{}", lib_name) // library_前缀
                    ];
                    
                    for variant in &lib_variants {
                        if variant == lib_name {
                            continue; // 跳过已尝试过的名称
                        }
                        
                        debug_println(&format!("尝试加载替代库名称: {}", variant));
                        match load_library(variant) {
                            Ok(functions) => {
                                // 库加载成功，将其添加到已导入库列表
                                interpreter.imported_libraries.insert(lib_name.clone(), functions.clone());
                                debug_println(&format!("库 '{}' 通过替代名称 '{}' 加载成功", lib_name, variant));
                                
                                // 将库中的所有函数添加到全局函数列表
                                debug_println(&format!("库 '{}' 中的函数:", lib_name));
                                for (func_name, _) in functions.iter() {
                                    debug_println(&format!("  - {}", func_name));
                                    
                                    // 检查是否是命名空间函数（包含::）
                                    if func_name.contains("::") {
                                        let parts: Vec<&str> = func_name.split("::").collect();
                                        if parts.len() >= 2 {
                                            let ns_name = parts[0];
                                            debug_println(&format!("  检测到命名空间: {} 在库 {}", ns_name, lib_name));
                                            // 记录命名空间与库的映射关系
                                            interpreter.library_namespaces.insert(ns_name.to_string(), lib_name.to_string());
                                        }
                                    }
                                    
                                    // 直接将库函数注册为全局函数，这样可以直接调用
                                    interpreter.library_functions.insert(func_name.to_string(), (lib_name.clone(), func_name.to_string()));
                                }
                                
                                // 成功找到替代库名称，不需要继续尝试
                                return ExecutionResult::None;
                            },
                            Err(_) => {
                                // 继续尝试下一个名称
                            }
                        }
                    }
                    
                    // 所有尝试都失败了，报告错误
                    panic!("无法加载库 '{}': {}。尝试了替代名称但均失败。", lib_name, err);
                }
            }
        }
    }
    
    ExecutionResult::None
}

pub fn handle_namespaced_function_call_statement(interpreter: &mut Interpreter, path: Vec<String>, args: Vec<Expression>) -> ExecutionResult {
    // 命名空间函数调用语句
    debug_println(&format!("命名空间函数调用: {:?}", path));

    // 检查路径长度
    if path.len() < 2 {
        panic!("无效的命名空间函数调用路径");
    }

    // 构建完整的函数路径
    let full_path = path.join("::");
    debug_println(&format!("尝试调用命名空间函数: {}", full_path));
    
    // 调试输出已注册的命名空间函数
    debug_println("已注册的命名空间函数:");
    for (path, _) in &interpreter.namespaced_functions {
        debug_println(&format!("  - {}", path));
    }
    
    // 计算参数值
    let mut arg_values = Vec::new();
    for arg in args {
        arg_values.push(interpreter.evaluate_expression(&arg));
    }
    
    // 检查是否是库函数调用
    let ns_name = &path[0];
    if let Some(lib_name) = interpreter.library_namespaces.get(ns_name) {
        debug_println(&format!("检测到库命名空间: {} -> 库: {}", ns_name, lib_name));
        
        // 构建库函数名 - 直接使用原始命名空间路径
        let func_name = full_path.clone();
        
        debug_println(&format!("尝试调用库函数: {}", func_name));
        
        // 将参数转换为字符串
        let string_args = convert_values_to_string_args(&arg_values);
        
        // 调用库函数
        match call_library_function(lib_name, &func_name, string_args) {
            Ok(result) => {
                debug_println(&format!("库函数调用成功: {} -> {}", func_name, result));
                return ExecutionResult::None;
            },
            Err(err) => {
                debug_println(&format!("调用库函数失败: {}", err));
                // 继续尝试其他方式
            }
        }
    }
    
    // 尝试作为普通命名空间函数调用
    debug_println(&format!("尝试作为普通命名空间函数调用: {}", full_path));
    
    // 直接查找完整路径函数
    if let Some(function) = interpreter.namespaced_functions.get(&full_path) {
        // 调用命名空间函数
        debug_println(&format!("找到并调用命名空间函数: {}", full_path));
        interpreter.call_function_impl(function, arg_values);
        return ExecutionResult::None;
    }

    // 新增：在所有已导入库的函数表里查找完整路径（如std::println、path::join等）
    for (lib_name, lib_functions) in &interpreter.imported_libraries {
        if let Some(func) = lib_functions.get(&full_path) {
            debug_println(&format!("在库 '{}' 中找到命名空间函数 '{}', 调用之", lib_name, full_path));
            let string_args = convert_values_to_string_args(&arg_values);
            let _ = func(string_args); // 忽略返回值（如有需要可处理）
            return ExecutionResult::None;
        }
    }
    
    // 如果是嵌套命名空间函数调用，需要特殊处理
    if path.len() > 2 {
        // 构建嵌套命名空间的完整路径
        let nested_path = path.join("::");
        debug_println(&format!("尝试调用嵌套命名空间函数: {}", nested_path));
        
        // 查找嵌套命名空间函数
        if let Some(function) = interpreter.namespaced_functions.get(&nested_path) {
            debug_println(&format!("找到并调用嵌套命名空间函数: {}", nested_path));
            interpreter.call_function_impl(function, arg_values);
                return ExecutionResult::None;
        }
    }
    
    panic!("未找到命名空间函数: {}", full_path);
} 