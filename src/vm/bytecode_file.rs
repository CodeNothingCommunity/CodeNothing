use crate::vm::bytecode::{ByteCode, CompiledProgram, CompiledFunction, SerializableValue};
use crate::interpreter::value::Value;
use crate::interpreter::library_loader::LibraryFunction;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Write, Read};
use serde::{Serialize, Deserialize};
use std::sync::Arc;

// SerializableValue 现在在 bytecode.rs 中定义

/// 简化的字节码指令，用于序列化
/// 只包含核心指令，与 ByteCode 保持同步
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializableBytecode {
    LoadConst(SerializableValue),
    LoadLocal(u16),
    StoreLocal(u16),
    LoadGlobal(String),
    StoreGlobal(String),
    Add,
    Sub,
    Mul,
    Div,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    Jump(u32),
    JumpIfFalse(u32),
    JumpIfTrue(u32),
    Call(u16, u8),
    CallLibrary(String, String, u8),
    Return,
    NewArray(u32),
    LoadArrayElement,
    StoreArrayElement,
    Pop,
    Dup,
    Print,
}

impl From<&ByteCode> for SerializableBytecode {
    fn from(bytecode: &ByteCode) -> Self {
        match bytecode {
            ByteCode::LoadConst(v) => SerializableBytecode::LoadConst(v.into()),
            ByteCode::LoadLocal(i) => SerializableBytecode::LoadLocal(*i),
            ByteCode::StoreLocal(i) => SerializableBytecode::StoreLocal(*i),
            ByteCode::LoadGlobal(s) => SerializableBytecode::LoadGlobal(s.clone()),
            ByteCode::StoreGlobal(s) => SerializableBytecode::StoreGlobal(s.clone()),
            ByteCode::Add => SerializableBytecode::Add,
            ByteCode::Sub => SerializableBytecode::Sub,
            ByteCode::Mul => SerializableBytecode::Mul,
            ByteCode::Div => SerializableBytecode::Div,
            ByteCode::Equal => SerializableBytecode::Equal,
            ByteCode::NotEqual => SerializableBytecode::NotEqual,
            ByteCode::Less => SerializableBytecode::Less,
            ByteCode::LessEqual => SerializableBytecode::LessEqual,
            ByteCode::Greater => SerializableBytecode::Greater,
            ByteCode::GreaterEqual => SerializableBytecode::GreaterEqual,

            ByteCode::Jump(addr) => SerializableBytecode::Jump(*addr),
            ByteCode::JumpIfFalse(addr) => SerializableBytecode::JumpIfFalse(*addr),
            ByteCode::JumpIfTrue(addr) => SerializableBytecode::JumpIfTrue(*addr),
            ByteCode::Call(idx, argc) => SerializableBytecode::Call(*idx, *argc),
            ByteCode::CallLibrary(lib, func, argc) => SerializableBytecode::CallLibrary(lib.clone(), func.clone(), *argc),
            ByteCode::Return => SerializableBytecode::Return,
            ByteCode::NewArray(size) => SerializableBytecode::NewArray(*size),
            ByteCode::LoadArrayElement => SerializableBytecode::LoadArrayElement,
            ByteCode::StoreArrayElement => SerializableBytecode::StoreArrayElement,
            ByteCode::Pop => SerializableBytecode::Pop,
            ByteCode::Dup => SerializableBytecode::Dup,
            ByteCode::Print => SerializableBytecode::Print,
            // 对于不支持的字节码，使用 Pop 作为占位符
            _ => SerializableBytecode::Pop,
        }
    }
}

impl Into<ByteCode> for SerializableBytecode {
    fn into(self) -> ByteCode {
        match self {
            SerializableBytecode::LoadConst(v) => ByteCode::LoadConst(v.into()),
            SerializableBytecode::LoadLocal(i) => ByteCode::LoadLocal(i),
            SerializableBytecode::StoreLocal(i) => ByteCode::StoreLocal(i),
            SerializableBytecode::LoadGlobal(s) => ByteCode::LoadGlobal(s),
            SerializableBytecode::StoreGlobal(s) => ByteCode::StoreGlobal(s),
            SerializableBytecode::Add => ByteCode::Add,
            SerializableBytecode::Sub => ByteCode::Sub,
            SerializableBytecode::Mul => ByteCode::Mul,
            SerializableBytecode::Div => ByteCode::Div,
            SerializableBytecode::Equal => ByteCode::Equal,
            SerializableBytecode::NotEqual => ByteCode::NotEqual,
            SerializableBytecode::Less => ByteCode::Less,
            SerializableBytecode::LessEqual => ByteCode::LessEqual,
            SerializableBytecode::Greater => ByteCode::Greater,
            SerializableBytecode::GreaterEqual => ByteCode::GreaterEqual,

            SerializableBytecode::Jump(addr) => ByteCode::Jump(addr),
            SerializableBytecode::JumpIfFalse(addr) => ByteCode::JumpIfFalse(addr),
            SerializableBytecode::JumpIfTrue(addr) => ByteCode::JumpIfTrue(addr),
            SerializableBytecode::Call(idx, argc) => ByteCode::Call(idx, argc),
            SerializableBytecode::CallLibrary(lib, func, argc) => ByteCode::CallLibrary(lib, func, argc),
            SerializableBytecode::Return => ByteCode::Return,
            SerializableBytecode::NewArray(size) => ByteCode::NewArray(size),
            SerializableBytecode::LoadArrayElement => ByteCode::LoadArrayElement,
            SerializableBytecode::StoreArrayElement => ByteCode::StoreArrayElement,
            SerializableBytecode::Pop => ByteCode::Pop,
            SerializableBytecode::Dup => ByteCode::Dup,
            SerializableBytecode::Print => ByteCode::Print,
        }
    }
}

/// 可序列化的编译函数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableCompiledFunction {
    pub name: String,
    pub param_count: u8,
    pub local_count: u16,
    pub bytecode: Vec<SerializableBytecode>,
    pub constants: Vec<SerializableValue>,
}

impl From<&CompiledFunction> for SerializableCompiledFunction {
    fn from(func: &CompiledFunction) -> Self {
        SerializableCompiledFunction {
            name: func.name.clone(),
            param_count: func.param_count,
            local_count: func.local_count,
            bytecode: func.bytecode.iter().map(|b| b.into()).collect(),
            constants: func.constants.iter().map(|v| v.into()).collect(),
        }
    }
}

impl Into<CompiledFunction> for SerializableCompiledFunction {
    fn into(self) -> CompiledFunction {
        CompiledFunction {
            name: self.name,
            param_count: self.param_count,
            local_count: self.local_count,
            bytecode: self.bytecode.into_iter().map(|b| b.into()).collect(),
            constants: self.constants.into_iter().map(|v| v.into()).collect(),
        }
    }
}

/// 库依赖信息 - 类似Java字节码中的import信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDependency {
    pub name: String,
    pub functions: Vec<String>, // 程序使用的函数列表
    pub version: Option<String>, // 库版本要求（可选）
}

/// .comcn文件格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BytecodeFile {
    /// 文件格式版本
    pub version: String,
    
    /// 编译时间戳
    pub timestamp: u64,
    
    /// 源文件路径
    pub source_path: String,
    
    /// 编译后的函数
    pub functions: HashMap<String, SerializableCompiledFunction>,

    /// 全局常量
    pub global_constants: Vec<SerializableValue>,
    
    /// 库依赖信息 - 运行时需要的库
    pub library_dependencies: Vec<LibraryDependency>,

    /// 元数据
    pub metadata: HashMap<String, String>,
}

impl BytecodeFile {
    /// 创建新的字节码文件
    pub fn new(source_path: String) -> Self {
        BytecodeFile {
            version: "1.0".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            source_path,
            functions: HashMap::new(),
            global_constants: Vec::new(),
            library_dependencies: Vec::new(),
            metadata: HashMap::new(),
        }
    }
    
    /// 保存到文件
    pub fn save_to_file(&self, path: &str) -> Result<(), String> {
        let serialized = bincode::serialize(self)
            .map_err(|e| format!("序列化失败: {}", e))?;
        
        let mut file = File::create(path)
            .map_err(|e| format!("创建文件失败: {}", e))?;
        
        file.write_all(&serialized)
            .map_err(|e| format!("写入文件失败: {}", e))?;
        
        Ok(())
    }
    
    /// 从文件加载
    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let mut file = File::open(path)
            .map_err(|e| format!("打开文件失败: {}", e))?;
        
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("读取文件失败: {}", e))?;
        
        let bytecode_file: BytecodeFile = bincode::deserialize(&buffer)
            .map_err(|e| format!("反序列化失败: {}", e))?;
        
        Ok(bytecode_file)
    }
}
