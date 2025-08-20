//! JIT compiler module for CodeNothing
//!
//! This module contains the JIT compilation system that translates bytecode
//! into native machine code for improved performance.
//!
//! The compiler has been split into multiple modules for better organization:
//! - `config.rs` - Configuration and statistics
//! - `analysis.rs` - Code analysis and suitability checking
//! - `cranelift_backend.rs` - Cranelift backend implementation
//! - `linear_compiler.rs` - Linear instruction compilation
//! - `recursive_compiler.rs` - Recursive function compilation
//! - `jit_compiler.rs` - Main JIT compiler implementation

// Module declarations
pub mod config;
pub mod analysis;
pub mod cranelift_backend;
pub mod linear_compiler;
pub mod recursive_compiler;
pub mod jit_compiler;

// Re-export main types and functions
pub use config::{JitConfig, CompilationStats};
pub use analysis::CodeAnalyzer;
pub use cranelift_backend::CraneliftBackend;
pub use linear_compiler::LinearCompiler;
pub use recursive_compiler::RecursiveCompiler;
pub use jit_compiler::JitCompiler;
