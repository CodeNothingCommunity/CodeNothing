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

/// Code generator
pub struct CodeGenerator {
    chunks: Vec<Chunk>,
    current_chunk: usize,
    current_namespace: Option<String>,
    variables: HashMap<String, VariableInfo>,
    functions: HashMap<String, FunctionInfo>,
    builtins: BuiltinRegistry,
    type_env: TypeEnvironment,
    local_count: usize,
    global_count: usize,
    errors: Vec<CodeGenError>,
    break_jumps: Vec<usize>,    // Jump addresses for break statements
    continue_jumps: Vec<usize>, // Jump addresses for continue statements
    memo_functions: HashSet<String>, // Names of memoized functions
}

impl CodeGenerator {
    pub fn new() -> Self {
        let mut generator = Self {
            chunks: vec![Chunk::new("main".to_string())],
            current_chunk: 0,
            current_namespace: None,
            variables: HashMap::new(),
            functions: HashMap::new(),
            builtins: BuiltinRegistry::new(),
            type_env: TypeEnvironment::new(),
            local_count: 0,
            global_count: 0,
            errors: Vec::new(),
            break_jumps: Vec::new(),
            continue_jumps: Vec::new(),
            memo_functions: HashSet::new(),
        };
        
        // Populate type environment with built-ins
        generator.builtins.populate_type_environment(&mut generator.type_env);
        
        generator
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
                _ => {}
            }
        }

        // Second pass: generate code for all declarations
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
        
        if self.errors.is_empty() {
            Ok(self.chunks.clone())
        } else {
            Err(self.errors.clone())
        }
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
                NamespaceItem::Variable(_) => {
                    // TODO: Handle namespace variables
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
        }
    }
    
    /// Generate code for nested namespace functions recursively
    fn generate_nested_namespace_functions(&mut self, parent_path: &[String], ns: &NamespaceDeclaration) {
        let mut current_path = parent_path.to_vec();
        current_path.push(ns.name.clone());

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
                    self.generate_statement(stmt);
                }
            }
        }
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
                
                let is_global = matches!(scope, VarScope::Global);

                // Get variable info (should already exist for globals)
                let var_info = if is_global {
                    if let Some(existing_info) = self.variables.get(name) {
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
                if let Some(var_info) = self.variables.get(name) {
                    if var_info.is_global {
                        self.emit_instruction(Instruction::with_u16(OpCode::LoadGlobal, var_info.index as u16, 0));
                    } else {
                        self.emit_instruction(Instruction::with_u8(OpCode::LoadLocal, var_info.index as u8, 0));
                    }
                } else {
                    self.errors.push(CodeGenError::new(
                        format!("Undefined variable '{}'", name),
                        position.clone()
                    ));
                }
            }
            Expression::Binary { left, operator, right, .. } => {
                self.generate_expression(left);
                self.generate_expression(right);
                
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
                        if let Expression::Identifier(name, position) = left.as_ref() {
                            if let Some(var_info) = self.variables.get(name) {
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
                    }
                }
            }
            Expression::Unary { operator, operand, .. } => {
                self.generate_expression(operand);
                
                match operator {
                    UnaryOperator::Minus => self.emit_instruction(Instruction::new(OpCode::Neg, 0)),
                    UnaryOperator::Not => self.emit_instruction(Instruction::new(OpCode::Not, 0)),
                    UnaryOperator::BitNot => self.emit_instruction(Instruction::new(OpCode::BitNot, 0)),
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
                        } else if let Some(namespace) = &self.current_namespace {
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
                            self.errors.push(CodeGenError::new(
                                format!("Undefined function '{}'", name),
                                position.clone()
                            ));
                        }
                    }
                    Expression::NamespaceAccess { namespace_path, name, .. } => {
                        let full_name = format!("{}::{}", namespace_path.join("::"), name);
                        if let Some((_, _)) = self.builtins.get(&full_name) {
                            // Built-in function call
                            let const_index = self.add_constant(Value::String(full_name.into()));
                            self.emit_instruction(Instruction::with_u16(OpCode::CallMethod, const_index, 0));
                        } else if let Some(func_info) = self.functions.get(&full_name).cloned() {
                            // User-defined namespace function call
                            let param_count = func_info.parameter_count;
                            let const_index = self.add_constant(Value::Int(param_count as i64));
                            self.emit_instruction(Instruction::with_u16(OpCode::Const, const_index, 0));
                            self.emit_instruction(Instruction::with_u16(OpCode::Call, func_info.chunk_index as u16, 0));
                        } else {
                            let namespace_str = namespace_path.join("::");
                            self.errors.push(CodeGenError::new(
                                format!("Undefined function '{}' in namespace '{}'", name, namespace_str),
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
                self.emit_instruction(Instruction::with_u16(OpCode::LoadField, 0, 0)); // Use 0 as placeholder for array access
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
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
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
