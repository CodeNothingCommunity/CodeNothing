//! Main JIT compiler implementation

use crate::core::bytecode::Chunk;
use crate::jit::native_function::{NativeFunction, FunctionSignature, FunctionMetadata};
use crate::jit::optimizer::JitOptimizer;

use super::{
    config::{JitConfig, CompilationStats},
    analysis::CodeAnalyzer,
    cranelift_backend::CraneliftBackend,
    linear_compiler::LinearCompiler,
    recursive_compiler::RecursiveCompiler,
};

use cranelift::prelude::*;
use std::fmt;

type CraneliftValue = cranelift::prelude::Value;

/// JIT compiler that translates bytecode to native code
pub struct JitCompiler {
    /// Cranelift backend
    backend: CraneliftBackend,
    /// Optimizer for bytecode preprocessing
    optimizer: JitOptimizer,
    /// Compilation configuration
    config: JitConfig,
    /// Compilation statistics
    stats: CompilationStats,
}

impl JitCompiler {
    /// Create a new JIT compiler with default configuration
    pub fn new() -> Self {
        Self::with_config(JitConfig::default())
    }

    /// Create a new JIT compiler with custom configuration
    pub fn with_config(config: JitConfig) -> Self {
        let backend = CraneliftBackend::new();
        let optimizer = JitOptimizer::new(config.optimization_level);

        Self {
            backend,
            optimizer,
            config,
            stats: CompilationStats::default(),
        }
    }

    /// Compile a bytecode chunk to native code
    pub fn compile_chunk(&mut self, chunk: &Chunk) -> Result<NativeFunction, String> {
        self.compile_chunk_with_debug(chunk, false)
    }

    /// Compile a bytecode chunk to native code with optional debug output
    pub fn compile_chunk_with_debug(&mut self, chunk: &Chunk, debug: bool) -> Result<NativeFunction, String> {
        let start_time = std::time::Instant::now();

        // Create analyzer and compilers
        let analyzer = CodeAnalyzer::new(&self.config);
        let linear_compiler = LinearCompiler::new(&self.backend);
        let recursive_compiler = RecursiveCompiler::new(&analyzer, &linear_compiler);

        // Check if function is suitable for JIT compilation
        if !analyzer.is_suitable_for_jit_with_debug(chunk, debug) {
            self.stats.compilation_failures += 1;
            return Err("Function not suitable for JIT compilation".to_string());
        }

        if debug {
            println!("🚀 JIT: Starting compilation for function: {}", chunk.name);
            println!("  - Instructions: {}", chunk.instruction_count());
            println!("  - Constants: {}", chunk.constants.len());
        }

        // Update statistics
        self.stats.total_instructions_processed += chunk.instruction_count();

        // Determine compilation strategy
        let result = if analyzer.has_recursive_pattern(chunk) {
            if debug {
                println!("🔄 JIT: Detected recursive pattern, using block-based compilation");
            }
            
            if analyzer.is_simple_recursive_function_with_debug(chunk, debug) {
                // Try block-based compilation for recursive functions
                let param_count = analyzer.count_function_parameters(chunk);
                let mut params: Vec<CraneliftValue> = Vec::new();
                
                // Create dummy parameters for compilation
                // In a real implementation, these would be provided by the caller
                // For now, we'll use empty parameters since the linear compiler doesn't use them
                let params = vec![];
                
                recursive_compiler.compile_recursive_function_blocks_with_debug(chunk, params, debug)
            } else {
                Err("Recursive function too complex for JIT compilation".to_string())
            }
        } else {
            if debug {
                println!("📈 JIT: Non-recursive function, using standard compilation");
            }
            
            // Standard compilation for non-recursive functions
            self.compile_standard_function_with_debug(chunk, debug)
        };

        // Update statistics
        let compilation_time = start_time.elapsed().as_micros() as u64;
        self.stats.total_compilation_time_us += compilation_time;

        match result {
            Ok(native_func) => {
                self.stats.functions_compiled += 1;
                if debug {
                    println!("✅ JIT: Compilation successful in {}μs", compilation_time);
                }
                Ok(native_func)
            }
            Err(e) => {
                self.stats.compilation_failures += 1;
                if debug {
                    println!("❌ JIT: Compilation failed: {}", e);
                }
                Err(e)
            }
        }
    }

    /// Compile a standard (non-recursive) function
    fn compile_standard_function(&mut self, chunk: &Chunk) -> Result<NativeFunction, String> {
        self.compile_standard_function_with_debug(chunk, false)
    }

    /// Compile a standard (non-recursive) function with debug output
    fn compile_standard_function_with_debug(&mut self, chunk: &Chunk, debug: bool) -> Result<NativeFunction, String> {
        if debug {
            println!("🔧 Standard JIT: Compiling non-recursive function");
        }

        // For now, use the linear compiler for standard functions
        let analyzer = CodeAnalyzer::new(&self.config);
        let linear_compiler = LinearCompiler::new(&self.backend);
        
        let safe_instructions = analyzer.extract_safe_instructions(chunk);

        if safe_instructions.is_empty() {
            return Err("No safe instructions found for compilation".to_string());
        }

        // Create dummy parameters
        let params = vec![];

        linear_compiler.compile_linear_instructions_with_debug(&safe_instructions, params, debug)
    }

    /// Get compilation statistics
    pub fn get_stats(&self) -> &CompilationStats {
        &self.stats
    }

    /// Reset compilation statistics
    pub fn reset_stats(&mut self) {
        self.stats = CompilationStats::default();
    }

    /// Set optimization level
    pub fn set_optimization_level(&mut self, level: crate::jit::optimizer::OptimizationLevel) {
        self.config.optimization_level = level;
        self.optimizer = JitOptimizer::new(level);
    }
}

impl fmt::Debug for JitCompiler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JitCompiler")
            .field("config", &self.config)
            .field("stats", &self.stats)
            .finish()
    }
}

impl Default for JitCompiler {
    fn default() -> Self {
        Self::new()
    }
}
