use crate::ast::{Program, Expression, Statement, BinaryOperator, Type, Namespace, CompareOperator, LogicalOperator, Function, NamespaceType, Class, Enum};
use crate::analyzer::{VariableLifetimeAnalyzer, LifetimeAnalysisResult};
use std::collections::HashMap;

/// 变量位置枚举，用于缓存变量查找结果
#[derive(Debug, Clone, PartialEq)]
pub enum VariableLocation {
    Constant,
    Local,
    Global,
    Function,
}
use super::value::{Value, ObjectInstance};
use super::evaluator::{Evaluator, perform_binary_operation, evaluate_compare_operation};
use super::executor::{Executor, ExecutionResult, update_variable_value, handle_increment, handle_decrement, execute_if_else};
use super::library_loader::{load_library, call_library_function, convert_values_to_string_args, convert_value_to_string_arg};
use std::sync::Arc;
use std::env;
use super::function_calls::FunctionCallHandler;
use super::expression_evaluator::ExpressionEvaluator;
use super::statement_executor::StatementExecutor;
use super::generic_manager::GenericTypeManager;

// 添加调试模式检查函数
fn is_debug_mode() -> bool {
    env::args().any(|arg| arg == "--cn-debug")
}

// 添加条件打印函数
pub fn debug_println(msg: &str) {
    if is_debug_mode() {
        println!("{}", msg);
    }
}

pub fn interpret(program: &Program, auto_namespace: bool) -> Value {
    interpret_with_timeout(program, auto_namespace, false)
}

pub fn interpret_with_timeout(program: &Program, auto_namespace: bool, check_timeout: bool) -> Value {
    // 创建解释器
    let mut interpreter = Interpreter::new(program, auto_namespace);

    // 设置是否启用超时检查
    interpreter.set_timeout_enabled(check_timeout);

    // v0.7.4新增：执行变量生命周期分析
    interpreter.perform_lifetime_analysis();

    // 处理顶层的命名空间导入
    for (ns_type, path) in &program.imported_namespaces {
        match ns_type {
            NamespaceType::Library => {
                if path.len() != 1 {
                    panic!("库名称应该是单个标识符");
                }
                
                let lib_name = &path[0];
                debug_println(&format!("导入顶层动态库: {}", lib_name));
                
                // 尝试加载库
                match load_library(lib_name) {
                    Ok(functions) => {
                        // 库加载成功，将其添加到已导入库列表
                        interpreter.imported_libraries.insert(lib_name.to_string(), functions);
                        debug_println(&format!("顶层库 '{}' 加载成功", lib_name));
                        
                        // 获取库支持的命名空间
                        if let Ok(namespaces) = super::library_loader::get_library_namespaces(lib_name) {
                            for ns in namespaces {
                                debug_println(&format!("注册库 '{}' 的命名空间: {}", lib_name, ns));
                                interpreter.library_namespaces.insert(ns.to_string(), lib_name.to_string());
                            }
                        }
                        
                        // 将库中的所有函数添加到全局函数列表
                        if let Some(lib_functions) = interpreter.imported_libraries.get(lib_name) {
                            debug_println(&format!("库 '{}' 中的函数:", lib_name));
                            let mut found_namespaces = std::collections::HashSet::new();
                            for (func_name, _) in lib_functions.iter() {
                                debug_println(&format!("  - {}", func_name));
                                // 检查是否是命名空间函数（包含::）
                                if func_name.contains("::") {
                                    let parts: Vec<&str> = func_name.split("::").collect();
                                    if parts.len() >= 2 {
                                        let ns_name = parts[0];
                                        // 自动注册所有命名空间前缀到library_namespaces
                                        if !found_namespaces.contains(ns_name) {
                                            debug_println(&format!("  自动注册命名空间: {} -> 库 {}", ns_name, lib_name));
                                            interpreter.library_namespaces.insert(ns_name.to_string(), lib_name.to_string());
                                            found_namespaces.insert(ns_name);
                                        }
                                    }
                                }
                                // 直接将库函数注册为全局函数，这样可以直接调用
                                interpreter.library_functions.insert(func_name.to_string(), (lib_name.to_string(), func_name.to_string()));
                            }
                        }
                    },
                    Err(err) => {
                        panic!("无法加载顶层库 '{}': {}", lib_name, err);
                    }
                }
            },
            NamespaceType::Code => {
                // 代码命名空间的导入在函数执行上下文中处理
                let namespace_path = path.join("::");
                debug_println(&format!("记录顶层命名空间导入: {}", namespace_path));
                
                // 将命名空间路径添加到全局导入列表，供后续函数使用
                interpreter.global_namespace_imports.push(path.clone());
            }
        }
    }
    
    interpreter.run()
}

pub struct Interpreter<'a> {
    pub program: &'a Program,
    pub functions: HashMap<String, &'a crate::ast::Function>,
    // 命名空间函数映射，键是完整路径，如 "math::add"
    pub namespaced_functions: HashMap<String, &'a crate::ast::Function>,
    // 导入的命名空间，键是函数名，值是完整路径
    pub imported_namespaces: HashMap<String, Vec<String>>,
    // 导入的库，键是库名
    pub imported_libraries: HashMap<String, Arc<HashMap<String, super::library_loader::LibraryFunction>>>,
    // 库函数映射，键是函数名，值是(库名, 函数名)
    pub library_functions: HashMap<String, (String, String)>,
    // 全局变量环境
    pub global_env: HashMap<String, Value>,
    // 局部变量环境（函数内）
    pub local_env: HashMap<String, Value>,
    // 全局命名空间导入（作为默认导入在所有函数中可用）
    pub global_namespace_imports: Vec<Vec<String>>,
    // 库命名空间映射，键是命名空间名称，值是库名
    pub library_namespaces: HashMap<String, String>,
    // 常量环境，键是常量名，值是常量值
    pub constants: HashMap<String, Value>,
    // 自动命名空间查找配置
    pub auto_namespace_lookup: bool,
    // 作用域级别命名空间导入栈（每层是一个map: 函数名->完整路径）
    pub namespace_import_stack: Vec<HashMap<String, Vec<String>>>,
    // 类定义存储
    pub classes: HashMap<String, &'a Class>,
    // 枚举定义存储
    pub enums: HashMap<String, &'a Enum>,
    // 静态成员存储
    pub static_members: HashMap<String, crate::interpreter::value::StaticMembers>,
    // 变量类型存储，键是变量名，值是声明的类型
    pub variable_types: HashMap<String, Type>,
    // v0.7.4新增：变量生命周期分析器
    pub lifetime_analyzer: VariableLifetimeAnalyzer,
    // 生命周期分析结果
    pub lifetime_analysis_result: Option<LifetimeAnalysisResult>,
    // 变量查找缓存：存储最近访问的变量位置
    pub variable_cache: HashMap<String, VariableLocation>,
    // 超时机制相关字段
    pub start_time: std::time::Instant,
    pub timeout_duration: std::time::Duration,
    pub operation_count: usize,
    pub max_operations: usize,
    pub timeout_enabled: bool,
    // 泛型类型管理器
    pub generic_manager: GenericTypeManager,
    // 作用域栈，用于管理局部变量环境
    pub scope_stack: Vec<HashMap<String, Value>>,
}

impl<'a> Interpreter<'a> {
    pub fn new(program: &'a Program, auto_namespace: bool) -> Self {
        let mut functions = HashMap::new();
        let mut namespaced_functions = HashMap::new();
        let library_namespaces = HashMap::new();
        let mut constants = HashMap::new(); // 初始化常量环境
        
        // 注册全局函数
        for function in &program.functions {
            functions.insert(function.name.clone(), function);
        }
        
        // 注册命名空间函数
        for namespace in &program.namespaces {
            Self::register_namespace_functions(namespace, &mut namespaced_functions, "");
        }
        
        // 初始化解释器
        let mut interpreter = Interpreter {
            program,
            functions,
            namespaced_functions,
            imported_namespaces: HashMap::new(),
            imported_libraries: HashMap::new(),
            library_functions: HashMap::new(),
            global_env: HashMap::new(),
            local_env: HashMap::new(),
            global_namespace_imports: Vec::new(),
            library_namespaces,
            constants, // 添加常量环境
            auto_namespace_lookup: auto_namespace, // 添加自动命名空间查找配置
            namespace_import_stack: vec![HashMap::new()], // 初始化栈，最外层一层
            classes: HashMap::new(),
            enums: HashMap::new(),
            static_members: HashMap::new(),
            variable_types: HashMap::new(), // 初始化变量类型映射
            variable_cache: HashMap::new(), // 初始化变量缓存
            // v0.7.4新增：初始化生命周期分析器
            lifetime_analyzer: VariableLifetimeAnalyzer::new(),
            lifetime_analysis_result: None,
            // 超时机制初始化
            start_time: std::time::Instant::now(),
            timeout_duration: std::time::Duration::from_secs(30), // 默认30秒超时
            operation_count: 0,
            max_operations: 1_000_000, // 默认最大100万次操作
            timeout_enabled: false, // 默认禁用超时检查
            // 泛型管理器初始化
            generic_manager: GenericTypeManager::new(),
            // 作用域栈初始化
            scope_stack: Vec::new(),
        };
        
        // 初始化常量
        for (name, _typ, expr) in &program.constants {
            // 计算常量值
            let value = interpreter.evaluate_expression_direct(expr);
            // 存储常量值
            interpreter.constants.insert(name.clone(), value);
        }
        
        // 注册类定义
        for class in &program.classes {
            interpreter.classes.insert(class.name.clone(), class);
            
            // 初始化静态成员
            let mut static_fields = HashMap::new();
            for field in &class.fields {
                if field.is_static {
                    let initial_value = match field.initial_value {
                        Some(ref expr) => interpreter.evaluate_expression_direct(expr),
                        None => match field.field_type {
                            crate::ast::Type::Int => crate::interpreter::value::Value::Int(0),
                            crate::ast::Type::Float => crate::interpreter::value::Value::Float(0.0),
                            crate::ast::Type::Bool => crate::interpreter::value::Value::Bool(false),
                            crate::ast::Type::String => crate::interpreter::value::Value::String(String::new()),
                            crate::ast::Type::Long => crate::interpreter::value::Value::Long(0),
                            _ => crate::interpreter::value::Value::None,
                        }
                    };
                    static_fields.insert(field.name.clone(), initial_value);
                }
            }
            
            interpreter.static_members.insert(class.name.clone(), crate::interpreter::value::StaticMembers {
                static_fields,
            });
        }

        // 注册枚举定义
        for enum_def in &program.enums {
            interpreter.enums.insert(enum_def.name.clone(), enum_def);
        }

        interpreter
    }

    /// 检查是否超时（只有启用时才检查）
    pub fn check_timeout(&mut self) -> Result<(), String> {
        // 如果未启用超时检查，直接返回成功
        if !self.timeout_enabled {
            return Ok(());
        }

        // 检查时间限制
        if self.start_time.elapsed() > self.timeout_duration {
            return Err(format!("程序执行超时 ({:?})", self.timeout_duration));
        }

        Ok(())
    }

    /// 重置超时计时器
    pub fn reset_timeout(&mut self) {
        self.start_time = std::time::Instant::now();
        self.operation_count = 0;
    }

    /// 设置超时时间
    pub fn set_timeout(&mut self, duration: std::time::Duration) {
        self.timeout_duration = duration;
    }

    /// 设置最大操作次数
    pub fn set_max_operations(&mut self, max_ops: usize) {
        self.max_operations = max_ops;
    }

    /// 设置是否启用超时检查
    pub fn set_timeout_enabled(&mut self, enabled: bool) {
        self.timeout_enabled = enabled;
    }
    
    // 递归注册命名空间中的所有函数
    fn register_namespace_functions(
        namespace: &'a Namespace, 
        map: &mut HashMap<String, &'a crate::ast::Function>,
        prefix: &str
    ) {
        let current_prefix = if prefix.is_empty() {
            namespace.name.clone()
        } else {
            format!("{}::{}", prefix, namespace.name)
        };
        
        debug_println(&format!("注册命名空间 '{}' (类型: {:?}) 中的函数", current_prefix, namespace.ns_type));
        
        // 注册当前命名空间中的函数
        for function in &namespace.functions {
            let full_path = format!("{}::{}", current_prefix, function.name);
            debug_println(&format!("  注册函数: {}", full_path));
            map.insert(full_path, function);
        }
        
        // 递归注册子命名空间中的函数
        for sub_namespace in &namespace.namespaces {
            debug_println(&format!("  处理子命名空间: {}", sub_namespace.name));
            Self::register_namespace_functions(sub_namespace, map, &current_prefix);
        }
    }
    
    pub fn run(&mut self) -> Value {
        // 重置超时计时器
        self.reset_timeout();

        // 直接执行，暂时禁用 panic 恢复机制以便调试
        self.run_internal()
    }

    fn run_internal(&mut self) -> Value {
        // 先应用全局命名空间导入
        for path in &self.global_namespace_imports {
            let namespace_path = path.join("::");
            debug_println(&format!("应用全局命名空间导入: {}", namespace_path));

            // 检查代码命名空间中的函数
            for (full_path, _) in &self.namespaced_functions {
                // 检查函数是否属于指定的命名空间
                if full_path.starts_with(&namespace_path) {
                    // 获取函数名（路径的最后一部分）
                    let parts: Vec<&str> = full_path.split("::").collect();
                    if let Some(func_name) = parts.last() {
                        // 将函数添加到导入的命名空间列表
                        self.imported_namespaces
                            .entry(func_name.to_string())
                            .or_insert_with(Vec::new)
                            .push(full_path.clone());

                        debug_println(&format!("  导入全局代码函数: {}", full_path));
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
                                self.imported_namespaces
                                    .entry(func_name.to_string())
                                    .or_insert_with(Vec::new)
                                    .push(func_full_path.clone());
                                debug_println(&format!("  导入全局库函数: {}", func_full_path));
                            }
                        }
                    }
                }
            }
        }
        
        // 查找 main 函数并执行
        if let Some(main_fn) = self.functions.get("main") {
            self.execute_function_direct(main_fn)
        } else {
            panic!("没有找到 main 函数");
        }
    }
    
    // 辅助函数：调用函数并处理参数
    pub fn call_function_impl(&mut self, function: &'a crate::ast::Function, arg_values: Vec<Value>) -> Value {
        // 保存当前的局部环境
        let old_local_env = self.local_env.clone();
        
        // 清空局部环境，为新函数调用准备
        self.local_env.clear();
        
        // 绑定参数值到参数名
        for (i, param) in function.parameters.iter().enumerate() {
            if i < arg_values.len() {
                // 如果提供了参数值，使用提供的值
                self.local_env.insert(param.name.clone(), arg_values[i].clone());
            } else if let Some(default_expr) = &param.default_value {
                // 如果参数有默认值，且未提供实参，计算默认值
                let default_value = ExpressionEvaluator::evaluate_expression(self, default_expr);
                self.local_env.insert(param.name.clone(), default_value);
            } else {
                // 如果参数既没有提供值，又没有默认值，报错
                panic!("函数 '{}' 需要参数 '{}'，但未提供值", function.name, param.name);
            }
        }
        
        // 执行函数体
        let result = self.execute_function_direct(function);
        
        // 恢复之前的局部环境
        self.local_env = old_local_env;
        
        result
    }
    
    // Getter methods for accessing internal state
    pub fn get_functions(&self) -> &HashMap<String, &'a crate::ast::Function> {
        &self.functions
    }
    
    pub fn get_namespaced_functions(&self) -> &HashMap<String, &'a crate::ast::Function> {
        &self.namespaced_functions
    }
    
    pub fn get_imported_namespaces(&self) -> &HashMap<String, Vec<String>> {
        &self.imported_namespaces
    }
    
    pub fn get_imported_libraries(&self) -> &HashMap<String, Arc<HashMap<String, super::library_loader::LibraryFunction>>> {
        &self.imported_libraries
    }
    
    pub fn get_library_functions(&self) -> &HashMap<String, (String, String)> {
        &self.library_functions
    }
    
    pub fn get_global_env(&self) -> &HashMap<String, Value> {
        &self.global_env
    }
    
    pub fn get_local_env(&self) -> &HashMap<String, Value> {
        &self.local_env
    }
    
    pub fn get_library_namespaces(&self) -> &HashMap<String, String> {
        &self.library_namespaces
    }
    
    pub fn get_constants(&self) -> &HashMap<String, Value> {
        &self.constants
    }
    
    // Mutable getter methods
    pub fn get_imported_namespaces_mut(&mut self) -> &mut HashMap<String, Vec<String>> {
        &mut self.imported_namespaces
    }
    
    pub fn get_imported_libraries_mut(&mut self) -> &mut HashMap<String, Arc<HashMap<String, super::library_loader::LibraryFunction>>> {
        &mut self.imported_libraries
    }
    
    pub fn get_library_functions_mut(&mut self) -> &mut HashMap<String, (String, String)> {
        &mut self.library_functions
    }
    
    pub fn get_global_env_mut(&mut self) -> &mut HashMap<String, Value> {
        &mut self.global_env
    }
    
    pub fn get_local_env_mut(&mut self) -> &mut HashMap<String, Value> {
        &mut self.local_env
    }
    
    pub fn get_library_namespaces_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.library_namespaces
    }

    /// v0.7.4新增：执行变量生命周期分析
    pub fn perform_lifetime_analysis(&mut self) {
        crate::lifetime_debug_println!("开始执行变量生命周期分析...");
        let start_time = std::time::Instant::now();

        // 执行生命周期分析
        let analysis_result = self.lifetime_analyzer.analyze_program(self.program);

        let analysis_time = start_time.elapsed();
        crate::lifetime_debug_println!("生命周期分析完成，耗时: {:?}", analysis_time);
        crate::lifetime_debug_println!("发现 {} 个安全变量", analysis_result.safe_variables.len());
        crate::lifetime_debug_println!("预估性能提升: {:.2}%", analysis_result.estimated_performance_gain * 100.0);

        // 存储分析结果
        self.lifetime_analysis_result = Some(analysis_result);
    }

    /// 检查变量是否可以跳过运行时检查
    pub fn can_skip_runtime_check(&self, var_name: &str) -> bool {
        if let Some(result) = &self.lifetime_analysis_result {
            result.safe_variables.contains(var_name)
        } else {
            false
        }
    }

    /// 获取生命周期分析结果
    pub fn get_lifetime_analysis_result(&self) -> Option<&LifetimeAnalysisResult> {
        self.lifetime_analysis_result.as_ref()
    }
}

// Implement all required traits for Interpreter
impl<'a> Evaluator for Interpreter<'a> {
    fn evaluate_expression(&mut self, expr: &Expression) -> Value {
        ExpressionEvaluator::evaluate_expression(self, expr)
    }
    
    fn perform_binary_operation(&self, left: &Value, op: &BinaryOperator, right: &Value) -> Value {
        ExpressionEvaluator::perform_binary_operation(self, left, op, right)
    }
    
    fn get_variable(&self, name: &str) -> Option<Value> {
        ExpressionEvaluator::get_variable(self, name)
    }
    
    fn call_function(&mut self, function_name: &str, args: Vec<Value>) -> Value {
        StatementExecutor::call_function(self, function_name, args)
    }
}

impl<'a> Executor for Interpreter<'a> {
    fn execute_statement(&mut self, statement: Statement) -> ExecutionResult {
        StatementExecutor::execute_statement(self, statement)
    }
    
    fn execute_function(&mut self, function: &Function) -> Value {
        StatementExecutor::execute_function(self, function)
    }
    
    fn update_variable(&mut self, name: &str, value: Value) -> Result<(), String> {
        StatementExecutor::update_variable(self, name, value)
    }
}

// Add convenience methods to avoid trait method conflicts
impl<'a> Interpreter<'a> {
    pub fn evaluate_expression_direct(&mut self, expr: &Expression) -> Value {
        ExpressionEvaluator::evaluate_expression(self, expr)
    }
    
    pub fn execute_statement_direct(&mut self, statement: Statement) -> ExecutionResult {
        StatementExecutor::execute_statement(self, statement)
    }
    
    pub fn execute_function_direct(&mut self, function: &Function) -> Value {
        StatementExecutor::execute_function(self, function)
    }
} 