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

            // Assignment operation
            BinaryOperator::Assign => {
                // Assignment should return the assigned value's type
                right_type.clone()
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
}
