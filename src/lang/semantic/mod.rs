//! Semantic analyzer for CodeNothing language
//! 
//! Performs type checking, scope analysis, and semantic validation
//!
//! The semantic analyzer has been split into multiple modules for better organization:
//! - `analyzer.rs` - Main semantic analyzer structure and core functionality
//! - `declaration_collector.rs` - Declaration collection for first pass analysis
//! - `declaration_analyzer.rs` - Declaration analysis for second pass
//! - `statement_analyzer.rs` - Statement analysis
//! - `expression_analyzer.rs` - Expression analysis
//! - `binary_operation_analyzer.rs` - Binary operation analysis
//! - `tests.rs` - Test cases

// Module declarations
pub mod analyzer;
pub mod declaration_collector;
pub mod declaration_analyzer;
pub mod statement_analyzer;
pub mod expression_analyzer;
pub mod binary_operation_analyzer;
pub mod tests;

// Re-export main types and functions
pub use analyzer::SemanticAnalyzer;
