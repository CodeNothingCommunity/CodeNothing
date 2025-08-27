//! CodeNothing Bytecode Executor
//! 
//! This module provides execution functionality for compiled
//! CodeNothing bytecode files (.comcn)

use std::path::Path;
use std::collections::HashMap;

use crate::core::{VirtualMachine, bytecode_file::ComcnFile};

/// Execution options
#[derive(Debug, Clone)]
pub struct ExecuteOptions {
    /// Verbose execution output
    pub verbose: bool,
    
    /// Memory configuration in MB
    pub memory_mb: Option<usize>,
    
    /// Stack size in MB
    pub stack_mb: Option<usize>,
    
    /// Enable debugging
    pub debug: bool,
}

impl Default for ExecuteOptions {
    fn default() -> Self {
        Self {
            verbose: false,
            memory_mb: None,
            stack_mb: None,
            debug: false,
        }
    }
}

/// Execution result
#[derive(Debug)]
pub struct ExecuteResult {
    /// Execution success
    pub success: bool,
    
    /// Exit code
    pub exit_code: i32,
    
    /// Execution messages
    pub messages: Vec<String>,
    
    /// Execution time in milliseconds
    pub execution_time_ms: u128,
}

/// CodeNothing Bytecode Executor
pub struct Executor {
    options: ExecuteOptions,
}

impl Executor {
    /// Create a new executor with default options
    pub fn new() -> Self {
        Self {
            options: ExecuteOptions::default(),
        }
    }
    
    /// Create a new executor with custom options
    pub fn with_options(options: ExecuteOptions) -> Self {
        Self { options }
    }
    
    /// Execute a compiled CodeNothing bytecode file
    pub fn execute<P: AsRef<Path>>(&self, bytecode_file: P) -> ExecuteResult {
        let start_time = std::time::Instant::now();
        let mut messages = Vec::new();
        
        if self.options.verbose {
            messages.push(format!("Loading bytecode: {}", bytecode_file.as_ref().display()));
        }
        
        // Load bytecode file
        let bytecode = match ComcnFile::load_from_file(&bytecode_file) {
            Ok(bytecode) => {
                if self.options.verbose {
                    messages.push("Bytecode loaded successfully".to_string());
                    messages.push(format!("  Source: {}", bytecode.source_file));
                    messages.push(format!("  Functions: {}", bytecode.chunks.len()));
                    messages.push(format!("  Entry point: {}", bytecode.entry_point));
                }
                bytecode
            }
            Err(e) => {
                messages.push(format!("Error loading bytecode file: {}", e));
                return ExecuteResult {
                    success: false,
                    exit_code: -1,
                    messages,
                    execution_time_ms: start_time.elapsed().as_millis(),
                };
            }
        };
        
        // Load required libraries
        if !bytecode.library_dependencies.is_empty() {
            if self.options.verbose {
                messages.push(format!("Loading {} required libraries...", bytecode.library_dependencies.len()));
            }

            for lib_name in &bytecode.library_dependencies {
                if self.options.verbose {
                    messages.push(format!("  Loading library: {}", lib_name));
                }

                // Pre-load the library to ensure it's available at runtime
                match crate::library_loader::load_library(lib_name) {
                    Ok(_) => {
                        if self.options.verbose {
                            messages.push(format!("  ✅ Library '{}' loaded successfully", lib_name));
                        }
                    }
                    Err(e) => {
                        messages.push(format!("❌ Failed to load required library '{}': {}", lib_name, e));
                        return ExecuteResult {
                            success: false,
                            exit_code: -1,
                            messages,
                            execution_time_ms: start_time.elapsed().as_millis(),
                        };
                    }
                }
            }
        }

        if self.options.verbose {
            messages.push("Initializing virtual machine...".to_string());
        }
        
        // Create and configure virtual machine
        let mut vm = VirtualMachine::new();

        // Configure memory if specified
        if let Some(memory_mb) = self.options.memory_mb {
            if self.options.verbose {
                messages.push(format!("Memory configuration: {} MB", memory_mb));
            }
        }

        // Enable debug mode if requested
        if self.options.debug {
            vm.set_debug_mode(true);
        }

        if self.options.verbose {
            messages.push("Loading functions into VM...".to_string());
        }
        
        // Load all chunks into the VM in the same order as they were compiled
        // This preserves the chunk indices used in Call instructions
        let mut function_index_map = HashMap::new();

        // Get all chunks sorted by their original index
        let mut chunks_with_indices: Vec<(usize, String, &crate::core::bytecode::Chunk)> = Vec::new();
        for (name, &index) in &bytecode.function_map {
            if let Some(chunk) = bytecode.get_chunk_by_index(index) {
                chunks_with_indices.push((index, name.clone(), chunk));
            }
        }

        // Sort by original index to maintain compilation order
        chunks_with_indices.sort_by_key(|(index, _, _)| *index);

        // Create index mapping: original_index -> vm_index
        let mut index_mapping = HashMap::new();

        // Load chunks in their original order and create mapping
        for (vm_index, (original_index, name, chunk)) in chunks_with_indices.iter().enumerate() {
            vm.add_chunk((*chunk).clone());
            function_index_map.insert(name.clone(), *original_index);
            index_mapping.insert(*original_index, vm_index);
            if self.options.verbose {
                messages.push(format!("  Loaded function: {} (original index {}, VM index {})", name, original_index, vm_index));
            }
        }

        // Set the index mapping in the VM
        vm.set_function_index_mapping(index_mapping.clone());

        // Set up memoization for memoized functions
        let memo_functions = bytecode.get_memo_functions();
        if self.options.debug {
            println!("🔧 MEMO: Found {} memo functions in bytecode", memo_functions.len());
        }

        // Register class inheritance relationships
        for (child_class, parent_class) in &bytecode.class_inheritance {
            vm.register_class_inheritance(child_class.clone(), parent_class.clone());
            if self.options.verbose {
                messages.push(format!("  Registered inheritance: {} extends {}", child_class, parent_class));
            }
        }

        // Register class definitions
        for (class_name, class_def) in &bytecode.class_definitions {
            vm.register_class_definition(class_def.clone());
            if self.options.verbose {
                messages.push(format!("  Registered class definition: {}", class_name));
            }
        }

        // Build VTables for all classes
        for class_name in bytecode.class_definitions.keys() {
            if let Err(e) = vm.build_vtable(class_name) {
                messages.push(format!("  Warning: Failed to build VTable for '{}': {}", class_name, e));
            } else if self.options.verbose {
                messages.push(format!("  Built VTable for class: {}", class_name));
            }
        }

        // Validate that all concrete classes implement required abstract methods
        for class_name in bytecode.class_definitions.keys() {
            if let Err(e) = vm.validate_concrete_class(class_name) {
                messages.push(format!("  Warning: Abstract method validation failed for '{}': {}", class_name, e));
            } else if self.options.verbose {
                messages.push(format!("  Validated concrete class: {}", class_name));
            }
        }
        for memo_func_name in memo_functions {
            if self.options.debug {
                println!("🔧 MEMO: Processing memo function: {}", memo_func_name);
            }
            if let Some(&vm_index) = function_index_map.get(memo_func_name) {
                if self.options.debug {
                    println!("🔧 MEMO: Marking function {} (index {}) as memoized", memo_func_name, vm_index);
                }
                vm.mark_function_as_memoized(vm_index);
                if self.options.verbose {
                    messages.push(format!("  Marked function as memoized: {}", memo_func_name));
                }
            } else if self.options.debug {
                println!("🔧 MEMO: ERROR - Function {} not found in function_index_map", memo_func_name);
                println!("🔧 MEMO: Available functions: {:?}", function_index_map.keys().collect::<Vec<_>>());
            }
        }
        
        if self.options.verbose {
            messages.push("Starting execution...".to_string());
        }

        // Enable debug mode if verbose
        if self.options.verbose {
            vm.set_debug_mode(true);
        }

        // Find and execute the main function
        let main_original_index = if let Some(&main_index) = bytecode.function_map.get("main") {
            main_index
        } else {
            messages.push("No main function found in bytecode".to_string());
            return ExecuteResult {
                success: false,
                exit_code: -1,
                messages,
                execution_time_ms: start_time.elapsed().as_millis(),
            };
        };

        // Convert original index to VM index
        let main_vm_index = if let Some(&vm_index) = index_mapping.get(&main_original_index) {
            vm_index
        } else {
            messages.push(format!("Main function index {} not found in VM mapping", main_original_index));
            return ExecuteResult {
                success: false,
                exit_code: -1,
                messages,
                execution_time_ms: start_time.elapsed().as_millis(),
            };
        };

        if self.options.verbose {
            messages.push(format!("Executing main function at original index {} (VM index {})...", main_original_index, main_vm_index));
        }

        // Execute the main function
        let exit_code = match vm.execute(main_vm_index) {
            Ok(result) => {
                if self.options.verbose {
                    messages.push("Execution completed successfully".to_string());
                }
                
                // Try to extract exit code from result
                match result {
                    crate::core::Value::Int(code) => code as i32,
                    _ => 0,
                }
            }
            Err(e) => {
                messages.push(format!("Runtime error: {:?}", e));
                return ExecuteResult {
                    success: false,
                    exit_code: -1,
                    messages,
                    execution_time_ms: start_time.elapsed().as_millis(),
                };
            }
        };
        
        let execution_time = start_time.elapsed().as_millis();
        
        if self.options.verbose {
            messages.push(format!("Execution time: {} ms", execution_time));
            messages.push(format!("Exit code: {}", exit_code));
        }
        
        ExecuteResult {
            success: true,
            exit_code,
            messages,
            execution_time_ms: execution_time,
        }
    }
    
    /// Get information about a bytecode file without executing it
    pub fn info<P: AsRef<Path>>(&self, bytecode_file: P) -> Result<String, Box<dyn std::error::Error>> {
        let bytecode = ComcnFile::load_from_file(bytecode_file)?;
        Ok(bytecode.info())
    }
}
