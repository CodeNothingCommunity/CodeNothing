//! Statement analysis

use crate::lang::ast::*;
use crate::lang::types::{Type, TypeError};
use super::analyzer::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// Analyze statement
    pub(super) fn analyze_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::VarDeclaration { scope: _, name, type_annotation, initializer, position } => {
                let declared_type = type_annotation.as_ref()
                    .map(|t| self.resolve_type_annotation(t));
                
                let inferred_type = if let Some(init) = initializer {
                    Some(self.analyze_expression(init))
                } else {
                    None
                };
                
                let final_type = match (declared_type, inferred_type) {
                    (Some(declared), Some(inferred)) => {
                        // Check if inferred type can be assigned to declared type (for polymorphism)
                        if !self.inference.can_unify_with_env(&inferred, &declared, Some(&self.type_env)) {
                            self.add_error(TypeError::new(
                                format!("Type mismatch: declared type '{}' but got '{}'",
                                       declared, inferred),
                                position.clone(),
                            ));
                            declared
                        } else {
                            declared
                        }
                    }
                    (Some(declared), None) => declared,
                    (None, Some(inferred)) => inferred,
                    (None, None) => {
                        self.add_error(TypeError::new(
                            "Variable declaration must have either type annotation or initializer".to_string(),
                            position.clone(),
                        ));
                        Type::Unknown
                    }
                };
                
                if let Err(e) = self.type_env.define_variable(name.clone(), final_type) {
                    self.add_error(TypeError::new(
                        format!("Variable '{}' already defined: {}", name, e),
                        position.clone(),
                    ));
                }
            }
            
            // Assignment is not a separate statement in the current AST
            // It's handled as an expression with Assign operator
            
            Statement::Expression(expr) => {
                self.analyze_expression(expr);
            }
            
            Statement::Return { value, position } => {
                let return_type = if let Some(val) = value {
                    // Set expected type for the return expression
                    let old_expected = self.expected_type.clone();
                    self.expected_type = self.current_function_return_type.clone();
                    let result = self.analyze_expression(val);
                    self.expected_type = old_expected;
                    result
                } else {
                    Type::Void
                };

                if let Some(expected_return_type) = &self.current_function_return_type {
                    if !self.inference.can_unify(expected_return_type, &return_type) {
                        self.add_error(TypeError::new(
                            format!("Return type mismatch: expected '{}' but got '{}' (expected: {:?}, actual: {:?})",
                                   expected_return_type, return_type, expected_return_type, return_type),
                            position.clone(),
                        ));
                    }
                } else {
                    self.add_error(TypeError::new(
                        "Return statement outside function".to_string(),
                        position.clone(),
                    ));
                }
            }
            
            Statement::Block { statements, position: _ } => {
                self.type_env.enter_scope();
                for stmt in statements {
                    self.analyze_statement(stmt);
                }
                self.type_env.exit_scope();
            }
            
            Statement::If { condition, then_branch, else_branch, position } => {
                let condition_type = self.analyze_expression(condition);
                if !matches!(condition_type, Type::Bool) {
                    self.add_error(TypeError::new(
                        format!("If condition must be boolean, got '{}'", condition_type),
                        position.clone(),
                    ));
                }
                
                self.analyze_statement(then_branch);
                if let Some(else_stmt) = else_branch {
                    self.analyze_statement(else_stmt);
                }
            }
            
            Statement::While { condition, body, position } => {
                let condition_type = self.analyze_expression(condition);
                if !matches!(condition_type, Type::Bool) {
                    self.add_error(TypeError::new(
                        format!("While condition must be boolean, got '{}'", condition_type),
                        position.clone(),
                    ));
                }
                
                let old_in_loop = self.in_loop;
                self.in_loop = true;
                self.analyze_statement(body);
                self.in_loop = old_in_loop;
            }
            
            Statement::For { initializer, condition, increment, body, position } => {
                self.type_env.enter_scope();

                if let Some(init_stmt) = initializer {
                    self.analyze_statement(init_stmt);
                }

                if let Some(cond_expr) = condition {
                    let condition_type = self.analyze_expression(cond_expr);
                    if !matches!(condition_type, Type::Bool) {
                        self.add_error(TypeError::new(
                            format!("For condition must be boolean, got '{}'", condition_type),
                            position.clone(),
                        ));
                    }
                }

                if let Some(incr_expr) = increment {
                    self.analyze_expression(incr_expr);
                }

                let old_in_loop = self.in_loop;
                self.in_loop = true;
                self.analyze_statement(body);
                self.in_loop = old_in_loop;

                self.type_env.exit_scope();
            }
            
            Statement::Break(position) => {
                if !self.in_loop {
                    self.add_error(TypeError::new(
                        "Break statement outside loop".to_string(),
                        position.clone(),
                    ));
                }
            }
            
            Statement::Continue(position) => {
                if !self.in_loop {
                    self.add_error(TypeError::new(
                        "Continue statement outside loop".to_string(),
                        position.clone(),
                    ));
                }
            }
            
            Statement::Switch { expression, cases, default_case, position } => {
                let switch_type = self.analyze_expression(expression);
                
                let old_in_switch = self.in_switch;
                self.in_switch = true;
                
                for case in cases {
                    let case_type = self.analyze_expression(&case.value);
                    if !self.inference.can_unify(&switch_type, &case_type) {
                        self.add_error(TypeError::new(
                            format!("Case value type '{}' doesn't match switch expression type '{}'",
                                   case_type, switch_type),
                            position.clone(),
                        ));
                    }

                    self.analyze_statement(&case.body);
                }
                
                if let Some(default) = default_case {
                    self.analyze_statement(default);
                }
                
                self.in_switch = old_in_switch;
            }
            
            Statement::Try { try_block, catch_clauses, finally_block, position: _ } => {
                self.analyze_statement(try_block);
                
                for catch in catch_clauses {
                    self.type_env.enter_scope();
                    
                    // Define exception variable if specified
                    if let Some(var_name) = &catch.exception_variable {
                        if let Err(e) = self.type_env.define_variable(var_name.clone(), Type::String) {
                            self.add_error(TypeError::new(
                                format!("Exception variable '{}' already defined: {}", var_name, e),
                                catch.position.clone(),
                            ));
                        }
                    }
                    
                    self.analyze_statement(&catch.body);
                    self.type_env.exit_scope();
                }
                
                if let Some(finally) = finally_block {
                    self.analyze_statement(finally);
                }
            }
            
            Statement::Throw { value, position: _ } => {
                // Exception expression should be a string or exception type
                let expr_type = self.analyze_expression(value);
                if !matches!(expr_type, Type::String) {
                    // For now, we only support string exceptions
                    // In the future, we might support custom exception types
                }
            }

            Statement::LibImport { library_name, import_type, position } => {
                if self.debug_mode {
                    println!("🔧 Semantic: Processing library import: {} ({:?})", library_name, import_type);
                }

                // Try to load the library and get its namespaces
                match crate::library_loader::get_library_namespaces(library_name) {
                    Ok(namespaces) => {
                        if self.debug_mode {
                            println!("🔧 Semantic: Library '{}' provides namespaces: {:?}", library_name, namespaces);
                        }

                        // Register each namespace from the library
                        for namespace in namespaces {
                            self.type_env.define_namespace(namespace.clone());
                            if self.debug_mode {
                                println!("🔧 Semantic: Registered namespace '{}' from library '{}'", namespace, library_name);
                            }

                            // Register all functions from this library in this namespace
                            if let Ok(functions) = crate::library_loader::get_library_functions(library_name) {
                                for function_name in functions {
                                    // Extract the function name without namespace prefix
                                    if let Some((ns, func)) = function_name.split_once("::") {
                                        if ns == namespace {
                                            // Register the function in the namespace
                                            // Create a special marker type for library functions
                                            // We'll use a custom type to indicate this is a library function
                                            let function_type = Type::LibraryFunction {
                                                return_type: Box::new(Type::String),
                                            };
                                            if let Err(e) = self.type_env.define_in_namespace(&namespace, func.to_string(), function_type) {
                                                if self.debug_mode {
                                                    println!("🔧 Semantic: Failed to register function '{}' in namespace '{}': {}", func, namespace, e);
                                                }
                                            } else if self.debug_mode {
                                                println!("🔧 Semantic: Registered function '{}' in namespace '{}'", func, namespace);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        self.add_error(TypeError::new(
                            format!("Failed to load library '{}': {}", library_name, e),
                            position.clone(),
                        ));
                    }
                }
            }

            Statement::ForRange { variable, range, body, position: _ } => {
                self.type_env.enter_scope();

                // Analyze range expression
                let range_type = self.analyze_expression(range);
                let element_type = match range_type {
                    Type::Array(ref element_type) => (**element_type).clone(),
                    _ => {
                        self.add_error(TypeError::new(
                            format!("For-range requires array or range type, got '{}'", range_type),
                            range.position().clone(),
                        ));
                        Type::Unknown
                    }
                };

                // Define loop variable in scope
                if let Err(e) = self.type_env.define_variable(variable.clone(), element_type) {
                    self.add_error(TypeError::new(
                        format!("Loop variable '{}' already defined: {}", variable, e),
                        range.position().clone(),
                    ));
                }

                let old_in_loop = self.in_loop;
                self.in_loop = true;
                self.analyze_statement(body);
                self.in_loop = old_in_loop;

                self.type_env.exit_scope();
            }

            Statement::ForIn { variable, iterable, body, position: _ } => {
                self.type_env.enter_scope();

                // Analyze iterable expression
                let iterable_type = self.analyze_expression(iterable);
                let element_type = match iterable_type {
                    Type::Array(ref element_type) => (**element_type).clone(),
                    _ => {
                        self.add_error(TypeError::new(
                            format!("For-in requires array type, got '{}'", iterable_type),
                            iterable.position().clone(),
                        ));
                        Type::Unknown
                    }
                };

                // Define loop variable in scope
                if let Err(e) = self.type_env.define_variable(variable.clone(), element_type) {
                    self.add_error(TypeError::new(
                        format!("Loop variable '{}' already defined: {}", variable, e),
                        iterable.position().clone(),
                    ));
                }

                let old_in_loop = self.in_loop;
                self.in_loop = true;
                self.analyze_statement(body);
                self.in_loop = old_in_loop;

                self.type_env.exit_scope();
            }

            Statement::DoWhile { body, condition, position: _ } => {
                let old_in_loop = self.in_loop;
                self.in_loop = true;

                // Analyze body first (since it executes at least once)
                self.analyze_statement(body);

                // Analyze condition
                let condition_type = self.analyze_expression(condition);
                if !matches!(condition_type, Type::Bool) {
                    self.add_error(TypeError::new(
                        format!("Do-while condition must be boolean, got '{}'", condition_type),
                        condition.position().clone(),
                    ));
                }

                self.in_loop = old_in_loop;
            }
        }
    }
}
