//! Binary operation analysis

use crate::lang::ast::*;
use crate::lang::types::{Type, TypeError};
use super::analyzer::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// Analyze binary operation
    pub(super) fn analyze_binary_operation(
        &mut self,
        left_type: &Type,
        operator: &BinaryOperator,
        right_type: &Type,
        position: &Position,
    ) -> Type {
        match operator {
            // Arithmetic operations
            BinaryOperator::Add | BinaryOperator::Subtract | 
            BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Modulo => {
                self.analyze_arithmetic_operation(left_type, operator, right_type, position)
            }
            
            // Comparison operations
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                // Equality can be checked between any types
                Type::Bool
            }
            
            BinaryOperator::Less | BinaryOperator::LessEqual | 
            BinaryOperator::Greater | BinaryOperator::GreaterEqual => {
                self.analyze_comparison_operation(left_type, operator, right_type, position)
            }
            
            // Logical operations
            BinaryOperator::And | BinaryOperator::Or => {
                self.analyze_logical_operation(left_type, operator, right_type, position)
            }

            // Bitwise operations
            BinaryOperator::BitAnd | BinaryOperator::BitOr | BinaryOperator::BitXor |
            BinaryOperator::LeftShift | BinaryOperator::RightShift => {
                self.analyze_bitwise_operation(left_type, operator, right_type, position)
            }

            // Assignment operations
            BinaryOperator::Assign => {
                // Assignment should return the assigned value's type
                right_type.clone()
            }

            // Pipe operation (function composition)
            BinaryOperator::PipeForward => {
                self.analyze_pipe_operation(left_type, right_type, position)
            }

            // Compound assignment operations
            BinaryOperator::AddAssign | BinaryOperator::SubtractAssign |
            BinaryOperator::MultiplyAssign | BinaryOperator::DivideAssign |
            BinaryOperator::ModuloAssign => {
                // For compound assignment, we need to check if the operation is valid
                // and return the left operand's type
                let operation_type = match operator {
                    BinaryOperator::AddAssign => BinaryOperator::Add,
                    BinaryOperator::SubtractAssign => BinaryOperator::Subtract,
                    BinaryOperator::MultiplyAssign => BinaryOperator::Multiply,
                    BinaryOperator::DivideAssign => BinaryOperator::Divide,
                    BinaryOperator::ModuloAssign => BinaryOperator::Modulo,
                    _ => unreachable!(),
                };

                // Check if the underlying operation is valid
                let _result_type = self.analyze_arithmetic_operation(left_type, &operation_type, right_type, position);

                // Return the left operand's type (the variable being assigned to)
                left_type.clone()
            }
        }
    }

    /// Analyze arithmetic operations
    fn analyze_arithmetic_operation(
        &mut self,
        left_type: &Type,
        operator: &BinaryOperator,
        right_type: &Type,
        position: &Position,
    ) -> Type {
        match (left_type, right_type) {
            // Integer operations
            (Type::Int, Type::Int) => Type::Int,
            
            // Float operations
            (Type::Float(size1), Type::Float(size2)) => {
                Type::Float((*size1).max(*size2))
            }
            
            // Mixed int/float operations
            (Type::Int, Type::Float(size)) | (Type::Float(size), Type::Int) => {
                Type::Float(*size)
            }
            
            // String concatenation (only for addition)
            (Type::String, Type::String) if matches!(operator, BinaryOperator::Add) => {
                Type::String
            }
            
            // Numeric type promotion
            (Type::Num(size1), Type::Num(size2)) => {
                Type::Num((*size1).max(*size2))
            }
            
            (Type::Int, Type::Num(size)) | (Type::Num(size), Type::Int) => {
                Type::Num(*size)
            }
            
            (Type::Float(fsize), Type::Num(nsize)) | (Type::Num(nsize), Type::Float(fsize)) => {
                Type::Float((*fsize).max(*nsize))
            }
            
            _ => {
                self.add_error(TypeError::new(
                    format!("Cannot apply {:?} to '{}' and '{}'", operator, left_type, right_type),
                    position.clone(),
                ));
                Type::Unknown
            }
        }
    }

    /// Analyze comparison operations
    fn analyze_comparison_operation(
        &mut self,
        left_type: &Type,
        operator: &BinaryOperator,
        right_type: &Type,
        position: &Position,
    ) -> Type {
        match (left_type, right_type) {
            // Numeric comparisons
            (Type::Int, Type::Int) |
            (Type::Float(_), Type::Float(_)) |
            (Type::Int, Type::Float(_)) |
            (Type::Float(_), Type::Int) |
            (Type::Num(_), Type::Num(_)) |
            (Type::Int, Type::Num(_)) |
            (Type::Num(_), Type::Int) |
            (Type::Float(_), Type::Num(_)) |
            (Type::Num(_), Type::Float(_)) => Type::Bool,
            
            // String comparisons
            (Type::String, Type::String) => Type::Bool,
            
            _ => {
                self.add_error(TypeError::new(
                    format!("Cannot compare '{}' and '{}'", left_type, right_type),
                    position.clone(),
                ));
                Type::Unknown
            }
        }
    }

    /// Analyze logical operations
    fn analyze_logical_operation(
        &mut self,
        left_type: &Type,
        operator: &BinaryOperator,
        right_type: &Type,
        position: &Position,
    ) -> Type {
        match (left_type, right_type) {
            (Type::Bool, Type::Bool) => Type::Bool,
            _ => {
                self.add_error(TypeError::new(
                    format!("Logical {:?} requires boolean operands, got '{}' and '{}'",
                           operator, left_type, right_type),
                    position.clone(),
                ));
                Type::Unknown
            }
        }
    }

    /// Analyze bitwise operations
    fn analyze_bitwise_operation(
        &mut self,
        left_type: &Type,
        operator: &BinaryOperator,
        right_type: &Type,
        position: &Position,
    ) -> Type {
        match (left_type, right_type) {
            // Bitwise operations require integer operands
            (Type::Int, Type::Int) => Type::Int,
            _ => {
                self.add_error(TypeError::new(
                    format!("Bitwise {:?} requires integer operands, got '{}' and '{}'",
                           operator, left_type, right_type),
                    position.clone(),
                ));
                Type::Unknown
            }
        }
    }

    /// Analyze pipe operation (left |> right)
    /// The pipe operator passes the left value as the first argument to the right function
    fn analyze_pipe_operation(
        &mut self,
        left_type: &Type,
        right_type: &Type,
        position: &Position,
    ) -> Type {
        match right_type {
            // Right side should be a function
            Type::Function { parameters, return_type, .. } => {
                // Check if the function can accept the left value as its first parameter
                if parameters.is_empty() {
                    self.add_error(TypeError::new(
                        "Cannot pipe value to function with no parameters".to_string(),
                        position.clone(),
                    ));
                    return Type::Unknown;
                }

                // Check if the left type matches the first parameter type
                let first_param_type = &parameters[0];
                if !self.types_compatible(left_type, first_param_type) {
                    self.add_error(TypeError::new(
                        format!(
                            "Cannot pipe '{}' to function expecting '{}' as first parameter",
                            left_type, first_param_type
                        ),
                        position.clone(),
                    ));
                    return Type::Unknown;
                }

                // Return the function's return type
                *return_type.clone()
            }

            // Right side could also be an arrow function
            Type::ArrowFunction { parameters, return_type, .. } => {
                // Check if the arrow function can accept the left value as its first parameter
                if parameters.is_empty() {
                    self.add_error(TypeError::new(
                        "Cannot pipe value to arrow function with no parameters".to_string(),
                        position.clone(),
                    ));
                    return Type::Unknown;
                }

                // Check if the left type matches the first parameter type
                let first_param_type = &parameters[0];
                if !self.types_compatible(left_type, first_param_type) {
                    self.add_error(TypeError::new(
                        format!(
                            "Cannot pipe '{}' to arrow function expecting '{}' as first parameter",
                            left_type, first_param_type
                        ),
                        position.clone(),
                    ));
                    return Type::Unknown;
                }

                // Return the arrow function's return type
                *return_type.clone()
            }

            _ => {
                self.add_error(TypeError::new(
                    format!("Right side of pipe operator must be a function, found '{}'", right_type),
                    position.clone(),
                ));
                Type::Unknown
            }
        }
    }

    /// Check if two types are compatible for pipe operation
    fn types_compatible(&self, left: &Type, right: &Type) -> bool {
        match (left, right) {
            // Exact match
            (a, b) if a == b => true,

            // Numeric compatibility
            (Type::Int, Type::Float(_)) | (Type::Float(_), Type::Int) => true,
            (Type::Int, Type::Num(_)) | (Type::Num(_), Type::Int) => true,
            (Type::Float(_), Type::Num(_)) | (Type::Num(_), Type::Float(_)) => true,

            // Any type can be passed to Any
            (_, Type::Any) => true,

            // Unknown types are compatible (error already reported elsewhere)
            (Type::Unknown, _) | (_, Type::Unknown) => true,

            _ => false,
        }
    }
}
