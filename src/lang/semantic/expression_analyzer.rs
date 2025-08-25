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
            Expression::InterpolatedString(parts, position) => {
                // 分析插值字符串中的表达式
                for part in parts {
                    if let crate::lang::lexer::InterpolationPart::Expression(expr_str) = part {
                        // 这里需要解析表达式字符串并分析类型
                        // 暂时简化处理，假设所有插值表达式都是有效的
                        // TODO: 实现完整的表达式解析和类型检查
                    }
                }
                Type::String // 插值字符串的结果总是字符串类型
            }
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
                    UnaryOperator::PreIncrement | UnaryOperator::PostIncrement |
                    UnaryOperator::PreDecrement | UnaryOperator::PostDecrement => {
                        if matches!(operand_type, Type::Int | Type::Float(_)) {
                            operand_type
                        } else {
                            self.add_error(TypeError::new(
                                format!("Increment/decrement requires numeric operand, got '{}'", operand_type),
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
                        if !self.inference.can_unify_with_env(&first_type, &element_type, Some(&self.type_env)) {
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
            Expression::Object(fields, position) => {
                // Object literal - try to infer struct type from context
                for (_key, value_expr) in fields {
                    self.analyze_expression(value_expr);
                }

                // Try to match against known struct types
                if let Some(expected_type) = &self.expected_type {
                    if let Type::Struct { name, fields: struct_fields } = expected_type {
                        // Verify that the object literal matches the struct
                        let mut field_map: std::collections::HashMap<String, Type> = std::collections::HashMap::new();
                        for (field_name, field_type) in struct_fields {
                            field_map.insert(field_name.clone(), field_type.clone());
                        }

                        // Check if all provided fields match the struct
                        let mut all_match = true;
                        for (key, _value_expr) in fields {
                            if !field_map.contains_key(key) {
                                all_match = false;
                                break;
                            }
                        }

                        if all_match {
                            return expected_type.clone();
                        }
                    }
                }

                // If no struct type matches, try to find a struct with matching fields
                for (type_name, type_def) in &self.type_env.types {
                    if let Type::Struct { name: _, fields: struct_fields } = type_def {
                        let mut field_map: std::collections::HashMap<String, Type> = std::collections::HashMap::new();
                        for (field_name, field_type) in struct_fields {
                            field_map.insert(field_name.clone(), field_type.clone());
                        }

                        // Check if all provided fields match this struct
                        let mut all_match = true;
                        for (key, _value_expr) in fields {
                            if !field_map.contains_key(key) {
                                all_match = false;
                                break;
                            }
                        }

                        if all_match && fields.len() == struct_fields.len() {
                            return Type::Struct {
                                name: type_name.clone(),
                                fields: struct_fields.clone(),
                            };
                        }
                    }
                }

                // Fallback to generic object type
                Type::Obj
            }

            Expression::NamespaceAccess { namespace_path, name, position } => {
                // Look up function or variable in the specified namespace
                // First try single-level namespace lookup (for static members)
                if namespace_path.len() == 1 {
                    if let Some(var_type) = self.type_env.lookup_in_namespace(&namespace_path[0], name) {
                        return var_type.clone();
                    }
                }

                // Then try nested namespace lookup
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
                    Type::Array(element_type) => {
                        match *element_type {
                            Type::Unknown => {
                                // For arrays with unknown element type, try to infer from context
                                // For now, assume int as a reasonable default
                                Type::Int
                            }
                            _ => *element_type
                        }
                    }
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

            Expression::FieldAccess { object, field, position } => {
                // Check if this is an enum member access (EnumName.VARIANT)
                if let Expression::Identifier(enum_name, _) = object.as_ref() {
                    if let Some(enum_type) = self.type_env.get_type(enum_name) {
                        if let Type::Enum(_) = enum_type {
                            // This is an enum variant access
                            let variant_name = format!("{}::{}", enum_name, field);
                            if let Some(variant_type) = self.type_env.lookup_variable(&variant_name) {
                                return variant_type.clone();
                            } else {
                                self.add_error(TypeError::new(
                                    format!("Enum variant '{}' not found in enum '{}'", field, enum_name),
                                    position.clone(),
                                ));
                                return Type::Unknown;
                            }
                        }
                    }
                }

                let object_type = self.analyze_expression(object);

                match object_type {
                    Type::Struct { name: _, fields } => {
                        // Look for the field in the struct
                        for (field_name, field_type) in &fields {
                            if field_name == field {
                                return field_type.clone();
                            }
                        }

                        // Field not found
                        self.add_error(TypeError::new(
                            format!("Field '{}' not found in struct", field),
                            position.clone(),
                        ));
                        Type::Unknown
                    }
                    Type::Class { name, .. } => {
                        // Look for the field in the class (including inherited fields)
                        let all_fields = self.type_env.get_class_fields(&name);
                        for (field_name, field_type) in &all_fields {
                            if field_name == field {
                                // Find which class this field belongs to for access control
                                let field_owner_class = self.find_field_owner_class(&name, field);
                                if let Some(owner_class) = field_owner_class {
                                    if let Some(class_decl) = super::analyzer::SemanticAnalyzer::get_class_declaration(self, &owner_class) {
                                        if let Some(field_decl) = class_decl.fields.iter().find(|f| &f.name == field) {
                                            if let Err(access_error) = self.access_checker.check_field_access_with_type_env(
                                                &owner_class,
                                                field,
                                                &field_decl.access_modifier,
                                                position,
                                                &self.type_env
                                            ) {
                                                self.add_error(access_error);
                                                return Type::Unknown;
                                            }
                                        }
                                    }
                                }
                                return field_type.clone();
                            }
                        }

                        // Field not found
                        self.add_error(TypeError::new(
                            format!("Field '{}' not found in class '{}'", field, name),
                            position.clone(),
                        ));
                        Type::Unknown
                    }
                    Type::Obj => {
                        // For object types, we can't determine the field type statically
                        // Return Unknown for now - in a full implementation, we'd need
                        // structural typing or object schemas
                        Type::Unknown
                    }
                    _ => {
                        self.add_error(TypeError::new(
                            format!("Cannot access field '{}' of type '{}'", field, object_type),
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

            Expression::Lambda { parameters, body, position } => {
                // Enter new scope for lambda
                self.type_env.enter_scope();

                // Define parameters in scope
                let mut param_types = Vec::new();
                for param in parameters {
                    let param_type = Type::from(param.type_annotation.clone());
                    param_types.push(param_type.clone());

                    if let Err(e) = self.type_env.define_variable(param.name.clone(), param_type) {
                        self.add_error(TypeError::new(
                            format!("Failed to define lambda parameter '{}': {}", param.name, e),
                            param.position.clone()
                        ));
                    }
                }

                // Analyze body
                let return_type = self.analyze_expression(body);

                self.type_env.exit_scope();

                Type::Lambda {
                    parameters: param_types,
                    return_type: Box::new(return_type),
                }
            }

            Expression::Match { expression, arms, position } => {
                let expr_type = self.analyze_expression(expression);

                if arms.is_empty() {
                    self.add_error(TypeError::new(
                        "Match expression must have at least one arm".to_string(),
                        position.clone()
                    ));
                    return Type::Unknown;
                }

                // Analyze each match arm
                let mut result_types = Vec::new();
                for arm in arms {
                    // Check pattern compatibility with expression type
                    self.analyze_pattern(&arm.pattern, &expr_type);

                    // Analyze guard if present
                    if let Some(guard) = &arm.guard {
                        let guard_type = self.analyze_expression(guard);
                        if !matches!(guard_type, Type::Bool) {
                            self.add_error(TypeError::new(
                                "Match guard must be boolean".to_string(),
                                guard.position().clone()
                            ));
                        }
                    }

                    // Analyze body
                    let body_type = self.analyze_expression(&arm.body);
                    result_types.push(body_type);
                }

                // All arms should return the same type
                let first_type = &result_types[0];
                for (i, arm_type) in result_types.iter().enumerate().skip(1) {
                    if !first_type.is_compatible_with(arm_type) {
                        self.add_error(TypeError::new(
                            format!("Match arm {} returns '{}', but first arm returns '{}'",
                                   i + 1, arm_type, first_type),
                            arms[i].position.clone()
                        ));
                    }
                }

                first_type.clone()
            }

            Expression::Ternary { condition, true_expr, false_expr, position } => {
                let condition_type = self.analyze_expression(condition);
                let true_type = self.analyze_expression(true_expr);
                let false_type = self.analyze_expression(false_expr);

                // Condition must be boolean
                if !matches!(condition_type, Type::Bool) {
                    self.add_error(TypeError::new(
                        format!("Ternary condition must be boolean, got '{}'", condition_type),
                        position.clone(),
                    ));
                }

                // Both branches should have compatible types
                if !true_type.is_compatible_with(&false_type) {
                    self.add_error(TypeError::new(
                        format!("Ternary branches have incompatible types: '{}' and '{}'",
                               true_type, false_type),
                        position.clone(),
                    ));
                    Type::Unknown
                } else {
                    true_type
                }
            }

            Expression::NullCoalescing { left, right, position } => {
                let left_type = self.analyze_expression(left);
                let right_type = self.analyze_expression(right);

                // 左侧可以是任何类型（包括可能为 null 的类型）
                // 右侧应该与左侧兼容，或者是左侧的非空版本
                if !left_type.is_compatible_with(&right_type) && !right_type.is_compatible_with(&left_type) {
                    self.add_error(TypeError::new(
                        format!("Null coalescing operands have incompatible types: '{}' and '{}'",
                               left_type, right_type),
                        position.clone(),
                    ));
                    Type::Unknown
                } else {
                    // 返回右侧类型，因为它是非空的保证
                    right_type
                }
            }

            Expression::When { arms, else_expr, position } => {
                let mut result_type = Type::Unknown;
                let mut first_arm = true;

                // 分析所有 when 分支
                for arm in arms {
                    let condition_type = self.analyze_expression(&arm.condition);
                    let body_type = self.analyze_expression(&arm.body);

                    // 条件必须是布尔类型
                    if !matches!(condition_type, Type::Bool) {
                        self.add_error(TypeError::new(
                            format!("When condition must be boolean, got '{}'", condition_type),
                            arm.position.clone(),
                        ));
                    }

                    // 检查分支类型兼容性
                    if first_arm {
                        result_type = body_type;
                        first_arm = false;
                    } else if !result_type.is_compatible_with(&body_type) {
                        self.add_error(TypeError::new(
                            format!("When branches have incompatible types: '{}' and '{}'",
                                   result_type, body_type),
                            arm.position.clone(),
                        ));
                        result_type = Type::Unknown;
                    }
                }

                // 分析 else 分支（如果存在）
                if let Some(else_expr) = else_expr {
                    let else_type = self.analyze_expression(else_expr);
                    if !result_type.is_compatible_with(&else_type) {
                        self.add_error(TypeError::new(
                            format!("When else branch has incompatible type: expected '{}', got '{}'",
                                   result_type, else_type),
                            position.clone(),
                        ));
                        result_type = Type::Unknown;
                    }
                }

                result_type
            }

            Expression::ArrowFunction { parameters, return_type, body, position } => {
                // Create a new scope for the arrow function
                self.type_env.push_scope();

                // Add parameters to the scope
                let mut param_types = Vec::new();
                for param in parameters {
                    let param_type = self.resolve_type_annotation(&param.type_annotation);
                    param_types.push(param_type.clone());
                    self.type_env.define_variable(param.name.clone(), param_type);
                }

                // Analyze function body and determine return type
                let inferred_return_type = match body {
                    crate::lang::ast::ArrowFunctionBody::Expression(expr) => {
                        self.analyze_expression(expr)
                    }
                    crate::lang::ast::ArrowFunctionBody::Block(statements) => {
                        let mut last_type = Type::Void;
                        for stmt in statements {
                            // For now, we'll analyze statements but not track return types properly
                            // This would need more sophisticated control flow analysis
                            last_type = Type::Void; // Simplified
                        }
                        last_type
                    }
                };

                // Check return type compatibility if specified
                if let Some(specified_return_type) = return_type {
                    let specified_type = self.resolve_type_annotation(specified_return_type);
                    if !specified_type.is_compatible_with(&inferred_return_type) {
                        self.add_error(TypeError::new(
                            format!("Arrow function return type mismatch: expected '{}', got '{}'",
                                   specified_type, inferred_return_type),
                            position.clone(),
                        ));
                    }
                }

                self.type_env.pop_scope();

                // Return function type
                Type::Function {
                    parameters: param_types,
                    return_type: Box::new(inferred_return_type),
                }
            }

            Expression::EnumVariant { enum_name, variant: _, value, position } => {
                // Check if enum exists
                let enum_type_opt = self.type_env.get_type(enum_name).cloned();
                if let Some(enum_type) = enum_type_opt {
                    if let Type::Enum(_) = enum_type {
                        // Check if variant exists and analyze value if present
                        if let Some(value_expr) = value {
                            self.analyze_expression(value_expr);
                        }

                        // Return the enum type
                        enum_type
                    } else {
                        self.add_error(TypeError::new(
                            format!("'{}' is not an enum", enum_name),
                            position.clone()
                        ));
                        Type::Unknown
                    }
                } else {
                    self.add_error(TypeError::new(
                        format!("Undefined enum '{}'", enum_name),
                        position.clone()
                    ));
                    Type::Unknown
                }
            }
            Expression::ClassInstantiation { class_name, arguments, position } => {
                // Analyze constructor arguments first
                let mut arg_types = Vec::new();
                for arg in arguments {
                    let arg_type = self.analyze_expression(arg);
                    arg_types.push(arg_type);
                }

                // Check access control for constructor
                if let Some(class_decl) = super::analyzer::SemanticAnalyzer::get_class_declaration(self, class_name) {
                    if let Some(constructor) = &class_decl.constructor {
                        if let Err(access_error) = self.access_checker.check_constructor_access(
                            class_name,
                            &constructor.access_modifier,
                            position
                        ) {
                            self.add_error(access_error);
                            return Type::Unknown;
                        }
                    }
                }

                // Look up class type
                if let Some(class_type) = self.type_env.get_type(class_name).cloned() {
                    // For now, just return the class type
                    // TODO: Validate constructor arguments
                    class_type
                } else {
                    self.add_error(TypeError::new(
                        format!("Unknown class '{}'", class_name),
                        position.clone(),
                    ));
                    Type::Unknown
                }
            }
            Expression::SuperCall { method_name, arguments, position } => {
                // Analyze super method call or constructor call
                if let Some(current_class) = &self.access_checker.get_context().current_class {
                    if let Some(Type::Class { base_class: Some(base_class_name), .. }) = self.type_env.get_type(current_class).cloned() {
                        // Analyze arguments first
                        let mut arg_types = Vec::new();
                        for arg in arguments {
                            let arg_type = self.analyze_expression(arg);
                            arg_types.push(arg_type);
                        }

                        // Check if this is a constructor call (method_name == base_class_name)
                        if method_name == &base_class_name {
                            // This is a super constructor call
                            if let Some(base_class_decl) = super::analyzer::SemanticAnalyzer::get_class_declaration(self, &base_class_name) {
                                if let Some(constructor) = &base_class_decl.constructor {
                                    // TODO: Validate argument types against constructor signature
                                    // For now, just return Void since constructor calls don't return values
                                    return Type::Void;
                                } else {
                                    self.add_error(TypeError::new(
                                        format!("Base class '{}' has no constructor", base_class_name),
                                        position.clone(),
                                    ));
                                    return Type::Unknown;
                                }
                            } else {
                                self.add_error(TypeError::new(
                                    format!("Base class '{}' declaration not found", base_class_name),
                                    position.clone(),
                                ));
                                return Type::Unknown;
                            }
                        } else {
                            // This is a regular super method call
                            let base_methods = self.type_env.get_class_methods(&base_class_name);

                            // Find the method in base class
                            for (base_method_name, method_type) in &base_methods {
                                if base_method_name == method_name {
                                    // TODO: Validate argument types against method signature
                                    if let Type::Function { return_type, .. } = method_type {
                                        return *return_type.clone();
                                    }
                                    return Type::Void;
                                }
                            }

                            self.add_error(TypeError::new(
                                format!("Method '{}' not found in base class '{}'", method_name, base_class_name),
                                position.clone(),
                            ));
                            Type::Unknown
                        }
                    } else {
                        self.add_error(TypeError::new(
                            format!("Class '{}' has no base class for super call", current_class),
                            position.clone(),
                        ));
                        Type::Unknown
                    }
                } else {
                    self.add_error(TypeError::new(
                        "Super call can only be used inside a class method".to_string(),
                        position.clone(),
                    ));
                    Type::Unknown
                }
            }
            Expression::SuperFieldAccess { field_name, position } => {
                // Analyze super field access
                if let Some(current_class) = &self.access_checker.get_context().current_class {
                    if let Some(Type::Class { base_class: Some(base_class_name), .. }) = self.type_env.get_type(current_class).cloned() {
                        // Look up field in base class
                        let base_fields = self.type_env.get_class_fields(&base_class_name);

                        // Find the field in base class
                        for (base_field_name, field_type) in &base_fields {
                            if base_field_name == field_name {
                                return field_type.clone();
                            }
                        }

                        self.add_error(TypeError::new(
                            format!("Field '{}' not found in base class '{}'", field_name, base_class_name),
                            position.clone(),
                        ));
                        Type::Unknown
                    } else {
                        self.add_error(TypeError::new(
                            format!("Class '{}' has no base class for super field access", current_class),
                            position.clone(),
                        ));
                        Type::Unknown
                    }
                } else {
                    self.add_error(TypeError::new(
                        "Super field access can only be used inside a class method".to_string(),
                        position.clone(),
                    ));
                    Type::Unknown
                }
            }
            Expression::MethodCall { object, method_name, arguments, position } => {
                let object_type = self.analyze_expression(object);

                // Analyze arguments
                let mut arg_types = Vec::new();
                for arg in arguments {
                    let arg_type = self.analyze_expression(arg);
                    arg_types.push(arg_type);
                }

                match object_type {
                    Type::Struct { name: _, fields: _ } => {
                        // For structs, we need to look up methods in impl blocks
                        // This is a simplified implementation
                        Type::Unknown
                    }
                    Type::Class { name, .. } => {
                        // Check access control for method call (including inherited methods)
                        let method_owner_class = self.find_method_owner_class(&name, method_name);
                        if let Some(owner_class) = method_owner_class {
                            if let Some(class_decl) = super::analyzer::SemanticAnalyzer::get_class_declaration(self, &owner_class) {
                                if let Some(method_decl) = class_decl.methods.iter().find(|m| &m.function.name == method_name) {
                                    if let Err(access_error) = self.access_checker.check_method_access_with_type_env(
                                        &owner_class,
                                        method_name,
                                        &method_decl.access_modifier,
                                        position,
                                        &self.type_env
                                    ) {
                                        self.add_error(access_error);
                                        return Type::Unknown;
                                    }
                                }
                            }
                        }

                        // For classes, check if method exists and handle virtual dispatch
                        if let Some(vtable) = self.vtable_manager.get_vtable(&name) {
                            if let Some(method_entry) = vtable.get_method(method_name) {
                                // TODO: Validate argument types against method signature
                                if let Type::Function { return_type, .. } = &method_entry.method_type {
                                    return *return_type.clone();
                                }
                                return Type::Void;
                            } else {
                                self.add_error(TypeError::new(
                                    format!("Method '{}' not found in class '{}'", method_name, name),
                                    position.clone(),
                                ));
                                return Type::Unknown;
                            }
                        } else {
                            self.add_error(TypeError::new(
                                format!("VTable not found for class '{}'", name),
                                position.clone(),
                            ));
                            return Type::Unknown;
                        }
                    }
                    Type::Obj => {
                        // For object types, we can't determine the method type statically
                        Type::Unknown
                    }
                    _ => {
                        self.add_error(TypeError::new(
                            format!("Cannot call method '{}' on type '{}'", method_name, object_type),
                            position.clone(),
                        ));
                        Type::Unknown
                    }
                }
            }
            Expression::StaticMethodCall { class_name, method_name, arguments, position } => {
                // Analyze arguments
                let mut arg_types = Vec::new();
                for arg in arguments {
                    let arg_type = self.analyze_expression(arg);
                    arg_types.push(arg_type);
                }

                // Check if class exists and has the static method
                if let Some(class_type) = self.type_env.get_type(class_name) {
                    if let Type::Class { static_methods, .. } = class_type {
                        for (static_method_name, method_type) in static_methods {
                            if static_method_name == method_name {
                                // Check access control for static method
                                if let Some(class_decl) = super::analyzer::SemanticAnalyzer::get_class_declaration(self, class_name) {
                                    if let Some(method_decl) = class_decl.methods.iter().find(|m| &m.function.name == method_name && m.is_static) {
                                        if let Err(access_error) = self.access_checker.check_static_method_access_with_type_env(
                                            class_name,
                                            method_name,
                                            &method_decl.access_modifier,
                                            position,
                                            &self.type_env
                                        ) {
                                            self.add_error(access_error);
                                            return Type::Unknown;
                                        }
                                    }
                                }

                                // TODO: Validate argument types against method signature
                                if let Type::Function { return_type, .. } = method_type {
                                    return *return_type.clone();
                                }
                                return Type::Void;
                            }
                        }
                        self.add_error(TypeError::new(
                            format!("Static method '{}' not found in class '{}'", method_name, class_name),
                            position.clone(),
                        ));
                        return Type::Unknown;
                    } else {
                        self.add_error(TypeError::new(
                            format!("'{}' is not a class", class_name),
                            position.clone(),
                        ));
                        return Type::Unknown;
                    }
                } else {
                    // Check if this is a library function call (namespace::function)
                    println!("🔧 Semantic: Checking if '{}' is a namespace", class_name);
                    if self.type_env.has_namespace(class_name) {
                        println!("🔧 Semantic: Found namespace '{}', checking for function '{}'", class_name, method_name);

                        // Check if the function exists in the library
                        let full_function_name = format!("{}::{}", class_name, method_name);
                        println!("🔧 Semantic: Checking library function '{}'", full_function_name);
                        match crate::library_loader::check_library_function_exists(&full_function_name) {
                            Ok(true) => {
                                println!("🔧 Semantic: Library function '{}' found", full_function_name);
                                // Most library functions return strings by default
                                return Type::String;
                            }
                            Ok(false) => {
                                println!("🔧 Semantic: Library function '{}' NOT found", full_function_name);
                                self.add_error(TypeError::new(
                                    format!("Function '{}' not found in library namespace '{}'", method_name, class_name),
                                    position.clone(),
                                ));
                                return Type::Unknown;
                            }
                            Err(e) => {
                                println!("🔧 Semantic: Error checking library function '{}': {}", full_function_name, e);
                                // If we can't check, assume it exists (will be validated at runtime)
                                return Type::String;
                            }
                        }
                    } else {
                        println!("🔧 Semantic: Namespace '{}' NOT found", class_name);
                    }

                    self.add_error(TypeError::new(
                        format!("Class or namespace '{}' not found", class_name),
                        position.clone(),
                    ));
                    return Type::Unknown;
                }
            }
            Expression::StaticFieldAccess { class_name, field_name, position } => {
                // Check if class exists and has the static field
                if let Some(class_type) = self.type_env.get_type(class_name) {
                    if let Type::Class { static_fields, .. } = class_type {
                        for (static_field_name, field_type) in static_fields {
                            if static_field_name == field_name {
                                // Check access control for static field
                                if let Some(class_decl) = super::analyzer::SemanticAnalyzer::get_class_declaration(self, class_name) {
                                    if let Some(field_decl) = class_decl.fields.iter().find(|f| &f.name == field_name && f.is_static) {
                                        if let Err(access_error) = self.access_checker.check_static_field_access_with_type_env(
                                            class_name,
                                            field_name,
                                            &field_decl.access_modifier,
                                            position,
                                            &self.type_env
                                        ) {
                                            self.add_error(access_error);
                                            return Type::Unknown;
                                        }
                                    }
                                }
                                return field_type.clone();
                            }
                        }
                        self.add_error(TypeError::new(
                            format!("Static field '{}' not found in class '{}'", field_name, class_name),
                            position.clone(),
                        ));
                        return Type::Unknown;
                    } else {
                        self.add_error(TypeError::new(
                            format!("'{}' is not a class", class_name),
                            position.clone(),
                        ));
                        return Type::Unknown;
                    }
                } else {
                    self.add_error(TypeError::new(
                        format!("Class '{}' not found", class_name),
                        position.clone(),
                    ));
                    return Type::Unknown;
                }
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
                                if !self.inference.can_unify_with_env(&arg_type, expected_type, Some(&self.type_env)) {
                                    self.add_error(TypeError::new(
                                        format!("Argument {} to function '{}': expected '{}', got '{}'",
                                               i + 1, name, expected_type, arg_type),
                                        arg.position().clone(),
                                    ));
                                }
                            }

                            *return_type
                        }
                        Type::LibraryFunction { return_type } => {
                            // Library functions - just analyze arguments and return the specified type
                            for arg in arguments {
                                self.analyze_expression(arg);
                            }
                            if self.debug_mode {
                                println!("🔧 Semantic: Library function '{}' called successfully", name);
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
                    // Check if this is a class constructor call
                    if let Some(class_type) = self.type_env.get_type(name).cloned() {
                        if let Type::Class { .. } = &class_type {
                            // This is a class constructor call
                            // TODO: Validate constructor arguments against constructor signature
                            // For now, just analyze the arguments and return the class type
                            for arg in arguments {
                                self.analyze_expression(arg);
                            }
                            return class_type;
                        }
                    }

                    self.add_error(TypeError::new(
                        format!("Undefined function '{}'", name),
                        position.clone(),
                    ));
                    Type::Unknown
                }
            }
            Expression::NamespaceAccess { namespace_path, name, .. } => {
                // Look up function in the specified namespace
                // First try single-level namespace lookup (for static methods)
                let function_type = if namespace_path.len() == 1 {
                    self.type_env.lookup_in_namespace(&namespace_path[0], name).cloned()
                        .or_else(|| self.type_env.lookup_in_nested_namespace(namespace_path, name).cloned())
                } else {
                    self.type_env.lookup_in_nested_namespace(namespace_path, name).cloned()
                };

                if let Some(function_type) = function_type {
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
                                if !self.inference.can_unify_with_env(&arg_type, expected_type, Some(&self.type_env)) {
                                    self.add_error(TypeError::new(
                                        format!("Argument {} to function '{}::{}': expected '{}', got '{}'",
                                               i + 1, namespace_path.join("::"), name, expected_type, arg_type),
                                        arg.position().clone(),
                                    ));
                                }
                            }

                            *return_type
                        }
                        Type::LibraryFunction { return_type } => {
                            // Library functions accept any number of arguments
                            // Just analyze the arguments for type checking but don't enforce parameter types
                            for arg in arguments {
                                self.analyze_expression(arg);
                            }

                            if self.debug_mode {
                                println!("🔧 Semantic: Library function '{}::{}' called with {} arguments",
                                         namespace_path.join("::"), name, arguments.len());
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
            Expression::FieldAccess { object, field, position: _ } => {
                // This is a method call: object.method(args)
                let object_type = self.analyze_expression(object);


                match object_type {
                    Type::Struct { name, fields: _ } | Type::Custom(name) => {
                        // Look for method in impl blocks for this struct
                        let method_name = format!("{}::{}", name, field);

                        if let Some(function_type) = self.type_env.lookup_function(&method_name).cloned() {
                            match function_type {
                                Type::Function { parameters, return_type } => {
                                    // For methods, the first parameter should be 'self'
                                    let expected_args = if parameters.len() > 0 {
                                        parameters.len() - 1 // Subtract 'self' parameter
                                    } else {
                                        0
                                    };

                                    if arguments.len() != expected_args {
                                        self.add_error(TypeError::new(
                                            format!("Method '{}' expects {} arguments, got {}",
                                                   field, expected_args, arguments.len()),
                                            position.clone(),
                                        ));
                                        return Type::Unknown;
                                    }

                                    // Check argument types (skip first parameter which is 'self')
                                    for (i, (arg, expected_type)) in arguments.iter().zip(parameters.iter().skip(1)).enumerate() {
                                        let arg_type = self.analyze_expression(arg);
                                        if !self.inference.can_unify(expected_type, &arg_type) {
                                            self.add_error(TypeError::new(
                                                format!("Argument {} to method '{}': expected '{}', got '{}'",
                                                       i + 1, field, expected_type, arg_type),
                                                arg.position().clone(),
                                            ));
                                        }
                                    }

                                    *return_type
                                }
                                _ => {
                                    self.add_error(TypeError::new(
                                        format!("'{}' is not a method", field),
                                        position.clone(),
                                    ));
                                    Type::Unknown
                                }
                            }
                        } else {
                            self.add_error(TypeError::new(
                                format!("Method '{}' not found for type '{}'", field, name),
                                position.clone(),
                            ));
                            Type::Unknown
                        }
                    }
                    _ => {
                        self.add_error(TypeError::new(
                            format!("Cannot call method '{}' on type '{}'", field, object_type),
                            position.clone(),
                        ));
                        Type::Unknown
                    }
                }
            }
            _ => {
                // For now, we only support direct function calls, namespace access, and method calls
                self.add_error(TypeError::new(
                    "Complex function expressions not supported".to_string(),
                    position.clone(),
                ));
                Type::Unknown
            }
        }
    }

    /// Analyze pattern and check compatibility with expected type
    fn analyze_pattern(&mut self, pattern: &Pattern, expected_type: &Type) {
        match pattern {
            Pattern::Wildcard(_) => {
                // Wildcard matches anything
            }
            Pattern::Literal(expr) => {
                let literal_type = self.analyze_expression(expr);
                if !literal_type.is_compatible_with(expected_type) {
                    self.add_error(TypeError::new(
                        format!("Pattern literal type '{}' doesn't match expected type '{}'",
                               literal_type, expected_type),
                        expr.position().clone()
                    ));
                }
            }
            Pattern::Identifier(name, position) => {
                // Bind identifier to the expected type
                if let Err(e) = self.type_env.define_variable(name.clone(), expected_type.clone()) {
                    self.add_error(TypeError::new(
                        format!("Failed to bind pattern variable '{}': {}", name, e),
                        position.clone()
                    ));
                }
            }
            Pattern::EnumVariant { enum_name, variant: _, value, position } => {
                // Check if enum matches expected type
                if let Type::Enum(expected_enum) = expected_type {
                    if enum_name != expected_enum {
                        self.add_error(TypeError::new(
                            format!("Pattern enum '{}' doesn't match expected enum '{}'",
                                   enum_name, expected_enum),
                            position.clone()
                        ));
                    }
                } else {
                    self.add_error(TypeError::new(
                        format!("Pattern expects enum '{}' but got type '{}'",
                               enum_name, expected_type),
                        position.clone()
                    ));
                }

                // Analyze nested pattern if present
                if let Some(nested_pattern) = value {
                    // For now, assume the nested type is unknown
                    // In a full implementation, we'd look up the variant's associated type
                    self.analyze_pattern(nested_pattern, &Type::Unknown);
                }
            }
            Pattern::Tuple(patterns, position) => {
                // For now, treat as array-like
                if let Type::Array(element_type) = expected_type {
                    for pattern in patterns {
                        self.analyze_pattern(pattern, element_type);
                    }
                } else {
                    self.add_error(TypeError::new(
                        format!("Tuple pattern doesn't match expected type '{}'", expected_type),
                        position.clone()
                    ));
                }
            }
            Pattern::Array(patterns, position) => {
                if let Type::Array(element_type) = expected_type {
                    for pattern in patterns {
                        self.analyze_pattern(pattern, element_type);
                    }
                } else {
                    self.add_error(TypeError::new(
                        format!("Array pattern doesn't match expected type '{}'", expected_type),
                        position.clone()
                    ));
                }
            }
        }
    }



    /// Find which class owns a specific field (including inheritance)
    fn find_field_owner_class(&self, class_name: &str, field_name: &str) -> Option<String> {
        // Check current class first
        if let Some(class_decl) = super::analyzer::SemanticAnalyzer::get_class_declaration(self, class_name) {
            if class_decl.fields.iter().any(|f| &f.name == field_name) {
                return Some(class_name.to_string());
            }

            // Check base class recursively
            if let Some(base_class) = &class_decl.base_class {
                return self.find_field_owner_class(base_class, field_name);
            }
        }
        None
    }

    /// Find which class owns a specific method (including inheritance)
    fn find_method_owner_class(&self, class_name: &str, method_name: &str) -> Option<String> {
        // Check current class first
        if let Some(class_decl) = super::analyzer::SemanticAnalyzer::get_class_declaration(self, class_name) {
            if class_decl.methods.iter().any(|m| &m.function.name == method_name) {
                return Some(class_name.to_string());
            }

            // Check base class recursively
            if let Some(base_class) = &class_decl.base_class {
                return self.find_method_owner_class(base_class, method_name);
            }
        }
        None
    }
}
