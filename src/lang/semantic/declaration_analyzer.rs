//! Declaration analysis for second pass

use crate::lang::ast::*;
use crate::lang::types::{Type, TypeError};
use super::analyzer::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// Analyze declaration (second pass)
    pub(super) fn analyze_declaration(&mut self, declaration: &Declaration) {
        match declaration {
            Declaration::Function(func) => {
                self.analyze_function(func);
            }
            Declaration::Namespace(ns) => {
                self.analyze_namespace_items(ns, &[]);
            }
            Declaration::Memo(memo) => {
                self.analyze_memo_declaration(memo);
            }
            Declaration::Using(using) => {
                // Import namespace functions into current scope
                self.analyze_using_declaration(using);
            }
            Declaration::Variable(stmt) => {
                // Handle global variable declarations
                self.analyze_statement(stmt);
            }
        }
    }
    
    /// Analyze namespace items recursively
    fn analyze_namespace_items(&mut self, ns: &NamespaceDeclaration, parent_path: &[String]) {
        let mut current_path = parent_path.to_vec();
        current_path.push(ns.name.clone());

        for item in &ns.items {
            match item {
                NamespaceItem::Function(func) => {
                    self.analyze_nested_namespace_function(&current_path, func);
                }
                NamespaceItem::Namespace(nested_ns) => {
                    self.analyze_namespace_items(nested_ns, &current_path);
                }
                NamespaceItem::Variable(stmt) => {
                    // Handle namespace variable declarations
                    self.analyze_statement(stmt);
                }
            }
        }
    }

    /// Analyze namespace function declaration
    fn analyze_namespace_function(&mut self, namespace: &str, function: &FunctionDeclaration) {
        // Enter function scope
        self.type_env.enter_scope();

        // Import all functions from the current namespace into the function scope
        // This allows functions in the same namespace to call each other
        let namespace_path = vec![namespace.to_string()];
        let functions_to_import: Vec<(String, Type)> = if let Some(namespace_functions) = self.type_env.get_namespace_functions(&namespace_path) {
            namespace_functions.iter().map(|(name, func_type)| (name.clone(), func_type.clone())).collect()
        } else {
            Vec::new()
        };

        for (name, func_type) in functions_to_import {
            if let Err(e) = self.type_env.define_function(name.clone(), func_type) {
                self.add_error(TypeError::new(
                    format!("Failed to import function '{}' from namespace '{}': {}", name, namespace, e),
                    function.position.clone(),
                ));
            }
        }

        // Set current function return type
        self.current_function_return_type = function.return_type.as_ref()
            .map(|t| Type::from(t.clone()));

        // Define parameters
        for param in &function.parameters {
            let param_type = Type::from(param.type_annotation.clone());
            if let Err(e) = self.type_env.define_variable(param.name.clone(), param_type) {
                self.add_error(TypeError::new(
                    format!("Parameter '{}' already defined: {}", param.name, e),
                    param.position.clone(),
                ));
            }
        }

        // Analyze function body
        self.analyze_statement(&function.body);

        self.type_env.exit_scope();
        self.current_function_return_type = None;
    }

    /// Analyze nested namespace function declaration
    fn analyze_nested_namespace_function(&mut self, namespace_path: &[String], function: &FunctionDeclaration) {
        // Enter function scope
        self.type_env.enter_scope();

        // Import all functions from the current namespace and parent namespaces
        // This allows functions to call each other within the same namespace and access parent namespace functions
        for i in 1..=namespace_path.len() {
            let current_namespace_path = &namespace_path[0..i];
            let namespace_name = current_namespace_path.join("::");

            let current_path: Vec<String> = current_namespace_path.iter().map(|s| s.clone()).collect();
            let functions_to_import: Vec<(String, Type)> = if let Some(namespace_functions) = self.type_env.get_namespace_functions(&current_path) {
                namespace_functions.iter()
                    .filter(|(name, _)| self.type_env.lookup_function(name).is_none())
                    .map(|(name, func_type)| (name.clone(), func_type.clone()))
                    .collect()
            } else {
                Vec::new()
            };

            for (name, func_type) in functions_to_import {
                if let Err(e) = self.type_env.define_function(name.clone(), func_type) {
                    self.add_error(TypeError::new(
                        format!("Failed to import function '{}' from namespace '{}': {}", name, namespace_name, e),
                        function.position.clone(),
                    ));
                }
            }
        }

        // Set current function return type
        self.current_function_return_type = function.return_type.as_ref()
            .map(|t| Type::from(t.clone()));

        // Define parameters
        for param in &function.parameters {
            let param_type = Type::from(param.type_annotation.clone());
            if let Err(e) = self.type_env.define_variable(param.name.clone(), param_type) {
                self.add_error(TypeError::new(
                    format!("Parameter '{}' already defined: {}", param.name, e),
                    param.position.clone(),
                ));
            }
        }

        // Analyze function body
        self.analyze_statement(&function.body);

        self.type_env.exit_scope();
        self.current_function_return_type = None;
    }

    /// Analyze function declaration
    fn analyze_function(&mut self, function: &FunctionDeclaration) {
        // Enter function scope
        self.type_env.enter_scope();
        
        // Set current function return type
        self.current_function_return_type = function.return_type.as_ref()
            .map(|t| Type::from(t.clone()));
        
        // Define parameters
        for param in &function.parameters {
            let param_type = Type::from(param.type_annotation.clone());
            if let Err(e) = self.type_env.define_variable(param.name.clone(), param_type) {
                self.add_error(TypeError::new(
                    format!("Parameter '{}' already defined: {}", param.name, e),
                    param.position.clone(),
                ));
            }
        }
        
        // Analyze function body
        self.analyze_statement(&function.body);
        
        self.type_env.exit_scope();
        self.current_function_return_type = None;
    }

    /// Analyze memo declaration
    pub(super) fn analyze_memo_declaration(&mut self, memo: &MemoDeclaration) {
        // Validate memo parameters
        if let Some(params) = &memo.params {
            if let Some(max_size) = params.max_size {
                if max_size <= 0 {
                    self.add_error(TypeError::new(
                        "Memo max_size must be positive".to_string(),
                        memo.position.clone(),
                    ));
                }
            }
            
            if let Some(ttl) = params.ttl {
                if ttl <= 0 {
                    self.add_error(TypeError::new(
                        "Memo TTL must be positive".to_string(),
                        memo.position.clone(),
                    ));
                }
            }
        }

        // For now, memo declarations are handled as content blocks
        // TODO: Extract and analyze function from memo content if needed
        let _ = memo; // Suppress unused warning
    }

    /// Analyze using declaration
    fn analyze_using_declaration(&mut self, using: &UsingDeclaration) {
        // Import all functions from the specified namespace into current scope
        let functions_to_import: Vec<(String, Type)> = if let Some(namespace_info) = self.type_env.get_namespaces().get(&using.namespace) {
            namespace_info.functions.iter()
                .map(|(name, func_type)| (name.clone(), func_type.clone()))
                .collect()
        } else {
            self.add_error(TypeError::new(
                format!("Undefined namespace '{}'", using.namespace),
                using.position.clone(),
            ));
            return;
        };

        // Now import the functions
        for (func_name, func_type) in functions_to_import {
            let _ = self.type_env.define_function(func_name, func_type);
        }
    }
}
