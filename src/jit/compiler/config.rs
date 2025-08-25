//! JIT compiler configuration

use crate::jit::optimizer::OptimizationLevel;

/// JIT compilation configuration
#[derive(Debug, Clone)]
pub struct JitConfig {
    /// Optimization level for compilation
    pub optimization_level: OptimizationLevel,
    /// Whether to enable debug information
    pub debug_info: bool,
    /// Maximum function size to compile (in instructions)
    pub max_function_size: usize,
    /// Whether to enable aggressive inlining
    pub enable_inlining: bool,
}

impl Default for JitConfig {
    fn default() -> Self {
        Self {
            optimization_level: OptimizationLevel::Aggressive,
            debug_info: false,
            max_function_size: 50000, // Allow larger functions to be compiled
            enable_inlining: true,  // Enable inlining for better performance
        }
    }
}

/// Compilation statistics
#[derive(Debug, Clone, Default)]
pub struct CompilationStats {
    /// Number of functions compiled
    pub functions_compiled: usize,
    /// Number of compilation failures
    pub compilation_failures: usize,
    /// Total compilation time in microseconds
    pub total_compilation_time_us: u64,
    /// Total bytecode instructions processed
    pub total_instructions_processed: usize,
    /// Total native instructions generated (estimated)
    pub total_native_instructions: usize,
}
