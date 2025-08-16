use crate::interpreter::value::Value;
use crate::vm::bytecode::{ByteCode, CompiledProgram, CompiledFunction};
use std::collections::HashMap;

/// 调用栈帧
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// 函数名（用于调试）
    pub function_name: String,
    
    /// 指令指针（当前执行位置）
    pub ip: usize,
    
    /// 局部变量（包括参数）
    pub locals: Vec<Value>,
    
    /// 返回地址
    pub return_ip: usize,
    
    /// 栈基址（用于参数传递）
    pub stack_base: usize,
}

/// 字节码虚拟机
pub struct VM {
    /// 操作栈
    stack: Vec<Value>,
    
    /// 调用栈
    call_frames: Vec<CallFrame>,
    
    /// 全局变量
    globals: HashMap<String, Value>,
    
    /// 当前执行的程序
    program: Option<CompiledProgram>,
    
    /// 当前执行的函数
    current_function: Option<CompiledFunction>,
    
    /// 指令指针
    ip: usize,
    
    /// 是否处于调试模式
    debug_mode: bool,

    /// 是否显示提示信息
    tip_mode: bool,
}

impl VM {
    /// 创建新的虚拟机实例
    pub fn new() -> Self {
        VM {
            stack: Vec::with_capacity(1024), // 预分配栈空间
            call_frames: Vec::with_capacity(256), // 预分配调用栈
            globals: HashMap::new(),
            program: None,
            current_function: None,
            ip: 0,
            debug_mode: false,
            tip_mode: false,
        }
    }
    
    /// 设置调试模式
    pub fn set_debug_mode(&mut self, debug: bool) {
        self.debug_mode = debug;
    }

    /// 设置提示模式
    pub fn set_tip_mode(&mut self, tip: bool) {
        self.tip_mode = tip;
    }
    
    /// 加载编译后的程序
    pub fn load_program(&mut self, program: CompiledProgram) {
        self.program = Some(program);
    }
    
    /// 执行程序
    pub fn run(&mut self) -> Result<Value, String> {
        let program = self.program.as_ref()
            .ok_or("没有加载程序")?;
        
        // 查找主函数
        let main_function = program.functions.get(&program.main_function)
            .ok_or(format!("找不到主函数: {}", program.main_function))?
            .clone();
        
        // 执行主函数
        self.call_function(main_function, Vec::new())
    }
    
    /// 调用函数
    pub fn call_function(&mut self, function: CompiledFunction, args: Vec<Value>) -> Result<Value, String> {
        if args.len() != function.param_count as usize {
            return Err(format!("函数 {} 期望 {} 个参数，但得到 {} 个",
                function.name, function.param_count, args.len()));
        }
        
        // 创建新的调用栈帧
        let mut locals = vec![Value::None; function.local_count as usize];
        
        // 设置参数
        for (i, arg) in args.into_iter().enumerate() {
            locals[i] = arg;
        }
        
        let frame = CallFrame {
            function_name: function.name.clone(),
            ip: 0,
            locals,
            return_ip: self.ip,
            stack_base: self.stack.len(),
        };
        
        self.call_frames.push(frame);
        self.current_function = Some(function);
        self.ip = 0;
        
        // 执行字节码
        self.execute()
    }
    
    /// 执行字节码指令
    fn execute(&mut self) -> Result<Value, String> {
        loop {
            let function = self.current_function.as_ref()
                .ok_or("没有当前函数")?;

            if self.ip >= function.bytecode.len() {
                // 函数结束，返回None
                return Ok(Value::None);
            }

            let instruction = function.bytecode[self.ip].clone();

            if self.debug_mode {
                println!("IP:{:04} | Stack:{:?} | {}",
                    self.ip, self.stack, instruction.to_string());
            }
            
            match &instruction {
                ByteCode::LoadConst(value) => {
                    self.stack.push(value.clone());
                    self.ip += 1;
                },
                
                ByteCode::LoadLocal(index) => {
                    let frame = self.call_frames.last()
                        .ok_or("没有调用栈帧")?;

                    if *index as usize >= frame.locals.len() {
                        return Err(format!("局部变量索引越界: {} (locals: {})", index, frame.locals.len()));
                    }

                    let value = frame.locals[*index as usize].clone();
                    self.stack.push(value);
                    self.ip += 1;
                },
                
                ByteCode::StoreLocal(index) => {
                    let value = self.stack.pop()
                        .ok_or("栈为空，无法存储局部变量")?;
                    
                    let frame = self.call_frames.last_mut()
                        .ok_or("没有调用栈帧")?;
                    
                    if *index as usize >= frame.locals.len() {
                        return Err(format!("局部变量索引越界: {}", index));
                    }
                    
                    frame.locals[*index as usize] = value;
                    self.ip += 1;
                },
                
                ByteCode::Add => {
                    let b = self.stack.pop().ok_or("栈为空，无法执行加法")?;
                    let a = self.stack.pop().ok_or("栈为空，无法执行加法")?;

                    let result = self.add_values(a, b)?;
                    self.stack.push(result);
                    self.ip += 1;
                },

                ByteCode::Mul => {
                    let b = self.stack.pop().ok_or("栈为空，无法执行乘法")?;
                    let a = self.stack.pop().ok_or("栈为空，无法执行乘法")?;

                    let result = self.mul_values(a, b)?;
                    self.stack.push(result);
                    self.ip += 1;
                },

                ByteCode::Div => {
                    let b = self.stack.pop().ok_or("栈为空，无法执行除法")?;
                    let a = self.stack.pop().ok_or("栈为空，无法执行除法")?;

                    let result = self.div_values(a, b)?;
                    self.stack.push(result);
                    self.ip += 1;
                },
                
                ByteCode::Sub => {
                    let b = self.stack.pop().ok_or("栈为空，无法执行减法")?;
                    let a = self.stack.pop().ok_or("栈为空，无法执行减法")?;
                    
                    let result = self.sub_values(a, b)?;
                    self.stack.push(result);
                    self.ip += 1;
                },
                
                ByteCode::LessEqual => {
                    let b = self.stack.pop().ok_or("栈为空，无法执行比较")?;
                    let a = self.stack.pop().ok_or("栈为空，无法执行比较")?;

                    let result = self.less_equal_values(a, b)?;
                    self.stack.push(result);
                    self.ip += 1;
                },

                ByteCode::Greater => {
                    let b = self.stack.pop().ok_or("栈为空，无法执行比较")?;
                    let a = self.stack.pop().ok_or("栈为空，无法执行比较")?;

                    let result = self.greater_values(a, b)?;
                    self.stack.push(result);
                    self.ip += 1;
                },

                ByteCode::GreaterEqual => {
                    let b = self.stack.pop().ok_or("栈为空，无法执行比较")?;
                    let a = self.stack.pop().ok_or("栈为空，无法执行比较")?;

                    let result = self.greater_equal_values(a, b)?;
                    self.stack.push(result);
                    self.ip += 1;
                },

                ByteCode::Equal => {
                    let b = self.stack.pop().ok_or("栈为空，无法执行比较")?;
                    let a = self.stack.pop().ok_or("栈为空，无法执行比较")?;

                    let result = self.equal_values(a, b)?;
                    self.stack.push(result);
                    self.ip += 1;
                },

                ByteCode::NotEqual => {
                    let b = self.stack.pop().ok_or("栈为空，无法执行比较")?;
                    let a = self.stack.pop().ok_or("栈为空，无法执行比较")?;

                    let result = self.not_equal_values(a, b)?;
                    self.stack.push(result);
                    self.ip += 1;
                },

                ByteCode::Less => {
                    let b = self.stack.pop().ok_or("栈为空，无法执行比较")?;
                    let a = self.stack.pop().ok_or("栈为空，无法执行比较")?;

                    let result = self.less_values(a, b)?;
                    self.stack.push(result);
                    self.ip += 1;
                },
                
                ByteCode::JumpIfFalse(addr) => {
                    let condition = self.stack.pop()
                        .ok_or("栈为空，无法执行条件跳转")?;
                    
                    if self.is_falsy(&condition) {
                        self.ip = *addr as usize;
                    } else {
                        self.ip += 1;
                    }
                },
                
                ByteCode::Jump(addr) => {
                    self.ip = *addr as usize;
                },
                
                ByteCode::Call(func_index, argc) => {
                    // 从栈中获取参数
                    let mut args = Vec::new();
                    for _ in 0..*argc {
                        args.push(self.stack.pop().ok_or("栈为空，无法获取函数参数")?);
                    }
                    args.reverse(); // 恢复参数顺序

                    // 根据函数索引查找函数
                    let program = self.program.as_ref().ok_or("没有加载程序")?;

                    // 通过索引查找函数名（使用编译器生成的索引映射）
                    let function_name = program.function_indices.iter()
                        .find(|(_, &index)| index == *func_index)
                        .map(|(name, _)| name.as_str())
                        .ok_or(format!("未知函数索引: {}", func_index))?;

                    if self.tip_mode {
                        println!("🔍 VM: 执行函数调用 {} (索引 {}) 参数数量 {}", function_name, func_index, argc);
                    }

                    let function = program.functions.get(function_name)
                        .ok_or(format!("找不到函数: {}", function_name))?
                        .clone();

                    // 创建新的调用栈帧
                    let mut locals = vec![Value::None; function.local_count as usize];

                    // 设置参数
                    for (i, arg) in args.into_iter().enumerate() {
                        if i < locals.len() {
                            locals[i] = arg;
                        }
                    }

                    let frame = CallFrame {
                        function_name: function.name.clone(),
                        ip: self.ip + 1, // 返回到下一条指令
                        locals: locals.clone(),
                        return_ip: self.ip + 1,
                        stack_base: self.stack.len(),
                    };

                    self.call_frames.push(frame);

                    // 更新当前函数和指令指针
                    self.current_function = Some(function);
                    self.ip = 0;
                },

                ByteCode::CallLibrary(lib_name, func_name, argc) => {
                    // 从栈中获取参数
                    let mut args = Vec::new();
                    for _ in 0..*argc {
                        args.push(self.stack.pop().ok_or("栈为空，无法获取库函数参数")?);
                    }
                    args.reverse(); // 恢复参数顺序

                    // 将参数转换为字符串
                    let string_args: Vec<String> = args.iter()
                        .map(|v| self.value_to_string(v))
                        .collect();

                    if self.debug_mode {
                        println!("🚀 VM: 调用库函数 {}::{} 参数: {:?}", lib_name, func_name, string_args);
                    }

                    // 调用库函数
                    match crate::interpreter::library_loader::call_library_function(lib_name, func_name, string_args) {
                        Ok(result_str) => {
                            // 尝试将结果转换为适当的值类型
                            let result_value = if let Ok(int_val) = result_str.parse::<i32>() {
                                Value::Int(int_val)
                            } else if let Ok(float_val) = result_str.parse::<f64>() {
                                Value::Float(float_val)
                            } else if result_str == "true" {
                                Value::Bool(true)
                            } else if result_str == "false" {
                                Value::Bool(false)
                            } else {
                                Value::String(result_str)
                            };

                            // 将结果压入栈
                            self.stack.push(result_value);
                            self.ip += 1;
                        },
                        Err(err) => {
                            return Err(format!("库函数调用失败: {}", err));
                        }
                    }
                },

                ByteCode::Return => {
                    let return_value = if self.stack.len() > 0 {
                        self.stack.pop().unwrap_or(Value::None)
                    } else {
                        Value::None
                    };

                    if self.tip_mode {
                        println!("🔍 VM: 函数返回，返回值: {:?}", return_value);
                    }

                    // 恢复调用栈
                    if let Some(frame) = self.call_frames.pop() {
                        // 如果还有调用栈，恢复上一个函数
                        if let Some(parent_frame) = self.call_frames.last() {
                            // 查找父函数
                            let program = self.program.as_ref().ok_or("没有加载程序")?;
                            if let Some(parent_function) = program.functions.get(&parent_frame.function_name) {
                                self.current_function = Some(parent_function.clone());
                                self.ip = frame.return_ip;

                                if self.tip_mode {
                                    println!("🔍 VM: 恢复到父函数 {} IP: {}", parent_frame.function_name, frame.return_ip);
                                }

                                // 将返回值压入栈
                                self.stack.push(return_value);
                                continue;
                            }
                        }

                        // 没有父调用栈，程序结束
                        return Ok(return_value);
                    } else {
                        // 没有调用栈，程序结束
                        return Ok(return_value);
                    }
                },
                

                
                ByteCode::Halt => {
                    return Ok(Value::None);
                },

                // === 数组操作指令 ===
                ByteCode::NewArray(size) => {
                    let array = Value::Array(vec![Value::None; *size as usize]);
                    self.stack.push(array);
                    self.ip += 1;
                },

                ByteCode::LoadArrayElement => {
                    let index = self.stack.pop().ok_or("栈为空，无法获取数组索引")?;
                    let array = self.stack.pop().ok_or("栈为空，无法获取数组")?;

                    if let (Value::Array(arr), Value::Int(idx)) = (&array, &index) {
                        if *idx >= 0 && (*idx as usize) < arr.len() {
                            self.stack.push(arr[*idx as usize].clone());
                        } else {
                            return Err("数组索引越界".to_string());
                        }
                    } else {
                        return Err("无效的数组访问操作".to_string());
                    }
                    self.ip += 1;
                },

                ByteCode::StoreArrayElement => {
                    let value = self.stack.pop().ok_or("栈为空，无法获取存储值")?;
                    let index = self.stack.pop().ok_or("栈为空，无法获取数组索引")?;
                    let mut array = self.stack.pop().ok_or("栈为空，无法获取数组")?;

                    if let (Value::Array(ref mut arr), Value::Int(idx)) = (&mut array, &index) {
                        if *idx >= 0 && (*idx as usize) < arr.len() {
                            arr[*idx as usize] = value;
                            // 不推回数组（用于数组元素赋值语句）
                        } else {
                            return Err("数组索引越界".to_string());
                        }
                    } else {
                        return Err("无效的数组存储操作".to_string());
                    }
                    self.ip += 1;
                },

                ByteCode::StoreArrayElementKeep => {
                    let value = self.stack.pop().ok_or("栈为空，无法获取存储值")?;
                    let index = self.stack.pop().ok_or("栈为空，无法获取数组索引")?;
                    let mut array = self.stack.pop().ok_or("栈为空，无法获取数组")?;

                    if let (Value::Array(ref mut arr), Value::Int(idx)) = (&mut array, &index) {
                        if *idx >= 0 && (*idx as usize) < arr.len() {
                            arr[*idx as usize] = value;
                            // 将修改后的数组推回栈（用于数组字面量构造）
                            self.stack.push(array);
                        } else {
                            return Err("数组索引越界".to_string());
                        }
                    } else {
                        return Err("无效的数组存储操作".to_string());
                    }
                    self.ip += 1;
                },

                ByteCode::ArrayLength => {
                    let array = self.stack.pop().ok_or("栈为空，无法获取数组")?;

                    if let Value::Array(arr) = &array {
                        self.stack.push(Value::Int(arr.len() as i32));
                    } else {
                        return Err("无效的数组长度操作".to_string());
                    }
                    self.ip += 1;
                },

                // === 栈操作指令 ===
                ByteCode::Pop => {
                    self.stack.pop().ok_or("栈为空，无法执行Pop操作")?;
                    self.ip += 1;
                },

                ByteCode::Dup => {
                    let value = self.stack.last().ok_or("栈为空，无法执行Dup操作")?.clone();
                    self.stack.push(value);
                    self.ip += 1;
                },

                ByteCode::Swap => {
                    if self.stack.len() < 2 {
                        return Err("栈中元素不足，无法执行Swap操作".to_string());
                    }
                    let len = self.stack.len();
                    self.stack.swap(len - 1, len - 2);
                    self.ip += 1;
                },

                // === 循环控制指令 ===
                ByteCode::LoopStart(_loop_id) => {
                    // 循环开始标记，主要用于调试
                    self.ip += 1;
                },

                ByteCode::LoopEnd(_loop_id) => {
                    // 循环结束标记，主要用于调试
                    self.ip += 1;
                },

                ByteCode::Break(addr) => {
                    // 跳出循环
                    self.ip = *addr as usize;
                },

                ByteCode::Continue(addr) => {
                    // 继续循环
                    self.ip = *addr as usize;
                },

                // === 迭代器指令 ===
                ByteCode::GetIterator => {
                    let collection = self.stack.pop().ok_or("栈为空，无法获取集合")?;

                    // 创建迭代器（简化实现）
                    match collection {
                        Value::Array(arr) => {
                            // 创建数组迭代器：[当前索引, 数组]
                            let iterator = Value::Array(vec![Value::Int(0), Value::Array(arr)]);
                            self.stack.push(iterator);
                        },
                        _ => {
                            return Err("不支持的集合类型".to_string());
                        }
                    }
                    self.ip += 1;
                },

                ByteCode::IteratorHasNext => {
                    let iterator = self.stack.pop().ok_or("栈为空，无法获取迭代器")?;

                    if let Value::Array(iter_data) = &iterator {
                        if iter_data.len() == 2 {
                            if let (Value::Int(index), Value::Array(arr)) = (&iter_data[0], &iter_data[1]) {
                                let has_next = (*index as usize) < arr.len();
                                self.stack.push(Value::Bool(has_next));
                                self.stack.push(iterator); // 将迭代器推回栈
                            } else {
                                return Err("无效的迭代器格式".to_string());
                            }
                        } else {
                            return Err("无效的迭代器数据".to_string());
                        }
                    } else {
                        return Err("无效的迭代器类型".to_string());
                    }
                    self.ip += 1;
                },

                ByteCode::IteratorNext => {
                    let mut iterator = self.stack.pop().ok_or("栈为空，无法获取迭代器")?;

                    if let Value::Array(ref mut iter_data) = iterator {
                        if iter_data.len() == 2 {
                            // 分别获取索引和数组的引用，避免借用冲突
                            let current_index = if let Value::Int(index) = &iter_data[0] {
                                *index
                            } else {
                                return Err("无效的迭代器索引格式".to_string());
                            };

                            let array_data = if let Value::Array(arr) = &iter_data[1] {
                                arr.clone()
                            } else {
                                return Err("无效的迭代器数组格式".to_string());
                            };

                            if (current_index as usize) < array_data.len() {
                                let value = array_data[current_index as usize].clone();
                                // 更新索引
                                iter_data[0] = Value::Int(current_index + 1);
                                self.stack.push(value);
                                self.stack.push(iterator); // 将更新后的迭代器推回栈
                            } else {
                                return Err("迭代器已到达末尾".to_string());
                            }
                        } else {
                            return Err("无效的迭代器数据".to_string());
                        }
                    } else {
                        return Err("无效的迭代器类型".to_string());
                    }
                    self.ip += 1;
                },

                // === 对象操作指令 ===
                ByteCode::NewObject(class_name) => {
                    // 创建新对象实例
                    let object = Value::Object(crate::interpreter::value::ObjectInstance {
                        class_name: class_name.clone(),
                        fields: std::collections::HashMap::new(),
                    });
                    self.stack.push(object);
                    self.ip += 1;
                },

                ByteCode::LoadField(field_name) => {
                    let object = self.stack.pop().ok_or("栈为空，无法获取对象")?;

                    if let Value::Object(obj) = &object {
                        if let Some(field_value) = obj.fields.get(field_name) {
                            self.stack.push(field_value.clone());
                        } else {
                            self.stack.push(Value::None); // 字段不存在时返回None
                        }
                    } else {
                        return Err("无效的对象字段访问".to_string());
                    }
                    self.ip += 1;
                },

                ByteCode::StoreField(field_name) => {
                    let value = self.stack.pop().ok_or("栈为空，无法获取字段值")?;
                    let mut object = self.stack.pop().ok_or("栈为空，无法获取对象")?;

                    if let Value::Object(ref mut obj) = object {
                        obj.fields.insert(field_name.clone(), value);
                        // 注意：这里不需要将对象推回栈，因为StoreField是消费性操作
                    } else {
                        return Err("无效的对象字段存储".to_string());
                    }
                    self.ip += 1;
                },

                ByteCode::CallMethod(method_name, argc) => {
                    // 从栈中获取参数
                    let mut args = Vec::new();
                    for _ in 0..*argc {
                        args.push(self.stack.pop().ok_or("栈为空，无法获取方法参数")?);
                    }
                    args.reverse(); // 恢复参数顺序

                    let object = self.stack.pop().ok_or("栈为空，无法获取对象")?;

                    if let Value::Object(obj) = &object {
                        // 构建方法的完整名称
                        let full_method_name = format!("{}::{}", obj.class_name, method_name);

                        // 查找方法
                        let program = self.program.as_ref().ok_or("没有加载程序")?;
                        if let Some(method_function) = program.functions.get(&full_method_name) {
                            // 将对象作为第一个参数（this）
                            let mut method_args = vec![object];
                            method_args.extend(args);

                            // 调用方法
                            let result = self.call_method_function(method_function.clone(), method_args)?;
                            self.stack.push(result);
                        } else {
                            return Err(format!("找不到方法: {}", full_method_name));
                        }
                    } else {
                        return Err("无效的方法调用".to_string());
                    }
                    self.ip += 1;
                },

                _ => {
                    return Err(format!("未实现的指令: {:?}", instruction));
                }
            }
        }
    }

    /// 值相加
    fn add_values(&self, a: Value, b: Value) -> Result<Value, String> {
        match (&a, &b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(a.clone() + b)),
            // 字符串和其他类型的拼接
            (Value::String(a), b) => Ok(Value::String(a.clone() + &self.value_to_string(b))),
            (a, Value::String(b)) => Ok(Value::String(self.value_to_string(a) + b)),
            _ => Err(format!("无法对 {:?} 和 {:?} 执行加法", a, b)),
        }
    }

    /// 值相减
    fn sub_values(&self, a: Value, b: Value) -> Result<Value, String> {
        match (&a, &b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
            _ => Err(format!("无法对 {:?} 和 {:?} 执行减法", a, b)),
        }
    }

    /// 值相乘
    fn mul_values(&self, a: Value, b: Value) -> Result<Value, String> {
        match (&a, &b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
            _ => Err(format!("无法对 {:?} 和 {:?} 执行乘法", a, b)),
        }
    }

    /// 值相除
    fn div_values(&self, a: Value, b: Value) -> Result<Value, String> {
        match (&a, &b) {
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    Err("除零错误".to_string())
                } else {
                    Ok(Value::Int(a / b))
                }
            },
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err("除零错误".to_string())
                } else {
                    Ok(Value::Float(a / b))
                }
            },
            (Value::Int(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err("除零错误".to_string())
                } else {
                    Ok(Value::Float(*a as f64 / b))
                }
            },
            (Value::Float(a), Value::Int(b)) => {
                if *b == 0 {
                    Err("除零错误".to_string())
                } else {
                    Ok(Value::Float(a / *b as f64))
                }
            },
            _ => Err(format!("无法对 {:?} 和 {:?} 执行除法", a, b)),
        }
    }

    /// 小于等于比较
    fn less_equal_values(&self, a: Value, b: Value) -> Result<Value, String> {
        match (&a, &b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a <= b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a <= b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) <= *b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a <= (*b as f64))),
            _ => Err(format!("无法对 {:?} 和 {:?} 执行比较", a, b)),
        }
    }

    /// 大于比较
    fn greater_values(&self, a: Value, b: Value) -> Result<Value, String> {
        match (&a, &b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a > b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a > b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) > *b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a > (*b as f64))),
            _ => Err(format!("无法对 {:?} 和 {:?} 执行比较", a, b)),
        }
    }

    /// 大于等于比较
    fn greater_equal_values(&self, a: Value, b: Value) -> Result<Value, String> {
        match (&a, &b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a >= b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a >= b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) >= *b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a >= (*b as f64))),
            _ => Err(format!("无法对 {:?} 和 {:?} 执行比较", a, b)),
        }
    }

    /// 等于比较
    fn equal_values(&self, a: Value, b: Value) -> Result<Value, String> {
        match (&a, &b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a == b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a == b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) == *b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a == (*b as f64))),
            (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a == b)),
            (Value::String(a), Value::String(b)) => Ok(Value::Bool(a == b)),
            _ => Ok(Value::Bool(false)), // 不同类型默认不相等
        }
    }

    /// 不等于比较
    fn not_equal_values(&self, a: Value, b: Value) -> Result<Value, String> {
        let equal_result = self.equal_values(a, b)?;
        match equal_result {
            Value::Bool(b) => Ok(Value::Bool(!b)),
            _ => Err("等于比较返回了非布尔值".to_string()),
        }
    }

    /// 小于比较
    fn less_values(&self, a: Value, b: Value) -> Result<Value, String> {
        match (&a, &b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a < b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) < *b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a < (*b as f64))),
            _ => Err(format!("无法对 {:?} 和 {:?} 执行比较", a, b)),
        }
    }

    /// 检查值是否为假
    fn is_falsy(&self, value: &Value) -> bool {
        match value {
            Value::Bool(b) => !b,
            Value::Int(i) => *i == 0,
            Value::Float(f) => *f == 0.0,
            Value::None => true,
            _ => false,
        }
    }

    /// 将值转换为字符串
    fn value_to_string(&self, value: &Value) -> String {
        match value {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::None => "None".to_string(),
            _ => format!("{:?}", value),
        }
    }

    /// 获取栈的当前状态（调试用）
    pub fn get_stack_info(&self) -> String {
        format!("Stack size: {}, Frames: {}",
            self.stack.len(), self.call_frames.len())
    }

    /// 调用方法函数（类似call_function但用于方法调用）
    fn call_method_function(&mut self, function: CompiledFunction, args: Vec<Value>) -> Result<Value, String> {
        if args.len() != function.param_count as usize {
            return Err(format!("方法 {} 期望 {} 个参数，但得到 {} 个",
                function.name, function.param_count, args.len()));
        }

        // 保存当前状态
        let saved_ip = self.ip;
        let saved_function = self.current_function.clone();

        // 创建新的调用栈帧
        let mut locals = vec![Value::None; function.local_count as usize];

        // 设置参数
        for (i, arg) in args.into_iter().enumerate() {
            locals[i] = arg;
        }

        let frame = CallFrame {
            function_name: function.name.clone(),
            ip: saved_ip,
            locals,
            return_ip: saved_ip,
            stack_base: self.stack.len(),
        };

        self.call_frames.push(frame);
        self.current_function = Some(function);
        self.ip = 0;

        // 执行方法
        let result = self.execute()?;

        // 恢复状态
        self.ip = saved_ip;
        self.current_function = saved_function;
        self.call_frames.pop();

        Ok(result)
    }
}
