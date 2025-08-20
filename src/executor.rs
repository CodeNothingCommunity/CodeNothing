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
        
        // Load all chunks into the VM, ensuring main function is at index 0
        let mut function_index_map = HashMap::new();

        // First, load main function if it exists
        if let Some(&main_index) = bytecode.function_map.get("main") {
            if let Some(chunk) = bytecode.get_chunk_by_index(main_index) {
                vm.add_chunk(chunk.clone());
                function_index_map.insert("main".to_string(), 0);
                if self.options.verbose {
                    messages.push(format!("  Loaded function: main"));
                }
            }
        }

        // Then load all other functions
        let mut vm_index = 1;
        for (name, &index) in &bytecode.function_map {
            if name != "main" {
                if let Some(chunk) = bytecode.get_chunk_by_index(index) {
                    vm.add_chunk(chunk.clone());
                    function_index_map.insert(name.clone(), vm_index);
                    vm_index += 1;
                    if self.options.verbose {
                        messages.push(format!("  Loaded function: {}", name));
                    }
                }
            }
        }

        // Set up memoization for memoized functions
        let memo_functions = bytecode.get_memo_functions();
        if self.options.debug {
            println!("🔧 MEMO: Found {} memo functions in bytecode", memo_functions.len());
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

        // Execute the entry point function (main function is usually at index 0)
        let exit_code = match vm.execute(0) {
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
