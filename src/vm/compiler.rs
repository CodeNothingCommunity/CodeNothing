use crate::ast::{Program, Function, Statement, Expression, BinaryOperator, CompareOperator, NamespaceType};
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
        }
    }
    
    /// 编译整个程序
    pub fn compile_program(&mut self, program: &Program) -> Result<CompiledProgram, String> {
        let mut compiled_functions = HashMap::new();
        let mut classes = HashMap::new();

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
                    // 代码命名空间暂时跳过
                    println!("🚀 VM: 跳过代码命名空间 {:?}", path);
                }
            }
        }

        // 为所有函数分配索引
        self.assign_function_indices(program);

        // 编译所有函数
        for function in &program.functions {
            let compiled_func = self.compile_function(function)?;
            compiled_functions.insert(function.name.clone(), compiled_func);
        }

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
                // 表达式语句的结果不需要保留在栈上
                // TODO: 添加Pop指令来清理栈
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

                // 生成函数调用指令
                // 暂时使用函数名的哈希作为索引，实际应该有函数索引表
                let func_index = self.get_function_index(name);
                self.emit(ByteCode::Call(func_index, args.len() as u8));
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
                // 处理静态方法调用，特别是std::println
                if class_name == "std" && method_name == "println" {
                    // 编译参数
                    for arg in args {
                        self.compile_expression(arg)?;
                    }

                    // 生成打印指令
                    self.emit(ByteCode::Print);
                } else {
                    // 检查是否是库函数调用
                    let full_func_name = format!("{}::{}", class_name, method_name);
                    let mut found_lib_name = None;

                    // 先查找库函数
                    for (lib_name, lib_functions) in &self.imported_libraries {
                        if lib_functions.contains_key(&full_func_name) {
                            found_lib_name = Some(lib_name.clone());
                            break;
                        }
                    }

                    if let Some(lib_name) = found_lib_name {
                        // 编译参数
                        for arg in args {
                            self.compile_expression(arg)?;
                        }

                        // 生成库函数调用指令
                        self.emit(ByteCode::CallLibrary(lib_name, full_func_name, args.len() as u8));
                    } else {
                        return Err(format!("暂不支持静态方法调用: {}::{}", class_name, method_name));
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

    /// 为所有函数分配索引
    fn assign_function_indices(&mut self, program: &Program) {
        self.function_indices.insert("main".to_string(), 0);
        self.next_function_index = 1;

        for function in &program.functions {
            if function.name != "main" {
                self.function_indices.insert(function.name.clone(), self.next_function_index);
                self.next_function_index += 1;
            }
        }
    }

    /// 获取函数索引
    fn get_function_index(&self, name: &str) -> u16 {
        self.function_indices.get(name).copied().unwrap_or(999)
    }
}
