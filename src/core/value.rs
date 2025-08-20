//! Runtime value types for CodeNothing VM
//!
//! This module defines the value types that can be stored and manipulated
//! by the virtual machine at runtime.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

/// Runtime value types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// Null/undefined value
    Null,

    /// Boolean value
    Bool(bool),

    /// 64-bit signed integer
    Int(i64),

    /// 64-bit floating point number
    Float(f64),

    /// String value (reference counted for efficiency)
    String(Rc<String>),

    /// Array of values
    Array(Rc<Vec<Value>>),

    /// Object with string keys and value values
    Object(Rc<HashMap<String, Value>>),

    /// Function reference
    Function(FunctionRef),

    /// Native function pointer
    NativeFunction(NativeFunctionRef),

    /// Exception value
    Exception(ExceptionRef),
}

/// Function reference containing bytecode
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionRef {
    pub name: String,
    pub arity: u8,           // Number of parameters
    pub chunk_index: usize,  // Index into the chunk table
    pub locals_count: u8,    // Number of local variables
}

/// Native function reference
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeFunctionRef {
    pub name: String,
    pub arity: u8,
    pub function_id: u32, // ID to look up the actual function pointer
}

/// Exception reference containing error information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptionRef {
    pub exception_type: ExceptionType,
    pub message: String,
    pub stack_trace: Vec<String>,
}

/// Built-in exception types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExceptionType {
    /// Generic runtime error
    RuntimeError,
    /// Type mismatch error
    TypeError,
    /// Array/object index out of bounds
    IndexError,
    /// Division by zero
    ArithmeticError,
    /// Null pointer access
    NullPointerError,
    /// Custom user-defined exception
    CustomError(String),
}

impl fmt::Display for ExceptionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExceptionType::RuntimeError => write!(f, "RuntimeError"),
            ExceptionType::TypeError => write!(f, "TypeError"),
            ExceptionType::IndexError => write!(f, "IndexError"),
            ExceptionType::ArithmeticError => write!(f, "ArithmeticError"),
            ExceptionType::NullPointerError => write!(f, "NullPointerError"),
            ExceptionType::CustomError(name) => write!(f, "{}", name),
        }
    }
}

impl Value {
    /// Check if the value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0 && !f.is_nan(),
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
            Value::Function(_) => true,
            Value::NativeFunction(_) => true,
            Value::Exception(_) => true, // Exceptions are always truthy
        }
    }

    /// Check if the value is falsy
    pub fn is_falsy(&self) -> bool {
        !self.is_truthy()
    }

    /// Get the type name of the value
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Function(_) => "function",
            Value::NativeFunction(_) => "native_function",
            Value::Exception(_) => "exception",
        }
    }

    /// Convert value to string representation
    pub fn to_string(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.as_ref().clone(),
            Value::Array(a) => {
                let elements: Vec<String> = a.iter().map(|v| v.to_string()).collect();
                format!("[{}]", elements.join(", "))
            }
            Value::Object(o) => {
                let pairs: Vec<String> = o.iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_string()))
                    .collect();
                format!("{{{}}}", pairs.join(", "))
            }
            Value::Function(f) => format!("function {}", f.name),
            Value::NativeFunction(f) => format!("native function {}", f.name),
            Value::Exception(e) => format!("{}: {}", e.exception_type, e.message),
        }
    }

    /// Try to convert value to integer
    pub fn to_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Float(f) => Some(*f as i64),
            Value::Bool(b) => Some(if *b { 1 } else { 0 }),
            Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// Try to convert value to float
    pub fn to_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// Check if two values are equal
    pub fn equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => *a as f64 == *b,
            (Value::Float(a), Value::Int(b)) => *a == *b as f64,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Object(a), Value::Object(b)) => a == b,
            (Value::Function(a), Value::Function(b)) => a == b,
            (Value::NativeFunction(a), Value::NativeFunction(b)) => a == b,
            (Value::Exception(a), Value::Exception(b)) => a == b,
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Arithmetic operations for values
impl Value {
    /// Add two values
    pub fn add(&self, other: &Value) -> Result<Value, String> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::String(a), Value::String(b)) => {
                let result = format!("{}{}", a, b);
                Ok(Value::String(Rc::new(result)))
            }
            (Value::String(a), other) => {
                let result = format!("{}{}", a, other.to_string());
                Ok(Value::String(Rc::new(result)))
            }
            (other, Value::String(b)) => {
                let result = format!("{}{}", other.to_string(), b);
                Ok(Value::String(Rc::new(result)))
            }
            _ => Err(format!("Cannot add {} and {}", self.type_name(), other.type_name())),
        }
    }

    /// Subtract two values
    pub fn sub(&self, other: &Value) -> Result<Value, String> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
            _ => Err(format!("Cannot subtract {} from {}", other.type_name(), self.type_name())),
        }
    }

    /// Multiply two values
    pub fn mul(&self, other: &Value) -> Result<Value, String> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
            _ => Err(format!("Cannot multiply {} and {}", self.type_name(), other.type_name())),
        }
    }

    /// Divide two values
    pub fn div(&self, other: &Value) -> Result<Value, String> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Float(*a as f64 / *b as f64))
                }
            }
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Float(a / b))
                }
            }
            (Value::Int(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Float(*a as f64 / b))
                }
            }
            (Value::Float(a), Value::Int(b)) => {
                if *b == 0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Float(a / *b as f64))
                }
            }
            _ => Err(format!("Cannot divide {} by {}", self.type_name(), other.type_name())),
        }
    }

    /// Modulo operation
    pub fn modulo(&self, other: &Value) -> Result<Value, String> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    Err("Modulo by zero".to_string())
                } else {
                    Ok(Value::Int(a % b))
                }
            }
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err("Modulo by zero".to_string())
                } else {
                    Ok(Value::Float(a % b))
                }
            }
            (Value::Int(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err("Modulo by zero".to_string())
                } else {
                    Ok(Value::Float(*a as f64 % b))
                }
            }
            (Value::Float(a), Value::Int(b)) => {
                if *b == 0 {
                    Err("Modulo by zero".to_string())
                } else {
                    Ok(Value::Float(a % *b as f64))
                }
            }
            _ => Err(format!("Cannot compute modulo of {} and {}", self.type_name(), other.type_name())),
        }
    }

    /// Negate a value
    pub fn negate(&self) -> Result<Value, String> {
        match self {
            Value::Int(i) => Ok(Value::Int(-i)),
            Value::Float(f) => Ok(Value::Float(-f)),
            _ => Err(format!("Cannot negate {}", self.type_name())),
        }
    }
}

/// Helper methods for creating exceptions
impl Value {
    /// Create a runtime error exception
    pub fn runtime_error(message: String) -> Self {
        Value::Exception(ExceptionRef {
            exception_type: ExceptionType::RuntimeError,
            message,
            stack_trace: Vec::new(),
        })
    }

    /// Create a type error exception
    pub fn type_error(message: String) -> Self {
        Value::Exception(ExceptionRef {
            exception_type: ExceptionType::TypeError,
            message,
            stack_trace: Vec::new(),
        })
    }

    /// Create an index error exception
    pub fn index_error(message: String) -> Self {
        Value::Exception(ExceptionRef {
            exception_type: ExceptionType::IndexError,
            message,
            stack_trace: Vec::new(),
        })
    }

    /// Create an arithmetic error exception
    pub fn arithmetic_error(message: String) -> Self {
        Value::Exception(ExceptionRef {
            exception_type: ExceptionType::ArithmeticError,
            message,
            stack_trace: Vec::new(),
        })
    }

    /// Create a null pointer error exception
    pub fn null_pointer_error(message: String) -> Self {
        Value::Exception(ExceptionRef {
            exception_type: ExceptionType::NullPointerError,
            message,
            stack_trace: Vec::new(),
        })
    }

    /// Create a custom error exception
    pub fn custom_error(error_type: String, message: String) -> Self {
        Value::Exception(ExceptionRef {
            exception_type: ExceptionType::CustomError(error_type),
            message,
            stack_trace: Vec::new(),
        })
    }

    /// Check if this value is an exception
    pub fn is_exception(&self) -> bool {
        matches!(self, Value::Exception(_))
    }
}