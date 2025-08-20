//! Main semantic analyzer structure and core functionality

use crate::lang::ast::*;
use crate::lang::types::{Type, TypeEnvironment, TypeError, TypeInference};
use crate::lang::builtins::BuiltinRegistry;

/// Semantic analyzer
pub struct SemanticAnalyzer {
    pub(super) type_env: TypeEnvironment,
    pub(super) builtins: BuiltinRegistry,
    pub(super) inference: TypeInference,
    pub(super) errors: Vec<TypeError>,
    pub(super) current_function_return_type: Option<Type>,
    pub(super) in_loop: bool,
    pub(super) in_switch: bool,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = Self {
            type_env: TypeEnvironment::new(),
            builtins: BuiltinRegistry::new(),
            inference: TypeInference::new(),
            errors: Vec::new(),
            current_function_return_type: None,
            in_loop: false,
            in_switch: false,
        };
        
        // Populate type environment with built-in functions
        analyzer.builtins.populate_type_environment(&mut analyzer.type_env);

        // Also register functions with their full names for backward compatibility
        for name in analyzer.builtins.get_names() {
            if let Some((_, func_type)) = analyzer.builtins.get(name) {
                let _ = analyzer.type_env.define_function(name.clone(), func_type.clone());
            }
        }
        
        analyzer
    }

    /// Get errors from semantic analysis
    pub fn get_errors(&self) -> &Vec<TypeError> {
        &self.errors
    }

    /// Get type environment (for testing)
    pub fn get_type_environment(&self) -> &TypeEnvironment {
        &self.type_env
    }

    /// Analyze a complete program
    pub fn analyze_program(&mut self, program: &Program) -> Result<(), Vec<TypeError>> {
        self.errors.clear();
        
        // First pass: collect all declarations
        for declaration in &program.declarations {
            self.collect_declaration(declaration);
        }
        
        // Second pass: analyze all declarations
        for declaration in &program.declarations {
            self.analyze_declaration(declaration);
        }
        
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    /// Add an error to the error list
    pub(super) fn add_error(&mut self, error: TypeError) {
        self.errors.push(error);
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
