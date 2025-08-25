//! Code generator for CodeNothing language
//! 
//! Generates bytecode from analyzed AST

use crate::core::bytecode::{Chunk, Instruction, OpCode};
use crate::core::value::Value;
use crate::lang::ast::*;
use crate::lang::types::{Type, TypeEnvironment};
use crate::lang::builtins::BuiltinRegistry;
use std::collections::{HashMap, HashSet};

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
}

impl CodeGenerator {
    pub fn new() -> Self {
        let mut generator = Self {
            chunks: vec![Chunk::new("main".to_string())],
            current_chunk: 0,
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
        };
        
        // Populate type environment with built-ins
        generator.builtins.populate_type_environment(&mut generator.type_env);
        
        generator
    }

    /// Set debug mode
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
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
        let mut chunks_map = HashMap::new();
        for (i, chunk) in self.chunks.iter().enumerate() {
            // Use the actual chunk name instead of generic chunk_N
            chunks_map.insert(chunk.name.clone(), chunk.clone());
        }
        chunks_map
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
        // For now, assume all methods are virtual except constructors
        // This is a safe default that enables polymorphism

        // Constructors are never virtual
        if method_name == class_name {
            return false;
        }

        // Check if we have class information
        if let Some(class_info) = self.classes.get(class_name) {
            // For now, treat all non-constructor methods as virtual
            // TODO: Add proper virtual method detection based on method modifiers
            return true;
        }

        // Default to virtual for unknown classes
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
                        // TODO: Look up field type in class definition
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
                Declaration::Variable(stmt) => {
                    // Process global variable declaration
                    if let Statement::VarDeclaration { name, .. } = stmt {
                        let var_info = VariableInfo {
                            index: self.global_count,
                            is_global: true,
                            var_type: Type::Unknown, // Will be determined later
                        };
                        self.variables.insert(name.clone(), var_info);
                        self.global_count += 1;
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
        if let Some(main_info) = self.functions.get("main").cloned() {
            let old_chunk = self.current_chunk;
            self.current_chunk = main_info.chunk_index;
            self.emit_instruction(Instruction::new(OpCode::Halt, 0));
            self.current_chunk = old_chunk;
        }
        
        // Set global count for main chunk
        if !self.chunks.is_empty() {
            self.chunks[0].global_count = self.global_count;
        }

        if self.errors.is_empty() {
            Ok(self.chunks.clone())
        } else {
            Err(self.errors.clone())
        }
    }

    /// Generate global variable initializers (executed before main)
    fn generate_global_initializers(&mut self, declaration: &Declaration) {
        match declaration {
            Declaration::Variable(stmt) => {
                // Generate global variable initialization
                self.generate_statement(stmt);
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
        self.current_namespace = Some(current_path.join("::"));

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

    /// Collect nested namespace functions recursively (first pass)
    fn collect_nested_namespace_functions(&mut self, parent_path: &[String], ns: &NamespaceDeclaration) {
        let mut current_path = parent_path.to_vec();
        current_path.push(ns.name.clone());

        for item in &ns.items {
            match item {
                NamespaceItem::Function(func) => {
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
                NamespaceItem::Namespace(nested_ns) => {
                    // Recursively collect nested namespace functions
                    self.collect_nested_namespace_functions(&current_path, nested_ns);
                }
                NamespaceItem::Variable(stmt) => {
                    // Handle namespace variables
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
        let full_name = format!("{}::{}", namespace, function.name);
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
        let chunk_index = if function.name == "main" {
            // Use the existing main chunk created in new()
            0
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
                let namespace_prefix = format!("{}::", using_decl.namespace);

                // Get all builtin function names and create aliases for those in this namespace
                let builtin_names: Vec<String> = self.builtins.get_names().into_iter().cloned().collect();

                for builtin_name in builtin_names {
                    if builtin_name.starts_with(&namespace_prefix) {
                        // Extract the short name (without namespace prefix)
                        let short_name = builtin_name.strip_prefix(&namespace_prefix).unwrap().to_string();

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
                            1
                        };

                        let func_info = FunctionInfo {
                            chunk_index: 0, // Built-ins don't have chunks
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
            Declaration::LibImport(stmt) => {
                // Library imports are handled at compile time, generate the statement
                self.generate_statement(stmt);
            }
        }
    }
    
    /// Generate code for nested namespace functions recursively
    fn generate_nested_namespace_functions(&mut self, parent_path: &[String], ns: &NamespaceDeclaration) {
        let mut current_path = parent_path.to_vec();
        current_path.push(ns.name.clone());

        // Save current namespace context
        let old_namespace = self.current_namespace.clone();
        self.current_namespace = Some(current_path.join("::"));

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

    /// Generate code for nested namespace function
    fn generate_nested_namespace_function(&mut self, namespace_path: &[String], function: &FunctionDeclaration) {
        let full_name = format!("{}::{}", namespace_path.join("::"), function.name);
        if let Some(func_info) = self.functions.get(&full_name).cloned() {
            let old_chunk = self.current_chunk;
            let old_namespace = self.current_namespace.clone();
            self.current_chunk = func_info.chunk_index;
            self.current_namespace = Some(namespace_path.join("::"));

            // Reset local variable count
            let old_local_count = self.local_count;
            self.local_count = 0;

            // Add parameters as local variables
            for (i, param) in function.parameters.iter().enumerate() {
                let var_info = VariableInfo {
                    index: i,
                    is_global: false,
                    var_type: Type::from(param.type_annotation.clone()),
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
        let full_name = format!("{}::{}", namespace, function.name);
        if let Some(func_info) = self.functions.get(&full_name).cloned() {
            let old_chunk = self.current_chunk;
            let old_namespace = self.current_namespace.clone();
            self.current_chunk = func_info.chunk_index;
            self.current_namespace = Some(namespace.to_string());

            // Reset local variable count
            let old_local_count = self.local_count;
            self.local_count = 0;

            // Add parameters as local variables
            for (i, param) in function.parameters.iter().enumerate() {
                let var_info = VariableInfo {
                    index: i,
                    is_global: false,
                    var_type: Type::from(param.type_annotation.clone()),
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

    /// Generate code for function
    fn generate_function(&mut self, function: &FunctionDeclaration) {
        if let Some(func_info) = self.functions.get(&function.name).cloned() {
            let old_chunk = self.current_chunk;
            self.current_chunk = func_info.chunk_index;



            // Reset local variable count
            let old_local_count = self.local_count;
            self.local_count = 0;
            
            // Add parameters as local variables
            for (i, param) in function.parameters.iter().enumerate() {
                let var_info = VariableInfo {
                    index: i,
                    is_global: false,
                    var_type: Type::from(param.type_annotation.clone()),
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
            self.local_count = old_local_count;
            
            // Remove parameters from variable map
            for param in &function.parameters {
                self.variables.remove(&param.name);
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
                
                // Variables are global only if they have Global scope
                let is_global = matches!(scope, VarScope::Global);

                // Get variable info (should already exist for globals)
                let var_info = if is_global {
                    // For global variables, try to find by full name (including namespace)
                    let lookup_name = if let Some(namespace) = &self.current_namespace {
                        format!("{}::{}", namespace, name)
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
                let continue_jumps = self.continue_jumps.clone();
                for continue_jump in continue_jumps {
                    self.patch_jump_to(continue_jump, loop_start);
                }

                self.emit_jump_to(OpCode::Jump, loop_start);
                self.patch_jump(exit_jump);

                // Break jumps to after loop
                let break_jumps = self.break_jumps.clone();
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
                        self.current_chunk_ref().instructions.len() - 1
                    } else {
                        loop_start
                    };
                    
                    let continue_jumps = self.continue_jumps.clone();
                    for continue_jump in continue_jumps {
                        self.patch_jump_to(continue_jump, continue_target);
                    }

                    self.emit_jump_to(OpCode::Jump, loop_start);
                    self.patch_jump(exit_jump);

                    // Break jumps to after loop
                    let break_jumps = self.break_jumps.clone();
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

                let mut case_jumps = Vec::new();
                let mut end_jumps = Vec::new();

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
                self.local_count -= 1;
            }
            Statement::Try { try_block, catch_clauses, finally_block, .. } => {
                // Generate try/catch/finally block

                let try_start_addr = self.current_chunk_ref().instructions.len();
                let catch_offset_placeholder = 0u16; // Will be patched

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
                let catch_offset = (catch_start_addr - try_start_addr - 1) as u16;
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
                // TODO: Emit throw instruction
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

                if let Expression::Range { start, end, .. } = range {
                    // Generate start value and store in variable
                    self.generate_expression(start);

                    // Store in local variable (simplified - should use proper variable management)
                    let var_index = self.local_count;
                    let var_info = VariableInfo {
                        index: var_index,
                        is_global: false,
                        var_type: Type::Int,
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

                    // Compare: i <= end
                    self.emit_instruction(Instruction::new(OpCode::Le, 0));

                    // Jump if false (exit loop)
                    let exit_jump = self.current_chunk().instructions.len();
                    self.emit_instruction(Instruction::with_u16(OpCode::JumpIfFalse, 0, 0)); // Will patch later

                    // Generate loop body
                    self.generate_statement(body);

                    // Increment loop variable: i = i + 1
                    self.emit_instruction(Instruction::with_u16(OpCode::LoadLocal, var_index as u16, 0));
                    let one_const = self.add_constant(Value::Int(1));
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
                        "ForRange statement requires a range expression".to_string(),
                        range.position().clone()
                    ));
                }
            }
        }
    }
    
    /// Generate code for expression
    fn generate_expression(&mut self, expression: &Expression) {
        match expression {
            Expression::Integer(value, _) => {
                let const_index = self.add_constant(Value::Int(*value));
                self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
            }
            Expression::Float(value, _) => {
                let const_index = self.add_constant(Value::Float(*value));
                self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
            }
            Expression::String(value, _) => {
                let const_index = self.add_constant(Value::String(value.clone().into()));
                self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
            }
            Expression::Boolean(value, _) => {
                if *value {
                    self.emit_instruction(Instruction::new(OpCode::True, 0));
                } else {
                    self.emit_instruction(Instruction::new(OpCode::False, 0));
                }
            }
            Expression::Null(_) => {
                self.emit_instruction(Instruction::new(OpCode::Null, 0));
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
                if let Some(var_info) = self.variables.get(name) {
                    if self.debug_mode {
                        println!("🔧 CodeGen: Found variable '{}' at index {}", name, var_info.index);
                    }
                    if var_info.is_global {
                        self.emit_instruction(Instruction::with_u16(OpCode::LoadGlobal, var_info.index as u16, 0));
                    } else {
                        self.emit_instruction(Instruction::with_u8(OpCode::LoadLocal, var_info.index as u8, 0));
                    }
                } else {
                    if self.debug_mode {
                        println!("🔧 CodeGen: Variable '{}' not found", name);
                    }
                    self.errors.push(CodeGenError::new(
                        format!("Undefined variable '{}'", name),
                        position.clone()
                    ));
                }
            }
            Expression::Binary { left, operator, right, .. } => {
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
                    BinaryOperator::Add => self.emit_instruction(Instruction::new(OpCode::Add, 0)),
                    BinaryOperator::Subtract => self.emit_instruction(Instruction::new(OpCode::Sub, 0)),
                    BinaryOperator::Multiply => self.emit_instruction(Instruction::new(OpCode::Mul, 0)),
                    BinaryOperator::Divide => self.emit_instruction(Instruction::new(OpCode::Div, 0)),
                    BinaryOperator::Modulo => self.emit_instruction(Instruction::new(OpCode::Mod, 0)),
                    BinaryOperator::Equal => self.emit_instruction(Instruction::new(OpCode::Eq, 0)),
                    BinaryOperator::NotEqual => self.emit_instruction(Instruction::new(OpCode::Ne, 0)),
                    BinaryOperator::Less => self.emit_instruction(Instruction::new(OpCode::Lt, 0)),
                    BinaryOperator::LessEqual => self.emit_instruction(Instruction::new(OpCode::Le, 0)),
                    BinaryOperator::Greater => self.emit_instruction(Instruction::new(OpCode::Gt, 0)),
                    BinaryOperator::GreaterEqual => self.emit_instruction(Instruction::new(OpCode::Ge, 0)),
                    BinaryOperator::And => self.emit_instruction(Instruction::new(OpCode::And, 0)),
                    BinaryOperator::Or => self.emit_instruction(Instruction::new(OpCode::Or, 0)),
                    BinaryOperator::BitAnd => self.emit_instruction(Instruction::new(OpCode::BitAnd, 0)),
                    BinaryOperator::BitOr => self.emit_instruction(Instruction::new(OpCode::BitOr, 0)),
                    BinaryOperator::BitXor => self.emit_instruction(Instruction::new(OpCode::BitXor, 0)),
                    BinaryOperator::LeftShift => self.emit_instruction(Instruction::new(OpCode::Shl, 0)),
                    BinaryOperator::RightShift => self.emit_instruction(Instruction::new(OpCode::Shr, 0)),
                    BinaryOperator::Assign => {
                        // Assignment is handled differently
                        match left.as_ref() {
                            Expression::Identifier(name, position) => {
                                if let Some(var_info) = self.variables.get(name).cloned() {
                                    // Duplicate the value on stack so assignment returns the assigned value
                                    self.emit_instruction(Instruction::new(OpCode::Dup, 0));
                                    if var_info.is_global {
                                        self.emit_instruction(Instruction::with_u16(OpCode::StoreGlobal, var_info.index as u16, 0));
                                    } else {
                                        self.emit_instruction(Instruction::with_u8(OpCode::StoreLocal, var_info.index as u8, 0));
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

                            Expression::SuperFieldAccess { field_name, .. } => {
                                // Super field assignment: super.field = value
                                // Stack: [value] (right side already generated)
                                // Duplicate the value first so we can return it as the assignment result
                                self.emit_instruction(Instruction::new(OpCode::Dup, 0));
                                // Stack: [value, value]

                                // Load 'self' onto the stack
                                if let Some(self_var) = self.variables.get("self") {
                                    self.emit_instruction(Instruction::with_u8(OpCode::LoadLocal, self_var.index as u8, 0));
                                } else {
                                    self.errors.push(CodeGenError::new(
                                        "Super field assignment requires 'self' context".to_string(),
                                        left.position().clone(),
                                    ));
                                    return;
                                }

                                // Now stack is: [value, value, self] but we need [value, self, value]
                                self.emit_instruction(Instruction::new(OpCode::Swap, 0));
                                // Now stack is: [value, self, value] - correct for SuperFieldStore

                                let field_index = self.add_constant(Value::String(field_name.clone().into()));
                                self.emit_instruction(Instruction::with_u16(OpCode::SuperFieldStore, field_index, 0));

                                // After SuperFieldStore, the modified object is on the stack: [value, modified_object]
                                // Update 'self' with the modified object
                                if let Some(self_var) = self.variables.get("self").cloned() {
                                    self.emit_instruction(Instruction::new(OpCode::Dup, 0)); // Duplicate the object
                                    self.emit_instruction(Instruction::with_u8(OpCode::StoreLocal, self_var.index as u8, 0));
                                    self.emit_instruction(Instruction::new(OpCode::Pop, 0)); // Remove the duplicate
                                }

                                // Pop the modified object, leaving the assigned value on the stack
                                self.emit_instruction(Instruction::new(OpCode::Pop, 0));
                            }

                            Expression::NamespaceAccess { namespace_path, name, position } => {
                                // Static field assignment: ClassName::field = value
                                let full_name = format!("{}::{}", namespace_path.join("::"), name);
                                if let Some(var_info) = self.variables.get(&full_name).cloned() {
                                    if var_info.is_global {
                                        // Duplicate the value on stack so assignment returns the assigned value
                                        self.emit_instruction(Instruction::new(OpCode::Dup, 0));
                                        self.emit_instruction(Instruction::with_u16(OpCode::StoreGlobal, var_info.index as u16, 0));
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
                                    "Unsupported assignment target".to_string(),
                                    Position::new(0, 0)
                                ));
                            }
                        }
                    }

                    // Compound assignment operations
                    BinaryOperator::AddAssign => {
                        // Stack: [current_value, new_value]
                        self.emit_instruction(Instruction::new(OpCode::Add, 0));
                        self.handle_assignment_target(left);
                    }
                    BinaryOperator::SubtractAssign => {
                        // Stack: [current_value, new_value]
                        self.emit_instruction(Instruction::new(OpCode::Sub, 0));
                        self.handle_assignment_target(left);
                    }
                    BinaryOperator::MultiplyAssign => {
                        // Stack: [current_value, new_value]
                        self.emit_instruction(Instruction::new(OpCode::Mul, 0));
                        self.handle_assignment_target(left);
                    }
                    BinaryOperator::DivideAssign => {
                        // Stack: [current_value, new_value]
                        self.emit_instruction(Instruction::new(OpCode::Div, 0));
                        self.handle_assignment_target(left);
                    }
                    BinaryOperator::ModuloAssign => {
                        // Stack: [current_value, new_value]
                        self.emit_instruction(Instruction::new(OpCode::Mod, 0));
                        self.handle_assignment_target(left);
                    }
                }
            }
            Expression::Unary { operator, operand, .. } => {
                match operator {
                    UnaryOperator::Minus | UnaryOperator::Not | UnaryOperator::BitNot => {
                        self.generate_expression(operand);
                        match operator {
                            UnaryOperator::Minus => self.emit_instruction(Instruction::new(OpCode::Neg, 0)),
                            UnaryOperator::Not => self.emit_instruction(Instruction::new(OpCode::Not, 0)),
                            UnaryOperator::BitNot => self.emit_instruction(Instruction::new(OpCode::BitNot, 0)),
                            _ => unreachable!(),
                        }
                    }
                    UnaryOperator::PreIncrement => {
                        // ++x: load x, add 1, store x, result is new value
                        self.generate_expression(operand);
                        let one_index = self.add_constant(Value::Int(1));
                        self.emit_instruction(Instruction::with_u16(OpCode::Const, one_index, 0));
                        self.emit_instruction(Instruction::new(OpCode::Add, 0));
                        self.emit_instruction(Instruction::new(OpCode::Dup, 0)); // Duplicate for return value
                        self.handle_assignment_target(operand);
                    }
                    UnaryOperator::PostIncrement => {
                        // x++: load x, dup, add 1, store x, result is old value
                        self.generate_expression(operand);
                        self.emit_instruction(Instruction::new(OpCode::Dup, 0)); // Keep old value for result
                        let one_index = self.add_constant(Value::Int(1));
                        self.emit_instruction(Instruction::with_u16(OpCode::Const, one_index, 0));
                        self.emit_instruction(Instruction::new(OpCode::Add, 0));
                        // Stack: [old_value, new_value]
                        // Store new value without duplicating
                        self.handle_assignment_target_no_dup(operand);
                        // Stack: [old_value] - this is our result
                    }
                    UnaryOperator::PreDecrement => {
                        // --x: load x, sub 1, store x, result is new value
                        self.generate_expression(operand);
                        let one_index = self.add_constant(Value::Int(1));
                        self.emit_instruction(Instruction::with_u16(OpCode::Const, one_index, 0));
                        self.emit_instruction(Instruction::new(OpCode::Sub, 0));
                        self.emit_instruction(Instruction::new(OpCode::Dup, 0)); // Duplicate for return value
                        self.handle_assignment_target(operand);
                    }
                    UnaryOperator::PostDecrement => {
                        // x--: load x, dup, sub 1, store x, result is old value
                        self.generate_expression(operand);
                        self.emit_instruction(Instruction::new(OpCode::Dup, 0)); // Keep old value for result
                        let one_index = self.add_constant(Value::Int(1));
                        self.emit_instruction(Instruction::with_u16(OpCode::Const, one_index, 0));
                        self.emit_instruction(Instruction::new(OpCode::Sub, 0));
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
                        // First, try to find a regular function with this name
                        if let Some(func_info) = self.functions.get(name).cloned() {
                            if func_info.parameter_count != arguments.len() {
                                self.errors.push(CodeGenError::new(
                                    format!("Function '{}' expects {} arguments, got {}",
                                            name, func_info.parameter_count, arguments.len()),
                                    position.clone()
                                ));
                            }

                            // Check if this is a user-defined function (has a chunk) or a builtin
                            if func_info.chunk_index > 0 {
                                // User-defined function - use Call instruction
                                let param_count_const = self.add_constant(Value::Int(arguments.len() as i64));
                                self.emit_instruction(Instruction::with_u16(OpCode::Const, param_count_const, 0));
                                self.emit_instruction(Instruction::with_u16(OpCode::Call, func_info.chunk_index as u16, 0));
                            } else {
                                // This might be an imported builtin function
                                // Try to find the full name in builtins
                                let mut found_builtin = false;
                                for builtin_name in self.builtins.get_names() {
                                    if builtin_name.ends_with(&format!("::{}", name)) {
                                        // Found the builtin - use CallMethod instruction
                                        let const_index = self.add_constant(Value::String(builtin_name.clone().into()));
                                        self.emit_instruction(Instruction::with_u16(OpCode::CallMethod, const_index, 0));
                                        found_builtin = true;
                                        break;
                                    }
                                }

                                if !found_builtin {
                                    // Regular user-defined function with chunk_index 0 (main function)
                                    let param_count_const = self.add_constant(Value::Int(arguments.len() as i64));
                                    self.emit_instruction(Instruction::with_u16(OpCode::Const, param_count_const, 0));
                                    self.emit_instruction(Instruction::with_u16(OpCode::Call, func_info.chunk_index as u16, 0));
                                }
                            }
                        }
                        // If no regular function found, check if this is a class constructor
                        else {
                            let constructor_name = format!("{}::{}", name, name);
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
                                for _ in 0..arguments.len() {
                                    self.emit_instruction(Instruction::new(OpCode::Pop, 0));
                                }

                                // 1. Create new object first
                                let class_index = self.add_constant(Value::String(name.clone().into()));
                                self.emit_instruction(Instruction::with_u16(OpCode::NewObject, class_index, 0));

                                // 2. Now generate arguments in the correct order
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
                            }
                        }

                        // If still not found, try namespace lookup
                        if !self.functions.contains_key(name) &&
                           !self.functions.contains_key(&format!("{}::{}", name, name)) {
                            if let Some(namespace) = &self.current_namespace {
                            // If we're in a namespace, try to find the function in the current namespace
                            let full_name = format!("{}::{}", namespace, name);
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
                            "Complex function expressions not yet supported".to_string(),
                            position.clone()
                        ));
                    }
                }
            }
            Expression::NamespaceAccess { namespace_path, name, position } => {
                let full_name = format!("{}::{}", namespace_path.join("::"), name);
                if let Some((_, _)) = self.builtins.get(&full_name) {
                    // Built-in function reference
                    let const_index = self.add_constant(Value::String(full_name.into()));
                    self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                } else if self.functions.contains_key(&full_name) {
                    // User-defined namespace function reference
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
            Expression::Range { start, end, .. } => {
                // For now, ranges are only used in for loops and are handled there
                // If we need to support ranges as first-class values, we'd create a range object here
                self.generate_expression(start);
                self.generate_expression(end);
                // TODO: Create range object or handle differently
            }
            Expression::Lambda { parameters: _, body, position } => {
                // For now, lambdas are not fully supported in codegen
                // This is a placeholder implementation
                self.errors.push(CodeGenError::new(
                    "Lambda expressions not yet supported in codegen".to_string(),
                    position.clone()
                ));
                // Generate a placeholder value
                let const_index = self.add_constant(Value::Null);
                self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
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

            Expression::Match { expression, arms, position } => {
                if let Err(e) = self.generate_match_expression(expression, arms, position) {
                    self.errors.push(e);
                }
            }
            Expression::EnumVariant { enum_name: _, variant: _, value, position } => {
                // For now, enum variants are not fully supported in codegen
                // This is a placeholder implementation
                if let Some(value_expr) = value {
                    self.generate_expression(value_expr);
                } else {
                    // Generate a placeholder value for simple variants
                    let const_index = self.add_constant(Value::Null);
                    self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                }
                self.errors.push(CodeGenError::new(
                    "Enum variants not yet supported in codegen".to_string(),
                    position.clone(),
                ));
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
        let mut end_jumps = Vec::new();

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
            Pattern::Identifier(_name, _) => {
                // Identifier pattern always matches
                // TODO: In a full implementation, we'd bind the value to the identifier
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
        self.current_chunk().add_instruction(instruction);
    }
    
    /// Add constant to current chunk
    fn add_constant(&mut self, value: Value) -> u16 {
        self.current_chunk().add_constant(value)
    }
    
    /// Emit jump instruction and return its address
    fn emit_jump(&mut self, opcode: OpCode) -> usize {
        let address = self.current_chunk_ref().instructions.len();
        self.emit_instruction(Instruction::with_u16(opcode, 0, 0)); // Placeholder
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
        for method in &class_decl.methods {
            let access_modifier = match method.access_modifier {
                crate::lang::ast::AccessModifier::Public => crate::core::vm::types::AccessModifier::Public,
                crate::lang::ast::AccessModifier::Private => crate::core::vm::types::AccessModifier::Private,
                crate::lang::ast::AccessModifier::Protected => crate::core::vm::types::AccessModifier::Protected,
                crate::lang::ast::AccessModifier::Package => crate::core::vm::types::AccessModifier::Package,
            };
            method_access.insert(method.function.name.clone(), access_modifier);
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
            // TODO: Register destructor as a function and generate its code
            // For now, just generate the body
            self.generate_statement(&destructor.body);
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

        // Reset local variable count for this function
        self.local_count = 0;

        // Add parameters as local variables
        for (i, param) in function.parameters.iter().enumerate() {
            let var_info = VariableInfo {
                index: i,
                is_global: false,
                var_type: Type::from(param.type_annotation.clone()),
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
            let param_type = Type::from(param.type_annotation.clone());

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
