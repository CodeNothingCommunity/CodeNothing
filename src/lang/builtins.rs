//! Built-in functions for CodeNothing language
//! 
//! Implements the lang namespace with built-in functions like println, input, etc.

use crate::core::value::Value;
use crate::lang::types::{Type, TypeEnvironment};
use std::collections::HashMap;
use std::io::{self, Write};

/// Built-in function signature
pub type BuiltinFunction = fn(&[Value]) -> Result<Value, String>;

/// Built-in function registry
pub struct BuiltinRegistry {
    functions: HashMap<String, (BuiltinFunction, Type)>,
}

impl BuiltinRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
        };
        
        registry.register_lang_functions();
        registry
    }
    
    /// Register a built-in function
    pub fn register(&mut self, name: String, function: BuiltinFunction, func_type: Type) {
        self.functions.insert(name, (function, func_type));
    }
    
    /// Get a built-in function
    pub fn get(&self, name: &str) -> Option<&(BuiltinFunction, Type)> {
        self.functions.get(name)
    }
    
    /// Get all function names
    pub fn get_names(&self) -> Vec<&String> {
        self.functions.keys().collect()
    }
    
    /// Register all lang namespace functions
    fn register_lang_functions(&mut self) {
        // lang::println() - print with newline
        self.register(
            "lang::println".to_string(),
            builtin_println,
            Type::Function {
                parameters: vec![Type::Auto(Box::new(Type::Unknown))],
                return_type: Box::new(Type::Null),
            }
        );
        
        // lang::print() - print without newline
        self.register(
            "lang::print".to_string(),
            builtin_print,
            Type::Function {
                parameters: vec![Type::Auto(Box::new(Type::Unknown))],
                return_type: Box::new(Type::Null),
            }
        );
        
        // lang::input() - read line from stdin
        self.register(
            "lang::input".to_string(),
            builtin_input,
            Type::Function {
                parameters: vec![],
                return_type: Box::new(Type::String),
            }
        );

        // lang::input_line() - read line with prompt
        self.register(
            "lang::input_line".to_string(),
            builtin_input_line,
            Type::Function {
                parameters: vec![Type::String],
                return_type: Box::new(Type::String),
            }
        );

        // lang::to_string() - convert value to string
        self.register(
            "lang::to_string".to_string(),
            builtin_to_string,
            Type::Function {
                parameters: vec![Type::Auto(Box::new(Type::Unknown))],
                return_type: Box::new(Type::String),
            }
        );
        
        // lang::to_int() - convert value to int
        self.register(
            "lang::to_int".to_string(),
            builtin_to_int,
            Type::Function {
                parameters: vec![Type::Auto(Box::new(Type::Unknown))],
                return_type: Box::new(Type::Int),
            }
        );
        
        // lang::to_float() - convert value to float
        self.register(
            "lang::to_float".to_string(),
            builtin_to_float,
            Type::Function {
                parameters: vec![Type::Auto(Box::new(Type::Unknown))],
                return_type: Box::new(Type::Float(64)), // Default to 64-bit float
            }
        );
        
        // lang::len() - get length of array or string
        self.register(
            "lang::len".to_string(),
            builtin_len,
            Type::Function {
                parameters: vec![Type::Auto(Box::new(Type::Unknown))],
                return_type: Box::new(Type::Int),
            }
        );
        
        // lang::type_of() - get type name of value
        self.register(
            "lang::type_of".to_string(),
            builtin_type_of,
            Type::Function {
                parameters: vec![Type::Auto(Box::new(Type::Unknown))],
                return_type: Box::new(Type::String),
            }
        );
    }
    
    /// Add built-in functions to type environment
    pub fn populate_type_environment(&self, env: &mut TypeEnvironment) {
        // Create lang namespace
        env.define_namespace("lang".to_string());
        
        // Add all functions to lang namespace
        for (name, (_, func_type)) in &self.functions {
            if let Some(short_name) = name.strip_prefix("lang::") {
                env.add_to_namespace("lang", short_name.to_string(), func_type.clone())
                    .unwrap_or_else(|e| eprintln!("Warning: {}", e));
            }
        }
    }
}

impl Default for BuiltinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Built-in function implementations

/// lang::println() - print value with newline
fn builtin_println(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        println!();
    } else {
        println!("{}", value_to_string(&args[0]));
    }
    Ok(Value::Null)
}

/// lang::print() - print value without newline
fn builtin_print(args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        print!("{}", value_to_string(&args[0]));
        io::stdout().flush().map_err(|e| e.to_string())?;
    }
    Ok(Value::Null)
}

/// lang::input() - read line from stdin
fn builtin_input(_args: &[Value]) -> Result<Value, String> {
    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| e.to_string())?;
    
    // Remove trailing newline
    if input.ends_with('\n') {
        input.pop();
        if input.ends_with('\r') {
            input.pop();
        }
    }
    
    Ok(Value::String(input.into()))
}

/// lang::input_line() - read line with prompt
fn builtin_input_line(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("input_line() requires a prompt argument".to_string());
    }
    
    print!("{}", value_to_string(&args[0]));
    io::stdout().flush().map_err(|e| e.to_string())?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| e.to_string())?;
    
    // Remove trailing newline
    if input.ends_with('\n') {
        input.pop();
        if input.ends_with('\r') {
            input.pop();
        }
    }
    
    Ok(Value::String(input.into()))
}

/// lang::to_string() - convert value to string
fn builtin_to_string(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("to_string() requires an argument".to_string());
    }
    
    Ok(Value::String(value_to_string(&args[0]).into()))
}

/// lang::to_int() - convert value to int
fn builtin_to_int(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("to_int() requires an argument".to_string());
    }
    
    match &args[0] {
        Value::Int(i) => Ok(Value::Int(*i)),
        Value::Float(f) => Ok(Value::Int(*f as i64)),
        Value::String(s) => {
            s.parse::<i64>()
                .map(Value::Int)
                .map_err(|_| format!("Cannot convert '{}' to int", s))
        }
        Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
        _ => Err(format!("Cannot convert {} to int", value_type_name(&args[0]))),
    }
}

/// lang::to_float() - convert value to float
fn builtin_to_float(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("to_float() requires an argument".to_string());
    }
    
    match &args[0] {
        Value::Int(i) => Ok(Value::Float(*i as f64)),
        Value::Float(f) => Ok(Value::Float(*f)),
        Value::String(s) => {
            s.parse::<f64>()
                .map(Value::Float)
                .map_err(|_| format!("Cannot convert '{}' to float", s))
        }
        Value::Bool(b) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
        _ => Err(format!("Cannot convert {} to float", value_type_name(&args[0]))),
    }
}

/// lang::len() - get length of array or string
fn builtin_len(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("len() requires an argument".to_string());
    }
    
    match &args[0] {
        Value::String(s) => Ok(Value::Int(s.len() as i64)),
        Value::Array(arr) => Ok(Value::Int(arr.len() as i64)),
        _ => Err(format!("Cannot get length of {}", value_type_name(&args[0]))),
    }
}

/// lang::type_of() - get type name of value
fn builtin_type_of(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("type_of() requires an argument".to_string());
    }
    
    Ok(Value::String(value_type_name(&args[0]).to_string().into()))
}

/// Convert value to string representation
fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.to_string(),
        Value::Array(arr) => {
            let elements: Vec<String> = arr.iter().map(value_to_string).collect();
            format!("[{}]", elements.join(", "))
        }
        Value::Object(obj) => {
            let fields: Vec<String> = obj.iter()
                .map(|(k, v)| format!("{}: {}", k, value_to_string(v)))
                .collect();
            format!("{{{}}}", fields.join(", "))
        }
        Value::Function(_) => "<function>".to_string(),
        Value::NativeFunction(_) => "<native function>".to_string(),
        Value::Closure { parameter_count, .. } => format!("<closure/{}>", parameter_count),
        Value::Exception(e) => format!("{}: {}", e.exception_type, e.message),
        Value::Reference { address, target_type, .. } => {
            format!("&{} @ 0x{:x}", target_type, address)
        }
        Value::MutableReference { address, target_type, .. } => {
            format!("&mut {} @ 0x{:x}", target_type, address)
        }
        Value::RawPointer { address, target_type, is_mutable } => {
            if *is_mutable {
                format!("*mut {} @ 0x{:x}", target_type, address)
            } else {
                format!("*const {} @ 0x{:x}", target_type, address)
            }
        }
    }
}

/// Get type name of value
fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Function(_) => "function",
        Value::NativeFunction(_) => "native_function",
        Value::Closure { .. } => "closure",
        Value::Exception(_) => "exception",
        Value::Reference { .. } => "reference",
        Value::MutableReference { .. } => "mutable_reference",
        Value::RawPointer { .. } => "raw_pointer",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    
    #[test]
    fn test_builtin_to_string() {
        let result = builtin_to_string(&[Value::Int(42)]).unwrap();
        assert_eq!(result, Value::String(Rc::new("42".to_string())));
        
        let result = builtin_to_string(&[Value::Bool(true)]).unwrap();
        assert_eq!(result, Value::String(Rc::new("true".to_string())));
    }
    
    #[test]
    fn test_builtin_to_int() {
        let result = builtin_to_int(&[Value::String(Rc::new("42".to_string()))]).unwrap();
        assert_eq!(result, Value::Int(42));
        
        let result = builtin_to_int(&[Value::Float(3.14)]).unwrap();
        assert_eq!(result, Value::Int(3));
        
        let result = builtin_to_int(&[Value::Bool(true)]).unwrap();
        assert_eq!(result, Value::Int(1));
    }
    
    #[test]
    fn test_builtin_len() {
        let result = builtin_len(&[Value::String(Rc::new("hello".to_string()))]).unwrap();
        assert_eq!(result, Value::Int(5));
        
        let arr = vec![Value::Int(1), Value::Int(2), Value::Int(3)];
        let result = builtin_len(&[Value::Array(arr.into())]).unwrap();
        assert_eq!(result, Value::Int(3));
    }
    
    #[test]
    fn test_builtin_type_of() {
        let result = builtin_type_of(&[Value::Int(42)]).unwrap();
        assert_eq!(result, Value::String(Rc::new("int".to_string())));

        let result = builtin_type_of(&[Value::String(Rc::new("hello".to_string()))]).unwrap();
        assert_eq!(result, Value::String(Rc::new("string".to_string())));
    }
}
