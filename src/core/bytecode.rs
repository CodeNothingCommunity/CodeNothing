//! Bytecode instruction set and operations for CodeNothing VM
//!
//! This module defines the complete instruction set for the stack-based virtual machine,
//! including arithmetic operations, control flow, memory operations, and function calls.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Bytecode instruction opcodes
///
/// Each instruction is designed to be compact and efficient for stack-based execution.
/// Instructions are encoded as a single byte opcode followed by optional operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum OpCode {
    // === Stack Operations ===
    /// Push a constant value onto the stack
    /// Operand: constant_index (u16)
    Const = 0x00,

    /// Push null value onto the stack
    Null = 0x01,

    /// Push true boolean value onto the stack
    True = 0x02,

    /// Push false boolean value onto the stack
    False = 0x03,

    /// Duplicate the top stack value
    Dup = 0x04,

    /// Pop the top stack value
    Pop = 0x05,

    /// Swap the top two stack values
    Swap = 0x06,

    // === Arithmetic Operations ===
    /// Add two values: b + a (where a is top of stack)
    Add = 0x10,

    /// Subtract two values: b - a
    Sub = 0x11,

    /// Multiply two values: b * a
    Mul = 0x12,

    /// Divide two values: b / a
    Div = 0x13,

    /// Modulo operation: b % a
    Mod = 0x14,

    /// Negate the top stack value: -a
    Neg = 0x15,

    // === Comparison Operations ===
    /// Equal comparison: b == a
    Eq = 0x20,

    /// Not equal comparison: b != a
    Ne = 0x21,

    /// Less than comparison: b < a
    Lt = 0x22,

    /// Less than or equal: b <= a
    Le = 0x23,

    /// Greater than comparison: b > a
    Gt = 0x24,

    /// Greater than or equal: b >= a
    Ge = 0x25,

    // === Logical Operations ===
    /// Logical AND: b && a
    And = 0x30,

    /// Logical OR: b || a
    Or = 0x31,

    /// Logical NOT: !a
    Not = 0x32,

    // === Bitwise Operations ===
    /// Bitwise AND: b & a
    BitAnd = 0x40,

    /// Bitwise OR: b | a
    BitOr = 0x41,

    /// Bitwise XOR: b ^ a
    BitXor = 0x42,

    /// Bitwise NOT: ~a
    BitNot = 0x43,

    /// Left shift: b << a
    Shl = 0x44,

    /// Right shift: b >> a
    Shr = 0x45,

    // === Memory Operations ===
    /// Load local variable onto stack
    /// Operand: local_index (u8)
    LoadLocal = 0x50,

    /// Store top stack value to local variable
    /// Operand: local_index (u8)
    StoreLocal = 0x51,

    /// Load global variable onto stack
    /// Operand: global_index (u16)
    LoadGlobal = 0x52,

    /// Store top stack value to global variable
    /// Operand: global_index (u16)
    StoreGlobal = 0x53,

    /// Load field from object
    /// Operand: field_index (u16)
    LoadField = 0x54,

    /// Store value to object field
    /// Operand: field_index (u16)
    StoreField = 0x55,

    /// Load array element: array[index]
    LoadIndex = 0x56,

    /// Store value to array element: array[index] = value
    StoreIndex = 0x57,

    // === Pointer Operations ===
    /// Take address of a variable (create reference)
    /// Operand: variable_index (u16), is_mutable (u8)
    /// Stack: [] -> [reference]
    AddressOf = 0x58,

    /// Dereference a pointer/reference
    /// Operand: none
    /// Stack: [pointer/reference] -> [value]
    Dereference = 0x59,

    /// Store value through pointer/reference
    /// Operand: none
    /// Stack: [pointer/reference, value] -> []
    StoreThrough = 0x5A,

    /// Create raw pointer from address
    /// Operand: is_mutable (u8)
    /// Stack: [address] -> [raw_pointer]
    CreateRawPointer = 0x5B,

    /// Get address from pointer/reference
    /// Operand: none
    /// Stack: [pointer/reference] -> [address]
    GetAddress = 0x5C,

    // === Control Flow Operations ===
    /// Unconditional jump
    /// Operand: offset (i16)
    Jump = 0x60,

    /// Jump if top stack value is false
    /// Operand: offset (i16)
    JumpIfFalse = 0x61,

    /// Jump if top stack value is true
    /// Operand: offset (i16)
    JumpIfTrue = 0x62,

    /// Jump if top stack value is null
    /// Operand: offset (i16)
    JumpIfNull = 0x63,

    /// Jump if top stack value is not null
    /// Operand: offset (i16)
    JumpIfNotNull = 0x64,

    // === Function Operations ===
    /// Call function
    /// Operand: function_index (u16), arg_count (u8)
    Call = 0x70,

    /// Call method on object
    /// Operand: method_index (u16), arg_count (u8)
    CallMethod = 0x71,

    /// Call super method (parent class method)
    /// Operand: method_index (u16), arg_count (u8)
    CallSuper = 0x74,

    /// Call virtual method (with dynamic dispatch)
    /// Operand: method_index (u16), arg_count (u8)
    CallVirtual = 0x75,

    /// Call static method
    /// Operand: method_index (u16), arg_count (u8)
    CallStatic = 0x76,

    /// Call function through function pointer
    /// Stack: [function_pointer, arg1, arg2, ..., argN, param_count]
    CallIndirect = 0x77,

    /// Return from function
    Return = 0x72,

    /// Return value from function
    ReturnValue = 0x73,

    // === Object Operations ===
    /// Create new object
    /// Operand: class_index (u16)
    NewObject = 0x80,

    /// Create new array
    /// Operand: size (u16)
    NewArray = 0x81,

    /// Create array from stack elements
    /// Operand: element_count (u16)
    CreateArray = 0x82,

    /// Create object from stack key-value pairs
    /// Operand: field_count (u16)
    CreateObject = 0x83,

    /// Get array length
    ArrayLength = 0x84,

    /// Type check
    /// Operand: type_index (u16)
    IsInstance = 0x85,

    /// Cast to type
    /// Operand: type_index (u16)
    Cast = 0x86,

    /// Load field from super class
    /// Operand: field_index (u16)
    SuperFieldLoad = 0x87,

    /// Store value to super class field
    /// Operand: field_index (u16)
    SuperFieldStore = 0x88,

    // === Exception Handling ===
    /// Throw exception
    Throw = 0x90,

    /// Try block start
    /// Operand: catch_offset (u16)
    TryStart = 0x91,

    /// Try block end
    TryEnd = 0x92,

    /// Catch block
    /// Operand: exception_type (u16)
    Catch = 0x93,

    /// Finally block
    Finally = 0x94,

    // === Special Operations ===
    /// No operation
    Nop = 0xF0,

    /// Halt execution
    Halt = 0xF1,

    /// Debug breakpoint
    Debug = 0xF2,

    /// Print top stack value (debug)
    Print = 0xF3,
}

impl OpCode {
    /// Get the number of operand bytes for this opcode
    pub fn operand_size(self) -> usize {
        match self {
            // No operands
            OpCode::Null | OpCode::True | OpCode::False | OpCode::Dup | OpCode::Pop | OpCode::Swap
            | OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div | OpCode::Mod | OpCode::Neg
            | OpCode::Eq | OpCode::Ne | OpCode::Lt | OpCode::Le | OpCode::Gt | OpCode::Ge
            | OpCode::And | OpCode::Or | OpCode::Not
            | OpCode::BitAnd | OpCode::BitOr | OpCode::BitXor | OpCode::BitNot | OpCode::Shl | OpCode::Shr
            | OpCode::LoadIndex | OpCode::StoreIndex | OpCode::Return | OpCode::ReturnValue
            | OpCode::ArrayLength | OpCode::IsInstance | OpCode::Cast
            | OpCode::Throw | OpCode::TryEnd | OpCode::Finally
            | OpCode::Nop | OpCode::Halt | OpCode::Debug | OpCode::Print => 0,

            // 1 byte operand
            OpCode::LoadLocal | OpCode::StoreLocal => 1,

            // 2 byte operands
            OpCode::Const | OpCode::LoadGlobal | OpCode::StoreGlobal | OpCode::LoadField | OpCode::StoreField
            | OpCode::Jump | OpCode::JumpIfFalse | OpCode::JumpIfTrue | OpCode::JumpIfNull | OpCode::JumpIfNotNull
            | OpCode::NewObject | OpCode::NewArray | OpCode::CreateArray | OpCode::CreateObject | OpCode::TryStart | OpCode::Catch
            | OpCode::SuperFieldLoad | OpCode::SuperFieldStore => 2,

            // 3 byte operands (function_index + arg_count)
            OpCode::Call | OpCode::CallMethod | OpCode::CallSuper | OpCode::CallVirtual | OpCode::CallStatic => 3,

            // CallIndirect has no operands (uses stack)
            OpCode::CallIndirect => 0,

            // Pointer operations
            OpCode::AddressOf => 3,        // variable_index (u16) + is_mutable (u8)
            OpCode::Dereference => 1,      // no operands
            OpCode::StoreThrough => 1,     // no operands
            OpCode::CreateRawPointer => 2, // is_mutable (u8)
            OpCode::GetAddress => 1,       // no operands
        }
    }

    /// Check if this opcode is a jump instruction
    pub fn is_jump(self) -> bool {
        matches!(self,
            OpCode::Jump | OpCode::JumpIfFalse | OpCode::JumpIfTrue |
            OpCode::JumpIfNull | OpCode::JumpIfNotNull
        )
    }

    /// Check if this opcode terminates execution flow
    pub fn is_terminator(self) -> bool {
        matches!(self, OpCode::Return | OpCode::ReturnValue | OpCode::Halt | OpCode::Throw)
    }
}

/// A single bytecode instruction with its operands
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instruction {
    pub opcode: OpCode,
    pub operands: Vec<u8>,
    pub line: u32, // Source line number for debugging
}

impl Instruction {
    /// Create a new instruction with no operands
    pub fn new(opcode: OpCode, line: u32) -> Self {
        Self {
            opcode,
            operands: Vec::new(),
            line,
        }
    }

    /// Create a new instruction with operands
    pub fn with_operands(opcode: OpCode, operands: Vec<u8>, line: u32) -> Self {
        Self {
            opcode,
            operands,
            line,
        }
    }

    /// Create instruction with a single u8 operand
    pub fn with_u8(opcode: OpCode, operand: u8, line: u32) -> Self {
        Self {
            opcode,
            operands: vec![operand],
            line,
        }
    }

    /// Create instruction with a single u16 operand
    pub fn with_u16(opcode: OpCode, operand: u16, line: u32) -> Self {
        let bytes = operand.to_le_bytes();
        Self {
            opcode,
            operands: bytes.to_vec(),
            line,
        }
    }

    /// Create instruction with u16 and u8 operands (for Call and CallMethod)
    pub fn with_u16_u8(opcode: OpCode, operand1: u16, operand2: u8, line: u32) -> Self {
        let bytes1 = operand1.to_le_bytes();
        Self {
            opcode,
            operands: vec![bytes1[0], bytes1[1], operand2],
            line,
        }
    }

    /// Create instruction with a single i16 operand
    pub fn with_i16(opcode: OpCode, operand: i16, line: u32) -> Self {
        let bytes = operand.to_le_bytes();
        Self {
            opcode,
            operands: bytes.to_vec(),
            line,
        }
    }

    /// Get u8 operand at index
    pub fn get_u8(&self, index: usize) -> Option<u8> {
        self.operands.get(index).copied()
    }

    /// Get u8 operand at index 2 (for CallMethod arg count)
    pub fn get_arg_count(&self) -> Option<u8> {
        self.operands.get(2).copied()
    }

    /// Get u16 operand starting at index
    pub fn get_u16(&self, index: usize) -> Option<u16> {
        if index + 1 < self.operands.len() {
            let bytes = [self.operands[index], self.operands[index + 1]];
            Some(u16::from_le_bytes(bytes))
        } else {
            None
        }
    }

    /// Get i16 operand starting at index
    pub fn get_i16(&self, index: usize) -> Option<i16> {
        if index + 1 < self.operands.len() {
            let bytes = [self.operands[index], self.operands[index + 1]];
            Some(i16::from_le_bytes(bytes))
        } else {
            None
        }
    }

    /// Set u16 operand starting at index
    pub fn set_u16(&mut self, index: usize, value: u16) {
        let bytes = value.to_le_bytes();
        // Ensure we have enough space
        while self.operands.len() <= index + 1 {
            self.operands.push(0);
        }
        self.operands[index] = bytes[0];
        self.operands[index + 1] = bytes[1];
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.opcode)?;
        if !self.operands.is_empty() {
            write!(f, " ")?;
            for (i, &byte) in self.operands.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{:02X}", byte)?;
            }
        }
        Ok(())
    }
}

/// Bytecode chunk containing instructions and constant pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<crate::core::value::Value>,
    pub name: String,
    pub global_count: usize,  // Number of global variables needed
    pub local_count: usize,   // Number of local variables needed
}

impl Chunk {
    /// Create a new empty chunk
    pub fn new(name: String) -> Self {
        Self {
            instructions: Vec::new(),
            constants: Vec::new(),
            name,
            global_count: 0,
            local_count: 0,
        }
    }

    /// Add an instruction to the chunk
    pub fn add_instruction(&mut self, instruction: Instruction) -> usize {
        self.instructions.push(instruction);
        self.instructions.len() - 1
    }

    /// Add a constant to the constant pool
    pub fn add_constant(&mut self, value: crate::core::value::Value) -> u16 {
        self.constants.push(value);
        (self.constants.len() - 1) as u16
    }

    /// Get instruction at index
    pub fn get_instruction(&self, index: usize) -> Option<&Instruction> {
        self.instructions.get(index)
    }

    /// Get the number of instructions in this chunk
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// Get constant at index
    pub fn get_constant(&self, index: u16) -> Option<&crate::core::value::Value> {
        self.constants.get(index as usize)
    }

    /// Disassemble the chunk for debugging
    pub fn disassemble(&self) -> String {
        let mut result = format!("=== {} ===\n", self.name);

        for (i, instruction) in self.instructions.iter().enumerate() {
            result.push_str(&format!("{:04} | {}\n", i, instruction));
        }

        if !self.constants.is_empty() {
            result.push_str("\n=== Constants ===\n");
            for (i, constant) in self.constants.iter().enumerate() {
                result.push_str(&format!("{:04} | {:?}\n", i, constant));
            }
        }

        result
    }
}