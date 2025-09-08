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
    /// Arrow function type (lambda functions)
    ArrowFunction {
        parameters: Vec<Type>,
        return_type: Box<Type>,
    },
    /// Library function type (accepts any parameters)
    LibraryFunction {
        return_type: Box<Type>,
    },
    Custom(String),
    Unknown,               // For type inference
    Any,                   // Any type (for pipe operations)
    Void,                  // Void type for functions with no return value
    Enum(String),          // Enum type
    Struct {               // Struct type
        name: String,
        fields: Vec<(String, Type)>,
    },
    Class {                // Class type
        name: String,
        generic_parameters: Vec<String>,  // 泛型参数名称
        base_class: Option<String>,  // 父类名称
        implemented_traits: Vec<String>,  // 实现的接口列表
        fields: Vec<(String, Type)>,
        methods: Vec<(String, Type)>,
        static_fields: Vec<(String, Type)>,  // 静态字段
        static_methods: Vec<(String, Type)>,  // 静态方法
    },
    Generic {              // 泛型类型实例
        base_name: String,
        type_arguments: Vec<Type>,
    },
    TypeParameter(String), // 泛型类型参数
    Trait {                // Trait type
        name: String,
        super_traits: Vec<String>,  // 父接口
        methods: Vec<(String, Type)>,
        default_methods: Vec<(String, Type)>,  // 带默认实现的方法
    },

    Lambda {               // Lambda/closure type
        parameters: Vec<Type>,
        return_type: Box<Type>,
    },

    /// Function pointer type (fn(T1, T2) -> T3)
    FunctionPointer {
        parameters: Vec<Type>,
        return_type: Box<Type>,
    },

    // Pointer and reference types
    Reference {            // Immutable reference (&T)
        target_type: Box<Type>,
        lifetime: Option<String>, // Optional lifetime annotation
    },
    MutableReference {     // Mutable reference (&mut T)
        target_type: Box<Type>,
        lifetime: Option<String>, // Optional lifetime annotation
    },
    RawPointer {           // Raw pointer (*T)
        target_type: Box<Type>,
        is_mutable: bool,  // *mut T vs *const T
    },
}

impl Type {
    /// Check if this type is compatible with another type
    pub fn is_compatible_with(&self, other: &Type) -> bool {
        self.is_compatible_with_env(other, None)
    }

    /// Check if this type is compatible with another type, with optional type environment for inheritance checking
    pub fn is_compatible_with_env(&self, other: &Type, type_env: Option<&TypeEnvironment>) -> bool {
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
            (Type::Auto(inner), other) => inner.is_compatible_with_env(other, type_env),
            (other, Type::Auto(inner)) => other.is_compatible_with_env(inner, type_env),

            // Array types must have compatible element types
            (Type::Array(a), Type::Array(b)) => a.is_compatible_with_env(b, type_env),
            
            // Function types must have compatible signatures
            (Type::Function { parameters: p1, return_type: r1 },
             Type::Function { parameters: p2, return_type: r2 }) => {
                p1.len() == p2.len() &&
                p1.iter().zip(p2.iter()).all(|(a, b)| a.is_compatible_with_env(b, type_env)) &&
                r1.is_compatible_with_env(r2, type_env)
            }
            
            // Custom types match by name
            (Type::Custom(a), Type::Custom(b)) => a == b,

            // Custom type can match with struct of same name
            (Type::Custom(name), Type::Struct { name: struct_name, .. }) => name == struct_name,
            (Type::Struct { name: struct_name, .. }, Type::Custom(name)) => struct_name == name,

            // Enum types match by name
            (Type::Enum(a), Type::Enum(b)) => a == b,

            // Struct types match by name and field compatibility
            (Type::Struct { name: n1, fields: f1 }, Type::Struct { name: n2, fields: f2 }) => {
                n1 == n2 && f1.len() == f2.len() &&
                f1.iter().zip(f2.iter()).all(|((name1, type1), (name2, type2))| {
                    name1 == name2 && type1.is_compatible_with_env(type2, type_env)
                })
            }

            // Class types match by name or inheritance relationship
            (Type::Class { name: n1, .. }, Type::Class { name: n2, .. }) => {
                if n1 == n2 {
                    true
                } else if let Some(env) = type_env {
                    // Check if n1 is a subclass of n2 (polymorphism: derived can be assigned to base)
                    env.is_subclass(n1, n2)
                } else {
                    false
                }
            }

            // Class can be compatible with trait if it implements the trait
            (Type::Class { implemented_traits, .. }, Type::Trait { name: trait_name, .. }) => {
                implemented_traits.contains(trait_name)
            }

            // Trait types match by name
            (Type::Trait { name: n1, .. }, Type::Trait { name: n2, .. }) => n1 == n2,

            // Custom type can match with class of same name
            (Type::Custom(name), Type::Class { name: class_name, .. }) => name == class_name,
            (Type::Class { name: class_name, .. }, Type::Custom(name)) => class_name == name,



            // Generic types match by name and parameter compatibility
            (Type::Generic { base_name: n1, type_arguments: p1 }, Type::Generic { base_name: n2, type_arguments: p2 }) => {
                n1 == n2 && p1.len() == p2.len() &&
                p1.iter().zip(p2.iter()).all(|(a, b)| a.is_compatible_with_env(b, type_env))
            }

            // Type parameters match by name
            (Type::TypeParameter(n1), Type::TypeParameter(n2)) => n1 == n2,

            // Lambda types match by signature compatibility
            (Type::Lambda { parameters: p1, return_type: r1 },
             Type::Lambda { parameters: p2, return_type: r2 }) => {
                p1.len() == p2.len() &&
                p1.iter().zip(p2.iter()).all(|(a, b)| a.is_compatible_with_env(b, type_env)) &&
                r1.is_compatible_with_env(r2, type_env)
            }

            // Lambda and Function types are compatible
            (Type::Lambda { parameters: p1, return_type: r1 },
             Type::Function { parameters: p2, return_type: r2 }) => {
                p1.len() == p2.len() &&
                p1.iter().zip(p2.iter()).all(|(a, b)| a.is_compatible_with_env(b, type_env)) &&
                r1.is_compatible_with_env(r2, type_env)
            }
            (Type::Function { parameters: p1, return_type: r1 },
             Type::Lambda { parameters: p2, return_type: r2 }) => {
                p1.len() == p2.len() &&
                p1.iter().zip(p2.iter()).all(|(a, b)| a.is_compatible_with_env(b, type_env)) &&
                r1.is_compatible_with_env(r2, type_env)
            }

            // FunctionPointer types match by signature compatibility
            (Type::FunctionPointer { parameters: p1, return_type: r1 },
             Type::FunctionPointer { parameters: p2, return_type: r2 }) => {
                p1.len() == p2.len() &&
                p1.iter().zip(p2.iter()).all(|(a, b)| a.is_compatible_with_env(b, type_env)) &&
                r1.is_compatible_with_env(r2, type_env)
            }

            // FunctionPointer is compatible with Function and Lambda types
            (Type::FunctionPointer { parameters: p1, return_type: r1 },
             Type::Function { parameters: p2, return_type: r2 }) => {
                p1.len() == p2.len() &&
                p1.iter().zip(p2.iter()).all(|(a, b)| a.is_compatible_with_env(b, type_env)) &&
                r1.is_compatible_with_env(r2, type_env)
            }
            (Type::Function { parameters: p1, return_type: r1 },
             Type::FunctionPointer { parameters: p2, return_type: r2 }) => {
                p1.len() == p2.len() &&
                p1.iter().zip(p2.iter()).all(|(a, b)| a.is_compatible_with_env(b, type_env)) &&
                r1.is_compatible_with_env(r2, type_env)
            }
            (Type::FunctionPointer { parameters: p1, return_type: r1 },
             Type::Lambda { parameters: p2, return_type: r2 }) => {
                p1.len() == p2.len() &&
                p1.iter().zip(p2.iter()).all(|(a, b)| a.is_compatible_with_env(b, type_env)) &&
                r1.is_compatible_with_env(r2, type_env)
            }
            (Type::Lambda { parameters: p1, return_type: r1 },
             Type::FunctionPointer { parameters: p2, return_type: r2 }) => {
                p1.len() == p2.len() &&
                p1.iter().zip(p2.iter()).all(|(a, b)| a.is_compatible_with_env(b, type_env)) &&
                r1.is_compatible_with_env(r2, type_env)
            }

            // Reference type compatibility
            (Type::Reference { target_type: t1, .. }, Type::Reference { target_type: t2, .. }) => {
                t1.is_compatible_with_env(t2, type_env)
            }
            (Type::MutableReference { target_type: t1, .. }, Type::MutableReference { target_type: t2, .. }) => {
                t1.is_compatible_with_env(t2, type_env)
            }
            // Mutable reference can be used as immutable reference (coercion)
            (Type::Reference { target_type: t1, .. }, Type::MutableReference { target_type: t2, .. }) => {
                t1.is_compatible_with_env(t2, type_env)
            }

            // Raw pointer compatibility
            (Type::RawPointer { target_type: t1, is_mutable: m1 }, Type::RawPointer { target_type: t2, is_mutable: m2 }) => {
                t1.is_compatible_with_env(t2, type_env) && m1 == m2
            }

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
            Type::LibraryFunction { return_type } => {
                write!(f, "lib_fn(...) -> {}", return_type)
            }
            Type::Custom(name) => write!(f, "{}", name),
            Type::Unknown => write!(f, "?"),
            Type::Void => write!(f, "void"),
            Type::Enum(name) => write!(f, "{}", name),
            Type::Struct { name, .. } => write!(f, "{}", name),
            Type::Class { name, .. } => write!(f, "{}", name),
            Type::Trait { name, .. } => write!(f, "{}", name),
            Type::Generic { base_name, type_arguments } => {
                let params: Vec<String> = type_arguments.iter().map(|p| p.to_string()).collect();
                write!(f, "{}<{}>", base_name, params.join(", "))
            }
            Type::Lambda { parameters, return_type } => {
                let params: Vec<String> = parameters.iter().map(|p| p.to_string()).collect();
                write!(f, "|{}| -> {}", params.join(", "), return_type)
            }
            Type::FunctionPointer { parameters, return_type } => {
                let params: Vec<String> = parameters.iter().map(|p| p.to_string()).collect();
                write!(f, "fn({}) -> {}", params.join(", "), return_type)
            }
            Type::TypeParameter(name) => write!(f, "{}", name),
            Type::ArrowFunction { parameters, return_type } => {
                write!(f, "(")?;
                for (i, param) in parameters.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") => {}", return_type)
            }
            Type::Any => write!(f, "any"),
            Type::Reference { target_type, lifetime } => {
                if let Some(lifetime) = lifetime {
                    write!(f, "&'{} {}", lifetime, target_type)
                } else {
                    write!(f, "&{}", target_type)
                }
            }
            Type::MutableReference { target_type, lifetime } => {
                if let Some(lifetime) = lifetime {
                    write!(f, "&'{}mut {}", lifetime, target_type)
                } else {
                    write!(f, "&mut {}", target_type)
                }
            }
            Type::RawPointer { target_type, is_mutable } => {
                if *is_mutable {
                    write!(f, "*mut {}", target_type)
                } else {
                    write!(f, "*const {}", target_type)
                }
            }
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
            TypeAnnotation::Enum(name) => Type::Enum(name),
            TypeAnnotation::Trait(name) => Type::Trait {
                name,
                super_traits: Vec::new(),
                methods: Vec::new(), // Will be filled during semantic analysis
                default_methods: Vec::new(),
            },
            TypeAnnotation::Generic { name, parameters } => Type::Generic {
                base_name: name,
                type_arguments: parameters.into_iter().map(Type::from).collect(),
            },
            TypeAnnotation::Function { parameters, return_type } => Type::Lambda {
                parameters: parameters.into_iter().map(Type::from).collect(),
                return_type: Box::new(Type::from(*return_type)),
            },
            TypeAnnotation::FunctionPointer { parameters, return_type } => Type::FunctionPointer {
                parameters: parameters.into_iter().map(Type::from).collect(),
                return_type: Box::new(Type::from(*return_type)),
            },
            TypeAnnotation::TypeParameter(name) => Type::TypeParameter(name),
            TypeAnnotation::Reference { target_type, lifetime } => Type::Reference {
                target_type: Box::new(Type::from(*target_type)),
                lifetime,
            },
            TypeAnnotation::MutableReference { target_type, lifetime } => Type::MutableReference {
                target_type: Box::new(Type::from(*target_type)),
                lifetime,
            },
            TypeAnnotation::RawPointer { target_type, is_mutable } => Type::RawPointer {
                target_type: Box::new(Type::from(*target_type)),
                is_mutable,
            },
        }
    }
}

/// Namespace structure for nested namespaces
#[derive(Debug, Clone)]
pub struct NamespaceInfo {
    pub functions: HashMap<String, Type>,
    pub variables: HashMap<String, Type>,
    pub nested_namespaces: HashMap<String, NamespaceInfo>,
}

impl NamespaceInfo {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            variables: HashMap::new(),
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
    pub types: HashMap<String, Type>, // User-defined types (enums, structs, traits)
    friend_relationships: HashMap<String, Vec<String>>, // class_name -> list of friend classes
}

impl TypeEnvironment {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()], // Global scope
            functions: HashMap::new(),
            namespaces: HashMap::new(),
            types: HashMap::new(),
            friend_relationships: HashMap::new(),
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

    /// Get variable type (alias for lookup_variable)
    pub fn get_variable(&self, name: &str) -> Option<&Type> {
        self.lookup_variable(name)
    }
    
    /// Define a function
    pub fn define_function(&mut self, name: String, func_type: Type) -> Result<(), String> {
        if self.functions.contains_key(&name) {
            return Err(format!("Function '{}' already defined", name));
        }
        self.functions.insert(name, func_type);
        Ok(())
    }

    /// Define a function in a specific namespace
    pub fn define_function_in_namespace(&mut self, namespace: &str, name: String, func_type: Type) -> Result<(), String> {
        // Create the namespace if it doesn't exist
        if !self.namespaces.contains_key(namespace) {
            self.namespaces.insert(namespace.to_string(), NamespaceInfo::new());
        }

        // Get the namespace and define the function
        if let Some(ns_info) = self.namespaces.get_mut(namespace) {
            if ns_info.functions.contains_key(&name) {
                return Err(format!("Function '{}' already defined in namespace '{}'", name, namespace));
            }
            ns_info.functions.insert(name, func_type);
            Ok(())
        } else {
            Err(format!("Failed to access namespace '{}'", namespace))
        }
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
        // Only create the namespace if it doesn't already exist
        if !self.namespaces.contains_key(&name) {
            self.namespaces.insert(name, NamespaceInfo::new());
        }
    }

    /// Check if a namespace exists
    pub fn has_namespace(&self, name: &str) -> bool {
        let result = self.namespaces.contains_key(name);
        println!("🔧 TypeEnv: has_namespace('{}') = {}, available namespaces: {:?}",
                 name, result, self.namespaces.keys().collect::<Vec<_>>());
        result
    }

    /// Define an item in a namespace
    pub fn define_in_namespace(&mut self, namespace: &str, name: String, item_type: Type) -> Result<(), String> {
        if let Some(namespace_info) = self.namespaces.get_mut(namespace) {
            namespace_info.functions.insert(name, item_type);
            Ok(())
        } else {
            Err(format!("Namespace '{}' not found", namespace))
        }
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
        let namespace_info = self.namespaces.get(namespace)?;
        // First try to find a function, then a variable
        namespace_info.functions.get(name)
            .or_else(|| namespace_info.variables.get(name))
    }

    /// Look up item in nested namespace
    pub fn lookup_in_nested_namespace(&self, namespace_path: &[String], name: &str) -> Option<&Type> {
        if namespace_path.is_empty() {
            println!("🔧 TypeEnv: lookup_in_nested_namespace - empty namespace path");
            return None;
        }

        println!("🔧 TypeEnv: lookup_in_nested_namespace - looking for '{}' in namespace path {:?}", name, namespace_path);
        println!("🔧 TypeEnv: Available namespaces: {:?}", self.namespaces.keys().collect::<Vec<_>>());

        // First try to find the full namespace path as a single namespace
        let full_namespace = namespace_path.join("::");
        println!("🔧 TypeEnv: Trying full namespace: '{}'", full_namespace);
        if let Some(namespace_info) = self.namespaces.get(&full_namespace) {
            println!("🔧 TypeEnv: Found full namespace '{}' with {} functions", full_namespace, namespace_info.functions.len());
            println!("🔧 TypeEnv: Functions in namespace: {:?}", namespace_info.functions.keys().collect::<Vec<_>>());
            let result = namespace_info.functions.get(name)
                .or_else(|| namespace_info.variables.get(name));
            if result.is_some() {
                println!("🔧 TypeEnv: Found function '{}' in full namespace '{}'", name, full_namespace);
            } else {
                println!("🔧 TypeEnv: Function '{}' not found in full namespace '{}'", name, full_namespace);
            }
            return result;
        }

        println!("🔧 TypeEnv: Full namespace '{}' not found, trying hierarchical lookup", full_namespace);

        // Fallback to hierarchical lookup
        let root_name = &namespace_path[0];
        let mut current_namespace = self.namespaces.get(root_name)?;

        // Navigate to the nested namespace
        for ns_name in &namespace_path[1..] {
            current_namespace = current_namespace.nested_namespaces.get(ns_name)?;
        }

        // First try to find a function, then a variable
        current_namespace.functions.get(name)
            .or_else(|| current_namespace.variables.get(name))
    }
    
    /// Get all namespaces
    pub fn get_namespaces(&self) -> &HashMap<String, NamespaceInfo> {
        &self.namespaces
    }

    /// Get all namespaces (mutable)
    pub fn get_namespaces_mut(&mut self) -> &mut HashMap<String, NamespaceInfo> {
        &mut self.namespaces
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

    /// Get variables from a specific namespace path
    pub fn get_namespace_variables(&self, namespace_path: &[String]) -> Option<&HashMap<String, Type>> {
        if namespace_path.is_empty() {
            return None;
        }

        let root_name = &namespace_path[0];
        let mut current_namespace = self.namespaces.get(root_name)?;

        // Navigate to the nested namespace
        for ns_name in &namespace_path[1..] {
            current_namespace = current_namespace.nested_namespaces.get(ns_name)?;
        }

        Some(&current_namespace.variables)
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
        let path_parts: Vec<&str> = namespace_path.split("::").collect();
        if path_parts.is_empty() {
            return Err("Empty namespace path".to_string());
        }

        let root_name = path_parts[0];

        // Ensure root namespace exists
        if !self.namespaces.contains_key(root_name) {
            self.namespaces.insert(root_name.to_string(), NamespaceInfo::new());
        }

        // Navigate to the target namespace, creating nested namespaces as needed
        let mut current_namespace = self.namespaces.get_mut(root_name).unwrap();
        for ns_name in &path_parts[1..] {
            if !current_namespace.nested_namespaces.contains_key(*ns_name) {
                current_namespace.nested_namespaces.insert(ns_name.to_string(), NamespaceInfo::new());
            }
            current_namespace = current_namespace.nested_namespaces.get_mut(*ns_name).unwrap();
        }

        // Define the variable in the target namespace
        current_namespace.variables.insert(name, var_type);
        Ok(())
    }

    /// Define a new type (enum, struct, trait)
    pub fn define_type(&mut self, name: String, type_def: Type) -> Result<(), String> {
        if self.types.contains_key(&name) {
            return Err(format!("Type '{}' is already defined", name));
        }

        self.types.insert(name, type_def);
        Ok(())
    }

    /// Update an existing type (used during analysis phase to complete type information)
    pub fn update_type(&mut self, name: String, type_def: Type) {
        self.types.insert(name, type_def);
    }

    /// Get a type definition
    pub fn get_type(&self, name: &str) -> Option<&Type> {
        self.types.get(name)
    }

    /// Check if a type exists
    pub fn type_exists(&self, name: &str) -> bool {
        self.types.contains_key(name)
    }

    /// Get all defined types
    pub fn get_all_types(&self) -> &HashMap<String, Type> {
        &self.types
    }

    /// Resolve type annotation to concrete type
    pub fn resolve_type_annotation(&self, annotation: &TypeAnnotation) -> Type {
        match annotation {
            TypeAnnotation::Int => Type::Int,
            TypeAnnotation::Long => Type::Long,
            TypeAnnotation::Float(size) => Type::Float(size.unwrap_or(32)),
            TypeAnnotation::String => Type::String,
            TypeAnnotation::Num(size) => Type::Num(size.unwrap_or(32)),
            TypeAnnotation::Bool => Type::Bool,
            TypeAnnotation::Array(inner) => {
                let inner_type = self.resolve_type_annotation(inner);
                Type::Array(Box::new(inner_type))
            }
            TypeAnnotation::Obj => Type::Obj,
            TypeAnnotation::Auto => Type::Auto(Box::new(Type::Unknown)),
            TypeAnnotation::Null => Type::Null,
            TypeAnnotation::Custom(name) => {
                // Look up custom type
                if let Some(custom_type) = self.get_type(name) {
                    custom_type.clone()
                } else {
                    Type::Custom(name.clone())
                }
            }
            TypeAnnotation::Void => Type::Void,
            TypeAnnotation::Enum(name) => Type::Enum(name.clone()),
            TypeAnnotation::Trait(name) => {
                if let Some(trait_type) = self.get_type(name) {
                    trait_type.clone()
                } else {
                    Type::Trait {
                        name: name.clone(),
                        super_traits: Vec::new(),
                        methods: Vec::new(),
                        default_methods: Vec::new(),
                    }
                }
            }
            TypeAnnotation::Generic { name, parameters } => {
                let param_types: Vec<Type> = parameters.iter()
                    .map(|p| self.resolve_type_annotation(p))
                    .collect();
                Type::Generic {
                    base_name: name.clone(),
                    type_arguments: param_types,
                }
            }
            TypeAnnotation::TypeParameter(name) => Type::TypeParameter(name.clone()),
            TypeAnnotation::Function { parameters, return_type } => {
                let param_types: Vec<Type> = parameters.iter()
                    .map(|p| self.resolve_type_annotation(p))
                    .collect();
                let ret_type = self.resolve_type_annotation(return_type);
                Type::Function {
                    parameters: param_types,
                    return_type: Box::new(ret_type),
                }
            }
            TypeAnnotation::FunctionPointer { parameters, return_type } => {
                let param_types: Vec<Type> = parameters.iter()
                    .map(|p| self.resolve_type_annotation(p))
                    .collect();
                let ret_type = self.resolve_type_annotation(return_type);
                Type::FunctionPointer {
                    parameters: param_types,
                    return_type: Box::new(ret_type),
                }
            }
            TypeAnnotation::Reference { target_type, lifetime } => Type::Reference {
                target_type: Box::new(self.resolve_type_annotation(target_type)),
                lifetime: lifetime.clone(),
            },
            TypeAnnotation::MutableReference { target_type, lifetime } => Type::MutableReference {
                target_type: Box::new(self.resolve_type_annotation(target_type)),
                lifetime: lifetime.clone(),
            },
            TypeAnnotation::RawPointer { target_type, is_mutable } => Type::RawPointer {
                target_type: Box::new(self.resolve_type_annotation(target_type)),
                is_mutable: *is_mutable,
            },
        }
    }

    /// Resolve optional type annotation to concrete type
    pub fn resolve_optional_type_annotation(&self, annotation: &Option<TypeAnnotation>) -> Type {
        if let Some(type_annotation) = annotation {
            self.resolve_type_annotation(type_annotation)
        } else {
            Type::Unknown
        }
    }

    /// Alias for enter_scope
    pub fn push_scope(&mut self) {
        self.enter_scope();
    }

    /// Alias for exit_scope
    pub fn pop_scope(&mut self) {
        self.exit_scope();
    }

    /// Check if class1 is a subclass of class2
    pub fn is_subclass(&self, class1: &str, class2: &str) -> bool {
        if let Some(Type::Class { base_class: Some(base), .. }) = self.get_type(class1) {
            if base == class2 {
                return true;
            }
            // Recursively check inheritance chain
            return self.is_subclass(base, class2);
        }
        false
    }

    /// Get all methods available to a class (including inherited methods)
    pub fn get_class_methods(&self, class_name: &str) -> Vec<(String, Type)> {
        let mut methods = Vec::new();

        // Get methods from current class
        if let Some(Type::Class { methods: class_methods, base_class, .. }) = self.get_type(class_name) {
            methods.extend(class_methods.clone());

            // Get methods from base class
            if let Some(base_name) = base_class {
                let base_methods = self.get_class_methods(base_name);
                // Add base methods that aren't overridden
                for (method_name, method_type) in base_methods {
                    if !methods.iter().any(|(name, _)| name == &method_name) {
                        methods.push((method_name, method_type));
                    }
                }
            }
        }

        methods
    }

    /// Get all fields available to a class (including inherited fields)
    pub fn get_class_fields(&self, class_name: &str) -> Vec<(String, Type)> {
        let mut fields = Vec::new();

        // Get fields from current class
        if let Some(Type::Class { fields: class_fields, base_class, .. }) = self.get_type(class_name) {
            fields.extend(class_fields.clone());

            // Get fields from base class
            if let Some(base_name) = base_class {
                let base_fields = self.get_class_fields(base_name);
                // Add base fields that aren't shadowed
                for (field_name, field_type) in base_fields {
                    if !fields.iter().any(|(name, _)| name == &field_name) {
                        fields.push((field_name, field_type));
                    }
                }
            }
        }

        fields
    }

    /// Check if a method signature is compatible for overriding (inheritance-aware)
    pub fn is_method_override_compatible(&self, derived_method: &Type, base_method: &Type) -> bool {
        match (derived_method, base_method) {
            (Type::Function { parameters: p1, return_type: r1 },
             Type::Function { parameters: p2, return_type: r2 }) => {
                // Parameter counts must match
                if p1.len() != p2.len() {
                    return false;
                }

                // Check parameter compatibility (contravariant for method parameters)
                for (derived_param, base_param) in p1.iter().zip(p2.iter()) {
                    if !self.is_parameter_override_compatible(derived_param, base_param) {
                        return false;
                    }
                }

                // Return types must be compatible (covariant for return types)
                r1.is_compatible_with(r2)
            }
            _ => derived_method.is_compatible_with(base_method)
        }
    }

    /// Check if a parameter type is compatible for method overriding
    fn is_parameter_override_compatible(&self, derived_param: &Type, base_param: &Type) -> bool {
        match (derived_param, base_param) {
            // For class types, check inheritance relationship
            (Type::Class { name: derived_class, .. }, Type::Class { name: base_class, .. }) => {
                // Derived class parameter can be the same class or a subclass of the base class parameter
                derived_class == base_class || self.is_subclass(derived_class, base_class)
            }
            // For other types, use regular compatibility
            _ => derived_param.is_compatible_with(base_param)
        }
    }

    /// Add a friend relationship between classes
    pub fn add_friend_relationship(&mut self, class_name: String, friend_class: String) {
        self.friend_relationships
            .entry(class_name)
            .or_insert_with(Vec::new)
            .push(friend_class);
    }

    /// Check if one class is a friend of another
    pub fn is_friend_class(&self, class_name: &str, potential_friend: &str) -> bool {
        if let Some(friends) = self.friend_relationships.get(class_name) {
            friends.contains(&potential_friend.to_string())
        } else {
            false
        }
    }

    /// Get all friend classes for a given class
    pub fn get_friend_classes(&self, class_name: &str) -> Vec<String> {
        self.friend_relationships
            .get(class_name)
            .cloned()
            .unwrap_or_default()
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

    /// Check if two types can be unified with type environment for inheritance checking
    pub fn can_unify_with_env(&self, expected: &Type, actual: &Type, type_env: Option<&TypeEnvironment>) -> bool {
        expected.is_compatible_with_env(actual, type_env)
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
