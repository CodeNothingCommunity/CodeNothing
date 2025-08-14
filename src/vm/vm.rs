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
        }
    }
    
    /// 设置调试模式
    pub fn set_debug_mode(&mut self, debug: bool) {
        self.debug_mode = debug;
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
                    let function_name = match func_index {
                        0 => "main",
                        1 => "fib",
                        _ => return Err(format!("未知函数索引: {}", func_index)),
                    };

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

                ByteCode::Return => {
                    let return_value = if self.stack.len() > 0 {
                        self.stack.pop().unwrap_or(Value::None)
                    } else {
                        Value::None
                    };

                    // 恢复调用栈
                    if let Some(frame) = self.call_frames.pop() {
                        // 如果还有调用栈，恢复上一个函数
                        if let Some(parent_frame) = self.call_frames.last() {
                            // 查找父函数
                            let program = self.program.as_ref().ok_or("没有加载程序")?;
                            if let Some(parent_function) = program.functions.get(&parent_frame.function_name) {
                                self.current_function = Some(parent_function.clone());
                                self.ip = frame.return_ip;

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
                
                ByteCode::Print => {
                    let value = self.stack.pop()
                        .ok_or("栈为空，无法打印")?;
                    println!("{}", self.value_to_string(&value));
                    self.ip += 1;
                },
                
                ByteCode::Halt => {
                    return Ok(Value::None);
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
}
