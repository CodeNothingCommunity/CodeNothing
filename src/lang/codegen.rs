//! Code generator for CodeNothing language
//!
//! Generates bytecode from analyzed AST

use crate::core::bytecode::{Chunk, Instruction, OpCode};
use crate::core::value::Value;
use crate::lang::ast::*;
use crate::lang::types::{Type, TypeEnvironment};
use crate::lang::builtins::BuiltinRegistry;
use std::collections::{HashMap, HashSet};

/// Constants for code generation
mod constants {
    use crate::core::value::Value;
    use crate::lang::types::Type;

    // Namespace and function constants
    pub const NAMESPACE_SEPARATOR: &str = "::";
    pub const MAIN_FUNCTION_NAME: &str = "main";
    pub const MAIN_CHUNK_INDEX: usize = 0;
    pub const BUILTIN_CHUNK_INDEX: usize = 0;

    // Default values
    pub const DEFAULT_PARAM_COUNT: usize = 1;
    pub const INSTRUCTION_OFFSET: usize = 1;
    pub const DECREMENT_VALUE: usize = 1;

    // Loop constants
    pub const LOOP_INCREMENT_VALUE: Value = Value::Int(1);
    pub const LOOP_INITIAL_INDEX: Value = Value::Int(0);
    pub const LOOP_VARIABLE_TYPE: Type = Type::Int;

    // String constants
    pub const PARSE_ERROR_PLACEHOLDER: &str = "${PARSE_ERROR}";
    pub const EMPTY_STRING: &str = "";

    // Type constants
    pub const DEFAULT_GLOBAL_VAR_TYPE: Type = Type::Unknown;

    // Instruction operand constants
    pub const DEFAULT_OPERAND: u32 = 0;
    pub const JUMP_PLACEHOLDER: u16 = 0;

    // Index constants
    pub const FIRST_INDEX: usize = 0;
    pub const SECOND_INDEX: usize = 1;
    pub const THIRD_INDEX: usize = 2;
    pub const SELF_PARAM_INDEX: u8 = 0;

    // Increment/decrement constants
    pub const INCREMENT_VALUE: i64 = 1;
    pub const DECREMENT_AMOUNT: i64 = 1;
    pub const SINGLE_PARAM_COUNT: i64 = 1;
    pub const TWO_PARAM_COUNT: usize = 2;
    pub const PARAM_COUNT_OFFSET: usize = 1;
    pub const SELF_PARAM_COUNT: usize = 1;

    // Boolean constants
    pub const IS_GLOBAL: bool = true;
    pub const IS_LOCAL: bool = false;
    pub const INSIDE_FUNCTION: bool = true;
    pub const OUTSIDE_FUNCTION: bool = false;

    // Special identifiers
    pub const SELF_IDENTIFIER: &str = "self";
    pub const DESTRUCTOR_PREFIX: &str = "~";

    // Reference type strings
    pub const MUTABLE_GLOBAL_REF: &str = "mutable_global";
    pub const GLOBAL_REF: &str = "global";
    pub const MUTABLE_LOCAL_REF: &str = "mutable_local";
    pub const LOCAL_REF: &str = "local";

    // Closure constants
    pub const CLOSURE_PLACEHOLDER: &str = "[Closure]";
    pub const LAMBDA_PREFIX: &str = "lambda_";
    pub const PIPE_CHAR: char = '|';
    pub const MIN_PIPE_COUNT: usize = 2;
    pub const CLOSURE_CALL_PATTERN: &str = ")(";
    pub const CLOSURE_CALL_OFFSET: usize = 2;

    // Arrow function constants
    pub const ARROW_SYNTAX: &str = " => ";
    pub const COMPLEX_EXPRESSION_PLACEHOLDER: &str = "{ complex_expression }";
    pub const BLOCK_START: &str = "{ ";
    pub const BLOCK_END: &str = " }";
    pub const EMPTY_BLOCK: &str = "{ }";
    pub const SINGLE_STATEMENT_PLACEHOLDER: &str = "{ single_statement }";
    pub const STATEMENT_SEPARATOR: &str = "; ";
    pub const PARAM_SEPARATOR: &str = ", ";
    pub const OPEN_PAREN: char = '(';
    pub const CLOSE_PAREN: char = ')';
    pub const MAX_SIMPLE_EXPR_LENGTH: usize = 50;
    pub const MAX_DEBUG_EXPR_LENGTH: usize = 100;

    // Expression string representations
    pub const NULL_STRING: &str = "null";
    pub const STRING_FORMAT: &str = "\"{}\"";
    pub const EMPTY_CALL: &str = "()";
    pub const COMPLEX_CALL: &str = "(...)";
    pub const ARROW_FUNCTION_STR: &str = "arrow_function";
    pub const WHEN_EXPRESSION_STR: &str = "when_expression";
    pub const TERNARY_EXPRESSION_STR: &str = "ternary_expression";
    pub const EXPRESSION_STR: &str = "expression";
    pub const STATEMENT_STR: &str = "statement";
    pub const RETURN_PREFIX: &str = "return ";
    pub const RETURN_STR: &str = "return";
    pub const VAR_DECL_FORMAT: &str = "var {} = {}";
    pub const VAR_DECL_NO_INIT_FORMAT: &str = "var {}";
    pub const IF_STATEMENT_STR: &str = "if_statement";
    pub const WHILE_LOOP_STR: &str = "while_loop";
    pub const FOR_LOOP_STR: &str = "for_loop";
    pub const BLOCK_FORMAT: &str = "block({}_statements)";

    // Closure pattern constants
    pub const CLOSURE_PATTERN_X_PLUS_1: &str = "|x| x + 1";
    pub const CLOSURE_PATTERN_X_TIMES_2: &str = "|x| x * 2";
    pub const CLOSURE_PATTERN_X_MINUS_1: &str = "|x| x - 1";
    pub const CLOSURE_PATTERN_X_TIMES_2_PLUS_1: &str = "|x| x * 2 + 1";
    pub const CLOSURE_PATTERN_X_SQUARED: &str = "|x| x * x";
    pub const CLOSURE_PATTERN_X_CUBED: &str = "|x| x * x * x";
    pub const CLOSURE_PATTERN_X_PLUS_Y: &str = "x + y";
    pub const CLOSURE_PATTERN_X_TIMES_Y: &str = "x * y";
    pub const CLOSURE_PATTERN_X_MINUS_Y: &str = "x - y";
    pub const CLOSURE_PATTERN_X: &str = "x";
    pub const CLOSURE_BODY_SEPARATOR: &str = "| ";
    pub const CLOSURE_BODY_OFFSET: usize = 2;
    pub const PLUS_OPERATOR: &str = "+";
    pub const TIMES_OPERATOR: &str = "*";
    pub const MINUS_OPERATOR: &str = "-";
    pub const COMPLEX_TIMES_PATTERN: &str = "* x *";

    // Error message constants
    pub const UNTERMINATED_STRING_ERROR: &str = "Unterminated string";
    pub const UNEXPECTED_CHAR_ERROR: &str = "Unexpected character '\\'";
    pub const EMPTY_ERROR: &str = "Empty";
    pub const UNEXPECTED_TOKEN_ERROR: &str = "Unexpected token";

    // 常用字符串常量，避免重复的 to_string() 调用
    pub const SUPER_FIELD_ASSIGNMENT_ERROR: &str = "Super field assignment requires 'self' context";
    pub const SUPER_METHOD_CALL_ERROR: &str = "Super method call requires 'self' context";
    pub const SUPER_FIELD_ACCESS_ERROR: &str = "Super field access requires 'self' context";
    pub const UNSUPPORTED_ASSIGNMENT_TARGET: &str = "Unsupported assignment target";
    pub const UNSUPPORTED_COMPOUND_ASSIGNMENT: &str = "Unsupported assignment target for compound assignment";
    pub const COMPLEX_FUNCTION_EXPRESSIONS_ERROR: &str = "Complex function expressions not yet supported";
    pub const ENUM_VARIANTS_ERROR: &str = "Enum variants not yet supported in codegen";
    pub const ADDRESS_OF_VARIABLES_ONLY: &str = "Address-of operation only supported for variables";
    pub const FOR_RANGE_REQUIRES_RANGE: &str = "ForRange statement requires a range expression";
    pub const SELF_PARAMETER: &str = "self";

    // 表达式和语句类型字符串
    pub const ARROW_FUNCTION_TYPE: &str = "arrow_function";
    pub const WHEN_EXPRESSION_TYPE: &str = "when_expression";
    pub const TERNARY_EXPRESSION_TYPE: &str = "ternary_expression";
    pub const EXPRESSION_TYPE: &str = "expression";
    pub const STATEMENT_TYPE: &str = "statement";
    pub const RETURN_TYPE: &str = "return";
    pub const IF_STATEMENT_TYPE: &str = "if_statement";
    pub const WHILE_LOOP_TYPE: &str = "while_loop";
    pub const FOR_LOOP_TYPE: &str = "for_loop";
    pub const EMPTY_BLOCK_TYPE: &str = "{}";
    pub const NULL_TYPE: &str = "null";
    pub const CLOSURE_PLACEHOLDER_TYPE: &str = "[Closure]";
}

/// Code generation errors
#[derive(Debug, Clone)]
pub struct CodeGenError {
    pub message: String,
    pub position: Position,
}

impl CodeGenError {
    pub fn new(message: String, position: Position) -> Self {
        Self { message, position }
    }
}

impl std::fmt::Display for CodeGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Code generation error at {}:{}: {}", 
               self.position.line, self.position.column, self.message)
    }
}

impl std::error::Error for CodeGenError {}

/// Variable information for code generation
#[derive(Debug, Clone)]
struct VariableInfo {
    index: usize,
    is_global: bool,
    var_type: Type,
}

/// Function information for code generation
#[derive(Debug, Clone)]
struct FunctionInfo {
    chunk_index: usize,
    parameter_count: usize,
    return_type: Type,
}

/// Class information for code generation
#[derive(Debug, Clone)]
struct ClassInfo {
    class_index: usize,
    has_constructor: bool,
    field_count: usize,
}

/// Code generator
pub struct CodeGenerator {
    chunks: Vec<Chunk>,
    current_chunk: usize,
    current_namespace: Option<String>,
    variables: HashMap<String, VariableInfo>,
    functions: HashMap<String, FunctionInfo>,
    classes: HashMap<String, ClassInfo>,
    class_inheritance: HashMap<String, String>, // child_class -> parent_class
    class_definitions: HashMap<String, crate::core::vm::types::ClassDefinition>, // class definitions for VM
    builtins: BuiltinRegistry,
    type_env: TypeEnvironment,
    local_count: usize,
    global_count: usize,
    errors: Vec<CodeGenError>,
    break_jumps: Vec<usize>,    // Jump addresses for break statements
    continue_jumps: Vec<usize>, // Jump addresses for continue statements
    memo_functions: HashSet<String>, // Names of memoized functions
    library_dependencies: HashSet<String>, // Library dependencies for bytecode
    debug_mode: bool,           // Debug mode flag
    show_all_details: bool,     // Show full details for arrow functions
    inside_function: bool,      // Track whether we're inside a function
}

impl CodeGenerator {
    pub fn new() -> Self {
        let mut generator = Self {
            chunks: vec![Chunk::new(constants::MAIN_FUNCTION_NAME.to_string())],
            current_chunk: constants::MAIN_CHUNK_INDEX,
            current_namespace: None,
            variables: HashMap::new(),
            functions: HashMap::new(),
            classes: HashMap::new(),
            class_inheritance: HashMap::new(),
            class_definitions: HashMap::new(),
            builtins: BuiltinRegistry::new(),
            type_env: TypeEnvironment::new(),
            local_count: 0,
            global_count: 0,
            errors: Vec::new(),
            break_jumps: Vec::new(),
            continue_jumps: Vec::new(),
            memo_functions: HashSet::new(),
            library_dependencies: HashSet::new(),
            debug_mode: false,
            show_all_details: false,
            inside_function: false,
        };
        
        // Populate type environment with built-ins
        generator.builtins.populate_type_environment(&mut generator.type_env);
        
        generator
    }

    /// Set debug mode
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
    }

    /// Set show all details mode for arrow functions
    pub fn set_show_all_details(&mut self, enabled: bool) {
        self.show_all_details = enabled;
    }

    /// Set the type environment (usually from semantic analysis)
    pub fn set_type_environment(&mut self, type_env: TypeEnvironment) {
        self.type_env = type_env;
    }

    /// Get errors from code generation
    pub fn get_errors(&self) -> &Vec<CodeGenError> {
        &self.errors
    }

    /// Get generated chunks
    pub fn get_chunks(&self) -> HashMap<String, Chunk> {
        let mut chunks_map = HashMap::with_capacity(self.chunks.len());
        for chunk in &self.chunks {
            // Use the actual chunk name instead of generic chunk_N
            chunks_map.insert(chunk.name.clone(), chunk.clone());
        }
        chunks_map
    }

    /// Get chunks in their original compilation order
    pub fn get_chunks_ordered(&self) -> Vec<(String, Chunk)> {
        let mut result = Vec::with_capacity(self.chunks.len());
        for chunk in &self.chunks {
            result.push((chunk.name.clone(), chunk.clone()));
        }
        result
    }

    /// Get class inheritance relationships
    pub fn get_class_inheritance(&self) -> &HashMap<String, String> {
        &self.class_inheritance
    }

    /// Get class definitions
    pub fn get_class_definitions(&self) -> &HashMap<String, crate::core::vm::types::ClassDefinition> {
        &self.class_definitions
    }

    /// Determine if a method call should use virtual dispatch
    fn should_use_virtual_call(&self, object_expr: &Expression, method_name: &str) -> bool {
        // Try to determine the object's type from the expression
        let object_type = self.infer_expression_type(object_expr);

        match object_type {
            Type::Class { name, .. } => {
                // Check if this class has the method marked as virtual
                self.is_method_virtual(&name, method_name)
            }
            _ => {
                // For non-class types or unknown types, default to virtual dispatch
                // This ensures polymorphism works even when we can't statically determine the type
                true
            }
        }
    }

    /// Check if a method is virtual in a given class
    fn is_method_virtual(&self, class_name: &str, method_name: &str) -> bool {
        // Constructors are never virtual
        if method_name == class_name {
            return false;
        }

        // Check if we have class definition with method information
        if let Some(class_def) = self.class_definitions.get(class_name) {
            // Look for the method in the class definition's VTable or method list
            // For now, we use a heuristic: methods are virtual unless explicitly static
            // This matches the behavior expected by the VM's VTable system
            return !class_def.methods.is_empty(); // If class has methods, assume they can be virtual
        }

        // Check if we have basic class information
        if let Some(_class_info) = self.classes.get(class_name) {
            // For classes we know about but don't have full definitions,
            // treat all non-constructor methods as potentially virtual (safe default)
            return true;
        }

        // Default to virtual for unknown classes (safe for polymorphism)
        true
    }

    /// Infer the type of an expression (simplified version)
    fn infer_expression_type(&self, expr: &Expression) -> Type {
        match expr {
            Expression::Identifier(name, _) => {
                // Try to get variable type from current scope
                if let Some(var_info) = self.variables.get(name) {
                    var_info.var_type.clone()
                } else {
                    Type::Unknown
                }
            }
            Expression::FieldAccess { object, field, .. } => {
                // For field access, try to infer from the object type
                let object_type = self.infer_expression_type(object);
                match object_type {
                    Type::Class { name, .. } => {
                        // Look up field type in class definition using TypeEnvironment
                        let class_fields = self.type_env.get_class_fields(&name);
                        for (field_name, field_type) in class_fields {
                            if field_name == *field {
                                return field_type;
                            }
                        }
                        Type::Unknown
                    }
                    Type::Struct { fields, .. } => {
                        // For structs, look up field type directly
                        for (field_name, field_type) in fields {
                            if field_name == *field {
                                return field_type;
                            }
                        }
                        Type::Unknown
                    }
                    _ => Type::Unknown
                }
            }
            Expression::MethodCall { object, .. } => {
                // For method calls, try to infer return type
                // This is complex, so for now return Unknown
                Type::Unknown
            }
            _ => Type::Unknown
        }
    }

    /// Collect all functions from a program (first pass only)
    pub fn collect_functions(&mut self, program: &Program) {
        // First pass: collect function declarations only
        for declaration in &program.declarations {
            match declaration {
                Declaration::Function(func) => {
                    self.collect_function(func);
                }
                Declaration::Namespace(ns) => {
                    // Collect functions from namespace recursively
                    self.collect_nested_namespace_functions(&[], ns);
                }
                Declaration::Package(pkg) => {
                    // Collect functions from package recursively
                    self.collect_package_functions(&[], pkg);
                }
                Declaration::Class(class_decl) => {
                    // Collect class methods
                    self.collect_class_methods(class_decl);
                }
                Declaration::Memo(memo) => {
                    // Collect memo functions
                    self.collect_memo_functions(memo);
                }
                _ => {
                    // Other declarations don't define functions
                }
            }
        }
    }

    /// Generate code for a complete program
    pub fn generate_program(&mut self, program: &Program) -> Result<Vec<Chunk>, Vec<CodeGenError>> {
        self.errors.clear();

        // First pass: collect function declarations and process global variables
        for declaration in &program.declarations {
            match declaration {
                Declaration::Function(func) => {
                    self.collect_function(func);
                }
                Declaration::Namespace(ns) => {
                    // Collect functions from namespace recursively
                    self.collect_nested_namespace_functions(&[], ns);
                }
                Declaration::Package(pkg) => {
                    // Collect functions from package recursively
                    self.collect_package_functions(&[], pkg);
                }
                Declaration::Variable(stmt) => {
                    // Process global variable declaration
                    if let Statement::VarDeclaration { scope, name, .. } = stmt {
                        // Only collect variables with Global scope
                        if matches!(scope, VarScope::Global) {
                            let var_info = VariableInfo {
                                index: self.global_count,
                                is_global: true,
                                var_type: constants::DEFAULT_GLOBAL_VAR_TYPE, // Will be determined later
                            };
                            if self.debug_mode {
                                println!("🔧 CodeGen: Collecting global variable '{}' at index {}", name, self.global_count);
                            }
                            self.variables.insert(name.clone(), var_info);
                            self.global_count += 1;
                        } else if self.debug_mode {
                            println!("🔧 CodeGen: Skipping non-global variable '{}' with scope {:?}", name, scope);
                        }
                    }
                }
                Declaration::Memo(memo) => {
                    // Collect functions from memo content
                    self.collect_memo_functions(memo);
                }
                Declaration::Impl(impl_decl) => {
                    // Collect methods from impl blocks
                    self.collect_impl_methods(impl_decl);
                }
                Declaration::Class(class_decl) => {
                    // Collect methods from class declarations
                    self.collect_class_methods(class_decl);
                }
                _ => {}
            }
        }

        // Second pass: generate global variable initialization code first
        self.current_chunk = 0; // Ensure we're in the main chunk
        for declaration in &program.declarations {
            self.generate_global_initializers(declaration);
        }

        // Third pass: generate code for all declarations
        for declaration in &program.declarations {
            self.generate_declaration(declaration);
        }
        
        // Add halt instruction to main function
        if let Some(main_info) = self.functions.get(constants::MAIN_FUNCTION_NAME).cloned() {
            let old_chunk = self.current_chunk;
            self.current_chunk = main_info.chunk_index;
            self.emit_instruction(Instruction::new(OpCode::Halt, 0));
            self.current_chunk = old_chunk;
        }
        
        // Set global count for main chunk
        if !self.chunks.is_empty() {
            self.chunks[constants::MAIN_CHUNK_INDEX].global_count = self.global_count;
        }

        if self.errors.is_empty() {
            Ok(self.chunks.clone())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    /// Generate global variable initializers (executed before main)
    fn generate_global_initializers(&mut self, declaration: &Declaration) {
        match declaration {
            Declaration::Variable(stmt) => {
                // Generate global variable initialization only for global scope variables
                if let Statement::VarDeclaration { scope, .. } = stmt {
                    if matches!(scope, VarScope::Global) {
                        self.generate_statement(stmt);
                    }
                }
            }
            Declaration::Namespace(ns) => {
                // Generate namespace variable initializers
                self.generate_namespace_initializers(&[], ns);
            }
            _ => {
                // Skip other declarations in this pass
            }
        }
    }

    /// Generate namespace variable initializers recursively
    fn generate_namespace_initializers(&mut self, parent_path: &[String], ns: &NamespaceDeclaration) {
        let mut current_path = parent_path.to_vec();
        current_path.push(ns.name.clone());

        // Save current namespace context
        let old_namespace = self.current_namespace.clone();
        self.current_namespace = Some(current_path.join(constants::NAMESPACE_SEPARATOR));

        for item in &ns.items {
            match item {
                NamespaceItem::Variable(stmt) => {
                    // Generate namespace variable initialization in main chunk
                    self.generate_statement(stmt);
                }
                NamespaceItem::Namespace(nested_ns) => {
                    // Recursively generate nested namespace variable initializers
                    self.generate_namespace_initializers(&current_path, nested_ns);
                }
                _ => {
                    // Skip functions in this pass
                }
            }
        }

        // Restore namespace context
        self.current_namespace = old_namespace;
    }

    /// Collect package functions recursively (first pass)
    fn collect_package_functions(&mut self, parent_path: &[String], pkg: &PackageDeclaration) {
        let mut current_path = parent_path.to_vec();
        current_path.push(pkg.name.clone());

        for item in &pkg.items {
            match item {
                PackageItem::Function(func) => {
                    let full_name = format!("{}::{}", current_path.join("::"), func.name);
                    let chunk_index = self.chunks.len();
                    let chunk = Chunk::new(full_name.clone());
                    self.chunks.push(chunk);

                    let return_type = func.return_type.as_ref()
                        .map(|t| Type::from(t.clone()))
                        .unwrap_or(Type::Null);

                    let func_info = FunctionInfo {
                        chunk_index,
                        parameter_count: func.parameters.len(),
                        return_type,
                    };

                    self.functions.insert(full_name, func_info);
                }
                PackageItem::Namespace(nested_ns) => {
                    // Recursively collect nested namespace functions
                    self.collect_nested_namespace_functions(&current_path, nested_ns);
                }
                PackageItem::Package(nested_pkg) => {
                    // Recursively collect nested package functions
                    self.collect_package_functions(&current_path, nested_pkg);
                }
                PackageItem::Variable(stmt) => {
                    // Handle package variables
                    if let Statement::VarDeclaration { name, type_annotation, .. } = stmt {
                        let full_name = format!("{}::{}", current_path.join("::"), name);
                        let var_type = type_annotation.as_ref()
                            .map(|t| Type::from(t.clone()))
                            .unwrap_or(Type::Auto(Box::new(Type::Unknown)));

                        let var_info = VariableInfo {
                            index: self.global_count,
                            is_global: true,
                            var_type,
                        };

                        if self.debug_mode {
                            println!("🔧 CodeGen: Collecting package variable '{}' at index {}", full_name, self.global_count);
                        }

                        self.variables.insert(full_name, var_info);
                        self.global_count += 1;
                    }
                }
                PackageItem::Enum(_) | PackageItem::Struct(_) | PackageItem::Class(_) |
                PackageItem::Trait(_) | PackageItem::Impl(_) => {
                    // These don't need collection in the first pass
                }
            }
        }
    }

    /// Collect nested namespace functions recursively (first pass)
    fn collect_nested_namespace_functions(&mut self, parent_path: &[String], ns: &NamespaceDeclaration) {
        let mut current_path = parent_path.to_vec();
        current_path.push(ns.name.clone());

        for item in &ns.items {
            match item {
                NamespaceItem::Function(func) => {
                    let full_name = format!("{}{}{}", current_path.join(constants::NAMESPACE_SEPARATOR), constants::NAMESPACE_SEPARATOR, func.name);
                    let chunk_index = self.chunks.len();
                    let chunk = Chunk::new(full_name.clone());
                    self.chunks.push(chunk);

                    let return_type = func.return_type.as_ref()
                        .map(|t| Type::from(t.clone()))
                        .unwrap_or(Type::Null);

                    let func_info = FunctionInfo {
                        chunk_index,
                        parameter_count: func.parameters.len(),
                        return_type,
                    };

                    self.functions.insert(full_name, func_info);
                }
                NamespaceItem::Namespace(nested_ns) => {
                    // Recursively collect nested namespace functions
                    self.collect_nested_namespace_functions(&current_path, nested_ns);
                }
                NamespaceItem::Variable(stmt) => {
                    // Handle namespace variables
                    if let Statement::VarDeclaration { name, type_annotation, .. } = stmt {
                        let full_name = format!("{}{}{}", current_path.join(constants::NAMESPACE_SEPARATOR), constants::NAMESPACE_SEPARATOR, name);
                        let var_type = type_annotation.as_ref()
                            .map(|t| Type::from(t.clone()))
                            .unwrap_or(Type::Auto(Box::new(Type::Unknown)));

                        let var_info = VariableInfo {
                            index: self.global_count,
                            is_global: true,
                            var_type,
                        };

                        if self.debug_mode {
                            println!("🔧 CodeGen: Storing namespace variable '{}' as '{}'", name, full_name);
                        }
                        self.variables.insert(full_name, var_info);
                        self.global_count += 1;
                    }
                }
                NamespaceItem::Enum(_enum_decl) => {
                    // Enums don't generate bytecode directly
                }
                NamespaceItem::Struct(_struct_decl) => {
                    // Structs don't generate bytecode directly
                }
                NamespaceItem::Trait(_trait_decl) => {
                    // Traits don't generate bytecode directly
                }
                NamespaceItem::Impl(_impl_decl) => {
                    // Impl blocks will be handled separately
                }
                NamespaceItem::Class(_class_decl) => {
                    // Classes will be handled separately
                }
            }
        }
    }

    /// Collect namespace function declaration (first pass)
    fn collect_namespace_function(&mut self, namespace: &str, function: &FunctionDeclaration) {
        let full_name = format!("{}{}{}", namespace, constants::NAMESPACE_SEPARATOR, function.name);
        let chunk_index = self.chunks.len();
        let chunk = Chunk::new(full_name.clone());
        self.chunks.push(chunk);

        let return_type = function.return_type.as_ref()
            .map(|t| Type::from(t.clone()))
            .unwrap_or(Type::Null);

        let func_info = FunctionInfo {
            chunk_index,
            parameter_count: function.parameters.len(),
            return_type,
        };

        self.functions.insert(full_name, func_info);
    }

    /// Collect function declaration (first pass)
    fn collect_function(&mut self, function: &FunctionDeclaration) {
        let chunk_index = if function.name == constants::MAIN_FUNCTION_NAME {
            // Use the existing main chunk created in new()
            constants::MAIN_CHUNK_INDEX
        } else {
            // Create a new chunk for other functions
            let chunk_index = self.chunks.len();
            let chunk = Chunk::new(function.name.clone());
            self.chunks.push(chunk);
            chunk_index
        };

        let return_type = function.return_type.as_ref()
            .map(|t| Type::from(t.clone()))
            .unwrap_or(Type::Null);

        let func_info = FunctionInfo {
            chunk_index,
            parameter_count: function.parameters.len(),
            return_type,
        };

        self.functions.insert(function.name.clone(), func_info);
    }
    
    /// Generate code for declaration
    fn generate_declaration(&mut self, declaration: &Declaration) {
        match declaration {
            Declaration::Function(func) => {
                self.generate_function(func);
            }
            Declaration::Namespace(ns) => {
                self.generate_nested_namespace_functions(&[], ns);
            }
            Declaration::Using(using_decl) => {
                // Import functions from the specified namespace
                // Create aliases for namespaced functions without the namespace prefix
                let namespace_prefix = format!("{}{}", using_decl.namespace, constants::NAMESPACE_SEPARATOR);

                // Get all builtin function names and create aliases for those in this namespace
                let builtin_names: Vec<String> = self.builtins.get_names().into_iter().cloned().collect();

                for builtin_name in builtin_names {
                    if builtin_name.starts_with(&namespace_prefix) {
                        // Extract the short name (without namespace prefix)
                        let short_name = match builtin_name.strip_prefix(&namespace_prefix) {
                            Some(name) => name.to_string(),
                            None => continue, // Skip if prefix removal fails
                        };

                        // Create a FunctionInfo entry for the short name that points to the builtin
                        // We'll handle this specially in function call generation
                        // Get the correct parameter count from the type environment
                        let param_count = if let Some(func_type) = self.type_env.lookup_in_namespace(&using_decl.namespace, &short_name) {
                            match func_type {
                                Type::Function { parameters, .. } => parameters.len(),
                                _ => 0,
                            }
                        } else {
                            // Default parameter count for unknown functions
                            constants::DEFAULT_PARAM_COUNT
                        };

                        let func_info = FunctionInfo {
                            chunk_index: constants::BUILTIN_CHUNK_INDEX, // Built-ins don't have chunks
                            parameter_count: param_count,
                            return_type: Type::Null, // Most built-ins return void
                        };

                        // Add the function without namespace prefix
                        self.functions.insert(short_name, func_info);
                    }
                }

                // Also handle user-defined namespace functions
                let namespace_functions: Vec<(String, FunctionInfo)> = self.functions.iter()
                    .filter(|(name, _)| name.starts_with(&namespace_prefix))
                    .map(|(name, info)| (name.clone(), info.clone()))
                    .collect();

                for (full_name, func_info) in namespace_functions {
                    if let Some(short_name) = full_name.strip_prefix(&namespace_prefix) {
                        // Create an alias for the function without the namespace prefix
                        self.functions.insert(short_name.to_string(), func_info);
                    }
                }
            }
            Declaration::Variable(stmt) => {
                self.generate_statement(stmt);
            }
            Declaration::Memo(memo) => {
                self.generate_memo_declaration(memo);
            }
            Declaration::Enum(_enum_decl) => {
                // Enums don't generate bytecode directly
            }
            Declaration::Struct(_struct_decl) => {
                // Structs don't generate bytecode directly
            }
            Declaration::Trait(_trait_decl) => {
                // Traits don't generate bytecode directly
            }
            Declaration::Impl(impl_decl) => {
                // Generate methods from impl blocks
                self.generate_impl_methods(impl_decl);
            }
            Declaration::Class(class_decl) => {
                // Generate class methods
                self.generate_class_methods(class_decl);
            }
            Declaration::Package(pkg) => {
                // Generate code for package items
                self.generate_package_declaration(pkg);
            },
            Declaration::LibImport(stmt) => {
                // Library imports are handled at compile time, generate the statement
                self.generate_statement(stmt);
            }
            Declaration::Import(_) => {
                // External package imports are handled by the semantic analyzer
                // No code generation needed for import statements
            }
        }
    }
    
    /// Generate code for nested namespace functions recursively
    fn generate_nested_namespace_functions(&mut self, parent_path: &[String], ns: &NamespaceDeclaration) {
        let mut current_path = parent_path.to_vec();
        current_path.push(ns.name.clone());

        // Save current namespace context
        let old_namespace = self.current_namespace.clone();
        self.current_namespace = Some(current_path.join(constants::NAMESPACE_SEPARATOR));

        for item in &ns.items {
            match item {
                NamespaceItem::Function(func) => {
                    self.generate_nested_namespace_function(&current_path, func);
                }
                NamespaceItem::Namespace(nested_ns) => {
                    // Recursively generate nested namespace functions
                    self.generate_nested_namespace_functions(&current_path, nested_ns);
                }
                NamespaceItem::Variable(stmt) => {
                    // Skip variable generation here - it's already done in generate_global_initializers
                }
                NamespaceItem::Enum(_enum_decl) => {
                    // Enums don't generate bytecode directly
                }
                NamespaceItem::Struct(_struct_decl) => {
                    // Structs don't generate bytecode directly
                }
                NamespaceItem::Trait(_trait_decl) => {
                    // Traits don't generate bytecode directly
                }
                NamespaceItem::Impl(_impl_decl) => {
                    // Impl blocks will be handled separately
                }
                NamespaceItem::Class(_class_decl) => {
                    // Classes will be handled separately
                }
            }
        }

        // Restore namespace context
        self.current_namespace = old_namespace;
    }

    /// Generate code for package declaration
    fn generate_package_declaration(&mut self, pkg: &PackageDeclaration) {
        self.generate_package_items(&[], pkg);
    }

    /// Generate code for package items recursively
    fn generate_package_items(&mut self, parent_path: &[String], pkg: &PackageDeclaration) {
        let mut current_path = parent_path.to_vec();
        current_path.push(pkg.name.clone());

        // Save current namespace context
        let old_namespace = self.current_namespace.clone();
        self.current_namespace = Some(current_path.join("::"));

        // Generate code for all package items
        for item in &pkg.items {
            match item {
                PackageItem::Function(func) => {
                    self.generate_package_function(&current_path, func);
                }
                PackageItem::Namespace(ns) => {
                    self.generate_nested_namespace_functions(&current_path, ns);
                }
                PackageItem::Package(nested_pkg) => {
                    self.generate_package_items(&current_path, nested_pkg);
                }
                PackageItem::Variable(stmt) => {
                    self.generate_statement(stmt);
                }
                PackageItem::Enum(enum_decl) => {
                    self.generate_declaration(&Declaration::Enum(enum_decl.clone()));
                }
                PackageItem::Struct(struct_decl) => {
                    self.generate_declaration(&Declaration::Struct(struct_decl.clone()));
                }
                PackageItem::Class(class_decl) => {
                    self.generate_declaration(&Declaration::Class(class_decl.clone()));
                }
                PackageItem::Trait(trait_decl) => {
                    self.generate_declaration(&Declaration::Trait(trait_decl.clone()));
                }
                PackageItem::Impl(impl_decl) => {
                    self.generate_declaration(&Declaration::Impl(impl_decl.clone()));
                }
            }
        }

        // Restore namespace context
        self.current_namespace = old_namespace;
    }

    /// Generate code for package function
    fn generate_package_function(&mut self, package_path: &[String], function: &FunctionDeclaration) {
        let full_name = format!("{}::{}", package_path.join("::"), function.name);
        if let Some(func_info) = self.functions.get(&full_name).cloned() {
            let old_chunk = self.current_chunk;
            let old_namespace = self.current_namespace.clone();
            self.current_chunk = func_info.chunk_index;
            self.current_namespace = Some(package_path.join("::"));

            // Save current variables and reset local variable count
            let old_variables = self.variables.clone();
            let old_local_count = self.local_count;
            self.local_count = 0;

            // Add function parameters as local variables
            for (i, param) in function.parameters.iter().enumerate() {
                let param_type = Type::from(param.type_annotation.clone());

                let var_info = VariableInfo {
                    index: i,
                    is_global: false,
                    var_type: param_type,
                };
                self.variables.insert(param.name.clone(), var_info);
                self.local_count += 1;
            }

            // Generate function body
            self.generate_statement(&function.body);

            // Ensure function returns a value
            if !self.chunks[self.current_chunk].instructions.is_empty() {
                let last_instruction = &self.chunks[self.current_chunk].instructions.last().unwrap().opcode;
                if !matches!(last_instruction, OpCode::Return) {
                    self.emit_instruction(Instruction::new(OpCode::Return, 0));
                }
            } else {
                self.emit_instruction(Instruction::new(OpCode::Return, 0));
            }

            // Restore state
            self.current_chunk = old_chunk;
            self.current_namespace = old_namespace;
            self.variables = old_variables;
            self.local_count = old_local_count;

            // Remove function parameters from variables
            for param in &function.parameters {
                self.variables.remove(&param.name);
            }
        }
    }

    /// Generate code for nested namespace function
    fn generate_nested_namespace_function(&mut self, namespace_path: &[String], function: &FunctionDeclaration) {
        let full_name = format!("{}{}{}", namespace_path.join(constants::NAMESPACE_SEPARATOR), constants::NAMESPACE_SEPARATOR, function.name);
        if let Some(func_info) = self.functions.get(&full_name).cloned() {
            let old_chunk = self.current_chunk;
            let old_namespace = self.current_namespace.clone();
            self.current_chunk = func_info.chunk_index;
            self.current_namespace = Some(namespace_path.join(constants::NAMESPACE_SEPARATOR));

            // Reset local variable count
            let old_local_count = self.local_count;
            self.local_count = 0;

            // Add parameters as local variables
            for (i, param) in function.parameters.iter().enumerate() {
                let var_info = VariableInfo {
                    index: i,
                    is_global: false,
                    var_type: if let Some(type_annotation) = &param.type_annotation {
                        Type::from(type_annotation.clone())
                    } else {
                        Type::Unknown
                    },
                };
                self.variables.insert(param.name.clone(), var_info);
            }
            self.local_count = function.parameters.len();

            // Generate function body
            self.generate_statement(&function.body);

            // Add implicit return if needed
            if !self.has_return_at_end() {
                if func_info.return_type == Type::Null {
                    self.emit_instruction(Instruction::new(OpCode::Return, 0));
                } else {
                    self.emit_instruction(Instruction::new(OpCode::Null, 0));
                    self.emit_instruction(Instruction::new(OpCode::ReturnValue, 0));
                }
            }

            // Restore state
            self.current_chunk = old_chunk;
            self.current_namespace = old_namespace;
            self.local_count = old_local_count;

            // Remove parameters from variable map
            for param in &function.parameters {
                self.variables.remove(&param.name);
            }
        }
    }

    /// Generate code for namespace function
    fn generate_namespace_function(&mut self, namespace: &str, function: &FunctionDeclaration) {
        let full_name = format!("{}{}{}", namespace, constants::NAMESPACE_SEPARATOR, function.name);
        if let Some(func_info) = self.functions.get(&full_name).cloned() {
            let old_chunk = self.current_chunk;
            let old_namespace = self.current_namespace.clone();
            self.current_chunk = func_info.chunk_index;
            self.current_namespace = Some(namespace.to_string());

            // Save current variables and reset local variable count
            let old_variables = self.variables.clone();
            let old_local_count = self.local_count;
            self.local_count = 0;

            // Clear only local variables, keep global variables
            self.variables.retain(|_, var_info| var_info.is_global);

            // Add parameters as local variables
            for (i, param) in function.parameters.iter().enumerate() {
                let var_info = VariableInfo {
                    index: i,
                    is_global: false,
                    var_type: if let Some(type_annotation) = &param.type_annotation {
                        Type::from(type_annotation.clone())
                    } else {
                        Type::Unknown
                    },
                };
                self.variables.insert(param.name.clone(), var_info);
            }
            self.local_count = function.parameters.len();

            // Generate function body
            self.generate_statement(&function.body);

            // Add implicit return if needed
            if !self.has_return_at_end() {
                if func_info.return_type == Type::Null {
                    self.emit_instruction(Instruction::new(OpCode::Return, 0));
                } else {
                    self.emit_instruction(Instruction::new(OpCode::Null, 0));
                    self.emit_instruction(Instruction::new(OpCode::ReturnValue, 0));
                }
            }

            // Restore state
            self.current_chunk = old_chunk;
            self.current_namespace = old_namespace;
            self.local_count = old_local_count;
            self.variables = old_variables;
        }
    }

    /// Generate code for function
    fn generate_function(&mut self, function: &FunctionDeclaration) {
        if let Some(func_info) = self.functions.get(&function.name).cloned() {
            if self.debug_mode {
                println!("🔧 CodeGen: Generating function '{}' in chunk {}", function.name, func_info.chunk_index);
            }

            // Save current state
            let old_chunk = self.current_chunk;
            let old_local_count = self.local_count;
            let old_variables = self.variables.clone();
            let old_inside_function = self.inside_function;

            // Switch to function's chunk
            self.current_chunk = func_info.chunk_index;
            self.local_count = 0;
            self.inside_function = true;

            if self.debug_mode {
                println!("🔧 CodeGen: Switched to chunk {} for function '{}'", self.current_chunk, function.name);
            }

            // Clear only local variables, keep global variables
            self.variables.retain(|_, var_info| var_info.is_global);

            // Add parameters as local variables
            for (i, param) in function.parameters.iter().enumerate() {
                let var_info = VariableInfo {
                    index: i,
                    is_global: false,
                    var_type: if let Some(type_annotation) = &param.type_annotation {
                        Type::from(type_annotation.clone())
                    } else {
                        Type::Unknown
                    },
                };
                self.variables.insert(param.name.clone(), var_info);
                if self.debug_mode {
                    println!("🔧 CodeGen: Added parameter '{}' at index {}", param.name, i);
                }
            }
            self.local_count = function.parameters.len();

            if self.debug_mode {
                println!("🔧 CodeGen: About to generate function body for '{}'", function.name);
            }

            // Generate function body
            self.generate_statement(&function.body);

            // Add implicit return if needed
            if !self.has_return_at_end() {
                if func_info.return_type == Type::Null {
                    self.emit_instruction(Instruction::new(OpCode::Return, 0));
                    if self.debug_mode {
                        println!("🔧 CodeGen: Added implicit Return for function '{}'", function.name);
                    }
                } else {
                    self.emit_instruction(Instruction::new(OpCode::Null, 0));
                    self.emit_instruction(Instruction::new(OpCode::ReturnValue, 0));
                    if self.debug_mode {
                        println!("🔧 CodeGen: Added implicit ReturnValue for function '{}'", function.name);
                    }
                }
            }

            // Save local count to the chunk
            if let Some(chunk) = self.chunks.get_mut(func_info.chunk_index) {
                chunk.local_count = self.local_count;
                if self.debug_mode {
                    println!("🔧 CodeGen: Function '{}' needs {} local variables", function.name, self.local_count);
                    println!("🔧 CodeGen: Function '{}' has {} instructions", function.name, chunk.instructions.len());
                }
            }

            // Restore state
            self.current_chunk = old_chunk;
            self.local_count = old_local_count;
            self.variables = old_variables;
            self.inside_function = old_inside_function;

            if self.debug_mode {
                println!("🔧 CodeGen: Restored to chunk {} after generating function '{}'", self.current_chunk, function.name);
            }
        }
    }
    
    /// Generate code for statement
    fn generate_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::VarDeclaration { scope, name, type_annotation, initializer, position } => {
                let var_type = type_annotation.as_ref()
                    .map(|t| Type::from(t.clone()))
                    .unwrap_or(Type::Unknown);
                
                // Variables are global if they have Global scope OR if they are namespace-level variables
                // (but not function-local variables inside namespace functions)
                // Only consider variables global if they are explicitly marked as Global scope
                // or if we're at the top level (main chunk) and NOT inside a function
                let is_global = matches!(scope, VarScope::Global) ||
                    (self.current_chunk == constants::MAIN_CHUNK_INDEX && !self.inside_function);

                // Get variable info (should already exist for globals)
                let var_info = if is_global {
                    // For global variables, try to find by full name (including namespace)
                    let lookup_name = if let Some(namespace) = &self.current_namespace {
                        format!("{}{}{}", namespace, constants::NAMESPACE_SEPARATOR, name)
                    } else {
                        name.clone()
                    };

                    if let Some(existing_info) = self.variables.get(&lookup_name) {
                        existing_info.clone()
                    } else {
                        // This shouldn't happen if we processed globals correctly
                        let idx = self.global_count;
                        self.global_count += 1;
                        VariableInfo {
                            index: idx,
                            is_global: true,
                            var_type,
                        }
                    }
                } else {
                    let idx = self.local_count;
                    self.local_count += 1;
                    VariableInfo {
                        index: idx,
                        is_global: false,
                        var_type,
                    }
                };
                
                // Store variable info if not already stored (for locals)
                if !is_global {
                    self.variables.insert(name.clone(), var_info.clone());
                }

                // Generate initializer if present
                if let Some(init) = initializer {
                    self.generate_expression(init);

                    if is_global {
                        self.emit_instruction(Instruction::with_u16(OpCode::StoreGlobal, var_info.index as u16, 0));
                    } else {
                        self.emit_instruction(Instruction::with_u16(OpCode::StoreLocal, var_info.index as u16, 0));
                    }
                } else {
                    // Initialize with null
                    self.emit_instruction(Instruction::new(OpCode::Null, 0));

                    if is_global {
                        self.emit_instruction(Instruction::with_u16(OpCode::StoreGlobal, var_info.index as u16, 0));
                    } else {
                        self.emit_instruction(Instruction::with_u16(OpCode::StoreLocal, var_info.index as u16, 0));
                    }
                }
            }
            Statement::Expression(expr) => {
                self.generate_expression(expr);
                // Pop expression result if not used
                // All expressions leave values on the stack, including assignments
                self.emit_instruction(Instruction::new(OpCode::Pop, 0));
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    self.generate_expression(expr);
                    self.emit_instruction(Instruction::new(OpCode::ReturnValue, 0));
                } else {
                    self.emit_instruction(Instruction::new(OpCode::Return, 0));
                }
            }
            Statement::If { condition, then_branch, else_branch, .. } => {
                self.generate_expression(condition);
                
                let else_jump = self.emit_jump(OpCode::JumpIfFalse);
                
                self.generate_statement(then_branch);
                
                if let Some(else_stmt) = else_branch {
                    let end_jump = self.emit_jump(OpCode::Jump);
                    self.patch_jump(else_jump);
                    self.generate_statement(else_stmt);
                    self.patch_jump(end_jump);
                } else {
                    self.patch_jump(else_jump);
                }
            }
            Statement::While { condition, body, .. } => {
                let loop_start = self.current_chunk_ref().instructions.len();
                
                self.generate_expression(condition);
                let exit_jump = self.emit_jump(OpCode::JumpIfFalse);
                
                // Save break/continue jump lists
                let old_breaks = std::mem::take(&mut self.break_jumps);
                let old_continues = std::mem::take(&mut self.continue_jumps);
                
                self.generate_statement(body);
                
                // Continue jumps to loop start
                let continue_jumps = std::mem::take(&mut self.continue_jumps);
                for continue_jump in continue_jumps {
                    self.patch_jump_to(continue_jump, loop_start);
                }

                self.emit_jump_to(OpCode::Jump, loop_start);
                self.patch_jump(exit_jump);

                // Break jumps to after loop
                let break_jumps = std::mem::take(&mut self.break_jumps);
                for break_jump in break_jumps {
                    self.patch_jump(break_jump);
                }
                
                // Restore break/continue jump lists
                self.break_jumps = old_breaks;
                self.continue_jumps = old_continues;
            }
            Statement::For { initializer, condition, increment, body, .. } => {
                // Generate initializer
                if let Some(init) = initializer {
                    self.generate_statement(init);
                }
                
                let loop_start = self.current_chunk_ref().instructions.len();
                
                // Generate condition
                if let Some(cond) = condition {
                    self.generate_expression(cond);
                    let exit_jump = self.emit_jump(OpCode::JumpIfFalse);
                    
                    // Save break/continue jump lists
                    let old_breaks = std::mem::take(&mut self.break_jumps);
                    let old_continues = std::mem::take(&mut self.continue_jumps);
                    
                    self.generate_statement(body);
                    
                    // Generate increment
                    if let Some(inc) = increment {
                        self.generate_expression(inc);
                        // All expressions leave values on the stack, discard result
                        self.emit_instruction(Instruction::new(OpCode::Pop, 0)); // Discard result
                    }
                    
                    // Continue jumps to increment/condition
                    let continue_target = if increment.is_some() {
                        // If there's an increment, continue should jump to it
                        self.current_chunk_ref().instructions.len() - constants::INSTRUCTION_OFFSET
                    } else {
                        loop_start
                    };
                    
                    let continue_jumps = std::mem::take(&mut self.continue_jumps);
                    for continue_jump in continue_jumps {
                        self.patch_jump_to(continue_jump, continue_target);
                    }

                    self.emit_jump_to(OpCode::Jump, loop_start);
                    self.patch_jump(exit_jump);

                    // Break jumps to after loop
                    let break_jumps = std::mem::take(&mut self.break_jumps);
                    for break_jump in break_jumps {
                        self.patch_jump(break_jump);
                    }
                    
                    // Restore break/continue jump lists
                    self.break_jumps = old_breaks;
                    self.continue_jumps = old_continues;
                }
            }
            Statement::Block { statements, .. } => {
                for stmt in statements {
                    self.generate_statement(stmt);
                }
            }
            Statement::Break(_) => {
                let jump = self.emit_jump(OpCode::Jump);
                self.break_jumps.push(jump);
            }
            Statement::Continue(_) => {
                let jump = self.emit_jump(OpCode::Jump);
                self.continue_jumps.push(jump);
            }
            Statement::Switch { expression, cases, default_case, .. } => {
                // Generate switch expression
                self.generate_expression(expression);

                // Store switch value in a temporary variable for comparison
                let switch_temp_index = self.local_count;
                self.emit_instruction(Instruction::with_u8(OpCode::StoreLocal, switch_temp_index as u8, 0));
                self.local_count += 1;

                let mut case_jumps = Vec::with_capacity(cases.len());
                let mut end_jumps = Vec::with_capacity(cases.len());

                // Generate case comparisons
                for case in cases {
                    // Load switch value for comparison
                    self.emit_instruction(Instruction::with_u8(OpCode::LoadLocal, switch_temp_index as u8, 0));

                    // Generate case value
                    self.generate_expression(&case.value);

                    // Compare values
                    self.emit_instruction(Instruction::new(OpCode::Eq, 0));

                    // Jump to case body if equal
                    let case_jump = self.emit_jump(OpCode::JumpIfTrue);
                    case_jumps.push(case_jump);
                }

                // Jump to default case or end if no cases match
                let default_jump = if default_case.is_some() {
                    Some(self.emit_jump(OpCode::Jump))
                } else {
                    Some(self.emit_jump(OpCode::Jump)) // Jump to end
                };

                // Generate case bodies
                for (i, case) in cases.iter().enumerate() {
                    self.patch_jump(case_jumps[i]);

                    self.generate_statement(&case.body);

                    // Jump to end after case body (fall-through prevention)
                    let end_jump = self.emit_jump(OpCode::Jump);
                    end_jumps.push(end_jump);
                }

                // Generate default case if present
                if let Some(default_stmt) = default_case {
                    if let Some(default_jump) = default_jump {
                        self.patch_jump(default_jump);
                    }

                    self.generate_statement(default_stmt);
                } else if let Some(default_jump) = default_jump {
                    self.patch_jump(default_jump);
                }

                // Patch all end jumps and break jumps to point here
                for end_jump in end_jumps {
                    self.patch_jump(end_jump);
                }

                // Patch all break jumps from switch cases to point here
                let break_jumps = std::mem::take(&mut self.break_jumps);
                for break_jump in break_jumps {
                    self.patch_jump(break_jump);
                }

                // Restore local count
                self.local_count -= constants::DECREMENT_VALUE;
            }
            Statement::Try { try_block, catch_clauses, finally_block, .. } => {
                // Generate try/catch/finally block

                let try_start_addr = self.current_chunk_ref().instructions.len();
                let catch_offset_placeholder = constants::JUMP_PLACEHOLDER; // Will be patched

                // Emit TryStart instruction
                self.emit_instruction(Instruction::with_u16(OpCode::TryStart, catch_offset_placeholder, 0));

                // Generate try block
                self.generate_statement(try_block);

                // Emit TryEnd instruction
                self.emit_instruction(Instruction::new(OpCode::TryEnd, 0));

                // Jump over catch/finally blocks if no exception
                let end_jump = self.emit_jump(OpCode::Jump);

                // Generate catch blocks
                let catch_start_addr = self.current_chunk_ref().instructions.len();

                // Patch the catch offset in TryStart instruction
                let catch_offset = (catch_start_addr - try_start_addr - constants::INSTRUCTION_OFFSET) as u16;
                self.current_chunk().instructions[try_start_addr].set_u16(0, catch_offset);

                // Generate catch clauses (for now, just the first one)
                if !catch_clauses.is_empty() {
                    self.emit_instruction(Instruction::new(OpCode::Catch, 0));
                    self.generate_statement(&catch_clauses[0].body);
                }

                // Generate finally block if present
                if let Some(finally_block) = finally_block {
                    self.emit_instruction(Instruction::new(OpCode::Finally, 0));
                    self.generate_statement(finally_block);
                }

                // Patch the end jump
                self.patch_jump(end_jump);
            }
            Statement::Throw { value, .. } => {
                // Generate the value to throw
                self.generate_expression(value);
                // Emit throw instruction
                self.emit_instruction(Instruction::new(OpCode::Throw, 0));
            }
            Statement::LibImport { library_name, import_type, .. } => {
                if self.debug_mode {
                    println!("🔧 CodeGen: Processing library import: {} ({:?})", library_name, import_type);
                }

                // Add library to dependencies for bytecode file
                self.library_dependencies.insert(library_name.clone());

                if self.debug_mode {
                    println!("🔧 CodeGen: Added library '{}' to dependencies", library_name);
                }

                // Library imports are handled at compile time, no runtime code needed
                // The library functions will be available through the library_loader
                // when CallMethod instructions are executed with library function names
            }
            Statement::ForRange { variable, range, body, .. } => {
                // For range-based loops, we need to expand them into traditional loops
                // for (i : start...end) { body } becomes:
                // let i = start;
                // while (i <= end) { body; i = i + 1; }

                if let Expression::Range { start, end, inclusive, .. } = range {
                    // Generate start value and store in variable
                    self.generate_expression(start);

                    // Store in local variable (simplified - should use proper variable management)
                    let var_index = self.local_count;
                    let var_info = VariableInfo {
                        index: var_index,
                        is_global: false,
                        var_type: constants::LOOP_VARIABLE_TYPE,
                    };
                    self.variables.insert(variable.clone(), var_info);
                    self.local_count += 1;

                    // Store the start value
                    self.emit_instruction(Instruction::with_u16(OpCode::StoreLocal, var_index as u16, 0));

                    // Loop start label
                    let loop_start = self.current_chunk().instructions.len();

                    // Load loop variable
                    self.emit_instruction(Instruction::with_u16(OpCode::LoadLocal, var_index as u16, 0));

                    // Load end value
                    self.generate_expression(end);

                    // Compare: i <= end (inclusive) or i < end (exclusive)
                    if *inclusive {
                        self.emit_instruction(Instruction::new(OpCode::Le, 0)); // <=
                    } else {
                        self.emit_instruction(Instruction::new(OpCode::Lt, 0)); // <
                    }

                    // Jump if false (exit loop)
                    let exit_jump = self.current_chunk().instructions.len();
                    self.emit_instruction(Instruction::with_u16(OpCode::JumpIfFalse, 0, 0)); // Will patch later

                    // Generate loop body
                    self.generate_statement(body);

                    // Increment loop variable: i = i + 1
                    self.emit_instruction(Instruction::with_u16(OpCode::LoadLocal, var_index as u16, 0));
                    let one_const = self.add_constant(constants::LOOP_INCREMENT_VALUE);
                    self.emit_instruction(Instruction::with_u16(OpCode::Const, one_const, 0));
                    self.emit_instruction(Instruction::new(OpCode::Add, 0));
                    self.emit_instruction(Instruction::with_u16(OpCode::StoreLocal, var_index as u16, 0));

                    // Jump back to loop start (use absolute address)
                    self.emit_instruction(Instruction::with_u16(OpCode::Jump, loop_start as u16, 0));

                    // Patch the exit jump
                    let exit_address = self.current_chunk().instructions.len() as u16;
                    if let Some(instruction) = self.current_chunk().instructions.get_mut(exit_jump) {
                        let bytes = exit_address.to_le_bytes();
                        instruction.operands = bytes.to_vec();
                    }
                } else {
                    self.errors.push(CodeGenError::new(
                        constants::FOR_RANGE_REQUIRES_RANGE.to_string(),
                        range.position().clone()
                    ));
                }
            }
            Statement::ForIn { variable, iterable, body, .. } => {
                // For-in loops: for (variable in iterable) { body }
                // We'll implement this as a traditional for loop with index

                // Generate the iterable expression and store it
                self.generate_expression(iterable);
                let array_var_index = self.local_count;
                self.local_count += 1;
                self.emit_instruction(Instruction::with_u16(OpCode::StoreLocal, array_var_index as u16, 0));

                // Initialize index to 0
                let index_var_index = self.local_count;
                self.local_count += 1;
                let zero_const = self.add_constant(constants::LOOP_INITIAL_INDEX);
                self.emit_instruction(Instruction::with_u16(OpCode::Const, zero_const, 0));
                self.emit_instruction(Instruction::with_u16(OpCode::StoreLocal, index_var_index as u16, 0));

                // Create loop variable
                let loop_var_index = self.local_count;
                let var_info = VariableInfo {
                    index: loop_var_index,
                    is_global: false,
                    var_type: Type::Auto(Box::new(Type::Unknown)),
                };
                self.variables.insert(variable.clone(), var_info);
                self.local_count += 1;

                // Loop start
                let loop_start = self.current_chunk().instructions.len();

                // Check if index < array.length
                self.emit_instruction(Instruction::with_u16(OpCode::LoadLocal, index_var_index as u16, 0));
                self.emit_instruction(Instruction::with_u16(OpCode::LoadLocal, array_var_index as u16, 0));

                // Get array length - for now, we'll use a simple approach
                // If it's an array literal, we know the length at compile time
                // For variables, we'll need to use ArrayLength opcode
                self.emit_instruction(Instruction::new(OpCode::ArrayLength, 0));
                self.emit_instruction(Instruction::new(OpCode::Lt, 0));

                // Jump if false (exit loop)
                let exit_jump = self.current_chunk().instructions.len();
                self.emit_instruction(Instruction::with_u16(OpCode::JumpIfFalse, 0, 0));

                // Load array[index] into loop variable
                self.emit_instruction(Instruction::with_u16(OpCode::LoadLocal, array_var_index as u16, 0));
                self.emit_instruction(Instruction::with_u16(OpCode::LoadLocal, index_var_index as u16, 0));
                self.emit_instruction(Instruction::new(OpCode::LoadIndex, 0));
                self.emit_instruction(Instruction::with_u16(OpCode::StoreLocal, loop_var_index as u16, 0));

                // Generate loop body
                self.generate_statement(body);

                // Increment index
                self.emit_instruction(Instruction::with_u16(OpCode::LoadLocal, index_var_index as u16, 0));
                let one_const = self.add_constant(constants::LOOP_INCREMENT_VALUE);
                self.emit_instruction(Instruction::with_u16(OpCode::Const, one_const, 0));
                self.emit_instruction(Instruction::new(OpCode::Add, 0));
                self.emit_instruction(Instruction::with_u16(OpCode::StoreLocal, index_var_index as u16, 0));

                // Jump back to loop start
                self.emit_instruction(Instruction::with_u16(OpCode::Jump, loop_start as u16, 0));

                // Patch exit jump
                let exit_address = self.current_chunk().instructions.len() as u16;
                if let Some(instruction) = self.current_chunk().instructions.get_mut(exit_jump) {
                    let bytes = exit_address.to_le_bytes();
                    instruction.operands = bytes.to_vec();
                }
            }
            Statement::DoWhile { body, condition, .. } => {
                // Do-while loops: do { body } while (condition);

                // Loop start - execute body first
                let loop_start = self.current_chunk().instructions.len();

                // Generate loop body
                self.generate_statement(body);

                // Generate condition
                self.generate_expression(condition);

                // Jump back to start if condition is true
                self.emit_instruction(Instruction::with_u16(OpCode::JumpIfTrue, loop_start as u16, 0));
            }
        }
    }
    
    /// Generate code for expression
    fn generate_expression(&mut self, expression: &Expression) {
        match expression {
            Expression::Integer(value, _) => {
                self.generate_integer_expression(*value);
            }
            Expression::Float(value, _) => {
                self.generate_float_expression(*value);
            }
            Expression::String(value, _) => {
                self.generate_string_expression(value);
            }
            Expression::InterpolatedString(parts, _) => {
                self.generate_interpolated_string_expression(parts);
            }
            Expression::Boolean(value, _) => {
                self.generate_boolean_expression(*value);
            }
            Expression::Null(_) => {
                self.generate_null_expression();
            }
            Expression::Array(elements, _) => {
                // Generate code for each element
                for element in elements {
                    self.generate_expression(element);
                }

                // Create array with the elements on stack
                self.emit_instruction(Instruction::with_u16(OpCode::CreateArray, elements.len() as u16, 0));
            }
            Expression::Object(fields, _) => {
                // Generate code for each field value and key
                for (key, value) in fields {
                    // Push key as string constant
                    let key_index = self.add_constant(Value::String(std::rc::Rc::new(key.clone())));
                    self.emit_instruction(Instruction::with_u16(OpCode::Const, key_index, 0));

                    // Generate value
                    self.generate_expression(value);
                }

                // Create object with the key-value pairs on stack
                self.emit_instruction(Instruction::with_u16(OpCode::CreateObject, fields.len() as u16, 0));
            }
            Expression::Identifier(name, position) => {
                if self.debug_mode {
                    println!("🔧 CodeGen: Generating identifier '{}'", name);
                }
                // Try to find variable by simple name first
                let var_info = if let Some(var_info) = self.variables.get(name) {
                    if self.debug_mode {
                        println!("🔧 CodeGen: Found variable '{}' by simple name", name);
                    }
                    Some(var_info)
                } else if let Some(current_ns) = &self.current_namespace {
                    // Try to find variable with namespace prefix
                    let full_name = format!("{}{}{}", current_ns, constants::NAMESPACE_SEPARATOR, name);
                    if self.debug_mode {
                        println!("🔧 CodeGen: Trying to find variable with full name '{}'", full_name);
                    }
                    self.variables.get(&full_name)
                } else {
                    None
                };

                if let Some(var_info) = var_info {
                    if self.debug_mode {
                        println!("🔧 CodeGen: Found variable '{}' at index {} (is_global: {})", name, var_info.index, var_info.is_global);
                    }
                    if var_info.is_global {
                        self.emit_instruction(Instruction::with_u16(OpCode::LoadGlobal, var_info.index as u16, 0));
                    } else {
                        self.emit_instruction(Instruction::with_u8(OpCode::LoadLocal, var_info.index as u8, 0));
                    }
                } else {
                    // If not found as variable, try to find as function
                    if let Some(func_info) = self.functions.get(name).cloned() {
                        if self.debug_mode {
                            println!("🔧 CodeGen: Found function '{}' for function pointer", name);
                        }
                        // Generate function reference for function pointer assignment
                        let const_index = self.add_constant(Value::String(name.clone().into()));
                        self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                    } else if let Some(current_ns) = &self.current_namespace {
                        // Try to find function with namespace prefix
                        let full_name = format!("{}{}{}", current_ns, constants::NAMESPACE_SEPARATOR, name);
                        if let Some(func_info) = self.functions.get(&full_name).cloned() {
                            if self.debug_mode {
                                println!("🔧 CodeGen: Found namespaced function '{}' for function pointer", full_name);
                            }
                            // Generate function reference for function pointer assignment
                            let const_index = self.add_constant(Value::String(full_name.into()));
                            self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                        } else {
                            if self.debug_mode {
                                println!("🔧 CodeGen: Neither variable nor function '{}' found", name);
                            }
                            self.errors.push(CodeGenError::new(
                                format!("Undefined variable '{}'", name),
                                position.clone()
                            ));
                        }
                    } else {
                        if self.debug_mode {
                            println!("🔧 CodeGen: Neither variable nor function '{}' found", name);
                        }
                        self.errors.push(CodeGenError::new(
                            format!("Undefined variable '{}'", name),
                            position.clone()
                        ));
                    }
                }
            }
            Expression::Binary { left, operator, right, .. } => {
                // Special handling for pipe operations
                if matches!(operator, BinaryOperator::PipeForward) {
                    self.generate_pipe_operation(left, right);
                    return;
                }

                // Special handling for assignment operations
                if matches!(operator, BinaryOperator::Assign | BinaryOperator::AddAssign |
                           BinaryOperator::SubtractAssign | BinaryOperator::MultiplyAssign |
                           BinaryOperator::DivideAssign | BinaryOperator::ModuloAssign) {
                    if self.debug_mode {
                        println!("🔧 CodeGen: Assignment operation - generating right side first");
                    }

                    // For compound assignment, we need the current value first
                    if !matches!(operator, BinaryOperator::Assign) {
                        self.generate_expression(left);  // Current value
                    }

                    self.generate_expression(right);  // New value

                    if self.debug_mode {
                        println!("🔧 CodeGen: Assignment operation - right side generated");
                    }
                } else {
                    // For other binary operations, generate left then right
                    self.generate_expression(left);
                    self.generate_expression(right);
                }

                match operator {
                    BinaryOperator::Add => self.emit_instruction(Instruction::new(OpCode::Add, constants::DEFAULT_OPERAND)),
                    BinaryOperator::Subtract => self.emit_instruction(Instruction::new(OpCode::Sub, constants::DEFAULT_OPERAND)),
                    BinaryOperator::Multiply => self.emit_instruction(Instruction::new(OpCode::Mul, constants::DEFAULT_OPERAND)),
                    BinaryOperator::Divide => self.emit_instruction(Instruction::new(OpCode::Div, constants::DEFAULT_OPERAND)),
                    BinaryOperator::Modulo => self.emit_instruction(Instruction::new(OpCode::Mod, constants::DEFAULT_OPERAND)),
                    BinaryOperator::Equal => self.emit_instruction(Instruction::new(OpCode::Eq, constants::DEFAULT_OPERAND)),
                    BinaryOperator::NotEqual => self.emit_instruction(Instruction::new(OpCode::Ne, constants::DEFAULT_OPERAND)),
                    BinaryOperator::Less => self.emit_instruction(Instruction::new(OpCode::Lt, constants::DEFAULT_OPERAND)),
                    BinaryOperator::LessEqual => self.emit_instruction(Instruction::new(OpCode::Le, constants::DEFAULT_OPERAND)),
                    BinaryOperator::Greater => self.emit_instruction(Instruction::new(OpCode::Gt, constants::DEFAULT_OPERAND)),
                    BinaryOperator::GreaterEqual => self.emit_instruction(Instruction::new(OpCode::Ge, constants::DEFAULT_OPERAND)),
                    BinaryOperator::And => self.emit_instruction(Instruction::new(OpCode::And, constants::DEFAULT_OPERAND)),
                    BinaryOperator::Or => self.emit_instruction(Instruction::new(OpCode::Or, constants::DEFAULT_OPERAND)),
                    BinaryOperator::BitAnd => self.emit_instruction(Instruction::new(OpCode::BitAnd, constants::DEFAULT_OPERAND)),
                    BinaryOperator::BitOr => self.emit_instruction(Instruction::new(OpCode::BitOr, constants::DEFAULT_OPERAND)),
                    BinaryOperator::BitXor => self.emit_instruction(Instruction::new(OpCode::BitXor, constants::DEFAULT_OPERAND)),
                    BinaryOperator::LeftShift => self.emit_instruction(Instruction::new(OpCode::Shl, constants::DEFAULT_OPERAND)),
                    BinaryOperator::RightShift => self.emit_instruction(Instruction::new(OpCode::Shr, constants::DEFAULT_OPERAND)),
                    BinaryOperator::Assign => {
                        // Assignment is handled differently
                        match left.as_ref() {
                            Expression::Identifier(name, position) => {
                                if let Some(var_info) = self.variables.get(name).cloned() {
                                    // Duplicate the value on stack so assignment returns the assigned value
                                    self.emit_instruction(Instruction::new(OpCode::Dup, constants::DEFAULT_OPERAND));
                                    if var_info.is_global {
                                        self.emit_instruction(Instruction::with_u16(OpCode::StoreGlobal, var_info.index as u16, constants::DEFAULT_OPERAND));
                                    } else {
                                        self.emit_instruction(Instruction::with_u8(OpCode::StoreLocal, var_info.index as u8, constants::DEFAULT_OPERAND));
                                    }
                                    // Now the assigned value is still on the stack
                                } else {
                                    self.errors.push(CodeGenError::new(
                                        format!("Undefined variable '{}'", name),
                                        position.clone()
                                    ));
                                }
                            }
                            Expression::FieldAccess { object, field, .. } => {
                                // Field assignment: object.field = value
                                // Stack: [value] (right side already generated)
                                // We need: [object, value] for StoreField
                                // Duplicate the value first so we can return it as the assignment result
                                self.emit_instruction(Instruction::new(OpCode::Dup, constants::DEFAULT_OPERAND));
                                // Stack: [value, value]

                                self.generate_expression(object);
                                // Now stack is: [value, value, object] but we need [value, object, value]
                                // Use Swap instruction to exchange the top two stack values
                                self.emit_instruction(Instruction::new(OpCode::Swap, constants::DEFAULT_OPERAND));
                                // Now stack is: [value, object, value] - correct for StoreField

                                let field_index = self.add_constant(Value::String(field.clone().into()));
                                self.emit_instruction(Instruction::with_u16(OpCode::StoreField, field_index, constants::DEFAULT_OPERAND));

                                // After StoreField, the modified object is on the stack: [value, modified_object]
                                // We need to store it back to the object variable if it's a local variable
                                if let Expression::Identifier(obj_name, _) = object.as_ref() {
                                    if let Some(var_info) = self.variables.get(obj_name) {
                                        if var_info.is_global {
                                            self.emit_instruction(Instruction::with_u16(OpCode::StoreGlobal, var_info.index as u16, constants::DEFAULT_OPERAND));
                                        } else {
                                            self.emit_instruction(Instruction::with_u8(OpCode::StoreLocal, var_info.index as u8, constants::DEFAULT_OPERAND));
                                        }
                                    }
                                }
                                // Now the assigned value is still on the stack as the result of the assignment expression
                            }
                            Expression::Dereference { operand, .. } => {
                                // Pointer dereference assignment: *ptr = value
                                // Stack: [value] (right side already generated)
                                // We need: [pointer, value] for StoreThrough
                                // Duplicate the value first so we can return it as the assignment result
                                self.emit_instruction(Instruction::new(OpCode::Dup, constants::DEFAULT_OPERAND));
                                // Stack: [value, value]

                                self.generate_expression(operand);
                                // Now stack is: [value, value, pointer] but we need [value, pointer, value]
                                // Use Swap instruction to exchange the top two stack values
                                self.emit_instruction(Instruction::new(OpCode::Swap, constants::DEFAULT_OPERAND));
                                // Now stack is: [value, pointer, value] - correct for StoreThrough

                                self.emit_instruction(Instruction::new(OpCode::StoreThrough, constants::DEFAULT_OPERAND));
                                // Now the assigned value is still on the stack as the result of the assignment expression
                            }

                            Expression::SuperFieldAccess { field_name, .. } => {
                                // Super field assignment: super.field = value
                                // Stack: [value] (right side already generated)
                                // Duplicate the value first so we can return it as the assignment result
                                self.emit_instruction(Instruction::new(OpCode::Dup, 0));
                                // Stack: [value, value]

                                // Load 'self' onto the stack
                                if let Some(self_var) = self.variables.get(constants::SELF_IDENTIFIER) {
                                    self.emit_instruction(Instruction::with_u8(OpCode::LoadLocal, self_var.index as u8, constants::DEFAULT_OPERAND));
                                } else {
                                    self.errors.push(CodeGenError::new(
                                        constants::SUPER_FIELD_ASSIGNMENT_ERROR.to_string(),
                                        left.position().clone(),
                                    ));
                                    return;
                                }

                                // Now stack is: [value, value, self] but we need [value, self, value]
                                self.emit_instruction(Instruction::new(OpCode::Swap, constants::DEFAULT_OPERAND));
                                // Now stack is: [value, self, value] - correct for SuperFieldStore

                                let field_index = self.add_constant(Value::String(field_name.clone().into()));
                                self.emit_instruction(Instruction::with_u16(OpCode::SuperFieldStore, field_index, constants::DEFAULT_OPERAND));

                                // After SuperFieldStore, the modified object is on the stack: [value, modified_object]
                                // Update 'self' with the modified object
                                if let Some(self_var) = self.variables.get(constants::SELF_IDENTIFIER).cloned() {
                                    self.emit_instruction(Instruction::new(OpCode::Dup, constants::DEFAULT_OPERAND)); // Duplicate the object
                                    self.emit_instruction(Instruction::with_u8(OpCode::StoreLocal, self_var.index as u8, constants::DEFAULT_OPERAND));
                                    self.emit_instruction(Instruction::new(OpCode::Pop, constants::DEFAULT_OPERAND)); // Remove the duplicate
                                }

                                // Pop the modified object, leaving the assigned value on the stack
                                self.emit_instruction(Instruction::new(OpCode::Pop, constants::DEFAULT_OPERAND));
                            }

                            Expression::NamespaceAccess { namespace_path, name, position } => {
                                // Static field assignment: ClassName::field = value
                                let full_name = format!("{}{}{}", namespace_path.join(constants::NAMESPACE_SEPARATOR), constants::NAMESPACE_SEPARATOR, name);
                                if let Some(var_info) = self.variables.get(&full_name).cloned() {
                                    if var_info.is_global {
                                        // Duplicate the value on stack so assignment returns the assigned value
                                        self.emit_instruction(Instruction::new(OpCode::Dup, constants::DEFAULT_OPERAND));
                                        self.emit_instruction(Instruction::with_u16(OpCode::StoreGlobal, var_info.index as u16, constants::DEFAULT_OPERAND));
                                        // Now the assigned value is still on the stack
                                    } else {
                                        self.errors.push(CodeGenError::new(
                                            format!("Static field '{}' should be global", full_name),
                                            position.clone()
                                        ));
                                    }
                                } else {
                                    self.errors.push(CodeGenError::new(
                                        format!("Undefined static field '{}'", full_name),
                                        position.clone()
                                    ));
                                }
                            }
                            _ => {
                                // Other assignment types not yet supported
                                self.errors.push(CodeGenError::new(
                                    constants::UNSUPPORTED_ASSIGNMENT_TARGET.to_string(),
                                    Position::new(constants::FIRST_INDEX, constants::FIRST_INDEX)
                                ));
                            }
                        }
                    }

                    // Compound assignment operations
                    BinaryOperator::AddAssign => {
                        // Stack: [current_value, new_value]
                        self.emit_instruction(Instruction::new(OpCode::Add, constants::DEFAULT_OPERAND));
                        self.handle_assignment_target(left);
                    }
                    BinaryOperator::SubtractAssign => {
                        // Stack: [current_value, new_value]
                        self.emit_instruction(Instruction::new(OpCode::Sub, constants::DEFAULT_OPERAND));
                        self.handle_assignment_target(left);
                    }
                    BinaryOperator::MultiplyAssign => {
                        // Stack: [current_value, new_value]
                        self.emit_instruction(Instruction::new(OpCode::Mul, constants::DEFAULT_OPERAND));
                        self.handle_assignment_target(left);
                    }
                    BinaryOperator::DivideAssign => {
                        // Stack: [current_value, new_value]
                        self.emit_instruction(Instruction::new(OpCode::Div, constants::DEFAULT_OPERAND));
                        self.handle_assignment_target(left);
                    }
                    BinaryOperator::ModuloAssign => {
                        // Stack: [current_value, new_value]
                        self.emit_instruction(Instruction::new(OpCode::Mod, constants::DEFAULT_OPERAND));
                        self.handle_assignment_target(left);
                    }

                    BinaryOperator::PipeForward => {
                        // This should not be reached as pipe operations are handled separately
                        unreachable!("Pipe operations should be handled in generate_pipe_operation");
                    }
                }
            }
            Expression::Unary { operator, operand, .. } => {
                match operator {
                    UnaryOperator::Minus | UnaryOperator::Not | UnaryOperator::BitNot => {
                        self.generate_expression(operand);
                        match operator {
                            UnaryOperator::Minus => self.emit_instruction(Instruction::new(OpCode::Neg, constants::DEFAULT_OPERAND)),
                            UnaryOperator::Not => self.emit_instruction(Instruction::new(OpCode::Not, constants::DEFAULT_OPERAND)),
                            UnaryOperator::BitNot => self.emit_instruction(Instruction::new(OpCode::BitNot, constants::DEFAULT_OPERAND)),
                            _ => unreachable!(),
                        }
                    }
                    UnaryOperator::PreIncrement => {
                        // ++x: load x, add 1, store x, result is new value
                        self.generate_expression(operand);
                        let one_index = self.add_constant(Value::Int(constants::INCREMENT_VALUE));
                        self.emit_instruction(Instruction::with_u16(OpCode::Const, one_index, constants::DEFAULT_OPERAND));
                        self.emit_instruction(Instruction::new(OpCode::Add, constants::DEFAULT_OPERAND));
                        self.emit_instruction(Instruction::new(OpCode::Dup, constants::DEFAULT_OPERAND)); // Duplicate for return value
                        self.handle_assignment_target(operand);
                    }
                    UnaryOperator::PostIncrement => {
                        // x++: load x, dup, add 1, store x, result is old value
                        self.generate_expression(operand);
                        self.emit_instruction(Instruction::new(OpCode::Dup, constants::DEFAULT_OPERAND)); // Keep old value for result
                        let one_index = self.add_constant(Value::Int(constants::INCREMENT_VALUE));
                        self.emit_instruction(Instruction::with_u16(OpCode::Const, one_index, constants::DEFAULT_OPERAND));
                        self.emit_instruction(Instruction::new(OpCode::Add, constants::DEFAULT_OPERAND));
                        // Stack: [old_value, new_value]
                        // Store new value without duplicating
                        self.handle_assignment_target_no_dup(operand);
                        // Stack: [old_value] - this is our result
                    }
                    UnaryOperator::PreDecrement => {
                        // --x: load x, sub 1, store x, result is new value
                        self.generate_expression(operand);
                        let one_index = self.add_constant(Value::Int(constants::DECREMENT_AMOUNT));
                        self.emit_instruction(Instruction::with_u16(OpCode::Const, one_index, constants::DEFAULT_OPERAND));
                        self.emit_instruction(Instruction::new(OpCode::Sub, constants::DEFAULT_OPERAND));
                        self.emit_instruction(Instruction::new(OpCode::Dup, constants::DEFAULT_OPERAND)); // Duplicate for return value
                        self.handle_assignment_target(operand);
                    }
                    UnaryOperator::PostDecrement => {
                        // x--: load x, dup, sub 1, store x, result is old value
                        self.generate_expression(operand);
                        self.emit_instruction(Instruction::new(OpCode::Dup, constants::DEFAULT_OPERAND)); // Keep old value for result
                        let one_index = self.add_constant(Value::Int(constants::DECREMENT_AMOUNT));
                        self.emit_instruction(Instruction::with_u16(OpCode::Const, one_index, constants::DEFAULT_OPERAND));
                        self.emit_instruction(Instruction::new(OpCode::Sub, constants::DEFAULT_OPERAND));
                        // Stack: [old_value, new_value]
                        // Store new value without duplicating
                        self.handle_assignment_target_no_dup(operand);
                        // Stack: [old_value] - this is our result
                    }
                }
            }
            Expression::Call { function, arguments, position } => {
                // Generate arguments
                for arg in arguments {
                    self.generate_expression(arg);
                }
                
                // Handle function call
                match function.as_ref() {
                    Expression::Identifier(name, _) => {
                        // First, check if this is a function pointer variable
                        if let Some(var_info) = self.variables.get(name) {
                            if self.debug_mode {
                                println!("🔧 CodeGen: Found variable '{}' for function call", name);
                            }
                            // This is a function pointer call
                            // 栈布局：[arg1, arg2, ..., argN, function_pointer, param_count]
                            // 参数已经在栈上了，现在加载函数指针

                            // 加载函数指针变量
                            if var_info.is_global {
                                self.emit_instruction(Instruction::with_u16(OpCode::LoadGlobal, var_info.index as u16, 0));
                            } else {
                                self.emit_instruction(Instruction::with_u8(OpCode::LoadLocal, var_info.index as u8, 0));
                            }

                            // 推送参数计数
                            let param_count_const = self.add_constant(Value::Int(arguments.len() as i64));
                            self.emit_instruction(Instruction::with_u16(OpCode::Const, param_count_const, 0));

                            // 调用函数指针
                            self.emit_instruction(Instruction::new(OpCode::CallIndirect, constants::DEFAULT_OPERAND));

                            if self.debug_mode {
                                println!("🔧 CodeGen: Generated CallIndirect for function pointer '{}'", name);
                            }
                            return; // 重要：找到函数指针变量后直接返回，不继续查找普通函数
                        }
                        // Then, try to find a regular function with this name
                        else if let Some(func_info) = self.functions.get(name).cloned() {
                            if func_info.parameter_count != arguments.len() {
                                self.errors.push(CodeGenError::new(
                                    format!("Function '{}' expects {} arguments, got {}",
                                            name, func_info.parameter_count, arguments.len()),
                                    position.clone()
                                ));
                            }

                            // Check if this is a user-defined function (has a chunk) or a builtin
                            if func_info.chunk_index > constants::BUILTIN_CHUNK_INDEX {
                                // User-defined function - use Call instruction
                                let param_count_const = self.add_constant(Value::Int(arguments.len() as i64));
                                self.emit_instruction(Instruction::with_u16(OpCode::Const, param_count_const, constants::DEFAULT_OPERAND));
                                self.emit_instruction(Instruction::with_u16(OpCode::Call, func_info.chunk_index as u16, constants::DEFAULT_OPERAND));
                            } else {
                                // This might be an imported builtin function
                                // Try to find the full name in builtins
                                let mut found_builtin = false;
                                for builtin_name in self.builtins.get_names() {
                                    if builtin_name.ends_with(&format!("{}{}", constants::NAMESPACE_SEPARATOR, name)) {
                                        // Found the builtin - use CallMethod instruction
                                        let const_index = self.add_constant(Value::String(builtin_name.clone().into()));
                                        self.emit_instruction(Instruction::with_u16(OpCode::CallMethod, const_index, constants::DEFAULT_OPERAND));
                                        found_builtin = true;
                                        break;
                                    }
                                }

                                if !found_builtin {
                                    // Regular user-defined function with chunk_index 0 (main function)
                                    let param_count_const = self.add_constant(Value::Int(arguments.len() as i64));
                                    self.emit_instruction(Instruction::with_u16(OpCode::Const, param_count_const, constants::DEFAULT_OPERAND));
                                    self.emit_instruction(Instruction::with_u16(OpCode::Call, func_info.chunk_index as u16, constants::DEFAULT_OPERAND));
                                }
                            }
                        }
                        // If no regular function found, check if this is a class constructor
                        else {
                            let constructor_name = format!("{}{}{}", name, constants::NAMESPACE_SEPARATOR, name);
                            if let Some(constructor_info) = self.functions.get(&constructor_name).cloned() {
                                // This is a class constructor call
                                // Constructor parameter count does NOT include 'self' (it's implicit)
                                if constructor_info.parameter_count != arguments.len() {
                                    self.errors.push(CodeGenError::new(
                                        format!("Constructor '{}' expects {} arguments, got {}",
                                                name, constructor_info.parameter_count, arguments.len()),
                                        position.clone()
                                    ));
                                }

                                // For constructor calls, we need to generate code differently
                                // We need: [new_object, arguments...] on the stack

                                // First, clear the arguments that were already generated
                                for _ in constants::FIRST_INDEX..arguments.len() {
                                    self.emit_instruction(Instruction::new(OpCode::Pop, constants::DEFAULT_OPERAND));
                                }

                                // 1. Create new object first
                                let class_index = self.add_constant(Value::String(name.clone().into()));
                                self.emit_instruction(Instruction::with_u16(OpCode::NewObject, class_index, constants::DEFAULT_OPERAND));

                                // 2. Now generate arguments in the correct order
                                for arg in arguments {
                                    self.generate_expression(arg);
                                }

                                // 3. Call constructor with the new object as 'self'
                                let const_index = self.add_constant(Value::String(constructor_name.into()));
                                let param_count = (arguments.len() + constants::SELF_PARAM_COUNT) as u16; // +1 for self

                                // Create operands: [const_index (2 bytes), param_count (2 bytes)]
                                let mut operands = Vec::new();
                                operands.extend_from_slice(&const_index.to_le_bytes());
                                operands.extend_from_slice(&param_count.to_le_bytes());

                                self.emit_instruction(Instruction::with_operands(OpCode::CallMethod, operands, constants::DEFAULT_OPERAND));
                            }
                        }

                        // If still not found, try namespace lookup
                        if !self.functions.contains_key(name) &&
                           !self.functions.contains_key(&format!("{}{}{}", name, constants::NAMESPACE_SEPARATOR, name)) {
                            if let Some(namespace) = &self.current_namespace {
                            // If we're in a namespace, try to find the function in the current namespace
                            let full_name = format!("{}{}{}", namespace, constants::NAMESPACE_SEPARATOR, name);
                            if let Some(func_info) = self.functions.get(&full_name).cloned() {
                                if func_info.parameter_count != arguments.len() {
                                    self.errors.push(CodeGenError::new(
                                        format!("Function '{}' expects {} arguments, got {}",
                                            name, func_info.parameter_count, arguments.len()),
                                        position.clone()
                                    ));
                                }

                                // User-defined namespace function - use Call instruction
                                let param_count_const = self.add_constant(Value::Int(arguments.len() as i64));
                                self.emit_instruction(Instruction::with_u16(OpCode::Const, param_count_const, 0));
                                self.emit_instruction(Instruction::with_u16(OpCode::Call, func_info.chunk_index as u16, 0));
                            } else {
                                // Try to find the function in parent namespaces
                                let namespace_parts: Vec<&str> = namespace.split("::").collect();
                                let mut found = false;

                                for i in 1..namespace_parts.len() {
                                    let parent_namespace = namespace_parts[0..i].join("::");
                                    let parent_full_name = format!("{}::{}", parent_namespace, name);

                                    if let Some(func_info) = self.functions.get(&parent_full_name).cloned() {
                                        if func_info.parameter_count != arguments.len() {
                                            self.errors.push(CodeGenError::new(
                                                format!("Function '{}' expects {} arguments, got {}",
                                                    name, func_info.parameter_count, arguments.len()),
                                                position.clone()
                                            ));
                                        }

                                        // User-defined parent namespace function - use Call instruction
                                        let param_count_const = self.add_constant(Value::Int(arguments.len() as i64));
                                        self.emit_instruction(Instruction::with_u16(OpCode::Const, param_count_const, 0));
                                        self.emit_instruction(Instruction::with_u16(OpCode::Call, func_info.chunk_index as u16, 0));
                                        found = true;
                                        break;
                                    }
                                }

                                if !found {
                                    self.errors.push(CodeGenError::new(
                                        format!("Undefined function '{}'", name),
                                        position.clone()
                                    ));
                                }
                            }
                            } else {
                                // Check if this is a library function that was imported via 'using ns'
                                // Get all library functions and find one that ends with this function name
                                match crate::library_loader::get_all_library_functions() {
                                    Ok(all_functions) => {
                                        let suffix = format!("::{}", name);
                                        let matching_function = all_functions.iter()
                                            .find(|func_name| func_name.ends_with(&suffix));

                                        if let Some(full_name) = matching_function {
                                            // This is a library function call (imported via using ns)
                                            if self.debug_mode {
                                                println!("🔧 CodeGen: Generated CallMethod for imported library function '{}' with {} args", full_name, arguments.len());
                                            }
                                            let const_index = self.add_constant(Value::String(full_name.clone().into()));
                                            self.emit_instruction(Instruction::with_u16_u8(OpCode::CallMethod, const_index, arguments.len() as u8, 0));
                                        } else {
                                            self.errors.push(CodeGenError::new(
                                                format!("Undefined function '{}'", name),
                                                position.clone()
                                            ));
                                        }
                                    }
                                    Err(_) => {
                                        self.errors.push(CodeGenError::new(
                                            format!("Undefined function '{}'", name),
                                            position.clone()
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    Expression::NamespaceAccess { namespace_path, name, .. } => {
                        let full_name = format!("{}::{}", namespace_path.join("::"), name);
                        if let Some((_, _)) = self.builtins.get(&full_name) {
                            // Built-in function call
                            let const_index = self.add_constant(Value::String(full_name.into()));
                            self.emit_instruction(Instruction::with_u16_u8(OpCode::CallMethod, const_index, arguments.len() as u8, 0));
                        } else if let Some(func_info) = self.functions.get(&full_name).cloned() {
                            // User-defined namespace function call
                            let param_count = func_info.parameter_count;
                            let const_index = self.add_constant(Value::Int(param_count as i64));
                            self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                            self.emit_instruction(Instruction::with_u16(OpCode::Call, func_info.chunk_index as u16, 0));
                        } else if let Some(func_type) = self.type_env.lookup_in_namespace(&namespace_path.join("::"), name) {
                            // Look up function in type environment namespace (external module function)
                            if let Type::Function { parameters, .. } = func_type {
                                let param_count = parameters.len();
                                if self.debug_mode {
                                    println!("🔧 CodeGen: Found external module function '{}' in namespace '{}' with {} parameters", name, namespace_path.join("::"), param_count);
                                }

                                // For external module functions, we need to map the aliased name to the actual function name
                                // Try different possible mappings for external module functions
                                let mut found_function = None;

                                // First try the full aliased name
                                if let Some(func_info) = self.functions.get(&full_name).cloned() {
                                    found_function = Some(func_info);
                                } else {
                                    // Try to find the original function name by removing the alias prefix
                                    // For example: MathLib::math::add -> math::add
                                    if namespace_path.len() >= 2 {
                                        let original_namespace = &namespace_path[1..]; // Skip the alias part
                                        let original_full_name = format!("{}::{}", original_namespace.join("::"), name);
                                        if let Some(func_info) = self.functions.get(&original_full_name).cloned() {
                                            if self.debug_mode {
                                                println!("🔧 CodeGen: Found external module function '{}' mapped from '{}' to '{}'", name, full_name, original_full_name);
                                            }
                                            found_function = Some(func_info);
                                        }
                                    }
                                }

                                if let Some(func_info) = found_function {
                                    // Function is already compiled, call it directly
                                    let const_index = self.add_constant(Value::Int(param_count as i64));
                                    self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                                    self.emit_instruction(Instruction::with_u16(OpCode::Call, func_info.chunk_index as u16, 0));
                                } else {
                                    // Function not compiled yet, this should be handled by ensuring all imported functions are compiled
                                    self.errors.push(CodeGenError::new(
                                        format!("External module function '{}' in namespace '{}' not compiled", name, namespace_path.join("::")),
                                        position.clone()
                                    ));
                                }
                            } else {
                                let namespace_str = namespace_path.join("::");
                                self.errors.push(CodeGenError::new(
                                    format!("'{}' in namespace '{}' is not a function", name, namespace_str),
                                    position.clone()
                                ));
                            }
                        } else {
                            // Check if this is a library function
                            if let Ok(exists) = crate::library_loader::check_library_function_exists(&full_name) {
                                if exists {
                                    // Library function call
                                    let const_index = self.add_constant(Value::String(full_name.clone().into()));
                                    self.emit_instruction(Instruction::with_u16_u8(OpCode::CallMethod, const_index, arguments.len() as u8, 0));
                                    if self.debug_mode {
                                        println!("🔧 CodeGen: Generated CallMethod for library function '{}' with {} args", full_name, arguments.len());
                                    }
                                } else {
                                    let namespace_str = namespace_path.join("::");
                                    self.errors.push(CodeGenError::new(
                                        format!("Undefined function '{}' in namespace '{}'", name, namespace_str),
                                        position.clone()
                                    ));
                                }
                            } else {
                                let namespace_str = namespace_path.join("::");
                                self.errors.push(CodeGenError::new(
                                    format!("Undefined function '{}' in namespace '{}'", name, namespace_str),
                                    position.clone()
                                ));
                            }
                        }
                    }
                    Expression::FieldAccess { object, field, .. } => {
                        // This is a method call: object.method(args)
                        // First, generate code for the object (this will be the 'self' parameter)
                        self.generate_expression(object);

                        // Generate code for the arguments
                        for arg in arguments {
                            self.generate_expression(arg);
                        }

                        // Try to find the method in the functions table
                        // We need to determine the object type to construct the method name
                        // For now, we'll try common patterns
                        let mut method_found = false;

                        // We need to determine the object type to find the correct method
                        // For now, let's try a more systematic approach


                        // First, try to find methods that match the pattern Type::method_name
                        for (func_name, func_info) in &self.functions.clone() {
                            // Check if this is a method call (contains "::")
                            if let Some(double_colon_pos) = func_name.find("::") {
                                let method_name = &func_name[double_colon_pos + 2..];
                                if method_name == field {
                                    // Found a method with matching name
                                    let param_count = func_info.parameter_count;
                                    let const_index = self.add_constant(Value::Int(param_count as i64));
                                    self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                                    self.emit_instruction(Instruction::with_u16(OpCode::Call, func_info.chunk_index as u16, 0));
                                    method_found = true;
                                    break;
                                }
                            }
                        }

                        if !method_found {
                            self.errors.push(CodeGenError::new(
                                format!("Method '{}' not found", field),
                                position.clone()
                            ));
                        }
                    }
                    _ => {
                        self.errors.push(CodeGenError::new(
                            constants::COMPLEX_FUNCTION_EXPRESSIONS_ERROR.to_string(),
                            position.clone()
                        ));
                    }
                }
            }
            Expression::NamespaceAccess { namespace_path, name, position } => {
                let full_name = format!("{}::{}", namespace_path.join("::"), name);
                let namespace_str = namespace_path.join("::");

                if let Some((_, _)) = self.builtins.get(&full_name) {
                    // Built-in function reference
                    let const_index = self.add_constant(Value::String(full_name.into()));
                    self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                } else if self.functions.contains_key(&full_name) {
                    // User-defined namespace function reference
                    let const_index = self.add_constant(Value::String(full_name.into()));
                    self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                } else if let Some(_func_type) = self.type_env.lookup_in_namespace(&namespace_str, name) {
                    // Function reference from type environment namespace
                    let const_index = self.add_constant(Value::String(full_name.into()));
                    self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                } else if let Some(var_info) = self.variables.get(&full_name) {
                    // Namespace variable access
                    if var_info.is_global {
                        self.emit_instruction(Instruction::with_u16(OpCode::LoadGlobal, var_info.index as u16, 0));
                    } else {
                        self.errors.push(CodeGenError::new(
                            format!("Namespace variable '{}' should be global", full_name),
                            position.clone()
                        ));
                    }
                } else {
                    let namespace_str = namespace_path.join("::");
                    self.errors.push(CodeGenError::new(
                        format!("Undefined item '{}' in namespace '{}'", name, namespace_str),
                        position.clone()
                    ));
                }
            }
            Expression::ArrayAccess { array, index, .. } => {
                self.generate_expression(array);
                self.generate_expression(index);
                self.emit_instruction(Instruction::new(OpCode::LoadIndex, 0)); // Use correct LoadIndex instruction
            }
            Expression::FieldAccess { object, field, .. } => {
                self.generate_expression(object);
                let field_index = self.add_constant(Value::String(field.clone().into()));
                self.emit_instruction(Instruction::with_u16(OpCode::LoadField, field_index, 0));
            }
            Expression::Range { start, end, inclusive, .. } => {
                // Create a range object as a simple object with start, end, and inclusive fields
                // This allows ranges to be used as first-class values

                // Generate start value
                self.generate_expression(start);

                // Generate end value
                self.generate_expression(end);

                // Push inclusive flag as a boolean constant
                let inclusive_const = self.add_constant(Value::Bool(*inclusive));
                self.emit_instruction(Instruction::with_u16(OpCode::Const, inclusive_const, 0));

                // Create an object with 3 fields: start, end, inclusive
                // The VM will pop 3 values from stack and create an object
                self.emit_instruction(Instruction::with_u16(OpCode::CreateObject, 3, 0));

                // Store field names as constants for the object
                let start_field = self.add_constant(Value::String("start".to_string().into()));
                let end_field = self.add_constant(Value::String("end".to_string().into()));
                let inclusive_field = self.add_constant(Value::String("inclusive".to_string().into()));

                // The VM should use these field names when creating the object
                // For now, we assume the VM handles this correctly
            }
            Expression::Lambda { parameters, body, position } => {
                // Generate lambda as a closure
                self.generate_lambda_closure(parameters, body, position);
            }
            Expression::Ternary { condition, true_expr, false_expr, .. } => {
                // Generate condition
                self.generate_expression(condition);

                // Jump to false branch if condition is false
                let false_jump = self.emit_jump(OpCode::JumpIfFalse);

                // Generate true expression
                self.generate_expression(true_expr);

                // Jump over false expression
                let end_jump = self.emit_jump(OpCode::Jump);

                // Patch false jump to point here
                self.patch_jump(false_jump);

                // Generate false expression
                self.generate_expression(false_expr);

                // Patch end jump to point here
                self.patch_jump(end_jump);
            }

            Expression::NullCoalescing { left, right, .. } => {
                // 生成空值合并操作符的字节码
                // 策略：计算左侧表达式，如果不为 null 则使用它，否则计算并使用右侧表达式

                // 生成左侧表达式
                self.generate_expression(left);

                // 复制栈顶值用于检查
                self.emit_instruction(Instruction::new(OpCode::Dup, 0));

                // 检查是否为 null
                let const_index = self.add_constant(Value::Null);
                self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                self.emit_instruction(Instruction::new(OpCode::Ne, 0)); // 使用 Ne 而不是 NotEqual

                // 如果不为 null，跳过右侧表达式
                let skip_right = self.emit_jump(OpCode::JumpIfTrue);

                // 弹出左侧的值（因为它是 null）
                self.emit_instruction(Instruction::new(OpCode::Pop, 0));

                // 生成右侧表达式
                self.generate_expression(right);

                // 修补跳转地址
                self.patch_jump(skip_right);
            }

            Expression::When { arms, else_expr, .. } => {
                // 生成 when 表达式的字节码
                // 策略：按顺序检查每个条件，第一个为真的执行对应的表达式

                let mut end_jumps = Vec::with_capacity(arms.len());

                for arm in arms {
                    // 生成条件表达式
                    self.generate_expression(&arm.condition);

                    // 如果条件为假，跳到下一个条件
                    let next_condition = self.emit_jump(OpCode::JumpIfFalse);

                    // 生成分支表达式
                    self.generate_expression(&arm.body);

                    // 跳到 when 表达式的结尾
                    let end_jump = self.emit_jump(OpCode::Jump);
                    end_jumps.push(end_jump);

                    // 修补跳转到下一个条件的地址
                    self.patch_jump(next_condition);
                }

                // 生成 else 分支（如果存在）
                if let Some(else_expr) = else_expr {
                    self.generate_expression(else_expr);
                } else {
                    // 如果没有 else 分支且所有条件都不满足，生成 null
                    let const_index = self.add_constant(Value::Null);
                    self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                }

                // 修补所有跳转到结尾的地址
                for end_jump in end_jumps {
                    self.patch_jump(end_jump);
                }
            }

            Expression::ArrowFunction { parameters, return_type: _, body, position } => {
                // Generate proper arrow function as closure
                if let Err(e) = self.generate_arrow_function_closure(parameters, body, position) {
                    self.errors.push(CodeGenError::new(e, position.clone()));
                }
            }

            Expression::Match { expression, arms, position } => {
                if let Err(e) = self.generate_match_expression(expression, arms, position) {
                    self.errors.push(e);
                }
            }
            Expression::EnumVariant { enum_name, variant, value, position } => {
                // Generate proper enum variant value
                if let Err(e) = self.generate_enum_variant(enum_name, variant, value, position) {
                    self.errors.push(CodeGenError::new(e, position.clone()));
                }
            }
            Expression::ClassInstantiation { class_name, arguments, position } => {
                // Generate code for class instantiation
                // This is similar to constructor call but uses ClassInstantiation syntax

                // Look for constructor function
                let constructor_name = format!("{}::{}", class_name, class_name);
                if let Some(constructor_info) = self.functions.get(&constructor_name).cloned() {
                    // Constructor parameter count does NOT include 'self' (it's implicit)
                    if constructor_info.parameter_count != arguments.len() {
                        self.errors.push(CodeGenError::new(
                            format!("Constructor '{}' expects {} arguments, got {}",
                                    class_name, constructor_info.parameter_count, arguments.len()),
                            position.clone()
                        ));
                    }

                    // 1. Create new object first
                    let class_index = self.add_constant(Value::String(class_name.clone().into()));
                    self.emit_instruction(Instruction::with_u16(OpCode::NewObject, class_index, 0));

                    // 2. Generate arguments
                    for arg in arguments {
                        self.generate_expression(arg);
                    }

                    // 3. Call constructor with the new object as 'self'
                    let const_index = self.add_constant(Value::String(constructor_name.into()));
                    let param_count = (arguments.len() + 1) as u16; // +1 for self

                    // Create operands: [const_index (2 bytes), param_count (2 bytes)]
                    let mut operands = Vec::new();
                    operands.extend_from_slice(&const_index.to_le_bytes());
                    operands.extend_from_slice(&param_count.to_le_bytes());

                    self.emit_instruction(Instruction::with_operands(OpCode::CallMethod, operands, 0));

                    if self.debug_mode {
                        println!("🔧 CodeGen: Generated class instantiation for '{}'", class_name);
                    }
                } else {
                    // No constructor found, just create empty object
                    let class_index = self.add_constant(Value::String(class_name.clone().into()));
                    self.emit_instruction(Instruction::with_u16(OpCode::NewObject, class_index, 0));

                    if self.debug_mode {
                        println!("🔧 CodeGen: Generated simple object creation for '{}' (no constructor)", class_name);
                    }
                }
            }
            Expression::SuperCall { method_name, arguments, position } => {
                // Generate code for super method call
                // First, load 'self' onto the stack
                if let Some(self_var) = self.variables.get("self") {
                    self.emit_instruction(Instruction::with_u8(OpCode::LoadLocal, self_var.index as u8, 0));
                } else {
                    self.errors.push(CodeGenError::new(
                        "Super method call requires 'self' context".to_string(),
                        position.clone(),
                    ));
                    return;
                }

                // Generate code for all arguments
                for arg in arguments {
                    self.generate_expression(arg);
                }

                // Create method name constant for CallSuper instruction
                let method_const = self.add_constant(Value::String(method_name.clone().into()));

                // Emit CallSuper instruction with method name and argument count
                self.emit_instruction(Instruction::with_u16_u8(OpCode::CallSuper, method_const, (arguments.len() + 1) as u8, 0)); // +1 for self
            }
            Expression::SuperFieldAccess { field_name, position } => {
                // Generate code for super field access
                // First, load 'self' onto the stack
                if let Some(self_var) = self.variables.get("self") {
                    self.emit_instruction(Instruction::with_u8(OpCode::LoadLocal, self_var.index as u8, 0));
                } else {
                    self.errors.push(CodeGenError::new(
                        "Super field access requires 'self' context".to_string(),
                        position.clone(),
                    ));
                    return;
                }

                // Create field name constant for SuperFieldLoad instruction
                let field_const = self.add_constant(Value::String(field_name.clone().into()));

                // Emit SuperFieldLoad instruction
                self.emit_instruction(Instruction::with_u16(OpCode::SuperFieldLoad, field_const, 0));
            }
            Expression::MethodCall { object, method_name, arguments, position } => {
                // Generate code for method call
                // First, generate code for the object (this will be the 'self' parameter)
                self.generate_expression(object);

                // Generate code for all arguments
                for arg in arguments {
                    self.generate_expression(arg);
                }

                // Create method name constant for method call instruction
                let method_const = self.add_constant(Value::String(method_name.clone().into()));

                // Determine if this should be a virtual method call
                let should_use_virtual = self.should_use_virtual_call(object, method_name);

                if should_use_virtual {
                    // Emit CallVirtual instruction for virtual method dispatch
                    self.emit_instruction(Instruction::with_u16_u8(OpCode::CallVirtual, method_const, (arguments.len() + 1) as u8, 0)); // +1 for self
                    if self.debug_mode {
                        println!("🔧 CodeGen: Generated CallVirtual for method '{}'", method_name);
                    }
                } else {
                    // Emit CallMethod instruction for regular method call
                    self.emit_instruction(Instruction::with_u16_u8(OpCode::CallMethod, method_const, (arguments.len() + 1) as u8, 0)); // +1 for self
                    if self.debug_mode {
                        println!("🔧 CodeGen: Generated CallMethod for method '{}'", method_name);
                    }
                }
            }
            Expression::StaticMethodCall { class_name, method_name, arguments, position } => {
                // Generate code for static method call
                // Generate code for all arguments
                for arg in arguments {
                    self.generate_expression(arg);
                }

                // Create the full static method name
                let full_method_name = format!("{}::{}", class_name, method_name);

                // Create method name constant for CallStatic instruction
                let method_const = self.add_constant(Value::String(full_method_name.clone().into()));

                // Emit CallStatic instruction with method name and argument count
                self.emit_instruction(Instruction::with_u16_u8(OpCode::CallStatic, method_const, arguments.len() as u8, 0));
                if self.debug_mode {
                    println!("🔧 CodeGen: Generated CallStatic for method '{}'", full_method_name);
                }
            }
            Expression::StaticFieldAccess { class_name, field_name, position } => {
                // Generate code for static field access
                let full_field_name = format!("{}::{}", class_name, field_name);

                // Look up the static field as a global variable
                if let Some(var_info) = self.variables.get(&full_field_name).cloned() {
                    if var_info.is_global {
                        // Load static field as global variable
                        self.emit_instruction(Instruction::with_u16(OpCode::LoadGlobal, var_info.index as u16, 0));
                    } else {
                        self.errors.push(CodeGenError::new(
                            format!("Static field '{}' should be global", full_field_name),
                            position.clone(),
                        ));
                    }
                } else {
                    self.errors.push(CodeGenError::new(
                        format!("Static field '{}' not found", full_field_name),
                        position.clone(),
                    ));
                }
            }
            Expression::AddressOf { operand, is_mutable, position } => {
                // Generate code for taking address of expression
                match operand.as_ref() {
                    Expression::Identifier(name, _) => {
                        // Taking address of a variable
                        if let Some(var_info) = self.variables.get(name) {
                            // Create a reference to the variable
                            let ref_type = if var_info.is_global {
                                if *is_mutable { "mutable_global" } else { "global" }
                            } else {
                                if *is_mutable { "mutable_local" } else { "local" }
                            };

                            let reference = if *is_mutable {
                                Value::MutableReference {
                                    address: var_info.index,
                                    target_type: ref_type.to_string(),
                                    lifetime: None,
                                }
                            } else {
                                Value::Reference {
                                    address: var_info.index,
                                    target_type: ref_type.to_string(),
                                    lifetime: None,
                                }
                            };
                            let const_index = self.add_constant(reference);
                            self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                        } else {
                            self.errors.push(CodeGenError::new(
                                format!("Variable '{}' not found for address-of operation", name),
                                position.clone(),
                            ));
                        }
                    }
                    _ => {
                        self.errors.push(CodeGenError::new(
                            "Address-of operation only supported for variables".to_string(),
                            position.clone(),
                        ));
                    }
                }
            }
            Expression::Dereference { operand, position } => {
                // Generate code for dereferencing expression
                self.generate_expression(operand);
                // Emit the dereference instruction to convert reference to value
                self.emit_instruction(Instruction::new(OpCode::Dereference, 0));
            }
        }
    }
    
    /// Get current chunk (mutable)
    fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.chunks[self.current_chunk]
    }

    /// Get current chunk (immutable)
    fn current_chunk_ref(&self) -> &Chunk {
        &self.chunks[self.current_chunk]
    }

    /// Generate code for match expression
    fn generate_match_expression(&mut self, expression: &Expression, arms: &[MatchArm], _position: &Position) -> Result<(), CodeGenError> {
        if self.debug_mode {
            println!("🔧 CodeGen: Generating match expression with {} arms", arms.len());
        }

        // Generate the expression to match against
        self.generate_expression(expression);

        // Simple implementation: use if-else chain for pattern matching
        let mut end_jumps = Vec::with_capacity(arms.len());

        for (i, arm) in arms.iter().enumerate() {
            // Duplicate the value on stack for pattern matching
            self.emit_instruction(Instruction::new(OpCode::Dup, 0));

            // Generate pattern matching code
            if let Err(e) = self.generate_pattern_match(&arm.pattern) {
                self.errors.push(e);
                continue;
            }

            // The pattern matching code leaves a boolean on the stack
            // If false, jump to next arm
            let next_arm_jump = self.emit_jump(OpCode::JumpIfFalse);

            // Pattern matched, check guard condition if present
            if let Some(guard) = &arm.guard {
                self.generate_expression(guard);
                let guard_fail_jump = self.emit_jump(OpCode::JumpIfFalse);
                self.patch_jump(guard_fail_jump);
            }

            // Pop the matched value from stack
            self.emit_instruction(Instruction::new(OpCode::Pop, 0));

            // Generate the arm body
            self.generate_expression(&arm.body);

            // Jump to end
            let end_jump = self.emit_jump(OpCode::Jump);
            end_jumps.push(end_jump);

            // Patch the jump to next arm
            self.patch_jump(next_arm_jump);
        }

        // Patch all end jumps
        for jump in end_jumps {
            self.patch_jump(jump);
        }

        Ok(())
    }

    /// Generate pattern matching code
    /// Generates code that leaves a boolean on the stack indicating if the pattern matches
    fn generate_pattern_match(&mut self, pattern: &Pattern) -> Result<(), CodeGenError> {
        match pattern {
            Pattern::Wildcard(_) => {
                // Wildcard always matches - push true
                let true_const = self.add_constant(Value::Bool(true));
                self.emit_instruction(Instruction::with_u16(OpCode::Const, true_const, 0));
                Ok(())
            }
            Pattern::Literal(expr) => {
                // Generate the literal value
                self.generate_expression(expr);
                // Compare with the value on stack
                self.emit_instruction(Instruction::new(OpCode::Eq, 0));
                Ok(())
            }
            Pattern::Identifier(name, _) => {
                // Identifier pattern always matches and binds the value to the identifier
                // First, duplicate the value on stack so we can both bind it and leave it for comparison
                self.emit_instruction(Instruction::new(OpCode::Dup, 0));

                // Bind the value to the identifier as a local variable
                let var_info = VariableInfo {
                    index: self.local_count,
                    is_global: false,
                    var_type: Type::Unknown, // Type will be inferred from the matched value
                };

                self.variables.insert(name.clone(), var_info);
                self.emit_instruction(Instruction::with_u16(OpCode::StoreLocal, self.local_count as u16, 0));
                self.local_count += 1;

                // Pattern always matches - push true
                let true_const = self.add_constant(Value::Bool(true));
                self.emit_instruction(Instruction::with_u16(OpCode::Const, true_const, 0));
                Ok(())
            }
            Pattern::EnumVariant { enum_name, variant, value: _, position } => {
                // For now, enum pattern matching is not implemented
                Err(CodeGenError::new(
                    format!("Enum pattern matching not yet implemented: {}::{}", enum_name, variant),
                    position.clone()
                ))
            }
            Pattern::Tuple(patterns, position) => {
                // For now, tuple pattern matching is not implemented
                Err(CodeGenError::new(
                    format!("Tuple pattern matching not yet implemented ({} elements)", patterns.len()),
                    position.clone()
                ))
            }
            Pattern::Array(patterns, position) => {
                // For now, array pattern matching is not implemented
                Err(CodeGenError::new(
                    format!("Array pattern matching not yet implemented ({} elements)", patterns.len()),
                    position.clone()
                ))
            }
        }
    }

    /// Emit instruction
    fn emit_instruction(&mut self, instruction: Instruction) {
        if self.debug_mode {
            let chunk_name = &self.chunks[self.current_chunk].name;
            let instruction_count = self.chunks[self.current_chunk].instructions.len();
            println!("🔧 CodeGen: Emitting {:?} to chunk '{}' (index {}) at position {}",
                instruction.opcode, chunk_name, self.current_chunk, instruction_count);
        }
        self.current_chunk().add_instruction(instruction);
    }
    
    /// Add constant to current chunk
    fn add_constant(&mut self, value: Value) -> u16 {
        self.current_chunk().add_constant(value)
    }
    
    /// Emit jump instruction and return its address
    fn emit_jump(&mut self, opcode: OpCode) -> usize {
        let address = self.current_chunk_ref().instructions.len();
        self.emit_instruction(Instruction::with_u16(opcode, constants::JUMP_PLACEHOLDER, 0)); // Will be patched later
        address
    }
    
    /// Emit jump to specific address
    fn emit_jump_to(&mut self, opcode: OpCode, target: usize) {
        self.emit_instruction(Instruction::with_u16(opcode, target as u16, 0));
    }
    
    /// Patch jump instruction with current address
    fn patch_jump(&mut self, jump_address: usize) {
        let target = self.current_chunk_ref().instructions.len();
        self.patch_jump_to(jump_address, target);
    }
    
    /// Patch jump instruction with specific target
    fn patch_jump_to(&mut self, jump_address: usize, target: usize) {
        if let Some(instruction) = self.current_chunk().instructions.get_mut(jump_address) {
            // Update the operands to contain the target address
            let target_bytes = (target as u16).to_le_bytes();
            instruction.operands = vec![target_bytes[0], target_bytes[1]];
        }
    }
    
    /// Check if current chunk ends with return
    fn has_return_at_end(&self) -> bool {
        if let Some(last_instruction) = self.chunks[self.current_chunk].instructions.last() {
            matches!(last_instruction.opcode, OpCode::Return | OpCode::ReturnValue)
        } else {
            false
        }
    }

    /// Generate lambda closure
    fn generate_lambda_closure(&mut self, parameters: &[Parameter], body: &Expression, position: &Position) {
        if self.debug_mode {
            println!("🔧 CodeGen: Generating lambda closure with {} parameters", parameters.len());
        }

        // Analyze captured variables before creating the closure
        let parameter_names: std::collections::HashSet<String> = parameters.iter()
            .map(|p| p.name.clone())
            .collect();
        let captured_vars = self.analyze_captured_variables(body, &parameter_names);

        if self.debug_mode {
            println!("🔧 CodeGen: Found {} captured variables: {:?}", captured_vars.len(), captured_vars);
        }

        // Create a new chunk for the lambda
        let lambda_chunk_index = self.chunks.len();
        let lambda_chunk = Chunk::new(format!("lambda_{}", lambda_chunk_index));

        // Save current state
        let old_chunk_index = self.current_chunk;
        let old_variables = self.variables.clone();
        let old_local_count = self.local_count;

        // Switch to lambda chunk
        self.chunks.push(lambda_chunk);
        self.current_chunk = lambda_chunk_index;
        self.local_count = 0;

        // Set up variable mapping for the closure scope
        // Parameters come first, then captured variables

        // Keep only global variables, clear local variables
        self.variables.retain(|_, var_info| var_info.is_global);

        // Add lambda parameters as local variables (starting from index 0)
        for (i, param) in parameters.iter().enumerate() {
            let var_info = VariableInfo {
                index: i,
                is_global: false,
                var_type: if let Some(type_annotation) = &param.type_annotation {
                    Type::from(type_annotation.clone())
                } else {
                    Type::Unknown
                },
            };
            self.variables.insert(param.name.clone(), var_info);
            if self.debug_mode {
                println!("🔧 CodeGen: Added lambda parameter '{}' at index {}", param.name, i);
            }
        }

        // Add captured variables as local variables (after parameters)
        for (i, captured_var_name) in captured_vars.iter().enumerate() {
            let captured_index = parameters.len() + i;
            if let Some(original_var_info) = old_variables.get(captured_var_name) {
                let var_info = VariableInfo {
                    index: captured_index,
                    is_global: false, // Captured variables become local in the closure
                    var_type: original_var_info.var_type.clone(),
                };
                self.variables.insert(captured_var_name.clone(), var_info);
                if self.debug_mode {
                    println!("🔧 CodeGen: Added captured variable '{}' at index {}", captured_var_name, captured_index);
                }
            }
        }

        // Set local count to include parameters and captured variables
        self.local_count = parameters.len() + captured_vars.len();

        // Generate lambda body
        self.generate_expression(body);

        // Add return instruction for lambda result
        self.emit_instruction(Instruction::new(OpCode::ReturnValue, 0));

        // Restore previous state
        self.current_chunk = old_chunk_index;
        self.local_count = old_local_count;

        // Create closure value with captured variables count
        let captured_values = self.create_captured_variable_values(&captured_vars);
        let closure_value = Value::Closure {
            chunk_index: lambda_chunk_index,
            parameter_count: parameters.len(),
            captured_variables: captured_values,
        };

        let const_index = self.add_constant(closure_value);

        // Emit instruction to create closure with captured variables from stack
        if captured_vars.is_empty() {
            // Simple case: no captured variables
            self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
        } else {
            // Complex case: need to create closure with captured variables from stack
            // First emit the closure template
            self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));

            // Then emit code to load captured variables onto the stack
            for captured_var_name in &captured_vars {
                if let Some(var_info) = old_variables.get(captured_var_name) {
                    // Generate code to load the captured variable value onto the stack
                    if var_info.is_global {
                        self.emit_instruction(Instruction::with_u16(OpCode::LoadGlobal, var_info.index as u16, 0));
                    } else {
                        self.emit_instruction(Instruction::with_u8(OpCode::LoadLocal, var_info.index as u8, 0));
                    }

                    if self.debug_mode {
                        println!("🔧 CodeGen: Loading captured variable '{}' from {} index {}",
                                 captured_var_name,
                                 if var_info.is_global { "global" } else { "local" },
                                 var_info.index);
                    }
                }
            }

            // Finally emit the instruction to create closure with captured variables
            self.emit_instruction(Instruction::with_u8(OpCode::CreateClosureWithCaptures, captured_vars.len() as u8, 0));
        }

        // Restore variables after generating the closure
        self.variables = old_variables;

        if self.debug_mode {
            println!("🔧 CodeGen: Generated lambda closure at chunk index {} with {} captured variables",
                     lambda_chunk_index, captured_vars.len());
        }
    }

    /// Analyze which variables are captured by a lambda expression
    fn analyze_captured_variables(&self, expr: &Expression, parameter_names: &std::collections::HashSet<String>) -> Vec<String> {
        let mut captured = Vec::new();
        self.find_captured_variables_in_expression(expr, parameter_names, &mut captured);
        captured.sort();
        captured.dedup();
        captured
    }

    /// Recursively find captured variables in an expression
    fn find_captured_variables_in_expression(&self, expr: &Expression, parameter_names: &std::collections::HashSet<String>, captured: &mut Vec<String>) {
        match expr {
            Expression::Identifier(name, _) => {
                // Check if this identifier refers to a captured variable
                if !parameter_names.contains(name) {
                    if let Some(var_info) = self.variables.get(name) {
                        if !var_info.is_global && !captured.contains(name) {
                            captured.push(name.clone());
                        }
                    }
                }
            }
            Expression::Binary { left, right, .. } => {
                self.find_captured_variables_in_expression(left, parameter_names, captured);
                self.find_captured_variables_in_expression(right, parameter_names, captured);
            }
            Expression::Unary { operand, .. } => {
                self.find_captured_variables_in_expression(operand, parameter_names, captured);
            }
            Expression::Call { function, arguments, .. } => {
                self.find_captured_variables_in_expression(function, parameter_names, captured);
                for arg in arguments {
                    self.find_captured_variables_in_expression(arg, parameter_names, captured);
                }
            }
            // Assignment is handled as a statement, not an expression in this AST
            Expression::Lambda { body, .. } => {
                // For nested lambdas, we need to be careful about scope
                self.find_captured_variables_in_expression(body, parameter_names, captured);
            }
            Expression::Ternary { condition, true_expr, false_expr, .. } => {
                self.find_captured_variables_in_expression(condition, parameter_names, captured);
                self.find_captured_variables_in_expression(true_expr, parameter_names, captured);
                self.find_captured_variables_in_expression(false_expr, parameter_names, captured);
            }
            Expression::InterpolatedString(parts, _) => {
                // InterpolationPart::Expression contains a String, not an Expression
                // Parse each expression string and analyze for captured variables
                for part in parts {
                    if let crate::lang::lexer::InterpolationPart::Expression(expr_str) = part {
                        // Parse the expression string and analyze for captured variables
                        if let Ok(parsed_expr) = self.parse_interpolation_expression_for_analysis(expr_str) {
                            self.find_captured_variables_in_expression(&parsed_expr, parameter_names, captured);
                        }
                        // If parsing fails, we skip this part (no captured variables found)
                    }
                }
            }
            // For other expression types, we don't need to capture variables
            _ => {}
        }
    }

    /// Parse interpolation expression string for captured variable analysis
    fn parse_interpolation_expression_for_analysis(&self, expr_str: &str) -> Result<Expression, String> {
        use crate::lang::lexer::Lexer;
        use crate::lang::parser::Parser;

        let trimmed_expr = expr_str.trim();
        if trimmed_expr.is_empty() {
            return Err("Empty expression".to_string());
        }

        // Create a temporary lexer and parser for the expression
        let mut lexer = Lexer::new(trimmed_expr);
        let tokens = lexer.tokenize().map_err(|e| format!("Lexer error: {}", e))?;

        if tokens.is_empty() {
            return Err("No tokens found".to_string());
        }

        let mut parser = Parser::new(tokens);
        parser.parse_expression().map_err(|e| format!("Parser error: {}", e))
    }

    /// Collect functions from memo declaration (first pass)
    fn collect_memo_functions(&mut self, memo: &MemoDeclaration) {
        match &memo.content {
            MemoContent::Function(func) => {
                // Mark this function as memoized before collecting
                self.memo_functions.insert(func.name.clone());
                self.collect_function(func);
            }
            MemoContent::Namespace(ns) => {
                // Mark all functions in the namespace as memoized
                for item in &ns.items {
                    if let NamespaceItem::Function(func) = item {
                        self.memo_functions.insert(func.name.clone());
                    }
                }
                self.collect_nested_namespace_functions(&[], ns);
            }
            MemoContent::Block(declarations) => {
                for decl in declarations {
                    if let Declaration::Function(func) = decl {
                        // Mark this function as memoized before collecting
                        self.memo_functions.insert(func.name.clone());
                        self.collect_function(func);
                    } else if let Declaration::Namespace(ns) = decl {
                        // Mark all functions in the namespace as memoized
                        for item in &ns.items {
                            if let NamespaceItem::Function(func) = item {
                                self.memo_functions.insert(func.name.clone());
                            }
                        }
                        self.collect_nested_namespace_functions(&[], ns);
                    }
                }
            }
        }
    }

    /// Generate code for memo declaration
    fn generate_memo_declaration(&mut self, memo: &MemoDeclaration) {
        // Mark functions as memoized and generate them normally
        match &memo.content {
            MemoContent::Function(func) => {
                // Mark this function as memoized
                self.memo_functions.insert(func.name.clone());
                self.generate_function(func);
            }
            MemoContent::Namespace(ns) => {
                // Mark all functions in the namespace as memoized
                for item in &ns.items {
                    if let NamespaceItem::Function(func) = item {
                        self.memo_functions.insert(func.name.clone());
                    }
                }
                self.generate_nested_namespace_functions(&[], ns);
            }
            MemoContent::Block(declarations) => {
                for decl in declarations {
                    // Mark functions in the block as memoized
                    if let Declaration::Function(func) = decl {
                        self.memo_functions.insert(func.name.clone());
                    }
                    self.generate_declaration(decl);
                }
            }
        }
    }

    /// Get the list of memoized function names
    pub fn get_memo_functions(&self) -> &HashSet<String> {
        &self.memo_functions
    }

    /// Get library dependencies
    pub fn get_library_dependencies(&self) -> &HashSet<String> {
        &self.library_dependencies
    }

    // ========== Expression Generation Helper Functions ==========

    /// Generate code for integer literal
    fn generate_integer_expression(&mut self, value: i64) {
        let const_index = self.add_constant(Value::Int(value));
        self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
    }

    /// Generate code for float literal
    fn generate_float_expression(&mut self, value: f64) {
        let const_index = self.add_constant(Value::Float(value));
        self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
    }

    /// Generate code for string literal
    fn generate_string_expression(&mut self, value: &str) {
        let const_index = self.add_constant(Value::String(value.to_string().into()));
        self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
    }

    /// Generate code for interpolated string
    fn generate_interpolated_string_expression(&mut self, parts: &[crate::lang::lexer::InterpolationPart]) {
        // 生成字符串插值的字节码
        // 策略：将所有部分连接成一个字符串
        let mut first_part = true;

        for part in parts {
            match part {
                crate::lang::lexer::InterpolationPart::Text(text) => {
                    // 文本部分直接作为常量
                    let const_index = self.add_constant(Value::String(text.clone().into()));
                    self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                }
                crate::lang::lexer::InterpolationPart::Expression(expr_str) => {
                    // 表达式部分需要解析并生成代码
                    match self.parse_and_generate_interpolation_expression(expr_str) {
                        Ok(()) => {
                            // 表达式已经生成到栈上，现在需要转换为字符串
                            self.emit_instruction(Instruction::new(OpCode::ToString, 0));
                        }
                        Err(e) => {
                            // 提供更详细的错误处理
                            let detailed_error = self.format_interpolation_error(expr_str, &e);
                            eprintln!("⚠️ INTERPOLATION ERROR: {}", detailed_error);

                            // 生成更友好的错误信息
                            let error_msg = format!("Parse error in interpolation: {}", expr_str);
                            let const_index = self.add_constant(Value::String(error_msg.into()));
                            self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                        }
                    }
                }
            }

            // 如果不是第一部分，需要连接字符串
            if !first_part {
                self.emit_instruction(Instruction::new(OpCode::Add, 0)); // 使用加法操作符连接字符串
            }
            first_part = false;
        }

        // 如果没有任何部分，生成空字符串
        if first_part {
            let const_index = self.add_constant(Value::String(constants::EMPTY_STRING.to_string().into()));
            self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
        }
    }

    /// Generate code for boolean literal
    fn generate_boolean_expression(&mut self, value: bool) {
        if value {
            self.emit_instruction(Instruction::new(OpCode::True, 0));
        } else {
            self.emit_instruction(Instruction::new(OpCode::False, 0));
        }
    }

    /// Generate code for null literal
    fn generate_null_expression(&mut self) {
        self.emit_instruction(Instruction::new(OpCode::Null, 0));
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGenerator {
    /// Collect methods from impl blocks (first pass)
    fn collect_impl_methods(&mut self, impl_decl: &ImplDeclaration) {
        for method in &impl_decl.methods {
            // Create method name with type prefix for inherent methods
            let method_name = if impl_decl.trait_name.is_some() {
                // For trait implementations, use the original method name
                method.name.clone()
            } else {
                // For inherent implementations, prefix with type name
                format!("{}::{}", impl_decl.type_name, method.name)
            };

            // Create a new chunk for the method
            let chunk_index = self.chunks.len();
            let chunk = Chunk::new(method_name.clone());
            self.chunks.push(chunk);

            let return_type = method.return_type.as_ref()
                .map(|t| Type::from(t.clone()))
                .unwrap_or(Type::Null);

            let func_info = FunctionInfo {
                chunk_index,
                parameter_count: method.parameters.len(),
                return_type,
            };

            self.functions.insert(method_name, func_info);
        }
    }

    /// Collect methods from class declarations (first pass)
    fn collect_class_methods(&mut self, class_decl: &ClassDeclaration) {
        // Collect constructor if present
        if let Some(constructor) = &class_decl.constructor {
            let constructor_name = format!("{}::{}", class_decl.name, class_decl.name);
            let chunk_index = self.chunks.len();
            let chunk = Chunk::new(constructor_name.clone());
            self.chunks.push(chunk);

            let func_info = FunctionInfo {
                chunk_index,
                parameter_count: constructor.parameters.len(),
                return_type: Type::Class {
                    name: class_decl.name.clone(),
                    generic_parameters: vec![],
                    base_class: None,
                    implemented_traits: vec![],
                    fields: vec![],
                    methods: vec![],
                    static_fields: vec![],
                    static_methods: vec![],
                }, // Constructor returns instance
            };

            self.functions.insert(constructor_name, func_info);
        }

        // Collect destructor if present
        if let Some(destructor) = &class_decl.destructor {
            let destructor_name = format!("{}::~{}", class_decl.name, class_decl.name);
            let chunk_index = self.chunks.len();
            let chunk = Chunk::new(destructor_name.clone());
            self.chunks.push(chunk);

            let func_info = FunctionInfo {
                chunk_index,
                parameter_count: 0, // Destructors typically have no parameters
                return_type: Type::Null,
            };

            self.functions.insert(destructor_name, func_info);
        }

        // Collect methods
        for method in &class_decl.methods {
            let method_name = format!("{}::{}", class_decl.name, method.function.name);
            let chunk_index = self.chunks.len();
            let chunk = Chunk::new(method_name.clone());
            self.chunks.push(chunk);

            let return_type = method.function.return_type.as_ref()
                .map(|t| Type::from(t.clone()))
                .unwrap_or(Type::Null);

            let func_info = FunctionInfo {
                chunk_index,
                parameter_count: method.function.parameters.len(),
                return_type,
            };

            self.functions.insert(method_name, func_info);
        }

        // Collect static fields as global variables and handle initialization
        for field in &class_decl.fields {
            if field.is_static {
                let static_field_name = format!("{}::{}", class_decl.name, field.name);
                let global_index = self.global_count;
                self.global_count += 1;

                let var_info = VariableInfo {
                    index: global_index,
                    is_global: true,
                    var_type: Type::from(field.type_annotation.clone()),
                };

                self.variables.insert(static_field_name.clone(), var_info);

                // Generate initialization code if field has an initializer
                if let Some(initializer) = &field.initializer {
                    // Generate the initializer expression
                    self.generate_expression(initializer);

                    // Store the result in the static field (global variable)
                    self.emit_instruction(Instruction::with_u16(OpCode::StoreGlobal, global_index as u16, 0));
                }
            }
        }

        // Collect static methods
        for method in &class_decl.methods {
            if method.is_static {
                let static_method_name = format!("{}::{}", class_decl.name, method.function.name);
                let chunk_index = self.chunks.len();
                let chunk = Chunk::new(static_method_name.clone());
                self.chunks.push(chunk);

                let return_type = method.function.return_type.as_ref()
                    .map(|t| Type::from(t.clone()))
                    .unwrap_or(Type::Null);

                let func_info = FunctionInfo {
                    chunk_index,
                    parameter_count: method.function.parameters.len(),
                    return_type,
                };

                self.functions.insert(static_method_name, func_info);
            }
        }
    }

    /// Generate methods from impl blocks (second pass)
    fn generate_impl_methods(&mut self, impl_decl: &ImplDeclaration) {
        for method in &impl_decl.methods {
            // Create method name with type prefix for inherent methods
            let method_name = if impl_decl.trait_name.is_some() {
                // For trait implementations, use the original method name
                method.name.clone()
            } else {
                // For inherent implementations, prefix with type name
                format!("{}::{}", impl_decl.type_name, method.name)
            };

            // Generate the method like a regular function
            if let Some(func_info) = self.functions.get(&method_name).cloned() {
                let old_chunk = self.current_chunk;
                self.current_chunk = func_info.chunk_index;

                // Generate method body
                self.generate_function_body(method);

                self.current_chunk = old_chunk;
            }
        }
    }

    /// Generate methods from class declarations
    fn generate_class_methods(&mut self, class_decl: &ClassDeclaration) {
        // Register class information
        let class_info = ClassInfo {
            class_index: self.classes.len(), // Use the current number of classes as index
            has_constructor: class_decl.constructor.is_some(),
            field_count: class_decl.fields.len(),
        };
        self.classes.insert(class_decl.name.clone(), class_info);

        // Register inheritance relationship if this class has a base class
        if let Some(base_class_name) = &class_decl.base_class {
            self.class_inheritance.insert(class_decl.name.clone(), base_class_name.clone());
            if self.debug_mode {
                println!("🔧 CodeGen: Registered inheritance: {} extends {}", class_decl.name, base_class_name);
            }
        }

        // Create field access modifiers map
        let mut field_access = std::collections::HashMap::new();
        for field in &class_decl.fields {
            let access_modifier = match field.access_modifier {
                crate::lang::ast::AccessModifier::Public => crate::core::vm::types::AccessModifier::Public,
                crate::lang::ast::AccessModifier::Private => crate::core::vm::types::AccessModifier::Private,
                crate::lang::ast::AccessModifier::Protected => crate::core::vm::types::AccessModifier::Protected,
                crate::lang::ast::AccessModifier::Package => crate::core::vm::types::AccessModifier::Package,
            };
            field_access.insert(field.name.clone(), access_modifier);
        }

        // Create method access modifiers map
        let mut method_access = std::collections::HashMap::new();
        let mut abstract_methods = std::collections::HashMap::new();
        for method in &class_decl.methods {
            let access_modifier = match method.access_modifier {
                crate::lang::ast::AccessModifier::Public => crate::core::vm::types::AccessModifier::Public,
                crate::lang::ast::AccessModifier::Private => crate::core::vm::types::AccessModifier::Private,
                crate::lang::ast::AccessModifier::Protected => crate::core::vm::types::AccessModifier::Protected,
                crate::lang::ast::AccessModifier::Package => crate::core::vm::types::AccessModifier::Package,
            };
            method_access.insert(method.function.name.clone(), access_modifier);
            abstract_methods.insert(method.function.name.clone(), method.is_abstract);
        }

        // Determine package from current namespace
        let package = if let Some(ref namespace) = self.current_namespace {
            // Use the top-level namespace as package
            Some(namespace.split("::").next().unwrap_or(namespace).to_string())
        } else {
            None
        };

        // Create class definition for VM
        let class_definition = crate::core::vm::types::ClassDefinition {
            name: class_decl.name.clone(),
            base_class: class_decl.base_class.clone(),
            fields: class_decl.fields.iter().map(|f| f.name.clone()).collect(),
            methods: class_decl.methods.iter().map(|m| m.function.name.clone()).collect(),
            is_abstract: class_decl.is_abstract,
            implemented_traits: class_decl.implemented_traits.clone(),
            field_access,
            method_access,
            abstract_methods,
            package,
            friend_classes: class_decl.friend_classes.clone(),
        };
        self.class_definitions.insert(class_decl.name.clone(), class_definition);

        // Generate constructor if present
        if let Some(constructor) = &class_decl.constructor {
            let constructor_name = format!("{}::{}", class_decl.name, class_decl.name);

            if let Some(func_info) = self.functions.get(&constructor_name).cloned() {
                let old_chunk = self.current_chunk;
                self.current_chunk = func_info.chunk_index;

                // Generate constructor body with proper parameter handling
                self.generate_constructor_body(constructor);

                self.current_chunk = old_chunk;
            }
        }

        // Generate destructor if present
        if let Some(destructor) = &class_decl.destructor {
            let destructor_name = format!("{}::~{}", class_decl.name, class_decl.name);

            // Register and generate destructor as a function
            if let Some(func_info) = self.functions.get(&destructor_name).cloned() {
                let old_chunk = self.current_chunk;
                self.current_chunk = func_info.chunk_index;

                // Save current variable state
                let old_variables = std::mem::take(&mut self.variables);
                let old_local_count = self.local_count;
                self.local_count = 0;

                // Add 'self' parameter for destructor
                let self_var = VariableInfo {
                    index: 0,
                    is_global: false,
                    var_type: Type::Class {
                        name: class_decl.name.clone(),
                        generic_parameters: vec![],
                        base_class: class_decl.base_class.clone(),
                        implemented_traits: class_decl.implemented_traits.clone(),
                        fields: vec![],
                        methods: vec![],
                        static_fields: vec![],
                        static_methods: vec![],
                    },
                };
                self.variables.insert(constants::SELF_PARAMETER.to_string(), self_var);
                self.local_count = 1;

                // Generate destructor body
                self.generate_statement(&destructor.body);

                // Emit return instruction
                self.emit_instruction(Instruction::new(OpCode::Return, 0));

                // Restore state
                self.variables = old_variables;
                self.local_count = old_local_count;
                self.current_chunk = old_chunk;
            }
        }

        // Generate methods
        for method in &class_decl.methods {
            let method_name = format!("{}::{}", class_decl.name, method.function.name);

            if let Some(func_info) = self.functions.get(&method_name).cloned() {
                let old_chunk = self.current_chunk;
                self.current_chunk = func_info.chunk_index;

                // Generate method body
                self.generate_function_body(&method.function);

                self.current_chunk = old_chunk;
            }
        }
    }

    /// Generate function body (shared by functions and methods)
    fn generate_function_body(&mut self, function: &FunctionDeclaration) {
        // Save current variable context
        let old_variables = self.variables.clone();
        let old_local_count = self.local_count;
        let old_inside_function = self.inside_function;

        // Reset local variable count for this function
        self.local_count = 0;
        self.inside_function = true;

        // Add parameters as local variables
        for (i, param) in function.parameters.iter().enumerate() {
            let var_info = VariableInfo {
                index: i,
                is_global: false,
                var_type: if let Some(type_annotation) = &param.type_annotation {
                    Type::from(type_annotation.clone())
                } else {
                    Type::Unknown
                },
            };
            self.variables.insert(param.name.clone(), var_info);
        }
        self.local_count = function.parameters.len();

        // Generate function body
        self.generate_statement(&function.body);

        // Add return instruction if the function doesn't end with one
        if let Some(last_instruction) = self.chunks[self.current_chunk].instructions.last() {
            if last_instruction.opcode != OpCode::Return {
                self.emit_instruction(Instruction::new(OpCode::Return, 0));
            }
        } else {
            // Empty function body
            self.emit_instruction(Instruction::new(OpCode::Return, 0));
        }

        // Set local count for this chunk (if the field exists)
        // Note: Chunk may not have local_count field, so we skip this for now

        // Restore variable context
        self.variables = old_variables;
        self.local_count = old_local_count;
        self.inside_function = old_inside_function;
    }

    /// Generate constructor body with proper parameter handling
    fn generate_constructor_body(&mut self, constructor: &Constructor) {
        // Save current variable context
        let old_variables = self.variables.clone();
        let old_local_count = self.local_count;

        // Reset local variable count for this constructor
        self.local_count = 0;

        // Add 'self' parameter (always the first parameter for constructors)
        let self_var_info = VariableInfo {
            index: self.local_count,
            is_global: false,
            var_type: Type::Unknown, // Will be determined later
        };
        if self.debug_mode {
            println!("🔧 CodeGen: Constructor - adding 'self' at index {}", self.local_count);
        }
        self.variables.insert("self".to_string(), self_var_info);
        self.local_count += 1;

        // Add constructor parameters to variable scope
        for param in &constructor.parameters {
            let param_type = if let Some(type_annotation) = &param.type_annotation {
                Type::from(type_annotation.clone())
            } else {
                Type::Unknown
            };

            let var_info = VariableInfo {
                index: self.local_count,
                is_global: false,
                var_type: param_type,
            };

            if self.debug_mode {
                println!("🔧 CodeGen: Constructor - adding '{}' at index {}", param.name, self.local_count);
            }
            self.variables.insert(param.name.clone(), var_info);
            self.local_count += 1;
        }

        if self.debug_mode {
            println!("🔧 CodeGen: Constructor - final variables: {:?}",
                     self.variables.iter().map(|(k, v)| (k, v.index)).collect::<Vec<_>>());
        }

        // Generate constructor body
        if self.debug_mode {
            println!("🔧 CodeGen: Constructor - generating body");
        }
        self.generate_statement(&constructor.body);
        if self.debug_mode {
            println!("🔧 CodeGen: Constructor - body generation completed");
        }

        // Constructor should return the modified self object
        // Load self from local variable 0 and return it
        if self.debug_mode {
            println!("🔧 CodeGen: Constructor - about to load self and return");
        }
        self.emit_instruction(Instruction::with_u8(OpCode::LoadLocal, 0, 0)); // Load self
        if self.debug_mode {
            println!("🔧 CodeGen: Constructor - emitted LoadLocal");
        }
        self.emit_instruction(Instruction::new(OpCode::ReturnValue, 0)); // Return self
        if self.debug_mode {
            println!("🔧 CodeGen: Constructor - emitted ReturnValue");
        }

        // Restore variable context
        self.variables = old_variables;
        self.local_count = old_local_count;
    }

    /// Check if an expression is an assignment expression
    fn is_assignment_expression(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Binary { operator, .. } => {
                matches!(operator, BinaryOperator::Assign | BinaryOperator::AddAssign |
                        BinaryOperator::SubtractAssign | BinaryOperator::MultiplyAssign |
                        BinaryOperator::DivideAssign | BinaryOperator::ModuloAssign)
            }
            Expression::Unary { operator, .. } => {
                matches!(operator, UnaryOperator::PreIncrement | UnaryOperator::PostIncrement |
                        UnaryOperator::PreDecrement | UnaryOperator::PostDecrement)
            }
            _ => false,
        }
    }

    /// Generate pipe operation (left |> right)
    /// This transforms `value |> func` into `func(value)`
    fn generate_pipe_operation(&mut self, left: &Expression, right: &Expression) {
        if self.debug_mode {
            println!("🔧 CodeGen: Generating pipe operation");
        }

        // Generate the left side (the value to be piped)
        self.generate_expression(left);

        // The right side should be a function or arrow function
        match right {
            // Direct function call: func |> becomes func(left_value)
            Expression::Identifier(func_name, _) => {
                // Check if it's a function
                if let Some(func_info) = self.functions.get(func_name).cloned() {
                    // Call the function with the piped value as first argument
                    // First push the parameter count to the stack
                    let param_count_const = self.add_constant(Value::Int(1)); // 1 argument (the piped value)
                    self.emit_instruction(Instruction::with_u16(OpCode::Const, param_count_const, 0));
                    // Then call the function
                    self.emit_instruction(Instruction::with_u16(OpCode::Call, func_info.chunk_index as u16, 0));
                } else {
                    // Try to call as a variable containing a function
                    if let Some(var_info) = self.variables.get(func_name).cloned() {
                        self.emit_instruction(Instruction::new(OpCode::LoadLocal, var_info.index as u32));
                        // For function variables, we need to use a different approach
                        // Load function reference and prepare for call
                        if let Err(e) = self.generate_function_variable_pipe_operation(func_name, &var_info) {
                            eprintln!("Warning: Function variable pipe operation failed: {}", e);
                        }
                    } else {
                        eprintln!("Error: Undefined function or variable '{}' in pipe operation", func_name);
                    }
                }
            }

            // Arrow function: value |> (x => x * 2)
            Expression::ArrowFunction { .. } => {
                // Generate the arrow function
                self.generate_expression(right);
                // Call it with the piped value
                // Call arrow function with 1 argument
                // First push the parameter count to the stack
                let param_count_const = self.add_constant(Value::Int(1)); // 1 argument (the piped value)
                self.emit_instruction(Instruction::with_u16(OpCode::Const, param_count_const, 0));
                // Arrow functions are handled differently - this needs more work
                eprintln!("Warning: Arrow functions in pipe operations not fully implemented");
            }

            // Function call: value |> func(arg2, arg3) becomes func(value, arg2, arg3)
            Expression::Call { function, arguments, .. } => {
                // Generate additional arguments first (they'll be on stack after the piped value)
                for arg in arguments {
                    self.generate_expression(arg);
                }

                // Now call the function with piped value + additional arguments
                match function.as_ref() {
                    Expression::Identifier(func_name, _) => {
                        if let Some(func_info) = self.functions.get(func_name).cloned() {
                            // Push parameter count to stack
                            let param_count_const = self.add_constant(Value::Int((arguments.len() + 1) as i64)); // +1 for piped value
                            self.emit_instruction(Instruction::with_u16(OpCode::Const, param_count_const, 0));
                            // Call the function
                            self.emit_instruction(Instruction::with_u16(OpCode::Call, func_info.chunk_index as u16, 0));
                        } else if let Some(var_info) = self.variables.get(func_name).cloned() {
                            self.emit_instruction(Instruction::new(OpCode::LoadLocal, var_info.index as u32));
                            // Function variables need special handling
                            eprintln!("Warning: Function variables in pipe operations not fully implemented");
                        } else {
                            eprintln!("Error: Undefined function '{}' in pipe operation", func_name);
                        }
                    }
                    _ => {
                        // Generate the function expression
                        self.generate_expression(function);
                        // Function expressions need special handling
                        eprintln!("Warning: Function expressions in pipe operations not fully implemented");
                    }
                }
            }

            _ => {
                eprintln!("Error: Right side of pipe operator must be a function or function call");
            }
        }
    }

    /// Parse and generate code for an interpolation expression
    fn parse_and_generate_interpolation_expression(&mut self, expr_str: &str) -> Result<(), String> {
        // 处理空表达式或只包含空格的表达式
        let trimmed_expr = expr_str.trim();
        if trimmed_expr.is_empty() {
            // 空表达式，生成空字符串常量
            let const_index = self.add_constant(Value::String("".to_string().into()));
            self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
            return Ok(());
        }

        // 检查是否包含闭包语法
        if self.contains_closure_syntax(trimmed_expr) {
            return self.generate_closure_interpolation(trimmed_expr);
        }

        // 创建一个临时的词法分析器来解析表达式
        use crate::lang::lexer::Lexer;
        use crate::lang::parser::Parser;

        let mut lexer = Lexer::new(trimmed_expr);
        let tokens = lexer.tokenize().map_err(|e| format!("Lexer error in interpolation '{}': {}", trimmed_expr, e))?;

        if tokens.is_empty() {
            // 如果词法分析后没有令牌，生成空字符串
            let const_index = self.add_constant(Value::String("".to_string().into()));
            self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
            return Ok(());
        }

        let mut parser = Parser::new(tokens);
        let expression = parser.parse_expression().map_err(|e| format!("Parser error in interpolation '{}': {}", trimmed_expr, e))?;

        // 生成表达式的字节码
        self.generate_expression(&expression);

        Ok(())
    }

    /// Check if expression contains closure syntax
    fn contains_closure_syntax(&self, expr_str: &str) -> bool {
        // Simple check for |...| closure pattern
        let pipe_count = expr_str.chars().filter(|&c| c == '|').count();
        pipe_count >= 2 && expr_str.contains('|')
    }

    /// Generate code for closure interpolation
    fn generate_closure_interpolation(&mut self, expr_str: &str) -> Result<(), String> {
        // NOTE: This is a proof-of-concept implementation for closure interpolation
        // A complete implementation would parse closure syntax and generate appropriate bytecode



        // For simple closure expressions, attempt direct evaluation
        // Supported patterns: |x| x + 1, |x| x * 2, |x| x - 1, etc.
        if let Some(result) = self.evaluate_closure_expression(expr_str) {
            let const_index = self.add_constant(result);
            self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
            return Ok(());
        }

        // 检查是否是立即执行的闭包 (例如: |x| x + 1)(5)
        if let Some(call_start) = expr_str.rfind(")(") {
            // 这是一个立即执行的闭包
            let closure_part = &expr_str[..call_start + 1];
            let args_part = &expr_str[call_start + 2..];

            // 简化处理：对于 |x| x + 1)(5) 这样的表达式
            // 我们提取参数并直接计算结果
            if let Some(result) = self.evaluate_simple_closure(closure_part, args_part) {
                let const_index = self.add_constant(result);
                self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                return Ok(());
            }
        }

        // 如果无法处理，生成适当的错误信息
        let error_msg = format!("Unable to evaluate closure expression: {}", expr_str);
        let const_index = self.add_constant(Value::String(error_msg.into()));
        self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
        Ok(())
    }

    /// Evaluate closure expression (without immediate call)
    /// NOTE: This is a placeholder implementation for closure evaluation
    /// In a full implementation, this would parse the closure syntax and create proper closure objects
    fn evaluate_closure_expression(&self, expr_str: &str) -> Option<Value> {
        // 这个方法处理纯闭包表达式，不包含立即调用
        // 例如：|x| x + 1

        // 当前实现：返回闭包的字符串表示，等待后续实现真正的闭包求值
        // 真正的实现应该：
        // 1. 解析闭包语法 (参数列表和函数体)
        // 2. 创建闭包对象
        // 3. 在适当的上下文中求值

        if expr_str.trim().starts_with('|') && expr_str.contains('|') {
            // 返回闭包的字符串表示，而不是硬编码的计算结果
            return Some(Value::String(format!("closure({})", expr_str.trim()).into()));
        }

        None
    }

    /// Evaluate simple closure for demonstration
    /// NOTE: This is a placeholder implementation for closure evaluation
    /// In a full implementation, this would use proper AST parsing and evaluation
    fn evaluate_simple_closure(&self, closure_str: &str, args_str: &str) -> Option<Value> {
        // 当前实现：基本的闭包识别和参数解析
        // 正确的实现应该：
        // 1. 使用现有的词法分析器和语法分析器解析闭包
        // 2. 创建闭包的AST表示
        // 3. 在闭包作用域中求值表达式
        // 4. 支持闭包捕获外部变量

        // 基本的参数解析
        let args_clean = args_str.trim_end_matches(')');

        // 返回闭包调用的占位符表示
        // 在真正的实现中，这里应该执行闭包的AST
        Some(Value::String(format!("closure_call({}, {})", closure_str, args_clean).into()))
    }

    /// Evaluate closure with single parameter
    /// NOTE: Placeholder implementation for single-parameter closure evaluation
    /// In a full implementation, this would parse closure parameters and body properly
    fn evaluate_single_param_closure(&self, closure_str: &str, arg: i64) -> Option<Value> {
        // 当前实现：返回闭包调用的占位符
        // 正确的实现应该：
        // 1. 解析闭包的参数列表和函数体
        // 2. 创建新的作用域，绑定参数
        // 3. 在新作用域中求值函数体表达式
        // 4. 返回求值结果

        Some(Value::String(format!("single_param_closure({}, {})", closure_str, arg).into()))
    }

    /// Evaluate closure with two parameters
    /// NOTE: Placeholder implementation for two-parameter closure evaluation
    /// In a full implementation, this would handle multi-parameter closure parsing and execution
    fn evaluate_two_param_closure(&self, closure_str: &str, arg1: i64, arg2: i64) -> Option<Value> {
        // 当前实现：返回闭包调用的占位符
        // 正确的实现应该：
        // 1. 解析闭包的参数列表 (|x, y|) 和函数体
        // 2. 创建新的作用域，绑定多个参数
        // 3. 在新作用域中求值函数体表达式
        // 4. 支持参数的任意命名，不限于 x, y

        Some(Value::String(format!("two_param_closure({}, {}, {})", closure_str, arg1, arg2).into()))
    }

    /// Evaluate closure body with given arguments
    /// NOTE: Placeholder implementation for closure body evaluation
    /// In a full implementation, this would use the existing expression evaluation system
    fn evaluate_closure_body(&self, body: &str, args: &[i64]) -> Option<Value> {
        // Current implementation: returns placeholder representation of closure body
        // Proper implementation should:
        // 1. Parse closure body as AST expression
        // 2. Use existing generate_expression system
        // 3. Evaluate in scope with parameter bindings
        // 4. Support arbitrary complex expressions, not just simple arithmetic

        let body = body.trim();
        Some(Value::String(format!("closure_body({}, args: {:?})", body, args).into()))
    }

    /// Format a detailed error message for interpolation expression parsing
    fn format_interpolation_error(&self, expr_str: &str, error: &str) -> String {
        let mut detailed_msg = format!("Failed to parse interpolation expression: '${{{}}}'", expr_str);
        detailed_msg.push_str(&format!("\n  Error: {}", error));

        // 提供一些常见错误的建议
        if error.contains("Unterminated string") {
            detailed_msg.push_str("\n  Suggestion: Check for unmatched quotes in string literals");
        } else if error.contains("Unexpected character '\\'") {
            detailed_msg.push_str("\n  Suggestion: String literals in interpolation expressions are not yet fully supported");
            detailed_msg.push_str("\n  Try using variables instead: var msg = \"text\"; \"${msg}\"");
        } else if error.contains("Empty") {
            detailed_msg.push_str("\n  Suggestion: Empty interpolation expressions ${} are now supported and will render as empty strings");
        } else if error.contains("Unexpected token") {
            detailed_msg.push_str("\n  Suggestion: Check expression syntax - ensure all operators and parentheses are correct");
        }

        detailed_msg.push_str(&format!("\n  Expression length: {} characters", expr_str.len()));
        if expr_str.len() > 100 {
            detailed_msg.push_str("\n  Note: Very long expressions may be harder to debug - consider breaking into smaller parts");
        }

        detailed_msg
    }

    /// Generate a simple string representation of an expression for arrow functions
    fn expression_to_string(&self, expr: &Expression) -> String {
        match expr {
            Expression::Integer(i, _) => i.to_string(),
            Expression::Float(f, _) => f.to_string(),
            Expression::String(s, _) => format!("\"{}\"", s),
            Expression::Boolean(b, _) => b.to_string(),
            Expression::Null(_) => constants::NULL_TYPE.to_string(),
            Expression::Identifier(name, _) => name.clone(),
            Expression::Binary { left, operator, right, .. } => {
                let left_str = self.expression_to_string(left);
                let right_str = self.expression_to_string(right);
                format!("{} {:?} {}", left_str, operator, right_str)
            }
            Expression::Unary { operator, operand, .. } => {
                let operand_str = self.expression_to_string(operand);
                format!("{:?}{}", operator, operand_str)
            }
            Expression::Call { function, arguments, .. } => {
                let function_str = self.expression_to_string(function);
                if arguments.is_empty() {
                    format!("{}()", function_str)
                } else {
                    format!("{}(...)", function_str)
                }
            }
            Expression::ArrowFunction { .. } => constants::ARROW_FUNCTION_TYPE.to_string(),
            Expression::When { .. } => constants::WHEN_EXPRESSION_TYPE.to_string(),
            Expression::Ternary { .. } => constants::TERNARY_EXPRESSION_TYPE.to_string(),
            _ => constants::EXPRESSION_TYPE.to_string(),
        }
    }

    /// Generate a simple string representation of a statement for arrow functions
    fn statement_to_string(&self, stmt: &Statement) -> String {
        match stmt {
            Statement::Expression(expression) => self.expression_to_string(expression),
            Statement::Return { value: Some(expr), .. } => {
                format!("return {}", self.expression_to_string(expr))
            }
            Statement::Return { value: None, .. } => constants::RETURN_TYPE.to_string(),
            Statement::VarDeclaration { name, initializer: Some(expr), .. } => {
                format!("var {} = {}", name, self.expression_to_string(expr))
            }
            Statement::VarDeclaration { name, initializer: None, .. } => {
                format!("var {}", name)
            }
            Statement::If { .. } => constants::IF_STATEMENT_TYPE.to_string(),
            Statement::While { .. } => constants::WHILE_LOOP_TYPE.to_string(),
            Statement::For { .. } => constants::FOR_LOOP_TYPE.to_string(),
            Statement::Block { statements, .. } => {
                if statements.is_empty() {
                    constants::EMPTY_BLOCK_TYPE.to_string()
                } else {
                    format!("block({}_statements)", statements.len())
                }
            }
            _ => constants::STATEMENT_TYPE.to_string(),
        }
    }

    /// Handle assignment to various target types (used by compound assignment and increment/decrement)
    fn handle_assignment_target(&mut self, target: &Expression) {
        match target {
            Expression::Identifier(name, position) => {
                if let Some(var_info) = self.variables.get(name).cloned() {
                    // Duplicate the value on stack so assignment returns the assigned value
                    self.emit_instruction(Instruction::new(OpCode::Dup, 0));
                    if var_info.is_global {
                        self.emit_instruction(Instruction::with_u16(OpCode::StoreGlobal, var_info.index as u16, 0));
                    } else {
                        self.emit_instruction(Instruction::with_u8(OpCode::StoreLocal, var_info.index as u8, 0));
                    }
                } else {
                    self.errors.push(CodeGenError::new(
                        format!("Undefined variable '{}'", name),
                        position.clone()
                    ));
                }
            }
            Expression::FieldAccess { object, field, .. } => {
                // Field assignment: object.field = value
                // Stack: [value] (computed value already on stack)
                // We need: [object, value] for StoreField
                // Duplicate the value first so we can return it as the assignment result
                self.emit_instruction(Instruction::new(OpCode::Dup, 0));
                // Stack: [value, value]

                self.generate_expression(object);
                // Now stack is: [value, value, object] but we need [value, object, value]
                // Use Swap instruction to exchange the top two stack values
                self.emit_instruction(Instruction::new(OpCode::Swap, 0));
                // Now stack is: [value, object, value] - correct for StoreField

                let field_index = self.add_constant(Value::String(field.clone().into()));
                self.emit_instruction(Instruction::with_u16(OpCode::StoreField, field_index, 0));

                // After StoreField, the modified object is on the stack: [value, modified_object]
                // We need to store it back to the object variable if it's a local variable
                if let Expression::Identifier(obj_name, _) = object.as_ref() {
                    if let Some(var_info) = self.variables.get(obj_name) {
                        if var_info.is_global {
                            self.emit_instruction(Instruction::with_u16(OpCode::StoreGlobal, var_info.index as u16, 0));
                        } else {
                            self.emit_instruction(Instruction::with_u8(OpCode::StoreLocal, var_info.index as u8, 0));
                        }
                    }
                }
                // Now the assigned value is still on the stack as the result of the assignment expression
            }
            Expression::Dereference { operand, .. } => {
                // Pointer dereference assignment: *ptr = value
                // Stack: [value] (computed value already on stack)
                // We need: [pointer, value] for StoreThrough
                // Duplicate the value first so we can return it as the assignment result
                self.emit_instruction(Instruction::new(OpCode::Dup, 0));
                // Stack: [value, value]

                self.generate_expression(operand);
                // Now stack is: [value, value, pointer] but we need [value, pointer, value]
                // Use Swap instruction to exchange the top two stack values
                self.emit_instruction(Instruction::new(OpCode::Swap, 0));
                // Now stack is: [value, pointer, value] - correct for StoreThrough

                self.emit_instruction(Instruction::new(OpCode::StoreThrough, 0));
                // Now the assigned value is still on the stack as the result of the assignment expression
            }
            _ => {
                // Other assignment types not yet supported
                self.errors.push(CodeGenError::new(
                    "Unsupported assignment target for compound assignment".to_string(),
                    target.position().clone()
                ));
            }
        }
    }

    /// Handle assignment to various target types without duplicating the value (used by postfix operators)
    fn handle_assignment_target_no_dup(&mut self, target: &Expression) {
        match target {
            Expression::Identifier(name, position) => {
                if let Some(var_info) = self.variables.get(name).cloned() {
                    // Don't duplicate - just store the value
                    if var_info.is_global {
                        self.emit_instruction(Instruction::with_u16(OpCode::StoreGlobal, var_info.index as u16, 0));
                    } else {
                        self.emit_instruction(Instruction::with_u8(OpCode::StoreLocal, var_info.index as u8, 0));
                    }
                } else {
                    self.errors.push(CodeGenError::new(
                        format!("Undefined variable '{}'", name),
                        position.clone()
                    ));
                }
            }
            Expression::FieldAccess { object, field, .. } => {
                // Field assignment: object.field = value
                // Stack: [value] (computed value already on stack)
                // We need: [object, value] for StoreField
                self.generate_expression(object);
                // Now stack is: [value, object] but we need [object, value]
                // Use Swap instruction to exchange the top two stack values
                self.emit_instruction(Instruction::new(OpCode::Swap, 0));
                // Now stack is: [object, value] - correct for StoreField

                let field_index = self.add_constant(Value::String(field.clone().into()));
                self.emit_instruction(Instruction::with_u16(OpCode::StoreField, field_index, 0));

                // After StoreField, the modified object is on the stack
                // We need to store it back to the object variable if it's a local variable
                if let Expression::Identifier(obj_name, _) = object.as_ref() {
                    if let Some(var_info) = self.variables.get(obj_name) {
                        if var_info.is_global {
                            self.emit_instruction(Instruction::with_u16(OpCode::StoreGlobal, var_info.index as u16, 0));
                        } else {
                            self.emit_instruction(Instruction::with_u8(OpCode::StoreLocal, var_info.index as u8, 0));
                        }
                    }
                }
            }
            Expression::Dereference { operand, .. } => {
                // Pointer dereference assignment: *ptr = value
                // Stack: [value] (computed value already on stack)
                // We need: [pointer, value] for StoreThrough
                self.generate_expression(operand);
                // Now stack is: [value, pointer] but we need [pointer, value]
                // Use Swap instruction to exchange the top two stack values
                self.emit_instruction(Instruction::new(OpCode::Swap, 0));
                // Now stack is: [pointer, value] - correct for StoreThrough

                self.emit_instruction(Instruction::new(OpCode::StoreThrough, 0));
            }
            _ => {
                // Other assignment types not yet supported
                self.errors.push(CodeGenError::new(
                    "Unsupported assignment target for compound assignment".to_string(),
                    target.position().clone()
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::lexer::Lexer;
    use crate::lang::parser::Parser;
    
    #[test]
    fn test_generate_simple_function() {
        let source = "fn add(x : int, y : int) : int { return x + y; }";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();
        
        let mut generator = CodeGenerator::new();
        let result = generator.generate_program(&program);
        
        assert!(result.is_ok(), "Code generation should succeed");
        let chunks = result.unwrap();
        assert_eq!(chunks.len(), 2); // main + add function
    }
    
    #[test]
    fn test_generate_variable_declaration() {
        let source = "var x : int = 42;";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();
        
        let mut generator = CodeGenerator::new();
        let result = generator.generate_program(&program);
        
        assert!(result.is_ok(), "Code generation should succeed");
    }
}

impl CodeGenerator {
    /// Generate proper arrow function as closure
    fn generate_arrow_function_closure(
        &mut self,
        parameters: &[crate::lang::ast::Parameter],
        body: &crate::lang::ast::ArrowFunctionBody,
        position: &crate::lang::ast::Position,
    ) -> Result<(), String> {
        // Create a new chunk for the arrow function
        let arrow_fn_name = format!("arrow_fn_{}", self.chunks.len());
        let mut arrow_chunk = Chunk::new(arrow_fn_name.clone());

        // Set up parameter information
        // Note: Chunk doesn't have parameter_count or parameter_names fields
        // These will be handled through the closure value itself

        // Save current state
        let saved_chunk_index = self.current_chunk;
        let saved_local_count = self.local_count;
        let saved_variables = self.variables.clone();
        let saved_inside_function = self.inside_function;

        // Switch to arrow function chunk
        let arrow_chunk_index = self.chunks.len();
        self.chunks.push(arrow_chunk);
        self.current_chunk = arrow_chunk_index;
        self.local_count = parameters.len();
        self.inside_function = true;

        // Clear variables and add parameters as local variables
        self.variables.clear();
        for (i, param) in parameters.iter().enumerate() {
            let var_info = VariableInfo {
                index: i,
                is_global: false,
                var_type: param.type_annotation.as_ref()
                    .map(|t| Type::from(t.clone()))
                    .unwrap_or(Type::Unknown),
            };
            self.variables.insert(param.name.clone(), var_info);
        }

        // Generate body
        match body {
            crate::lang::ast::ArrowFunctionBody::Expression(expr) => {
                self.generate_expression(expr);
                self.emit_instruction(Instruction::new(OpCode::ReturnValue, 0));
            }
            crate::lang::ast::ArrowFunctionBody::Block(statements) => {
                for stmt in statements {
                    self.generate_statement(stmt);
                }
                // Ensure return instruction
                if !self.has_return_at_end() {
                    self.emit_instruction(Instruction::new(OpCode::Null, 0));
                    self.emit_instruction(Instruction::new(OpCode::ReturnValue, 0));
                }
            }
        }

        // Set local count for the arrow function chunk
        if let Some(chunk) = self.chunks.get_mut(arrow_chunk_index) {
            chunk.local_count = self.local_count;
        }

        // Restore state
        self.current_chunk = saved_chunk_index;
        self.local_count = saved_local_count;
        self.variables = saved_variables;
        self.inside_function = saved_inside_function;

        // Create closure value
        let closure_value = Value::Closure {
            chunk_index: arrow_chunk_index,
            parameter_count: parameters.len(),
            captured_variables: Vec::new(), // Arrow functions don't capture variables by default
        };

        // Emit closure creation
        let const_index = self.add_constant(closure_value);
        self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));

        Ok(())
    }

    /// Generate enum variant value
    fn generate_enum_variant(
        &mut self,
        enum_name: &str,
        variant: &str,
        value: &Option<Box<Expression>>,
        position: &Position,
    ) -> Result<(), String> {
        // Create enum variant object
        // For now, we represent enum variants as objects with type and value fields

        // Create variant type string
        let variant_type = format!("{}::{}", enum_name, variant);
        let type_const_index = self.add_constant(Value::String(variant_type.into()));

        // Generate value if present
        if let Some(value_expr) = value {
            self.generate_expression(value_expr);
        } else {
            // Use null for variants without values
            self.emit_instruction(Instruction::new(OpCode::Null, 0));
        }

        // Create enum variant object with type and value
        // Stack: [value] -> [enum_variant_object]
        self.emit_instruction(Instruction::with_u16(OpCode::Const, type_const_index, 0)); // Push type
        self.emit_instruction(Instruction::with_u8(OpCode::CreateObject, 2, 0)); // Create object with 2 fields

        Ok(())
    }

    /// Create captured variable values for closures
    fn create_captured_variable_values(&self, captured_vars: &[String]) -> Vec<Value> {
        let mut values = Vec::with_capacity(captured_vars.len());

        for var_name in captured_vars {
            // Try to get the current value of the variable
            if let Some(var_info) = self.variables.get(var_name) {
                // NOTE: Runtime values are not available during compilation
                // These default values will be resolved at runtime
                match var_info.var_type {
                    Type::Int => values.push(Value::Int(0)),
                    Type::Float(_) => values.push(Value::Float(0.0)),
                    Type::Bool => values.push(Value::Bool(false)),
                    Type::String => values.push(Value::String(std::rc::Rc::new(String::new()))),
                    _ => values.push(Value::Null),
                }
            } else {
                // Variable not found, use null
                values.push(Value::Null);
            }
        }

        values
    }

    /// Generate function variable pipe operation
    fn generate_function_variable_pipe_operation(
        &mut self,
        var_name: &str,
        var_info: &VariableInfo,
    ) -> Result<(), String> {
        // Load the function variable
        if var_info.is_global {
            self.emit_instruction(Instruction::with_u16(OpCode::LoadGlobal, var_info.index as u16, 0));
        } else {
            self.emit_instruction(Instruction::with_u8(OpCode::LoadLocal, var_info.index as u8, 0));
        }

        // For function variables in pipe operations, we need to call the function
        // with the piped value as an argument
        // The piped value should already be on the stack

        // Check if this is a known function type
        match &var_info.var_type {
            Type::Function { parameters, .. } => {
                // Call the function with the appropriate number of parameters
                self.emit_instruction(Instruction::with_u8(OpCode::Call, parameters.len() as u8, 0));
            }
            _ => {
                // Assume it's a single-parameter function for pipe operations
                self.emit_instruction(Instruction::with_u8(OpCode::Call, 1, 0));
            }
        }

        Ok(())
    }
}
