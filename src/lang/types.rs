//! Type system for CodeNothing language
//! 
//! Implements strong typing with explicit type annotations and auto type inference

use std::collections::HashMap;
use std::fmt;
use crate::lang::ast::{TypeAnnotation, Position};

/// Runtime type representation
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Long,                  // 64-bit integer
    Float(u32),            // float(32) - arbitrary byte size
    String,                // String type
    Num(u32),              // num(16) - arbitrary byte size
    Bool,
    Array(Box<Type>),
    Obj,
    Auto(Box<Type>),       // Auto with inferred type
    Null,
    Function {
        parameters: Vec<Type>,
        return_type: Box<Type>,
    },
    Custom(String),
    Unknown,               // For type inference
    Void,                  // Void type for functions with no return value
}

impl Type {
    /// Check if this type is compatible with another type
    pub fn is_compatible_with(&self, other: &Type) -> bool {
        match (self, other) {
            // Exact matches
            (Type::Int, Type::Int) => true,
            (Type::Long, Type::Long) => true,
            (Type::Float(_), Type::Float(_)) => true, // Allow different float sizes
            (Type::String, Type::String) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Null, Type::Null) => true,
            (Type::Obj, Type::Obj) => true,
            (Type::Void, Type::Void) => true,

            // Integer type compatibility - int can be promoted to long
            (Type::Long, Type::Int) => true,
            (Type::Int, Type::Long) => true, // Allow both directions for now

            // Numeric type compatibility - int can be used as num
            (Type::Num(_), Type::Int) => true,
            (Type::Int, Type::Num(_)) => true,


            
            // Num types are compatible if sizes match
            (Type::Num(a), Type::Num(b)) => a == b,
            
            // Auto types are compatible with their inferred type
            (Type::Auto(inner), other) => inner.is_compatible_with(other),
            (other, Type::Auto(inner)) => other.is_compatible_with(inner),
            
            // Array types must have compatible element types
            (Type::Array(a), Type::Array(b)) => a.is_compatible_with(b),
            
            // Function types must have compatible signatures
            (Type::Function { parameters: p1, return_type: r1 }, 
             Type::Function { parameters: p2, return_type: r2 }) => {
                p1.len() == p2.len() &&
                p1.iter().zip(p2.iter()).all(|(a, b)| a.is_compatible_with(b)) &&
                r1.is_compatible_with(r2)
            }
            
            // Custom types match by name
            (Type::Custom(a), Type::Custom(b)) => a == b,
            
            // Null is compatible with any nullable type
            (Type::Null, _) => true,
            (_, Type::Null) => true,
            
            // Unknown is compatible with anything (for inference)
            (Type::Unknown, _) => true,
            (_, Type::Unknown) => true,
            
            // Numeric promotions
            (Type::Int, Type::Float(_)) => true,
            (Type::Float(_), Type::Num(_)) => true,
            
            _ => false,
        }
    }
    
    /// Get the promoted type when combining two types
    pub fn promote_with(&self, other: &Type) -> Option<Type> {
        match (self, other) {
            // Same types
            (a, b) if a == b => Some(a.clone()),
            
            // Numeric promotions
            (Type::Int, Type::Float(size)) | (Type::Float(size), Type::Int) => Some(Type::Float(*size)),
            (Type::Int, Type::Num(size)) | (Type::Num(size), Type::Int) => Some(Type::Num(*size)),
            (Type::Float(size), Type::Num(num_size)) | (Type::Num(num_size), Type::Float(size)) => Some(Type::Num(*num_size)),
            
            // Auto types
            (Type::Auto(inner), other) => inner.promote_with(other),
            (other, Type::Auto(inner)) => other.promote_with(inner),
            
            // Unknown types
            (Type::Unknown, other) => Some(other.clone()),
            (other, Type::Unknown) => Some(other.clone()),
            
            _ => None,
        }
    }
    
    /// Get the size in bytes for this type
    pub fn size_bytes(&self) -> Option<usize> {
        match self {
            Type::Int => Some(8),      // i64
            Type::Float(size) => Some(*size as usize / 8),    // Convert bits to bytes
            Type::Num(size) => Some(*size as usize),
            Type::Bool => Some(1),
            Type::Null => Some(0),
            Type::Auto(inner) => inner.size_bytes(),
            _ => None, // Dynamic size types
        }
    }
    
    /// Check if this is a numeric type
    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float(_) | Type::Num(_))
    }
    
    /// Check if this type can be used in arithmetic operations
    pub fn is_arithmetic(&self) -> bool {
        match self {
            Type::Int | Type::Float(_) | Type::Num(_) => true,
            Type::Auto(inner) => inner.is_arithmetic(),
            _ => false,
        }
    }
    
    /// Check if this type can be used in comparison operations
    pub fn is_comparable(&self) -> bool {
        match self {
            Type::Int | Type::Float(_) | Type::Num(_) | Type::Bool => true,
            Type::Auto(inner) => inner.is_comparable(),
            _ => false,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Long => write!(f, "long"),
            Type::Float(size) => write!(f, "float({})", size),
            Type::String => write!(f, "string"),
            Type::Num(size) => write!(f, "num({})", size),
            Type::Bool => write!(f, "bool"),
            Type::Array(inner) => write!(f, "array<{}>", inner),
            Type::Obj => write!(f, "obj"),
            Type::Auto(inner) => write!(f, "auto({})", inner),
            Type::Null => write!(f, "null"),
            Type::Function { parameters, return_type } => {
                let params: Vec<String> = parameters.iter().map(|p| p.to_string()).collect();
                write!(f, "fn({}) -> {}", params.join(", "), return_type)
            }
            Type::Custom(name) => write!(f, "{}", name),
            Type::Unknown => write!(f, "?"),
            Type::Void => write!(f, "void"),
        }
    }
}

impl From<TypeAnnotation> for Type {
    fn from(annotation: TypeAnnotation) -> Self {
        match annotation {
            TypeAnnotation::Int => Type::Int,
            TypeAnnotation::Long => Type::Long,
            TypeAnnotation::Float(Some(size)) => Type::Float(size),
            TypeAnnotation::Float(None) => Type::Float(32), // Default size
            TypeAnnotation::String => Type::String,
            TypeAnnotation::Num(Some(size)) => Type::Num(size),
            TypeAnnotation::Num(None) => Type::Num(16), // Default size
            TypeAnnotation::Bool => Type::Bool,
            TypeAnnotation::Array(inner) => Type::Array(Box::new(Type::from(*inner))),
            TypeAnnotation::Obj => Type::Obj,
            TypeAnnotation::Auto => Type::Unknown, // Will be inferred
            TypeAnnotation::Null => Type::Null,
            TypeAnnotation::Custom(name) => Type::Custom(name),
            TypeAnnotation::Void => Type::Void,
        }
    }
}

/// Namespace structure for nested namespaces
#[derive(Debug, Clone)]
pub struct NamespaceInfo {
    pub functions: HashMap<String, Type>,
    pub nested_namespaces: HashMap<String, NamespaceInfo>,
}

impl NamespaceInfo {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            nested_namespaces: HashMap::new(),
        }
    }
}

/// Type environment for tracking variable types
#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    scopes: Vec<HashMap<String, Type>>,
    functions: HashMap<String, Type>,
    namespaces: HashMap<String, NamespaceInfo>,
}

impl TypeEnvironment {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()], // Global scope
            functions: HashMap::new(),
            namespaces: HashMap::new(),
        }
    }
    
    /// Enter a new scope
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }
    
    /// Exit current scope
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }
    
    /// Define a variable in current scope
    pub fn define_variable(&mut self, name: String, var_type: Type) -> Result<(), String> {
        if let Some(current_scope) = self.scopes.last_mut() {
            if current_scope.contains_key(&name) {
                return Err(format!("Variable '{}' already defined in current scope", name));
            }
            current_scope.insert(name, var_type);
            Ok(())
        } else {
            Err("No scope available".to_string())
        }
    }
    
    /// Define a variable in global scope (allows redefinition)
    pub fn define_global_variable(&mut self, name: String, var_type: Type) {
        if let Some(global_scope) = self.scopes.first_mut() {
            global_scope.insert(name, var_type);
        }
    }
    
    /// Look up a variable type
    pub fn lookup_variable(&self, name: &str) -> Option<&Type> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(var_type) = scope.get(name) {
                return Some(var_type);
            }
        }
        None
    }
    
    /// Define a function
    pub fn define_function(&mut self, name: String, func_type: Type) -> Result<(), String> {
        if self.functions.contains_key(&name) {
            return Err(format!("Function '{}' already defined", name));
        }
        self.functions.insert(name, func_type);
        Ok(())
    }
    
    /// Look up a function type
    pub fn lookup_function(&self, name: &str) -> Option<&Type> {
        self.functions.get(name)
    }

    /// Get function type (alias for lookup_function)
    pub fn get_function(&self, name: &str) -> Option<&Type> {
        self.functions.get(name)
    }
    
    /// Define a namespace
    pub fn define_namespace(&mut self, name: String) {
        self.namespaces.insert(name, NamespaceInfo::new());
    }

    /// Define a nested namespace
    pub fn define_nested_namespace(&mut self, namespace_path: &[String]) {
        if namespace_path.is_empty() {
            return;
        }

        let root_name = &namespace_path[0];
        let namespace_info = self.namespaces.entry(root_name.clone()).or_insert_with(NamespaceInfo::new);

        // Navigate to the nested namespace
        let mut current_namespace = namespace_info;
        for ns_name in &namespace_path[1..] {
            current_namespace = current_namespace.nested_namespaces.entry(ns_name.clone()).or_insert_with(NamespaceInfo::new);
        }
    }
    
    /// Add item to namespace
    pub fn add_to_namespace(&mut self, namespace: &str, name: String, item_type: Type) -> Result<(), String> {
        if let Some(ns) = self.namespaces.get_mut(namespace) {
            ns.functions.insert(name, item_type);
            Ok(())
        } else {
            Err(format!("Namespace '{}' not found", namespace))
        }
    }

    /// Add item to nested namespace
    pub fn add_to_nested_namespace(&mut self, namespace_path: &[String], name: String, item_type: Type) -> Result<(), String> {
        if namespace_path.is_empty() {
            return Err("Empty namespace path".to_string());
        }

        let root_name = &namespace_path[0];
        let namespace_info = self.namespaces.get_mut(root_name)
            .ok_or_else(|| format!("Namespace '{}' not found", root_name))?;

        // Navigate to the nested namespace
        let mut current_namespace = namespace_info;
        for ns_name in &namespace_path[1..] {
            current_namespace = current_namespace.nested_namespaces.get_mut(ns_name)
                .ok_or_else(|| format!("Nested namespace '{}' not found", ns_name))?;
        }

        current_namespace.functions.insert(name, item_type);
        Ok(())
    }
    
    /// Look up item in namespace
    pub fn lookup_in_namespace(&self, namespace: &str, name: &str) -> Option<&Type> {
        self.namespaces.get(namespace)?.functions.get(name)
    }

    /// Look up item in nested namespace
    pub fn lookup_in_nested_namespace(&self, namespace_path: &[String], name: &str) -> Option<&Type> {
        if namespace_path.is_empty() {
            return None;
        }

        let root_name = &namespace_path[0];
        let mut current_namespace = self.namespaces.get(root_name)?;

        // Navigate to the nested namespace
        for ns_name in &namespace_path[1..] {
            current_namespace = current_namespace.nested_namespaces.get(ns_name)?;
        }

        current_namespace.functions.get(name)
    }
    
    /// Get all namespaces
    pub fn get_namespaces(&self) -> &HashMap<String, NamespaceInfo> {
        &self.namespaces
    }

    /// Get functions from a specific namespace path
    pub fn get_namespace_functions(&self, namespace_path: &[String]) -> Option<&HashMap<String, Type>> {
        if namespace_path.is_empty() {
            return None;
        }

        let root_name = &namespace_path[0];
        let mut current_namespace = self.namespaces.get(root_name)?;

        // Navigate to the nested namespace
        for ns_name in &namespace_path[1..] {
            current_namespace = current_namespace.nested_namespaces.get(ns_name)?;
        }

        Some(&current_namespace.functions)
    }

    /// Import functions from a namespace into the current scope
    pub fn import_namespace(&mut self, namespace_name: &str) -> Result<(), String> {
        if let Some(namespace_info) = self.namespaces.get(namespace_name) {
            for (func_name, func_type) in &namespace_info.functions {
                self.functions.insert(func_name.clone(), func_type.clone());
            }
            Ok(())
        } else {
            Err(format!("Unknown namespace '{}'", namespace_name))
        }
    }

    /// Define a variable in a specific namespace
    pub fn define_variable_in_namespace(&mut self, namespace_path: &str, name: String, var_type: Type) -> Result<(), String> {
        // For now, treat namespace variables as global variables with prefixed names
        // This is a simplified implementation
        let prefixed_name = format!("{}::{}", namespace_path, name);
        self.define_variable(prefixed_name, var_type)
    }
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

/// Type checker errors
#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub position: Position,
}

impl TypeError {
    pub fn new(message: String, position: Position) -> Self {
        Self { message, position }
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Type error at {}:{}: {}", 
               self.position.line, self.position.column, self.message)
    }
}

/// Type inference engine
pub struct TypeInference {
    constraints: Vec<(Type, Type, Position)>, // (expected, actual, position)
}

impl TypeInference {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
        }
    }
    
    /// Add a type constraint
    pub fn add_constraint(&mut self, expected: Type, actual: Type, position: Position) {
        self.constraints.push((expected, actual, position));
    }

    /// Check if two types can be unified
    pub fn can_unify(&self, expected: &Type, actual: &Type) -> bool {
        expected.is_compatible_with(actual)
    }
    
    /// Solve all constraints
    pub fn solve(&self) -> Result<HashMap<String, Type>, TypeError> {
        let mut substitutions = HashMap::new();
        
        for (expected, actual, position) in &self.constraints {
            if !expected.is_compatible_with(actual) {
                return Err(TypeError::new(
                    format!("Type mismatch: expected {}, found {}", expected, actual),
                    position.clone()
                ));
            }
        }
        
        Ok(substitutions)
    }
}

impl Default for TypeInference {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_type_compatibility() {
        assert!(Type::Int.is_compatible_with(&Type::Int));
        assert!(Type::Int.is_compatible_with(&Type::Float(32)));
        assert!(Type::Int.is_compatible_with(&Type::Num(16)));
        assert!(!Type::Bool.is_compatible_with(&Type::Int));
    }
    
    #[test]
    fn test_type_promotion() {
        assert_eq!(Type::Int.promote_with(&Type::Float(32)), Some(Type::Float(32)));
        assert_eq!(Type::Int.promote_with(&Type::Num(32)), Some(Type::Num(32)));
        assert_eq!(Type::Bool.promote_with(&Type::Int), None);
    }
    
    #[test]
    fn test_type_environment() {
        let mut env = TypeEnvironment::new();
        
        // Define variable
        env.define_variable("x".to_string(), Type::Int).unwrap();
        assert_eq!(env.lookup_variable("x"), Some(&Type::Int));
        
        // Enter new scope
        env.enter_scope();
        env.define_variable("y".to_string(), Type::Float(32)).unwrap();
        assert_eq!(env.lookup_variable("y"), Some(&Type::Float(32)));
        assert_eq!(env.lookup_variable("x"), Some(&Type::Int)); // Still visible
        
        // Exit scope
        env.exit_scope();
        assert_eq!(env.lookup_variable("y"), None); // No longer visible
        assert_eq!(env.lookup_variable("x"), Some(&Type::Int)); // Still visible
    }
}
