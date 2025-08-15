use crate::interpreter::value::Value;
use std::collections::HashMap;

/// 字节码指令集 - 最小可行版本
/// 专注于解决递归函数性能问题
#[derive(Debug, Clone, PartialEq)]
pub enum ByteCode {
    // === 栈操作指令 ===
    /// 加载常量到栈顶
    LoadConst(Value),
    
    /// 加载局部变量到栈顶 (变量索引)
    LoadLocal(u16),
    
    /// 从栈顶存储到局部变量 (变量索引)
    StoreLocal(u16),
    
    /// 加载全局变量到栈顶
    LoadGlobal(String),
    
    /// 从栈顶存储到全局变量
    StoreGlobal(String),
    
    // === 算术运算指令 ===
    /// 加法：pop b, pop a, push (a + b)
    Add,
    
    /// 减法：pop b, pop a, push (a - b)
    Sub,
    
    /// 乘法：pop b, pop a, push (a * b)
    Mul,
    
    /// 除法：pop b, pop a, push (a / b)
    Div,
    
    // === 比较运算指令 ===
    /// 等于：pop b, pop a, push (a == b)
    Equal,
    
    /// 不等于：pop b, pop a, push (a != b)
    NotEqual,
    
    /// 小于：pop b, pop a, push (a < b)
    Less,
    
    /// 小于等于：pop b, pop a, push (a <= b)
    LessEqual,
    
    /// 大于：pop b, pop a, push (a > b)
    Greater,
    
    /// 大于等于：pop b, pop a, push (a >= b)
    GreaterEqual,
    
    // === 控制流指令 ===
    /// 无条件跳转到指定地址
    Jump(u32),
    
    /// 如果栈顶为false则跳转
    JumpIfFalse(u32),
    
    /// 如果栈顶为true则跳转
    JumpIfTrue(u32),
    
    /// 函数调用 (函数索引, 参数数量)
    Call(u16, u8),

    /// 库函数调用 (库名, 函数名, 参数数量)
    CallLibrary(String, String, u8),

    /// 函数返回（返回值在栈顶）
    Return,
    
    // === 对象操作指令（基础版本）===
    /// 创建新对象 (类名)
    NewObject(String),
    
    /// 加载对象字段 (字段名)
    LoadField(String),
    
    /// 存储对象字段 (字段名)
    StoreField(String),
    
    // === 调试和工具指令 ===
    /// 打印栈顶值（调试用）
    Print,
    
    /// 程序结束
    Halt,
}

/// 编译后的函数信息
#[derive(Debug, Clone)]
pub struct CompiledFunction {
    /// 函数名
    pub name: String,
    
    /// 参数数量
    pub param_count: u8,
    
    /// 局部变量数量（包括参数）
    pub local_count: u16,
    
    /// 字节码指令序列
    pub bytecode: Vec<ByteCode>,
    
    /// 常量池
    pub constants: Vec<Value>,
}

/// 编译后的程序
#[derive(Debug, Clone)]
pub struct CompiledProgram {
    /// 所有函数
    pub functions: HashMap<String, CompiledFunction>,

    /// 主函数入口
    pub main_function: String,

    /// 全局常量
    pub global_constants: Vec<Value>,

    /// 类定义信息
    pub classes: HashMap<String, ClassInfo>,

    /// 导入的库映射
    pub imported_libraries: HashMap<String, std::sync::Arc<HashMap<String, crate::interpreter::library_loader::LibraryFunction>>>,

    /// 函数索引映射 (函数名 -> 索引)
    pub function_indices: HashMap<String, u16>,
}

/// 类信息
#[derive(Debug, Clone)]
pub struct ClassInfo {
    /// 类名
    pub name: String,

    /// 字段列表
    pub fields: Vec<String>,

    /// 方法列表
    pub methods: HashMap<String, String>, // 方法名 -> 函数名
}

impl ByteCode {
    /// 获取指令的操作码（用于序列化）
    pub fn opcode(&self) -> u8 {
        match self {
            ByteCode::LoadConst(_) => 0x01,
            ByteCode::LoadLocal(_) => 0x02,
            ByteCode::StoreLocal(_) => 0x03,
            ByteCode::LoadGlobal(_) => 0x04,
            ByteCode::StoreGlobal(_) => 0x05,
            ByteCode::Add => 0x10,
            ByteCode::Sub => 0x11,
            ByteCode::Mul => 0x12,
            ByteCode::Div => 0x13,
            ByteCode::Equal => 0x20,
            ByteCode::NotEqual => 0x21,
            ByteCode::Less => 0x22,
            ByteCode::LessEqual => 0x23,
            ByteCode::Greater => 0x24,
            ByteCode::GreaterEqual => 0x25,
            ByteCode::Jump(_) => 0x30,
            ByteCode::JumpIfFalse(_) => 0x31,
            ByteCode::JumpIfTrue(_) => 0x32,
            ByteCode::Call(_, _) => 0x40,
            ByteCode::CallLibrary(_, _, _) => 0x41,
            ByteCode::Return => 0x42,
            ByteCode::NewObject(_) => 0x50,
            ByteCode::LoadField(_) => 0x51,
            ByteCode::StoreField(_) => 0x52,
            ByteCode::Print => 0xF0,
            ByteCode::Halt => 0xFF,
        }
    }

    /// 检查指令是否需要操作数
    pub fn has_operand(&self) -> bool {
        matches!(self,
            ByteCode::LoadConst(_) |
            ByteCode::LoadLocal(_) |
            ByteCode::StoreLocal(_) |
            ByteCode::LoadGlobal(_) |
            ByteCode::StoreGlobal(_) |
            ByteCode::Jump(_) |
            ByteCode::JumpIfFalse(_) |
            ByteCode::JumpIfTrue(_) |
            ByteCode::Call(_, _) |
            ByteCode::CallLibrary(_, _, _) |
            ByteCode::NewObject(_) |
            ByteCode::LoadField(_) |
            ByteCode::StoreField(_)
        )
    }

    /// 获取指令的字符串表示（用于调试）
    pub fn to_string(&self) -> String {
        match self {
            ByteCode::LoadConst(val) => format!("LoadConst({:?})", val),
            ByteCode::LoadLocal(idx) => format!("LoadLocal({})", idx),
            ByteCode::StoreLocal(idx) => format!("StoreLocal({})", idx),
            ByteCode::LoadGlobal(name) => format!("LoadGlobal({})", name),
            ByteCode::StoreGlobal(name) => format!("StoreGlobal({})", name),
            ByteCode::Add => "Add".to_string(),
            ByteCode::Sub => "Sub".to_string(),
            ByteCode::Mul => "Mul".to_string(),
            ByteCode::Div => "Div".to_string(),
            ByteCode::Equal => "Equal".to_string(),
            ByteCode::NotEqual => "NotEqual".to_string(),
            ByteCode::Less => "Less".to_string(),
            ByteCode::LessEqual => "LessEqual".to_string(),
            ByteCode::Greater => "Greater".to_string(),
            ByteCode::GreaterEqual => "GreaterEqual".to_string(),
            ByteCode::Jump(addr) => format!("Jump({})", addr),
            ByteCode::JumpIfFalse(addr) => format!("JumpIfFalse({})", addr),
            ByteCode::JumpIfTrue(addr) => format!("JumpIfTrue({})", addr),
            ByteCode::Call(func_idx, argc) => format!("Call({}, {})", func_idx, argc),
            ByteCode::CallLibrary(lib_name, func_name, argc) => format!("CallLibrary({}, {}, {})", lib_name, func_name, argc),
            ByteCode::Return => "Return".to_string(),
            ByteCode::NewObject(class) => format!("NewObject({})", class),
            ByteCode::LoadField(field) => format!("LoadField({})", field),
            ByteCode::StoreField(field) => format!("StoreField({})", field),
            ByteCode::Print => "Print".to_string(),
            ByteCode::Halt => "Halt".to_string(),
        }
    }
}
