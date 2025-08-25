//! VM execution errors and error handling

use crate::core::{
    memory::MemoryError,
    stack::{StackOverflowError, StackUnderflowError},
};
use std::fmt;

/// VM execution errors
#[derive(Debug, Clone)]
pub enum VMError {
    StackOverflow,
    StackUnderflow,
    MemoryError(String),
    InvalidInstruction(String),
    InvalidOperand(String),
    DivisionByZero,
    TypeMismatch(String),
    UndefinedFunction(String),
    UndefinedVariable(String),
    InvalidJump(String),
    RuntimeError(String),
    AccessViolation(String),
}

impl fmt::Display for VMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VMError::StackOverflow => write!(f, "Stack overflow"),
            VMError::StackUnderflow => write!(f, "Stack underflow"),
            VMError::MemoryError(msg) => write!(f, "Memory error: {}", msg),
            VMError::InvalidInstruction(msg) => write!(f, "Invalid instruction: {}", msg),
            VMError::InvalidOperand(msg) => write!(f, "Invalid operand: {}", msg),
            VMError::DivisionByZero => write!(f, "Division by zero"),
            VMError::TypeMismatch(msg) => write!(f, "Type mismatch: {}", msg),
            VMError::UndefinedFunction(name) => write!(f, "Undefined function: {}", name),
            VMError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            VMError::InvalidJump(msg) => write!(f, "Invalid jump: {}", msg),
            VMError::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
            VMError::AccessViolation(msg) => write!(f, "Access violation: {}", msg),
        }
    }
}

impl std::error::Error for VMError {}

impl From<StackOverflowError> for VMError {
    fn from(_: StackOverflowError) -> Self {
        VMError::StackOverflow
    }
}

impl From<StackUnderflowError> for VMError {
    fn from(_: StackUnderflowError) -> Self {
        VMError::StackUnderflow
    }
}

impl From<MemoryError> for VMError {
    fn from(err: MemoryError) -> Self {
        VMError::MemoryError(err.message)
    }
}

/// VM execution result
pub type VMResult<T> = Result<T, VMError>;

/// Native function signature
pub type NativeFunction = fn(&mut super::VirtualMachine, &[crate::core::value::Value]) -> VMResult<crate::core::value::Value>;
