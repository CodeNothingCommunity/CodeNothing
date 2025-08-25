//! JIT (Just-In-Time) compilation module for CodeNothing VM
//! 
//! This module provides JIT compilation capabilities using Cranelift,
//! including hot spot detection, native function management, and
//! performance optimization.

pub mod compiler;
pub mod native_function;
pub mod hotspot;
pub mod optimizer;

pub use compiler::*;
pub use native_function::*;
pub use hotspot::*;
pub use optimizer::*;

use crate::core::bytecode::Chunk;
use crate::core::value::Value;
use std::collections::HashMap;

/// Compilation status for a function
#[derive(Debug, Clone, PartialEq)]
pub enum CompilationStatus {
    /// Function has not been evaluated for JIT compilation yet
    NotEvaluated,
    /// Function is being considered for compilation (execution count tracking)
    Evaluating,
    /// Function was deemed not suitable for JIT compilation
    NotSuitable,
    /// Function is currently being compiled
    Compiling,
    /// Function has been successfully compiled
    Compiled,
    /// Function compilation failed
    Failed(String),
}

/// JIT compilation result
#[derive(Debug)]
pub enum JitResult {
    /// Successfully compiled to native code
    Compiled(NativeFunction),
    /// Function not suitable for JIT compilation
    NotSuitable(String),
    /// Compilation failed
    Failed(String),
}

/// JIT compilation statistics
#[derive(Debug, Clone, Default)]
pub struct JitStats {
    /// Number of functions compiled
    pub functions_compiled: usize,
    /// Number of compilation attempts
    pub compilation_attempts: usize,
    /// Number of compilation failures
    pub compilation_failures: usize,
    /// Total time spent compiling (in microseconds)
    pub compilation_time_us: u64,
    /// Number of JIT function calls
    pub jit_calls: usize,
    /// Time saved by JIT execution (estimated, in microseconds)
    pub time_saved_us: u64,
}

/// JIT engine that manages compilation and execution
#[derive(Debug)]
pub struct JitEngine {
    /// JIT compiler instance
    compiler: JitCompiler,
    /// Hot spot detector
    hotspot_detector: HotspotDetector,
    /// Compiled native functions cache
    native_functions: HashMap<usize, NativeFunction>,
    /// JIT compilation statistics
    stats: JitStats,
    /// Whether JIT is enabled
    enabled: bool,
    /// Minimum execution count before considering JIT compilation
    jit_threshold: u32,
    /// Whether to show debug output
    pub debug: bool,
    /// Compilation status for each function
    compilation_status: HashMap<usize, CompilationStatus>,
}

impl JitEngine {
    /// Create a new JIT engine
    pub fn new() -> Self {
        Self {
            compiler: JitCompiler::new(),
            hotspot_detector: HotspotDetector::new(),
            native_functions: HashMap::new(),
            stats: JitStats::default(),
            enabled: true,
            jit_threshold: 5, // Very aggressive threshold for maximum performance
            debug: false,
            compilation_status: HashMap::new(),
        }
    }

    /// Create a JIT engine with custom threshold
    pub fn with_threshold(threshold: u32) -> Self {
        let mut engine = Self::new();
        engine.jit_threshold = threshold;
        engine
    }

    /// Enable or disable JIT compilation
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if JIT is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable debug output
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
        if debug {
            println!("🔧 JIT: Debug mode set to {}", debug);
        }
    }

    /// Record function execution for hot spot detection
    pub fn record_execution(&mut self, chunk_index: usize, execution_time_us: u64) {
        if !self.enabled {
            if self.debug {
                println!("🔧 JIT: Disabled, not recording execution for function {}", chunk_index);
            }
            return;
        }

        if self.debug {
            println!("🔧 JIT: Recording execution for function {}, time: {}us", chunk_index, execution_time_us);
        }
        self.hotspot_detector.record_execution(chunk_index, execution_time_us);
    }

    /// Check if a function should be JIT compiled
    pub fn should_compile(&mut self, chunk_index: usize) -> bool {
        if self.debug {
            println!("🔧 JIT: should_compile called for function {}", chunk_index);
        }

        // Check if JIT is enabled
        if !self.enabled {
            if self.debug {
                println!("🔧 JIT: JIT is disabled");
            }
            return false;
        }

        // Check current compilation status
        let current_status = self.compilation_status.get(&chunk_index).cloned()
            .unwrap_or(CompilationStatus::NotEvaluated);

        match current_status {
            CompilationStatus::NotEvaluated => {
                // First time seeing this function, start evaluating
                self.compilation_status.insert(chunk_index, CompilationStatus::Evaluating);
                if self.debug {
                    println!("🔧 JIT: Function {} now being evaluated", chunk_index);
                }
                false // Don't compile yet, just start tracking
            }
            CompilationStatus::Evaluating => {
                // Check if it has become a hotspot
                let execution_count = self.hotspot_detector.get_execution_count(chunk_index);
                let is_hotspot = self.hotspot_detector.is_hotspot(chunk_index, self.jit_threshold);

                if is_hotspot {
                    // Already compiled?
                    if self.native_functions.contains_key(&chunk_index) {
                        self.compilation_status.insert(chunk_index, CompilationStatus::Compiled);
                        if self.debug {
                            println!("🔧 JIT: Function {} already compiled", chunk_index);
                        }
                        false
                    } else {
                        // Ready to compile
                        self.compilation_status.insert(chunk_index, CompilationStatus::Compiling);
                        if self.debug {
                            println!("🔧 JIT: Function {} ready for compilation (executions: {})",
                                chunk_index, execution_count);
                        }
                        true
                    }
                } else {
                    if self.debug {
                        println!("🔧 JIT: Function {} not yet a hotspot (executions: {})",
                            chunk_index, execution_count);
                    }
                    false
                }
            }
            CompilationStatus::NotSuitable => {
                if self.debug {
                    println!("🔧 JIT: Function {} marked as not suitable", chunk_index);
                }
                false
            }
            CompilationStatus::Compiling => {
                if self.debug {
                    println!("🔧 JIT: Function {} is currently being compiled", chunk_index);
                }
                false // Avoid re-entry
            }
            CompilationStatus::Compiled => {
                if self.debug {
                    println!("🔧 JIT: Function {} already compiled", chunk_index);
                }
                false
            }
            CompilationStatus::Failed(_) => {
                if self.debug {
                    println!("🔧 JIT: Function {} compilation previously failed", chunk_index);
                }
                false
            }
        }
    }



    /// Attempt to JIT compile a chunk
    pub fn compile_chunk(&mut self, chunk_index: usize, chunk: &Chunk) -> JitResult {
        if !self.enabled {
            self.compilation_status.insert(chunk_index, CompilationStatus::NotSuitable);
            return JitResult::NotSuitable("JIT disabled".to_string());
        }

        // Mark as compiling
        self.compilation_status.insert(chunk_index, CompilationStatus::Compiling);

        self.stats.compilation_attempts += 1;
        let start_time = std::time::Instant::now();

        match self.compiler.compile_chunk_with_debug(chunk, self.debug) {
            Ok(native_function) => {
                let compilation_time = start_time.elapsed().as_micros() as u64;
                self.stats.compilation_time_us += compilation_time;
                self.stats.functions_compiled += 1;

                // Cache the compiled function
                self.native_functions.insert(chunk_index, native_function.clone());

                // Mark as successfully compiled
                self.compilation_status.insert(chunk_index, CompilationStatus::Compiled);

                if self.debug {
                    println!("✅ JIT: Successfully compiled function {}", chunk_index);
                }

                JitResult::Compiled(native_function)
            }
            Err(error) => {
                self.stats.compilation_failures += 1;

                // Mark as failed
                self.compilation_status.insert(chunk_index, CompilationStatus::Failed(error.clone()));

                if self.debug {
                    println!("❌ JIT: Failed to compile function {}: {}", chunk_index, error);
                }

                JitResult::Failed(error)
            }
        }
    }

    /// Get a compiled native function
    pub fn get_native_function(&self, chunk_index: usize) -> Option<&NativeFunction> {
        self.native_functions.get(&chunk_index)
    }

    /// Execute a native function if available
    pub fn execute_native(&mut self, chunk_index: usize, input: i64) -> Option<Result<i64, String>> {
        if let Some(native_function) = self.native_functions.get(&chunk_index) {
            self.stats.jit_calls += 1;
            Some(native_function.call(input))
        } else {
            None
        }
    }

    /// Get JIT compilation statistics
    pub fn get_stats(&self) -> &JitStats {
        &self.stats
    }

    /// Get hot spot information
    pub fn get_hotspots(&self) -> Vec<(usize, u32, u64)> {
        self.hotspot_detector.get_hotspots()
    }

    /// Clear all compiled functions and reset statistics
    pub fn reset(&mut self) {
        self.native_functions.clear();
        self.hotspot_detector.reset();
        self.stats = JitStats::default();
    }

    /// Get memory usage of compiled functions
    pub fn get_memory_usage(&self) -> usize {
        self.native_functions.len() * std::mem::size_of::<NativeFunction>()
    }
}

impl Default for JitEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// JIT configuration options
#[derive(Debug, Clone)]
pub struct JitConfig {
    /// Whether JIT is enabled
    pub enabled: bool,
    /// Execution count threshold for JIT compilation
    pub compilation_threshold: u32,
    /// Maximum number of functions to keep compiled
    pub max_compiled_functions: usize,
    /// Whether to enable aggressive optimizations
    pub aggressive_optimization: bool,
    /// Whether to collect detailed statistics
    pub collect_stats: bool,
}

impl Default for JitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            compilation_threshold: 100,
            max_compiled_functions: 1000,
            aggressive_optimization: false,
            collect_stats: true,
        }
    }
}

impl JitConfig {
    /// Create a configuration for development/debugging
    pub fn development() -> Self {
        Self {
            enabled: true,
            compilation_threshold: 10, // Lower threshold for testing
            max_compiled_functions: 100,
            aggressive_optimization: false,
            collect_stats: true,
        }
    }

    /// Create a configuration for production
    pub fn production() -> Self {
        Self {
            enabled: true,
            compilation_threshold: 1000, // Higher threshold for stability
            max_compiled_functions: 10000,
            aggressive_optimization: true,
            collect_stats: false, // Reduce overhead
        }
    }

    /// Create a configuration with JIT disabled
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            compilation_threshold: u32::MAX,
            max_compiled_functions: 0,
            aggressive_optimization: false,
            collect_stats: false,
        }
    }
}
