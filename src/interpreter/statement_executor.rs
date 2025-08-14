use crate::ast::{Statement, Expression, Type, NamespaceType, Function, SwitchCase, CasePattern, SwitchType, MatchArm};
use super::value::Value;
use super::executor::{Executor, ExecutionResult, update_variable_value, handle_increment, handle_decrement};
use super::library_loader::{load_library, call_library_function, convert_values_to_string_args};
use super::interpreter_core::{Interpreter, debug_println};
use super::expression_evaluator::ExpressionEvaluator;
use super::pattern_matcher::PatternMatcher;
use super::handlers;

pub trait StatementExecutor {
    fn execute_statement(&mut self, statement: Statement) -> ExecutionResult;
    fn execute_function(&mut self, function: &Function) -> Value;
    fn update_variable(&mut self, name: &str, value: Value) -> Result<(), String>;
    fn call_function(&mut self, function_name: &str, args: Vec<Value>) -> Value;
}

impl<'a> StatementExecutor for Interpreter<'a> {
    fn execute_statement(&mut self, statement: Statement) -> ExecutionResult {
        // 检查超时和操作次数限制
        if let Err(timeout_msg) = self.check_timeout() {
            eprintln!("⚠️ 执行超时: {}", timeout_msg);
            return ExecutionResult::Error(timeout_msg);
        }

        match statement {
            Statement::Return(expr) => {
                // 返回语句，计算表达式值并返回
                let value = if let Some(expr) = expr {
                    self.evaluate_expression(&expr)
                } else {
                    Value::None // void返回
                };
                ExecutionResult::Return(value)
            },
            Statement::VariableDeclaration(name, declared_type, expr) => {
                let mut value = self.evaluate_expression(&expr);
                
                // 如果声明的类型是 Auto，则不进行类型检查（弱类型）
                if !matches!(declared_type, Type::Auto) {
                    // 进行强类型检查，包括自动类型转换
                    let (type_matches, converted_value) = match (&declared_type, &value) {
                        (Type::Int, Value::Int(_)) => (true, value.clone()),
                        (Type::Float, Value::Float(_)) => (true, value.clone()),
                        (Type::Bool, Value::Bool(_)) => (true, value.clone()),
                        (Type::String, Value::String(_)) => (true, value.clone()),
                        (Type::Long, Value::Long(_)) => (true, value.clone()),
                        (Type::Void, Value::None) => (true, value.clone()),
                        // 自动类型转换：int -> long
                        (Type::Long, Value::Int(i)) => (true, Value::Long(*i as i64)),
                        // 自动类型转换：int -> float
                        (Type::Float, Value::Int(i)) => (true, Value::Float(*i as f64)),
                        (Type::Class(class_name), Value::Object(obj)) => (class_name == &obj.class_name, value.clone()),
                        (Type::Enum(enum_name), Value::EnumValue(enum_val)) => (enum_name == &enum_val.enum_name, value.clone()),
                        // 智能类型匹配：如果声明为Class类型，但值是EnumValue，检查名称是否匹配
                        (Type::Class(type_name), Value::EnumValue(enum_val)) => {
                            // 检查是否是已知的枚举类型
                            let matches = if self.enums.contains_key(type_name) {
                                type_name == &enum_val.enum_name
                            } else {
                                false
                            };
                            (matches, value.clone())
                        },
                        // 指针类型匹配
                        (Type::Pointer(expected_target), Value::Pointer(ptr)) => {
                            // 检查指针目标类型是否匹配
                            let matches = self.pointer_target_type_matches(expected_target, &ptr.target_type);
                            (matches, value.clone())
                        },
                        (Type::OptionalPointer(expected_target), Value::Pointer(ptr)) => {
                            let matches = self.pointer_target_type_matches(expected_target, &ptr.target_type);
                            (matches, value.clone())
                        },
                        (Type::OptionalPointer(_), Value::None) => (true, value.clone()), // 可选指针可以为null
                        (Type::FunctionPointer(expected_params, expected_return), Value::FunctionPointer(func_ptr)) => {
                            // 检查函数指针的参数和返回类型是否匹配
                            let matches = if func_ptr.param_types.len() != expected_params.len() {
                                false
                            } else {
                                // 检查参数类型
                                let params_match = func_ptr.param_types.iter()
                                    .zip(expected_params.iter())
                                    .all(|(actual, expected)| actual == expected);

                                // 检查返回类型
                                let return_match = &*func_ptr.return_type == expected_return.as_ref();

                                params_match && return_match
                            };
                            (matches, value.clone())
                        },
                        (Type::FunctionPointer(expected_params, expected_return), Value::LambdaFunctionPointer(lambda_ptr)) => {
                            // 检查Lambda函数指针的参数和返回类型是否匹配
                            let matches = if lambda_ptr.param_types.len() != expected_params.len() {
                                false
                            } else {
                                // 对于Lambda，允许Auto类型匹配任何类型
                                let params_match = lambda_ptr.param_types.iter()
                                    .zip(expected_params.iter())
                                    .all(|(actual, expected)| {
                                        actual == expected ||
                                        *actual == crate::ast::Type::Auto ||
                                        *expected == crate::ast::Type::Auto
                                    });

                                // 检查返回类型（允许Auto匹配）
                                let return_match = &*lambda_ptr.return_type == expected_return.as_ref() ||
                                                   *lambda_ptr.return_type == crate::ast::Type::Auto ||
                                                   **expected_return == crate::ast::Type::Auto;

                                params_match && return_match
                            };
                            (matches, value.clone())
                        },
                        (Type::FunctionPointer(_, _), Value::None) => (true, value.clone()), // 未初始化的函数指针
                        (Type::Array(expected_element_type), Value::Array(arr)) => {
                            // 检查数组元素类型是否匹配
                            let matches = if arr.is_empty() {
                                true // 空数组可以匹配任何数组类型
                            } else {
                                // 检查所有元素是否匹配期望的元素类型
                                arr.iter().all(|element| {
                                    self.value_matches_type(element, expected_element_type)
                                })
                            };
                            (matches, value.clone())
                        },
                        // 泛型类类型匹配
                        (Type::GenericClass(expected_class_name, _expected_type_args), Value::Object(obj)) => {
                            // 检查类名是否匹配
                            let matches = obj.class_name == *expected_class_name;
                            (matches, value.clone())
                        },
                        _ => (false, value.clone())
                    };

                    if !type_matches {
                        panic!("变量 '{}' 的类型不匹配：期望 {:?}，但得到 {:?}", name, declared_type, converted_value);
                    }

                    // 使用转换后的值
                    value = converted_value;
                }

                // 存储变量值和类型信息
                self.local_env.insert(name.clone(), value);
                // 存储变量的声明类型用于后续赋值检查
                self.variable_types.insert(name, declared_type);
                ExecutionResult::None
            },
            Statement::ConstantDeclaration(name, typ, expr) => {
                // 计算常量值
                let value = self.evaluate_expression(&expr);
                
                // 检查类型是否匹配
                let type_matches = match (&typ, &value) {
                    (Type::Int, Value::Int(_)) => true,
                    (Type::Float, Value::Float(_)) => true,
                    (Type::Bool, Value::Bool(_)) => true,
                    (Type::String, Value::String(_)) => true,
                    (Type::Long, Value::Long(_)) => true,
                    _ => false
                };
                
                if !type_matches {
                    panic!("常量 '{}' 的类型不匹配", name);
                }
                
                // 检查是否已存在同名常量
                if self.constants.contains_key(&name) {
                    panic!("常量 '{}' 已定义", name);
                }
                
                // 存储常量值
                self.constants.insert(name, value);
                
                ExecutionResult::None
            },
            Statement::VariableAssignment(name, expr) => {
                // 检查是否尝试修改常量
                if self.constants.contains_key(&name) {
                    panic!("无法修改常量 '{}'", name);
                }
                
                let mut value = self.evaluate_expression(&expr);

                // 检查变量是否存在
                let variable_exists = self.local_env.contains_key(&name) || self.global_env.contains_key(&name);
                if !variable_exists {
                    panic!("未定义的变量: {}", name);
                }

                // 检查类型约束（如果变量有声明类型且不是 Auto）
                if let Some(declared_type) = self.variable_types.get(&name) {
                    if !matches!(declared_type, Type::Auto) {
                        // 进行强类型检查，包括自动类型转换
                        let (type_matches, converted_value) = match (declared_type, &value) {
                            (Type::Int, Value::Int(_)) => (true, value.clone()),
                            (Type::Float, Value::Float(_)) => (true, value.clone()),
                            (Type::Bool, Value::Bool(_)) => (true, value.clone()),
                            (Type::String, Value::String(_)) => (true, value.clone()),
                            (Type::Long, Value::Long(_)) => (true, value.clone()),
                            (Type::Void, Value::None) => (true, value.clone()),
                            // 自动类型转换：int -> long
                            (Type::Long, Value::Int(i)) => (true, Value::Long(*i as i64)),
                            // 自动类型转换：int -> float
                            (Type::Float, Value::Int(i)) => (true, Value::Float(*i as f64)),
                            (Type::Class(class_name), Value::Object(obj)) => (class_name == &obj.class_name, value.clone()),
                            (Type::Enum(enum_name), Value::EnumValue(enum_val)) => (enum_name == &enum_val.enum_name, value.clone()),
                            // 智能类型匹配：如果声明为Class类型，但值是EnumValue，检查名称是否匹配
                            (Type::Class(type_name), Value::EnumValue(enum_val)) => {
                                // 检查是否是已知的枚举类型
                                let matches = if self.enums.contains_key(type_name) {
                                    type_name == &enum_val.enum_name
                                } else {
                                    false
                                };
                                (matches, value.clone())
                            },
                            // 指针类型匹配（第二个检查点）
                            (Type::Pointer(expected_target), Value::Pointer(ptr)) => {
                                let matches = self.pointer_target_type_matches(expected_target, &ptr.target_type);
                                (matches, value.clone())
                            },
                            (Type::OptionalPointer(expected_target), Value::Pointer(ptr)) => {
                                let matches = self.pointer_target_type_matches(expected_target, &ptr.target_type);
                                (matches, value.clone())
                            },
                            (Type::OptionalPointer(_), Value::None) => (true, value.clone()),
                            // 函数指针类型匹配（第二个检查点）
                            (Type::FunctionPointer(expected_params, expected_return), Value::FunctionPointer(func_ptr)) => {
                                // 检查函数指针的参数和返回类型是否匹配
                                let matches = if func_ptr.param_types.len() != expected_params.len() {
                                    false
                                } else {
                                    // 检查参数类型
                                    let params_match = func_ptr.param_types.iter()
                                        .zip(expected_params.iter())
                                        .all(|(actual, expected)| actual == expected);

                                    // 检查返回类型
                                    let return_match = &*func_ptr.return_type == expected_return.as_ref();

                                    params_match && return_match
                                };
                                (matches, value.clone())
                            },
                            (Type::FunctionPointer(expected_params, expected_return), Value::LambdaFunctionPointer(lambda_ptr)) => {
                                // 检查Lambda函数指针的参数和返回类型是否匹配
                                let matches = if lambda_ptr.param_types.len() != expected_params.len() {
                                    false
                                } else {
                                    // 对于Lambda，允许Auto类型匹配任何类型
                                    let params_match = lambda_ptr.param_types.iter()
                                        .zip(expected_params.iter())
                                        .all(|(actual, expected)| {
                                            actual == expected ||
                                            *actual == crate::ast::Type::Auto ||
                                            *expected == crate::ast::Type::Auto
                                        });

                                    // 检查返回类型（允许Auto匹配）
                                    let return_match = &*lambda_ptr.return_type == expected_return.as_ref() ||
                                                       *lambda_ptr.return_type == crate::ast::Type::Auto ||
                                                       **expected_return == crate::ast::Type::Auto;

                                    params_match && return_match
                                };
                                (matches, value.clone())
                            },
                            (Type::Array(expected_element_type), Value::Array(arr)) => {
                                // 检查数组元素类型是否匹配
                                let matches = if arr.is_empty() {
                                    true // 空数组可以匹配任何数组类型
                                } else {
                                    // 检查所有元素是否匹配期望的元素类型
                                    arr.iter().all(|element| {
                                        self.value_matches_type(element, expected_element_type)
                                    })
                                };
                                (matches, value.clone())
                            },
                            // 泛型类类型匹配（赋值检查）
                            (Type::GenericClass(expected_class_name, _expected_type_args), Value::Object(obj)) => {
                                // 检查类名是否匹配
                                let matches = obj.class_name == *expected_class_name;
                                (matches, value.clone())
                            },
                            _ => (false, value.clone())
                        };

                        if !type_matches {
                            panic!("变量 '{}' 类型不匹配：期望 {:?}，但尝试赋值 {:?}", name, declared_type, converted_value);
                        }

                        // 使用转换后的值
                        value = converted_value;
                    }
                }

                // 更新变量值
                if self.local_env.contains_key(&name) {
                    self.local_env.insert(name, value);
                } else {
                    self.global_env.insert(name, value);
                }
                ExecutionResult::None
            },
            Statement::Increment(name) => {
                // 使用辅助函数处理后置自增操作
                if let Err(err) = handle_increment(&mut self.local_env, &mut self.global_env, &name) {
                    panic!("{}", err);
                }
                ExecutionResult::None
            },
            Statement::Decrement(name) => {
                // 使用辅助函数处理后置自减操作
                if let Err(err) = handle_decrement(&mut self.local_env, &mut self.global_env, &name) {
                    panic!("{}", err);
                }
                ExecutionResult::None
            },
            Statement::PreIncrement(name) => {
                // 使用辅助函数处理前置自增操作
                if let Err(err) = handle_increment(&mut self.local_env, &mut self.global_env, &name) {
                    panic!("{}", err);
                }
                ExecutionResult::None
            },
            Statement::PreDecrement(name) => {
                // 使用辅助函数处理前置自减操作
                if let Err(err) = handle_decrement(&mut self.local_env, &mut self.global_env, &name) {
                    panic!("{}", err);
                }
                ExecutionResult::None
            },
            Statement::CompoundAssignment(name, op, expr) => {
                handlers::assignment_handler::handle_compound_assignment(self, name, op, expr)
            },
            Statement::ImportNamespace(ns_type, path) => {
                match ns_type {
                    NamespaceType::Code => {
                        // 作用域级别导入：将命名空间下所有函数名映射到完整路径
                        let ns_path = path.join("::");
                        let mut import_map = self.namespace_import_stack.last_mut().unwrap();

                        // 检查代码命名空间中的函数
                        for (full_path, _) in &self.namespaced_functions {
                            if full_path.starts_with(&ns_path) {
                                // 获取函数名
                                let parts: Vec<&str> = full_path.split("::").collect();
                                if let Some(func_name) = parts.last() {
                                    import_map.entry(func_name.to_string()).or_insert_with(Vec::new).push(full_path.clone());
                                }
                            }
                        }

                        // 检查库命名空间中的函数（新增）
                        if path.len() == 1 {
                            let ns_name = &path[0];
                            for (lib_name, lib_functions) in &self.imported_libraries {
                                let ns_prefix = format!("{}::", ns_name);
                                for (func_full_path, _) in lib_functions.iter() {
                                    if func_full_path.starts_with(&ns_prefix) {
                                        let parts: Vec<&str> = func_full_path.split("::").collect();
                                        if let Some(func_name) = parts.last() {
                                            import_map.entry(func_name.to_string()).or_insert_with(Vec::new).push(func_full_path.clone());
                                            debug_println(&format!("作用域导入库函数: {} -> {}", func_name, func_full_path));
                                        }
                                    }
                                }
                            }
                        }

                        ExecutionResult::None
                    },
                    _ => handlers::namespace_handler::handle_import_namespace(self, ns_type, path)
                }
            },
            Statement::FileImport(_file_path) => {
                // 文件导入已在预处理阶段处理，这里不需要额外处理
                ExecutionResult::None
            },
            Statement::FunctionCallStatement(expr) => {
                // 函数调用语句，计算表达式值但不返回
                self.evaluate_expression(&expr);
                ExecutionResult::None
            },
            Statement::NamespacedFunctionCallStatement(path, args) => {
                handlers::namespace_handler::handle_namespaced_function_call_statement(self, path, args)
            },
            Statement::LibraryFunctionCallStatement(lib_name, func_name, args) => {
                handlers::library_handler::handle_library_function_call_statement(self, lib_name, func_name, args)
            },
            Statement::IfElse(condition, if_block, else_blocks) => {
                handlers::control_flow::handle_if_else(self, condition, if_block, else_blocks)
            },
            Statement::ForLoop(variable_name, range_start, range_end, loop_body) => {
                handlers::control_flow::handle_for_loop(self, variable_name, range_start, range_end, loop_body)
            },
            Statement::ForEachLoop(variable_name, collection_expr, loop_body) => {
                handlers::control_flow::handle_foreach_loop(self, variable_name, collection_expr, loop_body)
            },
            Statement::WhileLoop(condition, loop_body) => {
                handlers::control_flow::handle_while_loop(self, condition, loop_body)
            },
            Statement::Break => {
                // 返回Break结果，由循环处理
                ExecutionResult::Break
            },
            Statement::Continue => {
                // 返回Continue结果，由循环处理
                ExecutionResult::Continue
            },
            Statement::TryCatch(try_block, catch_blocks, finally_block) => {
                handlers::exception_handler::handle_try_catch(self, try_block, catch_blocks, finally_block)
            },
            Statement::Throw(exception_expr) => {
                // 计算异常表达式并抛出
                let exception_value = self.evaluate_expression(&exception_expr);
                ExecutionResult::Throw(exception_value)
            },
            Statement::Switch(expr, cases, default_block, switch_type) => {
                // Switch 语句执行
                self.execute_switch_statement(expr, cases, default_block, switch_type)
            },
            Statement::Match(match_expr, arms) => {
                // Match 语句执行
                self.execute_match_statement(match_expr, arms)
            },
            // OOP相关语句的临时实现
            Statement::ClassDeclaration(_) => {
                ExecutionResult::Continue // 临时跳过，后续实现
            },
            Statement::FieldAssignment(obj_expr, field_name, value_expr) => {
                // 处理字段赋值
                self.handle_field_assignment(obj_expr, field_name, value_expr)
            },
            Statement::InterfaceDeclaration(_interface) => {
                // 接口声明在解释器初始化时已经处理，这里不需要额外操作
                ExecutionResult::Continue
            },
            Statement::EnumDeclaration(_enum_def) => {
                // 枚举声明在解释器初始化时已经处理，这里不需要额外操作
                ExecutionResult::Continue
            },
        }
    }
    
    fn execute_function(&mut self, function: &Function) -> Value {
        // 进入新作用域，push一层导入表
        self.namespace_import_stack.push(self.namespace_import_stack.last().cloned().unwrap_or_default());
        // 执行函数体
        for statement in &function.body {
            match self.execute_statement_direct(statement.clone()) {
                ExecutionResult::Return(value) => {
                    self.namespace_import_stack.pop();
                    return value
                },
                ExecutionResult::None => {},
                ExecutionResult::Break => {
                    self.namespace_import_stack.pop();
                    panic!("break语句只能在循环内部使用")
                },
                ExecutionResult::Continue => {
                    self.namespace_import_stack.pop();
                    panic!("continue语句只能在循环内部使用")
                },
                ExecutionResult::Throw(value) => {
                    self.namespace_import_stack.pop();
                    panic!("未捕获的异常: {:?}", value);
                },
                ExecutionResult::Error(msg) => {
                    self.namespace_import_stack.pop();
                    eprintln!("执行错误: {}", msg);
                    return Value::None;
                }
            }
        }
        // 如果函数没有明确的返回语句，则返回空值
        self.namespace_import_stack.pop();
        Value::None
    }
    
    fn update_variable(&mut self, name: &str, value: Value) -> Result<(), String> {
        update_variable_value(&mut self.local_env, &mut self.global_env, name, value)
    }
    
    fn call_function(&mut self, function_name: &str, args: Vec<Value>) -> Value {
        // 优先查找当前作用域导入的命名空间
        if let Some(import_map) = self.namespace_import_stack.last() {
            if let Some(paths) = import_map.get(function_name) {
                if paths.len() == 1 {
                    let full_path = &paths[0];

                    // 首先检查是否是代码命名空间函数
                    if let Some(function) = self.namespaced_functions.get(full_path) {
                        return self.call_function_impl(function, args);
                    }

                    // 然后检查是否是库函数
                    for (lib_name, lib_functions) in &self.imported_libraries {
                        if let Some(func) = lib_functions.get(full_path) {
                            debug_println(&format!("调用作用域导入的库函数: {} 来自库 '{}'", full_path, lib_name));
                            let string_args = convert_values_to_string_args(&args);
                            let result = func(string_args);
                            // 尝试将结果转换为适当的值类型
                            if let Ok(int_val) = result.parse::<i32>() {
                                return Value::Int(int_val);
                            } else if let Ok(float_val) = result.parse::<f64>() {
                                return Value::Float(float_val);
                            } else if result == "true" {
                                return Value::Bool(true);
                            } else if result == "false" {
                                return Value::Bool(false);
                            } else {
                                return Value::String(result);
                            }
                        }
                    }
                } else if paths.len() > 1 {
                    panic!("函数名 '{}' 有多个匹配: {:?}", function_name, paths);
                }
            }
        }
        // 先检查是否是导入的命名空间函数
        if let Some(paths) = self.imported_namespaces.get(function_name) {
            if paths.len() == 1 {
                // 只有一个匹配的函数，直接调用
                let full_path = &paths[0];
                if let Some(function) = self.namespaced_functions.get(full_path) {
                    return self.call_function_impl(function, args);
                }
            }
        }
        
        // 如果不是导入的函数，再检查全局函数
        if let Some(function) = self.functions.get(function_name) {
            self.call_function_impl(function, args)
        } else {
            panic!("未定义的函数: {}", function_name);
        }
    }
}

impl<'a> Interpreter<'a> {
    fn execute_switch_statement(&mut self, expr: Expression, cases: Vec<SwitchCase>, default_block: Option<Vec<Statement>>, switch_type: SwitchType) -> ExecutionResult {
        // 计算 switch 表达式的值
        let switch_value = self.evaluate_expression(&expr);
        // debug_println(&format!("Switch value: {:?}", switch_value));
        
        let mut matched = false;
        let mut fall_through = false;
        
        // 遍历所有 case
        for case in &cases {
            // 如果已经匹配过且没有 break，则继续执行（fall-through）
            if matched || fall_through {
                // 执行当前 case 的语句或表达式
                if let Some(expr) = &case.expression {
                    // 表达式形式，计算并返回值
                    let result_value = self.evaluate_expression(expr);
                    return ExecutionResult::Return(result_value);
                } else {
                    // 语句形式
                    for stmt in &case.statements {
                        match self.execute_statement_direct(stmt.clone()) {
                            ExecutionResult::None => {},
                            ExecutionResult::Return(value) => return ExecutionResult::Return(value),
                            ExecutionResult::Break => {
                                // break 跳出整个 switch
                                return ExecutionResult::None;
                            },
                            ExecutionResult::Continue => return ExecutionResult::Continue,
                            ExecutionResult::Throw(value) => return ExecutionResult::Throw(value),
                            ExecutionResult::Error(msg) => return ExecutionResult::Error(msg),
                        }
                    }
                }
                
                // 如果当前 case 有 break，则停止执行
                if case.has_break {
                    return ExecutionResult::None;
                }
                
                // 否则继续 fall-through
                fall_through = true;
                continue;
            }
            
            // 检查模式是否匹配
            let pattern_matches = self.pattern_matches(&case.pattern, &switch_value);
            
            if pattern_matches {
                matched = true;
                
                // 执行匹配的 case 语句或表达式
                if let Some(expr) = &case.expression {
                    // 表达式形式，计算并返回值
                    let result_value = self.evaluate_expression(expr);
                    return ExecutionResult::Return(result_value);
                } else {
                    // 语句形式
                    for stmt in &case.statements {
                        match self.execute_statement_direct(stmt.clone()) {
                            ExecutionResult::None => {},
                            ExecutionResult::Return(value) => return ExecutionResult::Return(value),
                            ExecutionResult::Break => {
                                // break 跳出整个 switch
                                return ExecutionResult::None;
                            },
                            ExecutionResult::Continue => return ExecutionResult::Continue,
                            ExecutionResult::Throw(value) => return ExecutionResult::Throw(value),
                            ExecutionResult::Error(msg) => return ExecutionResult::Error(msg),
                        }
                    }
                }
                
                // 如果当前 case 有 break，则停止执行
                if case.has_break {
                    return ExecutionResult::None;
                }
                
                // 否则继续 fall-through 到下一个 case
                fall_through = true;
            }
        }
        
        // 如果没有匹配的 case，执行 default 块
        if !matched {
            if let Some(default_statements) = default_block {
                for stmt in default_statements {
                    match self.execute_statement_direct(stmt) {
                        ExecutionResult::None => {},
                        ExecutionResult::Return(value) => return ExecutionResult::Return(value),
                        ExecutionResult::Break => {
                            // break 跳出整个 switch
                            return ExecutionResult::None;
                        },
                        ExecutionResult::Continue => return ExecutionResult::Continue,
                        ExecutionResult::Throw(value) => return ExecutionResult::Throw(value),
                        ExecutionResult::Error(msg) => return ExecutionResult::Error(msg),
                    }
                }
            }
        }
        
        ExecutionResult::None
    }
    
    fn values_equal(&self, val1: &Value, val2: &Value) -> bool {
        match (val1, val2) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Long(a), Value::Long(b)) => a == b,
            // 类型不同则不相等
            _ => false,
        }
    }

    fn pattern_matches(&mut self, pattern: &CasePattern, switch_value: &Value) -> bool {
        match pattern {
            CasePattern::Value(expr) => {
                let case_value = self.evaluate_expression(expr);
                self.values_equal(switch_value, &case_value)
            },
            CasePattern::Range(start_expr, end_expr) => {
                let start_value = self.evaluate_expression(start_expr);
                let end_value = self.evaluate_expression(end_expr);
                self.value_in_range(switch_value, &start_value, &end_value)
            },
            CasePattern::Guard(var_name, condition_expr) => {
                // 将switch值绑定到变量，然后检查guard条件
                let old_value = self.local_env.get(var_name).cloned();
                self.local_env.insert(var_name.clone(), switch_value.clone());
                
                let condition_result = self.evaluate_expression(condition_expr);
                
                // 恢复原来的变量值
                if let Some(old_val) = old_value {
                    self.local_env.insert(var_name.clone(), old_val);
                } else {
                    self.local_env.remove(var_name);
                }
                
                // 检查条件是否为真
                matches!(condition_result, Value::Bool(true))
            },
            CasePattern::Destructure(_) => {
                // 解构匹配暂时不实现，返回false
                false
            }
        }
    }

    fn value_in_range(&self, value: &Value, start: &Value, end: &Value) -> bool {
        match (value, start, end) {
            (Value::Int(v), Value::Int(s), Value::Int(e)) => v >= s && v <= e,
            (Value::Float(v), Value::Float(s), Value::Float(e)) => v >= s && v <= e,
            (Value::Long(v), Value::Long(s), Value::Long(e)) => v >= s && v <= e,
            // 混合类型比较
            (Value::Int(v), Value::Float(s), Value::Float(e)) => (*v as f64) >= *s && (*v as f64) <= *e,
            (Value::Float(v), Value::Int(s), Value::Int(e)) => *v >= (*s as f64) && *v <= (*e as f64),
            _ => false,
        }
    }

    // 检查指针目标类型是否匹配
    fn pointer_target_type_matches(&self, expected: &crate::ast::Type, actual: &crate::interpreter::value::PointerType) -> bool {
        use crate::interpreter::value::PointerType;

        match (expected, actual) {
            (crate::ast::Type::Int, PointerType::Int) => true,
            (crate::ast::Type::Float, PointerType::Float) => true,
            (crate::ast::Type::Bool, PointerType::Bool) => true,
            (crate::ast::Type::String, PointerType::String) => true,
            (crate::ast::Type::Long, PointerType::Long) => true,
            (crate::ast::Type::Class(expected_name), PointerType::Enum(actual_name)) => expected_name == actual_name,
            (crate::ast::Type::Class(expected_name), PointerType::Class(actual_name)) => expected_name == actual_name,
            (crate::ast::Type::Pointer(expected_inner), PointerType::Pointer(actual_inner)) => {
                self.pointer_target_type_matches(expected_inner, actual_inner)
            },
            _ => false,
        }
    }

    // 辅助方法：检查值是否匹配指定类型
    fn value_matches_type(&self, value: &Value, expected_type: &Type) -> bool {
        match (expected_type, value) {
            (Type::Int, Value::Int(_)) => true,
            (Type::Float, Value::Float(_)) => true,
            (Type::Bool, Value::Bool(_)) => true,
            (Type::String, Value::String(_)) => true,
            (Type::Long, Value::Long(_)) => true,
            (Type::Array(expected_element_type), Value::Array(arr)) => {
                if arr.is_empty() {
                    true
                } else {
                    arr.iter().all(|element| self.value_matches_type(element, expected_element_type))
                }
            },
            (Type::FunctionPointer(expected_params, expected_return), Value::FunctionPointer(func_ptr)) => {
                if func_ptr.param_types.len() != expected_params.len() {
                    false
                } else {
                    let params_match = func_ptr.param_types.iter()
                        .zip(expected_params.iter())
                        .all(|(actual, expected)| actual == expected);
                    let return_match = &*func_ptr.return_type == expected_return.as_ref();
                    params_match && return_match
                }
            },
            (Type::FunctionPointer(expected_params, expected_return), Value::LambdaFunctionPointer(lambda_ptr)) => {
                if lambda_ptr.param_types.len() != expected_params.len() {
                    false
                } else {
                    let params_match = lambda_ptr.param_types.iter()
                        .zip(expected_params.iter())
                        .all(|(actual, expected)| {
                            actual == expected ||
                            *actual == crate::ast::Type::Auto ||
                            *expected == crate::ast::Type::Auto
                        });
                    let return_match = &*lambda_ptr.return_type == expected_return.as_ref() ||
                                       *lambda_ptr.return_type == crate::ast::Type::Auto ||
                                       **expected_return == crate::ast::Type::Auto;
                    params_match && return_match
                }
            },
            _ => false
        }
    }

    /// 处理字段赋值
    fn handle_field_assignment(&mut self, obj_expr: &Box<Expression>, field_name: &String, value_expr: &Expression) -> ExecutionResult {
        // 计算新值
        let new_value = self.evaluate_expression(value_expr);

        // 处理对象表达式
        match obj_expr.as_ref() {
            Expression::Variable(var_name) => {
                // 获取对象
                if let Value::Object(mut obj) = self.get_variable_fast(var_name) {
                    // 更新字段
                    obj.fields.insert(field_name.clone(), new_value);

                    // 更新变量
                    self.set_variable(var_name, Value::Object(obj));

                    ExecutionResult::Continue
                } else {
                    // 对象不存在或类型错误
                    ExecutionResult::Continue
                }
            },
            _ => {
                // 暂时不支持复杂的对象表达式
                ExecutionResult::Continue
            }
        }
    }
}