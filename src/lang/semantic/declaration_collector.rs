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
                    .map(|p| self.resolve_type_annotation(&p.type_annotation))
                    .collect();

                let return_type = func.return_type.as_ref()
                    .map(|t| self.resolve_type_annotation(t))
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
            Declaration::Enum(enum_decl) => {
                // Define enum type
                let enum_type = Type::Enum(enum_decl.name.clone());
                if let Err(e) = self.type_env.define_type(enum_decl.name.clone(), enum_type) {
                    self.add_error(TypeError::new(
                        format!("Failed to define enum '{}': {}", enum_decl.name, e),
                        enum_decl.position.clone()
                    ));
                }
            }
            Declaration::Struct(struct_decl) => {
                // Define struct type with proper fields
                let fields: Vec<(String, Type)> = struct_decl.fields.iter()
                    .map(|field| {
                        let field_type = self.resolve_type_annotation(&field.type_annotation);
                        (field.name.clone(), field_type)
                    })
                    .collect();

                let struct_type = Type::Struct {
                    name: struct_decl.name.clone(),
                    fields,
                };
                if let Err(e) = self.type_env.define_type(struct_decl.name.clone(), struct_type) {
                    self.add_error(TypeError::new(
                        format!("Failed to define struct '{}': {}", struct_decl.name, e),
                        struct_decl.position.clone()
                    ));
                }
            }
            Declaration::Trait(trait_decl) => {
                // Collect trait methods
                let mut methods = Vec::new();
                let mut default_methods = Vec::new();

                for trait_method in &trait_decl.methods {
                    let param_types: Vec<Type> = trait_method.parameters.iter()
                        .map(|p| self.resolve_type_annotation(&p.type_annotation))
                        .collect();

                    let return_type = trait_method.return_type.as_ref()
                        .map(|t| self.resolve_type_annotation(t))
                        .unwrap_or(Type::Void);

                    let method_type = Type::Function {
                        parameters: param_types,
                        return_type: Box::new(return_type),
                    };

                    if trait_method.default_body.is_some() {
                        default_methods.push((trait_method.name.clone(), method_type));
                    } else {
                        methods.push((trait_method.name.clone(), method_type));
                    }
                }

                // Define trait type
                let trait_type = Type::Trait {
                    name: trait_decl.name.clone(),
                    super_traits: trait_decl.super_traits.clone(),
                    methods,
                    default_methods,
                };
                if let Err(e) = self.type_env.define_type(trait_decl.name.clone(), trait_type) {
                    self.add_error(TypeError::new(
                        format!("Failed to define trait '{}': {}", trait_decl.name, e),
                        trait_decl.position.clone()
                    ));
                }
            }
            Declaration::Impl(impl_decl) => {
                // Collect method signatures from impl blocks
                self.collect_impl_declaration(impl_decl);
            }
            Declaration::Class(class_decl) => {
                // Collect class declaration
                self.collect_class_declaration(class_decl);
            }
            Declaration::LibImport(_) => {
                // Library imports are handled at compile time, no collection needed
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
                NamespaceItem::Enum(enum_decl) => {
                    // Define enum type in namespace context
                    let enum_type = Type::Enum(enum_decl.name.clone());
                    if let Err(e) = self.type_env.define_type(enum_decl.name.clone(), enum_type) {
                        self.add_error(TypeError::new(
                            format!("Failed to define enum '{}' in namespace: {}", enum_decl.name, e),
                            enum_decl.position.clone()
                        ));
                    }
                }
                NamespaceItem::Struct(struct_decl) => {
                    // Define struct type in namespace context
                    let struct_type = Type::Struct {
                        name: struct_decl.name.clone(),
                        fields: Vec::new(),
                    };
                    if let Err(e) = self.type_env.define_type(struct_decl.name.clone(), struct_type) {
                        self.add_error(TypeError::new(
                            format!("Failed to define struct '{}' in namespace: {}", struct_decl.name, e),
                            struct_decl.position.clone()
                        ));
                    }
                }
                NamespaceItem::Trait(trait_decl) => {
                    // Define trait type in namespace context
                    let trait_type = Type::Trait {
                        name: trait_decl.name.clone(),
                        super_traits: Vec::new(),
                        methods: Vec::new(),
                        default_methods: Vec::new(),
                    };
                    if let Err(e) = self.type_env.define_type(trait_decl.name.clone(), trait_type) {
                        self.add_error(TypeError::new(
                            format!("Failed to define trait '{}' in namespace: {}", trait_decl.name, e),
                            trait_decl.position.clone()
                        ));
                    }
                }
                NamespaceItem::Impl(impl_decl) => {
                    // Collect method signatures from impl blocks
                    self.collect_impl_declaration(impl_decl);
                }
                NamespaceItem::Class(class_decl) => {
                    // Collect class declaration
                    self.collect_class_declaration(class_decl);
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

    /// Collect impl declaration methods
    fn collect_impl_declaration(&mut self, impl_decl: &ImplDeclaration) {
        // Check if the type being implemented exists
        if !self.type_env.type_exists(&impl_decl.type_name) {
            self.add_error(TypeError::new(
                format!("Cannot implement methods for undefined type '{}'", impl_decl.type_name),
                impl_decl.position.clone()
            ));
            return;
        }

        // Collect each method signature
        for method in &impl_decl.methods {
            let param_types: Vec<Type> = method.parameters.iter()
                .map(|p| Type::from(p.type_annotation.clone()))
                .collect();

            let return_type = method.return_type.as_ref()
                .map(|t| Type::from(t.clone()))
                .unwrap_or(Type::Void);

            let function_type = Type::Function {
                parameters: param_types,
                return_type: Box::new(return_type),
            };

            // Create method name with type prefix for inherent methods
            let method_name = if impl_decl.trait_name.is_some() {
                // For trait implementations, use the original method name
                method.name.clone()
            } else {
                // For inherent implementations, prefix with type name
                format!("{}::{}", impl_decl.type_name, method.name)
            };

            // Register the method as a function
            if let Err(e) = self.type_env.define_function(method_name.clone(), function_type.clone()) {
                self.add_error(TypeError::new(
                    format!("Method '{}' already defined: {}", method_name, e),
                    method.position.clone(),
                ));
            }
        }
    }

    /// Collect class declaration
    fn collect_class_declaration(&mut self, class_decl: &ClassDeclaration) {
        // Create class type with fields and methods
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        // Collect fields
        for field in &class_decl.fields {
            let field_type = self.type_env.resolve_type_annotation(&field.type_annotation);
            fields.push((field.name.clone(), field_type));
        }

        // Collect methods
        for method in &class_decl.methods {
            let param_types: Vec<Type> = method.function.parameters.iter()
                .map(|p| self.type_env.resolve_type_annotation(&p.type_annotation))
                .collect();

            let return_type = method.function.return_type.as_ref()
                .map(|t| self.type_env.resolve_type_annotation(t))
                .unwrap_or(Type::Void);

            let method_type = Type::Function {
                parameters: param_types,
                return_type: Box::new(return_type),
            };
            methods.push((method.function.name.clone(), method_type.clone()));

            // Register method as a function with class prefix
            let method_name = format!("{}::{}", class_decl.name, method.function.name);
            if let Err(e) = self.type_env.define_function(method_name.clone(), method_type) {
                self.add_error(TypeError::new(
                    format!("Method '{}' already defined: {}", method_name, e),
                    method.function.position.clone(),
                ));
            }
        }

        // Separate static and instance members
        let mut static_fields = Vec::new();
        let mut static_methods = Vec::new();
        let mut instance_fields = Vec::new();
        let mut instance_methods = Vec::new();

        for (name, field_type) in fields {
            // Check if field is static by looking at the original declaration
            let is_static = class_decl.fields.iter()
                .find(|f| f.name == name)
                .map(|f| f.is_static)
                .unwrap_or(false);

            if is_static {
                static_fields.push((name, field_type));
            } else {
                instance_fields.push((name, field_type));
            }
        }

        for (name, method_type) in methods {
            // Check if method is static by looking at the original declaration
            let method_decl = class_decl.methods.iter().find(|m| m.function.name == name);
            let is_static = method_decl.map(|m| m.is_static).unwrap_or(false);

            if is_static {
                static_methods.push((name.clone(), method_type));
            } else {
                instance_methods.push((name.clone(), method_type));
            }
        }

        // Create and register class type
        let class_type = Type::Class {
            name: class_decl.name.clone(),
            generic_parameters: class_decl.generic_parameters.iter().map(|p| p.name.clone()).collect(),
            base_class: class_decl.base_class.clone(),
            implemented_traits: class_decl.implemented_traits.clone(),
            fields: instance_fields,
            methods: instance_methods,
            static_fields,
            static_methods,
        };

        if let Err(e) = self.type_env.define_type(class_decl.name.clone(), class_type) {
            self.add_error(TypeError::new(
                format!("Class '{}' already defined: {}", class_decl.name, e),
                class_decl.position.clone(),
            ));
        }

        // Store the class declaration for access control checking
        self.add_class_declaration(class_decl.name.clone(), class_decl.clone());
    }
}
