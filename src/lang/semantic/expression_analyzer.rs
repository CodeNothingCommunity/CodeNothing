//! Expression analysis

use crate::lang::ast::*;
use crate::lang::types::{Type, TypeError};
use super::analyzer::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// Analyze expression and return its type
    pub(super) fn analyze_expression(&mut self, expression: &Expression) -> Type {
        match expression {
            Expression::Integer(_, _) => Type::Int,
            Expression::Float(_, _) => Type::Float(64), // Default to 64-bit float
            Expression::String(_, _) => Type::String,
            Expression::Boolean(_, _) => Type::Bool,
            Expression::Null(_) => Type::Null,
            
            Expression::Identifier(name, position) => {
                if let Some(var_type) = self.type_env.lookup_variable(name) {
                    var_type.clone()
                } else if let Some(func_type) = self.type_env.lookup_function(name) {
                    func_type.clone()
                } else {
                    self.add_error(TypeError::new(
                        format!("Undefined variable '{}'", name),
                        position.clone(),
                    ));
                    Type::Unknown
                }
            }
            
            Expression::Binary { left, operator, right, position } => {
                let left_type = self.analyze_expression(left);
                let right_type = self.analyze_expression(right);
                self.analyze_binary_operation(&left_type, operator, &right_type, position)
            }
            
            Expression::Unary { operator, operand, position } => {
                let operand_type = self.analyze_expression(operand);
                match operator {
                    UnaryOperator::Not => {
                        if matches!(operand_type, Type::Bool) {
                            Type::Bool
                        } else {
                            self.add_error(TypeError::new(
                                format!("Logical NOT requires boolean operand, got '{}'", operand_type),
                                position.clone(),
                            ));
                            Type::Unknown
                        }
                    }
                    UnaryOperator::Minus => {
                        if matches!(operand_type, Type::Int | Type::Float(_)) {
                            operand_type
                        } else {
                            self.add_error(TypeError::new(
                                format!("Unary minus requires numeric operand, got '{}'", operand_type),
                                position.clone(),
                            ));
                            Type::Unknown
                        }
                    }
                    UnaryOperator::BitNot => {
                        if matches!(operand_type, Type::Int) {
                            Type::Int
                        } else {
                            self.add_error(TypeError::new(
                                format!("Bitwise NOT requires integer operand, got '{}'", operand_type),
                                position.clone(),
                            ));
                            Type::Unknown
                        }
                    }
                }
            }
            
            Expression::Call { function, arguments, position } => {
                self.analyze_call_expression(function, arguments, position)
            }
            
            Expression::Array(elements, _position) => {
                if elements.is_empty() {
                    Type::Array(Box::new(Type::Unknown))
                } else {
                    let first_type = self.analyze_expression(&elements[0]);
                    
                    // Check that all elements have the same type
                    for element in elements.iter().skip(1) {
                        let element_type = self.analyze_expression(element);
                        if !self.inference.can_unify(&first_type, &element_type) {
                            self.add_error(TypeError::new(
                                format!("Array elements must have the same type: expected '{}', got '{}'", 
                                       first_type, element_type),
                                element.position().clone(),
                            ));
                        }
                    }
                    
                    Type::Array(Box::new(first_type))
                }
            }
            
            // Handle additional expression types
            Expression::Object(fields, _position) => {
                // Object literal - for now, return Obj type
                for (_key, value_expr) in fields {
                    self.analyze_expression(value_expr);
                }
                Type::Obj
            }

            Expression::NamespaceAccess { namespace_path, name, position } => {
                // Look up function or variable in the specified namespace
                if let Some(var_type) = self.type_env.lookup_in_nested_namespace(namespace_path, name) {
                    var_type.clone()
                } else {
                    self.add_error(TypeError::new(
                        format!("Undefined identifier '{}' in namespace '{}'", name, namespace_path.join("::")),
                        position.clone(),
                    ));
                    Type::Unknown
                }
            }

            Expression::ArrayAccess { array, index, position } => {
                let array_type = self.analyze_expression(array);
                let index_type = self.analyze_expression(index);

                if !matches!(index_type, Type::Int) {
                    self.add_error(TypeError::new(
                        format!("Array index must be integer, got '{}'", index_type),
                        position.clone(),
                    ));
                }

                match array_type {
                    Type::Array(element_type) => *element_type,
                    Type::String => Type::String, // String indexing returns a character (string)
                    _ => {
                        self.add_error(TypeError::new(
                            format!("Cannot index into type '{}'", array_type),
                            position.clone(),
                        ));
                        Type::Unknown
                    }
                }
            }

            Expression::FieldAccess { object, field: _, position } => {
                let object_type = self.analyze_expression(object);

                match object_type {
                    Type::Obj => {
                        // For object types, we can't determine the field type statically
                        // Return Unknown for now - in a full implementation, we'd need
                        // structural typing or object schemas
                        Type::Unknown
                    }
                    _ => {
                        self.add_error(TypeError::new(
                            format!("Cannot access field of type '{}'", object_type),
                            position.clone(),
                        ));
                        Type::Unknown
                    }
                }
            }

            Expression::Range { start, end, position: _ } => {
                let start_type = self.analyze_expression(start);
                let end_type = self.analyze_expression(end);

                if !matches!(start_type, Type::Int) || !matches!(end_type, Type::Int) {
                    self.add_error(TypeError::new(
                        "Range bounds must be integers".to_string(),
                        start.position().clone(),
                    ));
                }

                // Range type - for now, return Array of Int
                Type::Array(Box::new(Type::Int))
            }
        }
    }

    /// Analyze function call expression
    pub(super) fn analyze_call_expression(
        &mut self,
        function: &Expression,
        arguments: &[Expression],
        position: &Position,
    ) -> Type {
        match function {
            Expression::Identifier(name, _) => {
                if let Some(function_type) = self.type_env.lookup_function(name).cloned() {
                    match function_type {
                        Type::Function { parameters, return_type } => {
                            if arguments.len() != parameters.len() {
                                self.add_error(TypeError::new(
                                    format!("Function '{}' expects {} arguments, got {}",
                                           name, parameters.len(), arguments.len()),
                                    position.clone(),
                                ));
                                return Type::Unknown;
                            }

                            for (i, (arg, expected_type)) in arguments.iter().zip(parameters.iter()).enumerate() {
                                let arg_type = self.analyze_expression(arg);
                                if !self.inference.can_unify(expected_type, &arg_type) {
                                    self.add_error(TypeError::new(
                                        format!("Argument {} to function '{}': expected '{}', got '{}'",
                                               i + 1, name, expected_type, arg_type),
                                        arg.position().clone(),
                                    ));
                                }
                            }

                            *return_type
                        }
                        _ => {
                            self.add_error(TypeError::new(
                                format!("'{}' is not a function", name),
                                position.clone(),
                            ));
                            Type::Unknown
                        }
                    }
                } else {
                    self.add_error(TypeError::new(
                        format!("Undefined function '{}'", name),
                        position.clone(),
                    ));
                    Type::Unknown
                }
            }
            Expression::NamespaceAccess { namespace_path, name, .. } => {
                // Look up function in the specified namespace
                if let Some(function_type) = self.type_env.lookup_in_nested_namespace(namespace_path, name).cloned() {
                    match function_type {
                        Type::Function { parameters, return_type } => {
                            if arguments.len() != parameters.len() {
                                self.add_error(TypeError::new(
                                    format!("Function '{}::{}' expects {} arguments, got {}",
                                           namespace_path.join("::"), name, parameters.len(), arguments.len()),
                                    position.clone(),
                                ));
                                return Type::Unknown;
                            }

                            for (i, (arg, expected_type)) in arguments.iter().zip(parameters.iter()).enumerate() {
                                let arg_type = self.analyze_expression(arg);
                                if !self.inference.can_unify(expected_type, &arg_type) {
                                    self.add_error(TypeError::new(
                                        format!("Argument {} to function '{}::{}': expected '{}', got '{}'",
                                               i + 1, namespace_path.join("::"), name, expected_type, arg_type),
                                        arg.position().clone(),
                                    ));
                                }
                            }

                            *return_type
                        }
                        _ => {
                            self.add_error(TypeError::new(
                                format!("'{}::{}' is not a function", namespace_path.join("::"), name),
                                position.clone(),
                            ));
                            Type::Unknown
                        }
                    }
                } else {
                    self.add_error(TypeError::new(
                        format!("Undefined function '{}::{}' in namespace", namespace_path.join("::"), name),
                        position.clone(),
                    ));
                    Type::Unknown
                }
            }
            _ => {
                // For now, we only support direct function calls and namespace access
                self.add_error(TypeError::new(
                    "Complex function expressions not supported".to_string(),
                    position.clone(),
                ));
                Type::Unknown
            }
        }
    }
}
