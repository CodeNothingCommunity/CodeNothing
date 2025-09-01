//! Main semantic analyzer structure and core functionality

use crate::lang::ast::*;
use crate::lang::types::{Type, TypeEnvironment, TypeError, TypeInference};
use crate::lang::builtins::BuiltinRegistry;
use super::access_control::AccessChecker;
use super::vtable::VTableManager;
use std::collections::HashMap;

/// Semantic analyzer
pub struct SemanticAnalyzer {
    pub(super) type_env: TypeEnvironment,
    pub(super) builtins: BuiltinRegistry,
    pub(super) inference: TypeInference,
    pub(super) access_checker: AccessChecker,
    pub(super) vtable_manager: VTableManager,
    pub(super) class_declarations: HashMap<String, ClassDeclaration>,
    pub(super) errors: Vec<TypeError>,
    pub(super) current_function_return_type: Option<Type>,
    pub(super) expected_type: Option<Type>,
    pub(super) in_loop: bool,
    pub(super) in_switch: bool,
    pub(super) debug_mode: bool,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = Self {
            type_env: TypeEnvironment::new(),
            builtins: BuiltinRegistry::new(),
            inference: TypeInference::new(),
            access_checker: AccessChecker::new(),
            vtable_manager: VTableManager::new(),
            class_declarations: HashMap::new(),
            errors: Vec::new(),
            current_function_return_type: None,
            expected_type: None,
            in_loop: false,
            in_switch: false,
            debug_mode: false,
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

    /// Set debug mode
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
    }

    /// Get errors from semantic analysis
    pub fn get_errors(&self) -> &Vec<TypeError> {
        &self.errors
    }

    /// Get type environment (for testing)
    pub fn get_type_environment(&self) -> &TypeEnvironment {
        &self.type_env
    }

    /// Get class declaration by name
    pub fn get_class_declaration(&self, class_name: &str) -> Option<&ClassDeclaration> {
        self.class_declarations.get(class_name)
    }

    /// Add class declaration
    pub fn add_class_declaration(&mut self, class_name: String, class_decl: ClassDeclaration) {
        self.class_declarations.insert(class_name, class_decl);
    }

    /// Analyze a complete program
    pub fn analyze_program(&mut self, program: &Program) -> Result<(), Vec<TypeError>> {
        self.errors.clear();

        // First pass: collect all declarations
        for declaration in &program.declarations {
            self.collect_declaration(declaration);
        }

        // Second pass: process library imports first to ensure namespaces are available
        for declaration in &program.declarations {
            if let Declaration::LibImport(stmt) = declaration {
                self.analyze_declaration(declaration);
            }
        }

        // Third pass: process using declarations after libraries are loaded
        for declaration in &program.declarations {
            if let Declaration::Using(_) = declaration {
                self.analyze_declaration(declaration);
            }
        }

        // Fourth pass: collect class declarations and build VTables with dependency resolution
        let class_declarations: Vec<&ClassDeclaration> = program.declarations.iter()
            .filter_map(|decl| if let Declaration::Class(class_decl) = decl { Some(class_decl) } else { None })
            .collect();

        if !class_declarations.is_empty() {
            if let Err(e) = self.vtable_manager.build_vtable_with_dependencies(&class_declarations, &self.type_env) {
                self.add_error(TypeError::new(
                    format!("Failed to build VTables: {}", e),
                    Position::new(0, 0), // Use default position for global error
                ));
            }
        }

        // Fifth pass: analyze all other declarations including class method bodies
        for declaration in &program.declarations {
            match declaration {
                Declaration::LibImport(_) | Declaration::Using(_) => {
                    // Already processed
                }
                _ => {
                    self.analyze_declaration(declaration);
                }
            }
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

    /// Resolve type annotation to concrete type, considering the type environment
    pub(super) fn resolve_type_annotation(&self, annotation: &TypeAnnotation) -> Type {
        match annotation {
            TypeAnnotation::Int => Type::Int,
            TypeAnnotation::Long => Type::Long,
            TypeAnnotation::Float(Some(size)) => Type::Float(*size),
            TypeAnnotation::Float(None) => Type::Float(32), // Default size
            TypeAnnotation::String => Type::String,
            TypeAnnotation::Num(Some(size)) => Type::Num(*size),
            TypeAnnotation::Num(None) => Type::Num(16), // Default size
            TypeAnnotation::Bool => Type::Bool,
            TypeAnnotation::Array(inner) => Type::Array(Box::new(self.resolve_type_annotation(inner))),
            TypeAnnotation::Obj => Type::Obj,
            TypeAnnotation::Auto => Type::Unknown, // Will be inferred
            TypeAnnotation::Null => Type::Null,
            TypeAnnotation::Void => Type::Void,
            TypeAnnotation::Enum(name) => Type::Enum(name.clone()),
            TypeAnnotation::Trait(name) => Type::Trait {
                name: name.clone(),
                super_traits: Vec::new(),
                methods: Vec::new(), // Will be filled during semantic analysis
                default_methods: Vec::new(),
            },
            TypeAnnotation::Custom(name) => {
                // Check if this is a known user-defined type
                if let Some(user_type) = self.type_env.types.get(name) {
                    user_type.clone()
                } else {
                    // Fallback to custom type
                    Type::Custom(name.clone())
                }
            },
            TypeAnnotation::Generic { name, parameters } => {
                let resolved_params: Vec<Type> = parameters.iter()
                    .map(|p| self.resolve_type_annotation(p))
                    .collect();
                Type::Generic {
                    base_name: name.clone(),
                    type_arguments: resolved_params,
                }
            },
            TypeAnnotation::Function { parameters, return_type } => {
                let resolved_params: Vec<Type> = parameters.iter()
                    .map(|p| self.resolve_type_annotation(p))
                    .collect();
                let resolved_return = Box::new(self.resolve_type_annotation(return_type));
                Type::Function {
                    parameters: resolved_params,
                    return_type: resolved_return,
                }
            }
            TypeAnnotation::FunctionPointer { parameters, return_type } => {
                let resolved_params: Vec<Type> = parameters.iter()
                    .map(|p| self.resolve_type_annotation(p))
                    .collect();
                let resolved_return = Box::new(self.resolve_type_annotation(return_type));
                Type::FunctionPointer {
                    parameters: resolved_params,
                    return_type: resolved_return,
                }
            }
            TypeAnnotation::TypeParameter(name) => Type::TypeParameter(name.clone()),
            TypeAnnotation::Reference { target_type, lifetime } => Type::Reference {
                target_type: Box::new(self.resolve_type_annotation(target_type)),
                lifetime: lifetime.clone(),
            },
            TypeAnnotation::MutableReference { target_type, lifetime } => Type::MutableReference {
                target_type: Box::new(self.resolve_type_annotation(target_type)),
                lifetime: lifetime.clone(),
            },
            TypeAnnotation::RawPointer { target_type, is_mutable } => Type::RawPointer {
                target_type: Box::new(self.resolve_type_annotation(target_type)),
                is_mutable: *is_mutable,
            },
        }
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
