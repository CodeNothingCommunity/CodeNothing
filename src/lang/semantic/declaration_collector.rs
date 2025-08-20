//! Declaration collection for first pass analysis

use crate::lang::ast::*;
use crate::lang::types::{Type, TypeError};
use super::analyzer::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// Collect declaration signatures (first pass)
    pub(super) fn collect_declaration(&mut self, declaration: &Declaration) {
        match declaration {
            Declaration::Function(func) => {
                let param_types: Vec<Type> = func.parameters.iter()
                    .map(|p| Type::from(p.type_annotation.clone()))
                    .collect();
                
                let return_type = func.return_type.as_ref()
                    .map(|t| Type::from(t.clone()))
                    .unwrap_or(Type::Void);

                let function_type = Type::Function {
                    parameters: param_types,
                    return_type: Box::new(return_type),
                };

                if let Err(e) = self.type_env.define_function(func.name.clone(), function_type) {
                    self.add_error(TypeError::new(
                        format!("Function '{}' already defined: {}", func.name, e),
                        func.position.clone(),
                    ));
                }
            }
            Declaration::Namespace(ns) => {
                self.collect_namespace_declaration(ns, &[]);
            }
            Declaration::Memo(memo) => {
                self.collect_memo_declaration(memo);
            }
            Declaration::Using(_) => {
                // Using declarations are handled in code generation phase
            }
            Declaration::Variable(stmt) => {
                // Handle global variable declarations in collection phase
                if let Statement::VarDeclaration { name, type_annotation, .. } = stmt {
                    let var_type = type_annotation.as_ref()
                        .map(|t| Type::from(t.clone()))
                        .unwrap_or(Type::Unknown);

                    if let Err(e) = self.type_env.define_variable(name.clone(), var_type) {
                        self.add_error(TypeError::new(
                            format!("Global variable '{}' already defined: {}", name, e),
                            stmt.position().clone(),
                        ));
                    }
                }
            }
        }
    }

    /// Collect namespace declaration recursively
    fn collect_namespace_declaration(&mut self, ns: &NamespaceDeclaration, parent_path: &[String]) {
        let mut current_path = parent_path.to_vec();
        current_path.push(ns.name.clone());

        // Define the namespace
        self.type_env.define_nested_namespace(&current_path);

        for item in &ns.items {
            match item {
                NamespaceItem::Function(func) => {
                    let param_types: Vec<Type> = func.parameters.iter()
                        .map(|p| Type::from(p.type_annotation.clone()))
                        .collect();
                    
                    let return_type = func.return_type.as_ref()
                        .map(|t| Type::from(t.clone()))
                        .unwrap_or(Type::Void);

                    let function_type = Type::Function {
                        parameters: param_types,
                        return_type: Box::new(return_type),
                    };

                    // Define function in namespace
                    if let Err(e) = self.type_env.add_to_nested_namespace(&current_path, func.name.clone(), function_type) {
                        self.add_error(TypeError::new(
                            format!("Function '{}' already defined in namespace '{}': {}", 
                                   func.name, current_path.join("::"), e),
                            func.position.clone(),
                        ));
                    }
                }
                NamespaceItem::Namespace(nested_ns) => {
                    self.collect_namespace_declaration(nested_ns, &current_path);
                }
                NamespaceItem::Variable(stmt) => {
                    // Handle namespace variable declarations in collection phase
                    if let Statement::VarDeclaration { name, type_annotation, .. } = stmt {
                        let var_type = type_annotation.as_ref()
                            .map(|t| Type::from(t.clone()))
                            .unwrap_or(Type::Unknown);

                        if let Err(e) = self.type_env.define_variable_in_namespace(
                            &current_path.join("::"),
                            name.clone(),
                            var_type
                        ) {
                            self.add_error(TypeError::new(
                                format!("Namespace variable '{}' already defined: {}", name, e),
                                stmt.position().clone(),
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Collect memo declaration
    fn collect_memo_declaration(&mut self, memo: &MemoDeclaration) {
        match &memo.content {
            MemoContent::Function(func) => {
                // Collect function signature like in collect_declaration
                let param_types: Vec<Type> = func.parameters.iter()
                    .map(|p| Type::from(p.type_annotation.clone()))
                    .collect();

                let return_type = func.return_type.as_ref()
                    .map(|t| Type::from(t.clone()))
                    .unwrap_or(Type::Void);

                let function_type = Type::Function {
                    parameters: param_types,
                    return_type: Box::new(return_type),
                };

                if let Err(e) = self.type_env.define_function(func.name.clone(), function_type) {
                    self.add_error(TypeError::new(
                        format!("Function '{}' already defined: {}", func.name, e),
                        func.position.clone(),
                    ));
                }
            }
            MemoContent::Namespace(ns) => {
                self.collect_namespace_declaration(ns, &[]);
            }
            MemoContent::Block(declarations) => {
                for decl in declarations {
                    self.collect_declaration(decl);
                }
            }
        }
    }
}
