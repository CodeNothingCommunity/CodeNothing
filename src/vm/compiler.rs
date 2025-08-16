use crate::ast::{Program, Function, Statement, Expression, BinaryOperator, CompareOperator, NamespaceType, Namespace};
use crate::vm::bytecode::{ByteCode, CompiledProgram, CompiledFunction, ClassInfo};
use crate::interpreter::value::Value;
use crate::interpreter::library_loader::{load_library, call_library_function};
use std::collections::HashMap;
use std::sync::Arc;

/// 编译器 - 将AST编译为字节码
pub struct Compiler {
    /// 当前编译的函数的字节码
    bytecode: Vec<ByteCode>,

    /// 常量池
    constants: Vec<Value>,

    /// 局部变量映射 (变量名 -> 索引)
    locals: HashMap<String, u16>,

    /// 当前局部变量计数
    local_count: u16,

    /// 跳转标签映射
    labels: HashMap<String, u32>,

    /// 待回填的跳转地址
    pending_jumps: Vec<(usize, String)>,

    /// 导入的库映射 (库名 -> 函数映射)
    imported_libraries: HashMap<String, Arc<HashMap<String, crate::interpreter::library_loader::LibraryFunction>>>,

    /// 函数索引映射 (函数名 -> 索引)
    function_indices: HashMap<String, u16>,

    /// 下一个可用的函数索引
    next_function_index: u16,

    /// 命名空间函数映射 (完整路径 -> 函数)
    namespaced_functions: HashMap<String, Function>,

    /// 循环控制：当前循环ID
    current_loop_id: u32,

    /// 循环控制：循环栈 (循环ID, 开始地址, 结束地址占位符)
    loop_stack: Vec<(u32, u32, Vec<usize>)>,

    /// 循环控制：break语句地址列表 (需要回填的地址)
    break_addresses: Vec<usize>,

    /// 循环控制：continue语句地址列表 (需要回填的地址)
    continue_addresses: Vec<usize>,
}

impl Compiler {
    /// 创建新的编译器实例
    pub fn new() -> Self {
        Compiler {
            bytecode: Vec::new(),
            constants: Vec::new(),
            locals: HashMap::new(),
            local_count: 0,
            labels: HashMap::new(),
            pending_jumps: Vec::new(),
            imported_libraries: HashMap::new(),
            function_indices: HashMap::new(),
            next_function_index: 0,
            namespaced_functions: HashMap::new(),
            current_loop_id: 0,
            loop_stack: Vec::new(),
            break_addresses: Vec::new(),
            continue_addresses: Vec::new(),
        }
    }
    
    /// 编译整个程序
    pub fn compile_program(&mut self, program: &Program) -> Result<CompiledProgram, String> {
        let mut compiled_functions = HashMap::new();
        let mut classes = HashMap::new();

        // 注意：println 等内置函数现在通过特殊处理实现，不需要加载标准库

        // 处理库导入
        for (ns_type, path) in &program.imported_namespaces {
            match ns_type {
                NamespaceType::Library => {
                    if path.len() != 1 {
                        return Err("库名称应该是单个标识符".to_string());
                    }

                    let lib_name = &path[0];
                    println!("🚀 VM: 加载库 {}", lib_name);

                    // 加载库
                    match crate::interpreter::library_loader::load_library(lib_name) {
                        Ok(functions) => {
                            self.imported_libraries.insert(lib_name.clone(), functions);
                        },
                        Err(err) => {
                            return Err(format!("无法加载库 '{}': {}", lib_name, err));
                        }
                    }
                },
                NamespaceType::Code => {
                    // 处理代码命名空间（支持多层级）
                    let namespace_path = path.join("::");
                    println!("🚀 VM: 处理代码命名空间 {}", namespace_path);

                    // 查找所有库中以该命名空间开头的函数
                    for (lib_name, lib_functions) in &self.imported_libraries {
                        let ns_prefix = format!("{}::", namespace_path);
                        for (func_full_path, _) in lib_functions.iter() {
                            if func_full_path.starts_with(&ns_prefix) {
                                println!("🚀 VM: 发现命名空间函数 {} 在库 {}", func_full_path, lib_name);

                                // 获取函数名（路径的最后一部分）
                                let parts: Vec<&str> = func_full_path.split("::").collect();
                                if let Some(func_name) = parts.last() {
                                    // 将函数添加到导入的命名空间映射
                                    // 这样可以通过简单的函数名调用命名空间函数
                                    println!("🚀 VM: 映射函数 {} -> {}", func_name, func_full_path);
                                }
                            }
                        }
                    }

                    // TODO: 处理代码定义的命名空间函数（非库函数）
                    // 这需要在编译时收集所有命名空间函数的信息
                }
            }
        }

        // 收集所有命名空间函数
        self.collect_namespaced_functions(program);

        // 为所有函数分配索引
        self.assign_function_indices(program);

        // 编译所有函数
        for function in &program.functions {
            let compiled_func = self.compile_function(function)?;
            compiled_functions.insert(function.name.clone(), compiled_func);
        }

        // 编译所有命名空间函数
        self.compile_namespaced_functions(&mut compiled_functions)?;

        // 编译所有类
        for class in &program.classes {
            let class_info = ClassInfo {
                name: class.name.clone(),
                fields: class.fields.iter().map(|f| f.name.clone()).collect(),
                methods: HashMap::new(), // TODO: 实现方法编译
            };
            classes.insert(class.name.clone(), class_info);
        }

        Ok(CompiledProgram {
            functions: compiled_functions,
            main_function: "main".to_string(),
            global_constants: self.constants.clone(),
            classes,
            imported_libraries: self.imported_libraries.clone(),
            function_indices: self.function_indices.clone(),
        })
    }
    
    /// 编译单个函数
    pub fn compile_function(&mut self, function: &Function) -> Result<CompiledFunction, String> {
        // 重置编译器状态
        self.bytecode.clear();
        self.constants.clear();
        self.locals.clear();
        self.local_count = 0;
        self.labels.clear();
        self.pending_jumps.clear();
        self.current_loop_id = 0;
        self.loop_stack.clear();
        self.break_addresses.clear();
        self.continue_addresses.clear();
        
        // 为参数分配局部变量索引
        for param in &function.parameters {
            self.locals.insert(param.name.clone(), self.local_count);
            self.local_count += 1;
        }
        
        // 编译函数体
        for statement in &function.body {
            self.compile_statement(statement)?;
        }
        
        // 如果函数没有显式返回，添加默认返回
        if self.bytecode.is_empty() || !matches!(self.bytecode.last(), Some(ByteCode::Return)) {
            self.emit(ByteCode::LoadConst(Value::None));
            self.emit(ByteCode::Return);
        }
        
        Ok(CompiledFunction {
            name: function.name.clone(),
            param_count: function.parameters.len() as u8,
            local_count: self.local_count,
            bytecode: self.bytecode.clone(),
            constants: self.constants.clone(),
        })
    }
    
    /// 编译语句
    fn compile_statement(&mut self, statement: &Statement) -> Result<(), String> {
        match statement {
            Statement::VariableDeclaration(name, _, expr) => {
                // 编译初始化表达式
                self.compile_expression(expr)?;
                
                // 分配局部变量
                let index = self.local_count;
                self.locals.insert(name.clone(), index);
                self.local_count += 1;
                
                // 存储到局部变量
                self.emit(ByteCode::StoreLocal(index));
            },
            
            Statement::IfElse(condition, then_body, else_branches) => {
                // 编译条件表达式
                self.compile_expression(condition)?;
                
                // 条件跳转到else分支
                let else_label = format!("else_{}", self.bytecode.len());
                self.emit(ByteCode::JumpIfFalse(0)); // 地址稍后回填
                let else_jump_addr = self.bytecode.len() - 1;
                
                // 编译then分支
                for stmt in then_body {
                    self.compile_statement(stmt)?;
                }
                
                // 跳转到if语句结束
                let end_label = format!("end_if_{}", self.bytecode.len());
                self.emit(ByteCode::Jump(0)); // 地址稍后回填
                let end_jump_addr = self.bytecode.len() - 1;
                
                // else分支开始位置
                let else_addr = self.bytecode.len() as u32;
                
                // 编译else分支 - 简化版本，只处理第一个else分支
                if let Some((_, else_body)) = else_branches.first() {
                    for stmt in else_body {
                        self.compile_statement(stmt)?;
                    }
                }
                
                // if语句结束位置
                let end_addr = self.bytecode.len() as u32;
                
                // 回填跳转地址
                if let ByteCode::JumpIfFalse(_) = &mut self.bytecode[else_jump_addr] {
                    self.bytecode[else_jump_addr] = ByteCode::JumpIfFalse(else_addr);
                }
                if let ByteCode::Jump(_) = &mut self.bytecode[end_jump_addr] {
                    self.bytecode[end_jump_addr] = ByteCode::Jump(end_addr);
                }
            },
            
            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    self.compile_expression(expr)?;
                } else {
                    self.emit(ByteCode::LoadConst(Value::None));
                }
                self.emit(ByteCode::Return);
            },
            
            Statement::FunctionCallStatement(expr) => {
                self.compile_expression(expr)?;
                // 函数调用语句的结果不需要保留在栈上
                self.emit(ByteCode::Pop);
            },

            Statement::WhileLoop(condition, body) => {
                self.compile_while_loop(condition, body)?;
            },

            Statement::ForLoop(var_name, start_expr, end_expr, body) => {
                self.compile_for_loop(var_name, start_expr, end_expr, body)?;
            },

            Statement::ForEachLoop(var_name, collection_expr, body) => {
                self.compile_foreach_loop(var_name, collection_expr, body)?;
            },

            Statement::Break => {
                if self.loop_stack.is_empty() {
                    return Err("break语句只能在循环内使用".to_string());
                }
                // 记录需要回填的break地址
                self.emit(ByteCode::Break(0)); // 地址稍后回填
                self.break_addresses.push(self.bytecode.len() - 1);
            },

            Statement::Continue => {
                if self.loop_stack.is_empty() {
                    return Err("continue语句只能在循环内使用".to_string());
                }
                // 记录需要回填的continue地址
                self.emit(ByteCode::Continue(0)); // 地址稍后回填
                self.continue_addresses.push(self.bytecode.len() - 1);
            },

            Statement::VariableDeclaration(var_name, _var_type, init_expr) => {
                // 编译初始化表达式
                self.compile_expression(init_expr)?;

                // 分配局部变量索引
                let var_index = self.local_count;
                self.locals.insert(var_name.clone(), var_index);
                self.local_count += 1;

                // 存储到局部变量
                self.emit(ByteCode::StoreLocal(var_index));
            },

            Statement::VariableAssignment(var_name, value_expr) => {
                // 编译赋值表达式
                self.compile_expression(value_expr)?;

                // 查找变量索引
                if let Some(&var_index) = self.locals.get(var_name) {
                    self.emit(ByteCode::StoreLocal(var_index));
                } else {
                    // 如果不是局部变量，则作为全局变量处理
                    self.emit(ByteCode::StoreGlobal(var_name.clone()));
                }
            },

            Statement::FieldAssignment(object_expr, field_name, value_expr) => {
                // 编译对象表达式
                self.compile_expression(object_expr)?;

                // 编译值表达式
                self.compile_expression(value_expr)?;

                // 存储字段
                self.emit(ByteCode::StoreField(field_name.clone()));
            },

            Statement::ArrayElementAssignment(array_expr, index_expr, value_expr) => {
                // 编译数组表达式
                self.compile_expression(array_expr)?;

                // 编译索引表达式
                self.compile_expression(index_expr)?;

                // 编译值表达式
                self.compile_expression(value_expr)?;

                // 存储数组元素
                self.emit(ByteCode::StoreArrayElement);
            },

            _ => {
                return Err(format!("暂不支持编译语句: {:?}", statement));
            }
        }
        
        Ok(())
    }
    
    /// 编译表达式
    fn compile_expression(&mut self, expression: &Expression) -> Result<(), String> {
        match expression {
            Expression::IntLiteral(value) => {
                self.emit(ByteCode::LoadConst(Value::Int(*value)));
            },
            
            Expression::FloatLiteral(value) => {
                self.emit(ByteCode::LoadConst(Value::Float(*value)));
            },
            
            Expression::StringLiteral(value) => {
                self.emit(ByteCode::LoadConst(Value::String(value.clone())));
            },
            
            Expression::BoolLiteral(value) => {
                self.emit(ByteCode::LoadConst(Value::Bool(*value)));
            },
            
            Expression::Variable(name) => {
                if let Some(&index) = self.locals.get(name) {
                    self.emit(ByteCode::LoadLocal(index));
                } else {
                    self.emit(ByteCode::LoadGlobal(name.clone()));
                }
            },
            
            Expression::BinaryOp(left, op, right) => {
                // 编译左操作数
                self.compile_expression(left)?;
                
                // 编译右操作数
                self.compile_expression(right)?;
                
                // 生成运算指令
                match op {
                    BinaryOperator::Add => self.emit(ByteCode::Add),
                    BinaryOperator::Subtract => self.emit(ByteCode::Sub),
                    BinaryOperator::Multiply => self.emit(ByteCode::Mul),
                    BinaryOperator::Divide => self.emit(ByteCode::Div),
                    // 注意：BinaryOperator中没有比较操作，需要处理CompareOp
                    // 这里暂时注释掉，稍后处理CompareOp
                    // BinaryOperator::LessEqual => self.emit(ByteCode::LessEqual),
                    _ => return Err(format!("暂不支持的运算符: {:?}", op)),
                }
            },
            
            Expression::FunctionCall(name, args) => {
                // 编译参数
                for arg in args {
                    self.compile_expression(arg)?;
                }

                // 检查是否是库函数（通过命名空间导入的函数）
                let mut found_library_call = None;

                // 查找所有导入的库中是否有匹配的函数
                for (lib_name, lib_functions) in &self.imported_libraries {
                    // 检查直接函数名
                    if lib_functions.contains_key(name) {
                        found_library_call = Some((lib_name.clone(), name.clone()));
                        break;
                    }

                    // 检查所有可能的命名空间前缀（不再特殊处理std）
                    // 遍历所有已知的命名空间前缀
                    for (full_func_name, _) in lib_functions.iter() {
                        if let Some(ns_end) = full_func_name.find("::") {
                            let func_suffix = &full_func_name[ns_end + 2..];
                            if func_suffix == name {
                                found_library_call = Some((lib_name.clone(), full_func_name.clone()));
                                break;
                            }
                        }
                    }

                    if found_library_call.is_some() {
                        break;
                    }
                }

                if let Some((lib_name, func_name)) = found_library_call {
                    self.emit(ByteCode::CallLibrary(lib_name, func_name, args.len() as u8));
                } else {
                    // 生成普通函数调用指令
                    let func_index = self.get_function_index(name);
                    self.emit(ByteCode::Call(func_index, args.len() as u8));
                }
            },

            Expression::CompareOp(left, op, right) => {
                // 编译左操作数
                self.compile_expression(left)?;

                // 编译右操作数
                self.compile_expression(right)?;

                // 生成比较指令
                match op {
                    CompareOperator::LessEqual => self.emit(ByteCode::LessEqual),
                    CompareOperator::Equal => self.emit(ByteCode::Equal),
                    CompareOperator::NotEqual => self.emit(ByteCode::NotEqual),
                    CompareOperator::Less => self.emit(ByteCode::Less),
                    CompareOperator::Greater => self.emit(ByteCode::Greater),
                    CompareOperator::GreaterEqual => self.emit(ByteCode::GreaterEqual),
                }
            },

            Expression::StaticMethodCall(class_name, method_name, args) => {
                // 支持多层级命名空间：将StaticMethodCall转换为NamespacedFunctionCall处理
                let path = vec![class_name.clone(), method_name.clone()];

                // 编译参数
                for arg in args {
                    self.compile_expression(arg)?;
                }

                // 构建完整的函数路径
                let full_func_name = path.join("::");
                let mut found_library_call = None;

                // 首先查找库函数
                for (lib_name, lib_functions) in &self.imported_libraries {
                    if lib_functions.contains_key(&full_func_name) {
                        found_library_call = Some((lib_name.clone(), full_func_name.clone()));
                        break;
                    }
                }

                if let Some((lib_name, func_name)) = found_library_call {
                    self.emit(ByteCode::CallLibrary(lib_name, func_name, args.len() as u8));
                } else {
                    // 查找代码定义的命名空间函数
                    if self.namespaced_functions.contains_key(&full_func_name) {
                        // 获取函数索引
                        let func_index = self.get_function_index(&full_func_name);
                        self.emit(ByteCode::Call(func_index, args.len() as u8));
                    } else {
                        return Err(format!("未找到静态方法: {}", full_func_name));
                    }
                }
            },

            Expression::ArrayLiteral(elements) => {
                // 创建数组
                self.emit(ByteCode::NewArray(elements.len() as u32));

                // 填充数组元素
                for (index, element) in elements.iter().enumerate() {
                    self.emit(ByteCode::Dup); // 复制数组引用
                    self.emit(ByteCode::LoadConst(Value::Int(index as i32))); // 索引
                    self.compile_expression(element)?; // 值
                    self.emit(ByteCode::StoreArrayElementKeep); // 使用保留数组的版本
                }
                // 注意：最后栈顶应该还有数组引用
            },

            Expression::ArrayAccess(array, index) => {
                self.compile_expression(array)?;   // 数组
                self.compile_expression(index)?;   // 索引
                self.emit(ByteCode::LoadArrayElement);
            },

            Expression::ObjectCreation(class_name, args) => {
                // 编译构造函数参数
                for arg in args {
                    self.compile_expression(arg)?;
                }

                // 创建对象实例
                self.emit(ByteCode::NewObject(class_name.clone()));

                // 调用构造函数（如果有参数）
                if !args.is_empty() {
                    let constructor_name = format!("{}::constructor", class_name);
                    if let Some(&func_index) = self.function_indices.get(&constructor_name) {
                        self.emit(ByteCode::Call(func_index, args.len() as u8));
                    }
                }
            },

            Expression::FieldAccess(object, field_name) => {
                self.compile_expression(object)?;
                self.emit(ByteCode::LoadField(field_name.clone()));
            },

            Expression::MethodCall(object, method_name, args) => {
                // 编译对象表达式
                self.compile_expression(object)?;

                // 编译参数
                for arg in args {
                    self.compile_expression(arg)?;
                }

                // 调用方法
                self.emit(ByteCode::CallMethod(method_name.clone(), args.len() as u8));
            },

            Expression::NamespacedFunctionCall(path, args) => {
                // 编译参数
                for arg in args {
                    self.compile_expression(arg)?;
                }

                // 构建完整的函数路径
                let full_func_name = path.join("::");
                let mut found_library_call = None;

                // 首先查找库函数
                for (lib_name, lib_functions) in &self.imported_libraries {
                    if lib_functions.contains_key(&full_func_name) {
                        found_library_call = Some((lib_name.clone(), full_func_name.clone()));
                        break;
                    }
                }

                if let Some((lib_name, func_name)) = found_library_call {
                    self.emit(ByteCode::CallLibrary(lib_name, func_name, args.len() as u8));
                } else {
                    // 查找代码定义的命名空间函数
                    if self.namespaced_functions.contains_key(&full_func_name) {
                        // 获取函数索引
                        let func_index = self.get_function_index(&full_func_name);
                        self.emit(ByteCode::Call(func_index, args.len() as u8));
                    } else {
                        return Err(format!("未找到命名空间函数: {}", full_func_name));
                    }
                }
            },

            _ => {
                return Err(format!("暂不支持编译表达式: {:?}", expression));
            }
        }
        
        Ok(())
    }
    
    /// 发射字节码指令
    fn emit(&mut self, instruction: ByteCode) {
        self.bytecode.push(instruction);
    }

    /// 收集所有命名空间函数
    fn collect_namespaced_functions(&mut self, program: &Program) {
        for namespace in &program.namespaces {
            self.collect_namespace_functions(namespace, "");
        }
    }

    /// 递归收集命名空间中的函数
    fn collect_namespace_functions(&mut self, namespace: &Namespace, parent_path: &str) {
        let current_path = if parent_path.is_empty() {
            namespace.name.clone()
        } else {
            format!("{}::{}", parent_path, namespace.name)
        };

        // 收集当前命名空间的函数
        for function in &namespace.functions {
            let full_path = format!("{}::{}", current_path, function.name);
            println!("🚀 VM: 收集命名空间函数 {}", full_path);
            self.namespaced_functions.insert(full_path, function.clone());
        }

        // 递归收集嵌套命名空间的函数
        for nested_namespace in &namespace.namespaces {
            self.collect_namespace_functions(nested_namespace, &current_path);
        }
    }

    /// 编译所有命名空间函数
    fn compile_namespaced_functions(&mut self, compiled_functions: &mut HashMap<String, CompiledFunction>) -> Result<(), String> {
        for (full_path, function) in &self.namespaced_functions.clone() {
            println!("🚀 VM: 编译命名空间函数 {}", full_path);
            let compiled_func = self.compile_function(function)?;
            compiled_functions.insert(full_path.clone(), compiled_func);
        }
        Ok(())
    }

    /// 为所有函数分配索引
    fn assign_function_indices(&mut self, program: &Program) {
        self.function_indices.insert("main".to_string(), 0);
        self.next_function_index = 1;

        // 为普通函数分配索引（包括main函数）
        for function in &program.functions {
            self.function_indices.insert(function.name.clone(), self.next_function_index);
            self.next_function_index += 1;
        }

        // 为命名空间函数分配索引
        for (full_path, _) in &self.namespaced_functions {
            self.function_indices.insert(full_path.clone(), self.next_function_index);
            self.next_function_index += 1;
            println!("🚀 VM: 为命名空间函数 {} 分配索引 {}", full_path, self.next_function_index - 1);
        }
    }

    /// 获取函数索引
    fn get_function_index(&self, name: &str) -> u16 {
        self.function_indices.get(name).copied().unwrap_or(999)
    }

    /// 编译while循环
    fn compile_while_loop(&mut self, condition: &Expression, body: &[Statement]) -> Result<(), String> {
        let loop_id = self.current_loop_id;
        self.current_loop_id += 1;

        // 循环开始
        let loop_start = self.bytecode.len() as u32;
        self.emit(ByteCode::LoopStart(loop_id));

        // 编译条件
        self.compile_expression(condition)?;

        // 条件为假时跳出循环
        self.emit(ByteCode::JumpIfFalse(0)); // 地址稍后回填
        let exit_jump = self.bytecode.len() - 1;

        // 保存当前循环信息
        self.loop_stack.push((loop_id, loop_start, vec![exit_jump]));
        let old_break_len = self.break_addresses.len();
        let old_continue_len = self.continue_addresses.len();

        // 编译循环体
        for stmt in body {
            self.compile_statement(stmt)?;
        }

        // 跳回循环开始
        self.emit(ByteCode::Continue(loop_start));

        // 循环结束
        let loop_end = self.bytecode.len() as u32;
        self.emit(ByteCode::LoopEnd(loop_id));

        // 回填退出地址
        if let ByteCode::JumpIfFalse(_) = &mut self.bytecode[exit_jump] {
            self.bytecode[exit_jump] = ByteCode::JumpIfFalse(loop_end);
        }

        // 回填break和continue地址
        self.backfill_loop_jumps(old_break_len, old_continue_len, loop_start, loop_end)?;

        // 恢复循环栈
        self.loop_stack.pop();

        Ok(())
    }

    /// 编译for循环
    fn compile_for_loop(&mut self, var_name: &str, start_expr: &Expression, end_expr: &Expression, body: &[Statement]) -> Result<(), String> {
        let loop_id = self.current_loop_id;
        self.current_loop_id += 1;

        // 分配循环变量
        let var_index = self.local_count;
        self.locals.insert(var_name.to_string(), var_index);
        self.local_count += 1;

        // 初始化循环变量
        self.compile_expression(start_expr)?;
        self.emit(ByteCode::StoreLocal(var_index));

        // 编译结束值并存储到临时变量
        let end_var_index = self.local_count;
        self.local_count += 1;
        self.compile_expression(end_expr)?;
        self.emit(ByteCode::StoreLocal(end_var_index));

        // 循环开始
        let loop_start = self.bytecode.len() as u32;
        self.emit(ByteCode::LoopStart(loop_id));

        // 检查循环条件 (var <= end)
        self.emit(ByteCode::LoadLocal(var_index));
        self.emit(ByteCode::LoadLocal(end_var_index));
        self.emit(ByteCode::LessEqual);

        // 条件为假时跳出循环
        self.emit(ByteCode::JumpIfFalse(0)); // 地址稍后回填
        let exit_jump = self.bytecode.len() - 1;

        // 保存当前循环信息
        // continue应该跳转到递增部分，而不是循环开始
        let continue_target = 0; // 稍后设置
        self.loop_stack.push((loop_id, loop_start, vec![exit_jump]));
        let old_break_len = self.break_addresses.len();
        let old_continue_len = self.continue_addresses.len();

        // 编译循环体
        for stmt in body {
            self.compile_statement(stmt)?;
        }

        // 递增部分开始（continue应该跳转到这里）
        let increment_start = self.bytecode.len() as u32;

        // 递增循环变量
        self.emit(ByteCode::LoadLocal(var_index));
        self.emit(ByteCode::LoadConst(Value::Int(1)));
        self.emit(ByteCode::Add);
        self.emit(ByteCode::StoreLocal(var_index));

        // 跳回循环开始
        self.emit(ByteCode::Continue(loop_start));

        // 循环结束
        let loop_end = self.bytecode.len() as u32;
        self.emit(ByteCode::LoopEnd(loop_id));

        // 回填退出地址
        if let ByteCode::JumpIfFalse(_) = &mut self.bytecode[exit_jump] {
            self.bytecode[exit_jump] = ByteCode::JumpIfFalse(loop_end);
        }

        // 回填break和continue地址（continue跳转到递增部分）
        self.backfill_loop_jumps_for(old_break_len, old_continue_len, increment_start, loop_end)?;

        // 恢复循环栈
        self.loop_stack.pop();

        Ok(())
    }

    /// 编译foreach循环
    fn compile_foreach_loop(&mut self, var_name: &str, collection_expr: &Expression, body: &[Statement]) -> Result<(), String> {
        let loop_id = self.current_loop_id;
        self.current_loop_id += 1;

        // 分配循环变量
        let var_index = self.local_count;
        self.locals.insert(var_name.to_string(), var_index);
        self.local_count += 1;

        // 分配迭代器变量
        let iter_index = self.local_count;
        self.local_count += 1;

        // 获取集合的迭代器
        self.compile_expression(collection_expr)?;
        self.emit(ByteCode::GetIterator);
        self.emit(ByteCode::StoreLocal(iter_index));

        // 循环开始
        let loop_start = self.bytecode.len() as u32;
        self.emit(ByteCode::LoopStart(loop_id));

        // 检查迭代器是否有下一个元素
        self.emit(ByteCode::LoadLocal(iter_index));
        self.emit(ByteCode::IteratorHasNext);

        // 没有下一个元素时跳出循环
        self.emit(ByteCode::JumpIfFalse(0)); // 地址稍后回填
        let exit_jump = self.bytecode.len() - 1;

        // 获取下一个元素
        self.emit(ByteCode::LoadLocal(iter_index));
        self.emit(ByteCode::IteratorNext);
        self.emit(ByteCode::StoreLocal(var_index));

        // 保存当前循环信息
        self.loop_stack.push((loop_id, loop_start, vec![exit_jump]));
        let old_break_len = self.break_addresses.len();
        let old_continue_len = self.continue_addresses.len();

        // 编译循环体
        for stmt in body {
            self.compile_statement(stmt)?;
        }

        // 跳回循环开始
        self.emit(ByteCode::Continue(loop_start));

        // 循环结束
        let loop_end = self.bytecode.len() as u32;
        self.emit(ByteCode::LoopEnd(loop_id));

        // 回填退出地址
        if let ByteCode::JumpIfFalse(_) = &mut self.bytecode[exit_jump] {
            self.bytecode[exit_jump] = ByteCode::JumpIfFalse(loop_end);
        }

        // 回填break和continue地址
        self.backfill_loop_jumps(old_break_len, old_continue_len, loop_start, loop_end)?;

        // 恢复循环栈
        self.loop_stack.pop();

        Ok(())
    }

    /// 回填循环跳转地址
    fn backfill_loop_jumps(&mut self, old_break_len: usize, old_continue_len: usize, loop_start: u32, loop_end: u32) -> Result<(), String> {
        // 回填break地址
        for &addr in &self.break_addresses[old_break_len..] {
            if let ByteCode::Break(_) = &mut self.bytecode[addr] {
                self.bytecode[addr] = ByteCode::Break(loop_end);
            }
        }
        self.break_addresses.truncate(old_break_len);

        // 回填continue地址
        for &addr in &self.continue_addresses[old_continue_len..] {
            if let ByteCode::Continue(_) = &mut self.bytecode[addr] {
                self.bytecode[addr] = ByteCode::Continue(loop_start);
            }
        }
        self.continue_addresses.truncate(old_continue_len);

        Ok(())
    }

    /// 回填for循环跳转地址（continue跳转到递增部分）
    fn backfill_loop_jumps_for(&mut self, old_break_len: usize, old_continue_len: usize, continue_target: u32, loop_end: u32) -> Result<(), String> {
        // 回填break地址
        for &addr in &self.break_addresses[old_break_len..] {
            if let ByteCode::Break(_) = &mut self.bytecode[addr] {
                self.bytecode[addr] = ByteCode::Break(loop_end);
            }
        }
        self.break_addresses.truncate(old_break_len);

        // 回填continue地址（跳转到递增部分）
        for &addr in &self.continue_addresses[old_continue_len..] {
            if let ByteCode::Continue(_) = &mut self.bytecode[addr] {
                self.bytecode[addr] = ByteCode::Continue(continue_target);
            }
        }
        self.continue_addresses.truncate(old_continue_len);

        Ok(())
    }
}
