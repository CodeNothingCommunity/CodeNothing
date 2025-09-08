//! CodeNothing Language Frontend
//! 
//! This module implements the complete language frontend including:
//! - Lexical analysis (tokenization)
//! - Syntax analysis (parsing)
//! - Semantic analysis (type checking)
//! - Code generation (bytecode emission)

pub mod lexer;
pub mod parser;
pub mod ast;
pub mod types;
pub mod semantic;
pub mod codegen;
pub mod builtins;
pub mod include_processor;

pub use lexer::*;
pub use parser::*;
pub use ast::*;
pub use types::*;
pub use semantic::*;
pub use codegen::*;
pub use builtins::*;
