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
                // Handle global variable declarations - only analyze initializer, don't redefine
                if let Statement::VarDeclaration { initializer, .. } = stmt {
                    if let Some(init_expr) = initializer {
                        // Only analyze the initializer expression, variable was already defined in collection phase
                        self.analyze_expression(init_expr);
                    }
                }
            }
            Declaration::Enum(enum_decl) => {
                self.analyze_enum_declaration(enum_decl);
            }
            Declaration::Struct(struct_decl) => {
                self.analyze_struct_declaration(struct_decl);
            }
            Declaration::Trait(trait_decl) => {
                self.analyze_trait_declaration(trait_decl);
            }
            Declaration::Impl(impl_decl) => {
                self.analyze_impl_declaration(impl_decl);
            }
            Declaration::Class(class_decl) => {
                self.analyze_class_declaration(class_decl);
            }
            Declaration::LibImport(stmt) => {
                // Library imports are handled by statement analyzer
                self.analyze_statement(stmt);
            }
            Declaration::Include(_) => {
                // Include declarations are processed before semantic analysis
                // No additional analysis needed here
            }
            Declaration::Package(_) => {
                // Package declarations are metadata only
            }
            Declaration::Import(import_decl) => {
                // Process import declarations to add symbols to current scope
                self.analyze_import_declaration(import_decl);
            }
            Declaration::Export(export_decl) => {
                // Analyze the exported declaration
                self.analyze_declaration(&export_decl.declaration);
            }
        }
    }
    
    /// Analyze namespace items recursively
    fn analyze_namespace_items(&mut self, ns: &NamespaceDeclaration, parent_path: &[String]) {
        let mut current_path = parent_path.to_vec();
        current_path.push(ns.name.clone());

        // Set package context for top-level namespaces
        let old_package = if parent_path.is_empty() {
            // This is a top-level namespace, set it as the current package
            let old_package = self.access_checker.get_package_context().clone();
            self.access_checker.set_package_context(Some(ns.name.clone()));
            old_package
        } else {
            None
        };

        // Enter namespace context
        self.access_checker.get_context_mut().enter_namespace(ns.name.clone());

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
                    self.analyze_namespace_variable(&current_path, stmt);
                }
                NamespaceItem::Enum(enum_decl) => {
                    self.analyze_enum_declaration(enum_decl);
                }
                NamespaceItem::Struct(struct_decl) => {
                    self.analyze_struct_declaration(struct_decl);
                }
                NamespaceItem::Trait(trait_decl) => {
                    self.analyze_trait_declaration(trait_decl);
                }
                NamespaceItem::Impl(impl_decl) => {
                    self.analyze_impl_declaration(impl_decl);
                }
                NamespaceItem::Class(class_decl) => {
                    self.analyze_class_declaration(class_decl);
                }
            }
        }

        // Exit namespace context
        self.access_checker.get_context_mut().exit_namespace();

        // Restore previous package context for top-level namespaces
        if parent_path.is_empty() {
            self.access_checker.set_package_context(old_package);
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
            .map(|t| self.resolve_type_annotation(t));

        // Define parameters
        for param in &function.parameters {
            let param_type = self.resolve_optional_type_annotation(&param.type_annotation);
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

        // Import all functions and variables from the current namespace and parent namespaces
        // This allows functions to call each other within the same namespace and access namespace variables
        for i in 1..=namespace_path.len() {
            let current_namespace_path = &namespace_path[0..i];
            let namespace_name = current_namespace_path.join("::");

            let current_path: Vec<String> = current_namespace_path.iter().map(|s| s.clone()).collect();

            // Import functions
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

            // Import variables
            let variables_to_import: Vec<(String, Type)> = if let Some(namespace_variables) = self.type_env.get_namespace_variables(&current_path) {
                namespace_variables.iter()
                    .filter(|(name, _)| self.type_env.lookup_variable(name).is_none())
                    .map(|(name, var_type)| (name.clone(), var_type.clone()))
                    .collect()
            } else {
                Vec::new()
            };

            for (name, var_type) in variables_to_import {
                if let Err(e) = self.type_env.define_variable(name.clone(), var_type) {
                    self.add_error(TypeError::new(
                        format!("Failed to import variable '{}' from namespace '{}': {}", name, namespace_name, e),
                        function.position.clone(),
                    ));
                }
            }
        }

        // Set current function return type
        self.current_function_return_type = function.return_type.as_ref()
            .map(|t| self.resolve_type_annotation(t));

        // Define parameters
        for param in &function.parameters {
            let param_type = self.resolve_optional_type_annotation(&param.type_annotation);
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
            .map(|t| self.resolve_type_annotation(t));
        
        // Define parameters
        for param in &function.parameters {
            let param_type = self.resolve_optional_type_annotation(&param.type_annotation);
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

        // Memo declarations are treated as content blocks for documentation
        // NOTE: Future enhancement could extract embedded function definitions
        let _ = memo; // Suppress unused warning
    }

    /// Analyze using declaration
    fn analyze_using_declaration(&mut self, using: &UsingDeclaration) {
        if self.debug_mode {
            println!("🔧 Semantic: Processing using ns declaration: {}", using.namespace);
        }

        // Import all functions from the specified namespace into current scope
        let functions_to_import: Vec<(String, Type)> = if let Some(namespace_info) = self.type_env.get_namespaces().get(&using.namespace) {
            if self.debug_mode {
                println!("🔧 Semantic: Found namespace '{}' with {} functions",
                         using.namespace, namespace_info.functions.len());
            }
            namespace_info.functions.iter()
                .map(|(name, func_type)| (name.clone(), func_type.clone()))
                .collect()
        } else {
            // Check if this is a library namespace that hasn't been loaded yet
            if self.debug_mode {
                println!("🔧 Semantic: Namespace '{}' not found in type environment, checking libraries...", using.namespace);
            }

            // Try to preload common libraries that might contain this namespace
            let common_libs = ["io", "math", "time", "os", "fs", "http", "json"];
            for lib_name in &common_libs {
                let _ = crate::library_loader::load_library(lib_name);
            }

            // Try to get all functions from all loaded libraries that match this namespace
            match crate::library_loader::get_all_library_functions() {
                Ok(all_functions) => {
                    if self.debug_mode {
                        println!("🔧 Semantic: Found {} total library functions", all_functions.len());
                    }
                    let namespace_prefix = format!("{}::", using.namespace);
                    let matching_functions: Vec<(String, Type)> = all_functions.into_iter()
                        .filter(|func_name| func_name.starts_with(&namespace_prefix))
                        .map(|func_name| {
                            let short_name = func_name.strip_prefix(&namespace_prefix).unwrap_or(&func_name);
                            if self.debug_mode {
                                println!("🔧 Semantic: Found library function '{}' -> '{}'", func_name, short_name);
                            }
                            // Create a library function type
                            let func_type = Type::LibraryFunction {
                                return_type: Box::new(Type::String),
                            };
                            (short_name.to_string(), func_type)
                        })
                        .collect();

                    if matching_functions.is_empty() {
                        self.add_error(TypeError::new(
                            format!("Namespace '{}' not found or has no functions", using.namespace),
                            using.position.clone(),
                        ));
                        return;
                    }

                    matching_functions
                }
                Err(e) => {
                    if self.debug_mode {
                        println!("🔧 Semantic: Error getting library functions: {}", e);
                    }
                    self.add_error(TypeError::new(
                        format!("Failed to load namespace '{}': {}", using.namespace, e),
                        using.position.clone(),
                    ));
                    return;
                }
            }
        };

        if self.debug_mode {
            println!("🔧 Semantic: Importing {} functions from namespace '{}'",
                     functions_to_import.len(), using.namespace);
        }

        // Now import the functions
        for (func_name, func_type) in functions_to_import {
            if self.debug_mode {
                println!("🔧 Semantic: Importing function '{}' with type {:?}", func_name, func_type);
            }
            let _ = self.type_env.define_function(func_name, func_type);
        }
    }

    /// Analyze namespace variable declaration
    fn analyze_namespace_variable(&mut self, namespace_path: &[String], stmt: &Statement) {
        if let Statement::VarDeclaration { name, type_annotation, initializer, .. } = stmt {
            // Resolve the variable type
            let var_type = if let Some(type_ann) = type_annotation {
                self.resolve_type_annotation(type_ann)
            } else if let Some(init_expr) = initializer {
                // Infer type from initializer
                self.analyze_expression(init_expr)
            } else {
                Type::Auto(Box::new(Type::Unknown))
            };

            // Create namespace path string
            let namespace_str = namespace_path.join("::");

            // Define the variable in the namespace
            if let Err(e) = self.type_env.define_variable_in_namespace(&namespace_str, name.clone(), var_type) {
                self.add_error(TypeError::new(
                    format!("Failed to define namespace variable '{}::{}': {}", namespace_str, name, e),
                    stmt.position().clone(),
                ));
            }

            // Also analyze the initializer expression if present
            if let Some(init_expr) = initializer {
                self.analyze_expression(init_expr);
            }
        }
    }

    /// Analyze enum declaration
    fn analyze_enum_declaration(&mut self, enum_decl: &EnumDeclaration) {
        // The enum type should already be defined by the collector
        // Just verify it exists and analyze the variants
        if !self.type_env.type_exists(&enum_decl.name) {
            self.add_error(TypeError::new(
                format!("Enum '{}' was not properly collected", enum_decl.name),
                enum_decl.position.clone()
            ));
            return;
        }

        let enum_type = Type::Enum(enum_decl.name.clone());

        // Analyze each variant
        for variant in &enum_decl.variants {
            // If variant has an associated value, analyze its type
            if let Some(value_expr) = &variant.value {
                let value_type = self.analyze_expression(value_expr);

                // Define the variant constructor function
                let variant_name = format!("{}::{}", enum_decl.name, variant.name);
                let constructor_type = Type::Function {
                    parameters: vec![value_type],
                    return_type: Box::new(enum_type.clone()),
                };

                if let Err(e) = self.type_env.define_function(variant_name.clone(), constructor_type) {
                    self.add_error(TypeError::new(
                        format!("Failed to define enum variant constructor '{}': {}", variant_name, e),
                        variant.position.clone()
                    ));
                }
            } else {
                // Simple variant without value
                let variant_name = format!("{}::{}", enum_decl.name, variant.name);
                if let Err(e) = self.type_env.define_variable(variant_name.clone(), enum_type.clone()) {
                    self.add_error(TypeError::new(
                        format!("Failed to define enum variant '{}': {}", variant_name, e),
                        variant.position.clone()
                    ));
                }
            }
        }
    }

    /// Analyze struct declaration
    fn analyze_struct_declaration(&mut self, struct_decl: &StructDeclaration) {
        // The struct type should already be defined by the collector
        // Just verify it exists and update with field information
        if !self.type_env.type_exists(&struct_decl.name) {
            self.add_error(TypeError::new(
                format!("Struct '{}' was not properly collected", struct_decl.name),
                struct_decl.position.clone()
            ));
            return;
        }

        // Collect field types and validate them
        let mut field_types = Vec::new();
        for field in &struct_decl.fields {
            let field_type = self.resolve_type_annotation(&field.type_annotation);
            field_types.push((field.name.clone(), field_type));
        }

        // Update the struct type with field information
        let struct_type = Type::Struct {
            name: struct_decl.name.clone(),
            fields: field_types,
        };

        // Update the type environment with the complete struct definition
        self.type_env.types.insert(struct_decl.name.clone(), struct_type);
    }

    /// Analyze trait declaration
    fn analyze_trait_declaration(&mut self, trait_decl: &TraitDeclaration) {
        // Collect method signatures
        let mut method_signatures = Vec::new();
        for method in &trait_decl.methods {
            let param_types: Vec<Type> = method.parameters.iter()
                .map(|p| self.resolve_optional_type_annotation(&p.type_annotation))
                .collect();

            let return_type = method.return_type.as_ref()
                .map(|t| self.resolve_type_annotation(t))
                .unwrap_or(Type::Void);

            method_signatures.push((
                method.name.clone(),
                Type::Function {
                    parameters: param_types,
                    return_type: Box::new(return_type),
                }
            ));

            // If method has default implementation, analyze it
            if let Some(default_body) = &method.default_body {
                // Enter method scope for analysis
                self.type_env.enter_scope();

                // Define parameters in scope
                for param in &method.parameters {
                    let param_type = self.resolve_optional_type_annotation(&param.type_annotation);
                    if let Err(e) = self.type_env.define_variable(param.name.clone(), param_type) {
                        self.add_error(TypeError::new(
                            format!("Failed to define parameter '{}': {}", param.name, e),
                            param.position.clone()
                        ));
                    }
                }

                // Analyze method body
                self.analyze_statement(default_body);

                self.type_env.exit_scope();
            }
        }

        // Update the trait type with method information
        let trait_type = Type::Trait {
            name: trait_decl.name.clone(),
            super_traits: Vec::new(),
            methods: method_signatures,
            default_methods: Vec::new(),
        };

        // Update the type environment with the complete trait definition
        self.type_env.types.insert(trait_decl.name.clone(), trait_type);
    }

    /// Analyze impl declaration
    fn analyze_impl_declaration(&mut self, impl_decl: &ImplDeclaration) {
        // Check if the type being implemented exists
        if !self.type_env.type_exists(&impl_decl.type_name) {
            self.add_error(TypeError::new(
                format!("Cannot implement methods for undefined type '{}'", impl_decl.type_name),
                impl_decl.position.clone()
            ));
            return;
        }

        // If implementing a trait, check if trait exists
        if let Some(trait_name) = &impl_decl.trait_name {
            if !self.type_env.type_exists(trait_name) {
                self.add_error(TypeError::new(
                    format!("Cannot implement undefined trait '{}'", trait_name),
                    impl_decl.position.clone()
                ));
                return;
            }

            // Check that all required trait methods are implemented
            if let Some(trait_name) = &impl_decl.trait_name {
                self.validate_trait_implementation_completeness(impl_decl, trait_name);
            }
        }

        // Analyze each method implementation
        for method in &impl_decl.methods {
            // Create method name with type prefix for inherent methods
            let method_name = if impl_decl.trait_name.is_some() {
                // For trait implementations, use the original method name
                method.name.clone()
            } else {
                // For inherent implementations, prefix with type name
                format!("{}::{}", impl_decl.type_name, method.name)
            };

            // Analyze the method (for now, just use regular function analysis)
            self.analyze_function(method);
        }
    }

    /// Analyze class declaration
    fn analyze_class_declaration(&mut self, class_decl: &ClassDeclaration) {
        // Validate base class if present
        if let Some(base_class_name) = &class_decl.base_class {
            if !self.type_env.get_type(base_class_name).is_some() {
                self.add_error(TypeError::new(
                    format!("Base class '{}' not found", base_class_name),
                    class_decl.position.clone(),
                ));
            } else {
                // Check for circular inheritance
                if self.has_circular_inheritance(&class_decl.name, base_class_name) {
                    self.add_error(TypeError::new(
                        format!("Circular inheritance detected: class '{}' cannot extend '{}'",
                               class_decl.name, base_class_name),
                        class_decl.position.clone(),
                    ));
                }
            }
        }

        // Process friend class declarations
        for friend_class_name in &class_decl.friend_classes {
            // Validate that the friend class exists (or will exist)
            // Note: We don't require the friend class to exist yet since classes can be forward-declared
            // or defined later in the same compilation unit

            // Add the friend relationship to the type environment
            self.type_env.add_friend_relationship(
                class_decl.name.clone(),
                friend_class_name.clone()
            );
        }

        // Enter class context for access control
        self.access_checker.get_context_mut().enter_class(class_decl.name.clone());

        // Set inheritance context if this class extends another class
        if let Some(base_class_name) = &class_decl.base_class {
            self.access_checker.get_context_mut().set_inheritance_context(
                class_decl.name.clone(),
                base_class_name.clone()
            );
        }

        // Build class fields list for self type (non-static fields only)
        let class_fields: Vec<(String, Type)> = class_decl.fields.iter()
            .filter(|field| !field.is_static)
            .map(|field| (field.name.clone(), self.type_env.resolve_type_annotation(&field.type_annotation)))
            .collect();

        // Build static fields list and analyze their initializers
        let mut static_fields: Vec<(String, Type)> = Vec::new();
        for field in class_decl.fields.iter().filter(|field| field.is_static) {
            let field_type = self.type_env.resolve_type_annotation(&field.type_annotation);

            // Analyze field initializer if present
            if let Some(initializer) = &field.initializer {
                let initializer_type = self.analyze_expression(initializer);

                // Check type compatibility
                if !field_type.is_compatible_with_env(&initializer_type, Some(&self.type_env)) {
                    self.add_error(TypeError::new(
                        format!("Cannot initialize static field '{}' of type {:?} with value of type {:?}",
                               field.name, field_type, initializer_type),
                        field.position.clone(),
                    ));
                }
            }

            static_fields.push((field.name.clone(), field_type));
        }

        // Build static methods list
        let static_methods: Vec<(String, Type)> = class_decl.methods.iter()
            .filter(|method| method.is_static)
            .map(|method| {
                let param_types: Vec<Type> = method.function.parameters.iter()
                    .map(|p| self.type_env.resolve_optional_type_annotation(&p.type_annotation))
                    .collect();
                let return_type = method.function.return_type.as_ref()
                    .map(|t| self.type_env.resolve_type_annotation(t))
                    .unwrap_or(Type::Void);
                (method.function.name.clone(), Type::Function {
                    parameters: param_types,
                    return_type: Box::new(return_type),
                })
            })
            .collect();

        // Register static members BEFORE analyzing constructor (so they're available during constructor analysis)
        self.register_static_members(class_decl, &static_fields, &static_methods);

        // Analyze constructor if present
        if let Some(constructor) = &class_decl.constructor {
            // Create a scope for constructor analysis
            self.type_env.push_scope();
            self.access_checker.get_context_mut().enter_method();

            // Set current function return type for constructor (returns the class type)
            self.current_function_return_type = Some(Type::Class {
                name: class_decl.name.clone(),
                generic_parameters: class_decl.generic_parameters.iter().map(|p| p.name.clone()).collect(),
                base_class: class_decl.base_class.clone(),
                implemented_traits: class_decl.implemented_traits.clone(),
                fields: class_fields.clone(),
                methods: Vec::new(),
                static_fields: static_fields.clone(),
                static_methods: static_methods.clone(),
            });

            // Add implicit 'self' parameter to constructor scope
            let self_type = Type::Class {
                name: class_decl.name.clone(),
                generic_parameters: class_decl.generic_parameters.iter().map(|p| p.name.clone()).collect(),
                base_class: class_decl.base_class.clone(),
                implemented_traits: class_decl.implemented_traits.clone(),
                fields: class_fields.clone(),
                methods: Vec::new(),
                static_fields: static_fields.clone(),
                static_methods: static_methods.clone(),
            };
            if let Err(e) = self.type_env.define_variable("self".to_string(), self_type) {
                self.add_error(TypeError::new(
                    format!("Failed to define 'self' parameter: {}", e),
                    class_decl.position.clone(),
                ));
            }

            // Add constructor parameters to scope
            for param in &constructor.parameters {
                let param_type = self.type_env.resolve_optional_type_annotation(&param.type_annotation);
                if let Err(e) = self.type_env.define_variable(param.name.clone(), param_type) {
                    self.add_error(TypeError::new(
                        format!("Parameter '{}' already defined: {}", param.name, e),
                        param.position.clone(),
                    ));
                }
            }

            // Analyze constructor body
            self.analyze_statement(&constructor.body);

            self.access_checker.get_context_mut().exit_method();
            self.type_env.pop_scope();
            self.current_function_return_type = None;
        }

        // Analyze destructor if present
        if let Some(destructor) = &class_decl.destructor {
            self.type_env.push_scope();
            self.access_checker.get_context_mut().enter_method();
            self.analyze_statement(&destructor.body);
            self.access_checker.get_context_mut().exit_method();
            self.type_env.pop_scope();
        }

        // Analyze methods
        for method in &class_decl.methods {
            // Create a new scope for method analysis
            self.type_env.push_scope();
            self.access_checker.get_context_mut().enter_method();

            // Set current function return type for method
            self.current_function_return_type = method.function.return_type.as_ref()
                .map(|t| self.type_env.resolve_type_annotation(t));

            // Add method parameters to scope (including 'self' parameter)
            for param in &method.function.parameters {
                let param_type = if param.name == "self" {
                    // For 'self' parameter, use the current class type
                    Type::Class {
                        name: class_decl.name.clone(),
                        generic_parameters: class_decl.generic_parameters.iter().map(|p| p.name.clone()).collect(),
                        base_class: class_decl.base_class.clone(),
                        implemented_traits: class_decl.implemented_traits.clone(),
                        fields: class_fields.clone(),
                        methods: Vec::new(), // Will be filled later
                        static_fields: static_fields.clone(),
                        static_methods: static_methods.clone(),
                    }
                } else {
                    self.type_env.resolve_optional_type_annotation(&param.type_annotation)
                };

                if let Err(e) = self.type_env.define_variable(param.name.clone(), param_type) {
                    self.add_error(TypeError::new(
                        format!("Parameter '{}' already defined: {}", param.name, e),
                        param.position.clone(),
                    ));
                }
            }

            // Validate method override
            if method.is_override {
                self.validate_method_override(class_decl, method);
            }

            // Validate virtual method
            if method.is_virtual && method.is_override {
                self.add_error(TypeError::new(
                    format!("Method '{}' cannot be both virtual and override", method.function.name),
                    method.position.clone(),
                ));
            }

            // Validate abstract method
            if method.is_abstract {
                self.validate_abstract_method(method, class_decl);
            }

            // Analyze method body
            self.analyze_statement(&method.function.body);

            self.access_checker.get_context_mut().exit_method();
            self.type_env.pop_scope();
            self.current_function_return_type = None;
        }

        // VTable should already be built in the main analyzer, so we skip building it here
        // to avoid duplicate VTable construction

        // Validate abstract methods are implemented (if not abstract class)
        if let Some(vtable) = self.vtable_manager.get_vtable(&class_decl.name) {
            let unimplemented = vtable.validate_abstract_methods();
            if !unimplemented.is_empty() && !class_decl.is_abstract {
                for method_name in unimplemented {
                    self.add_error(TypeError::new(
                        format!("Class '{}' must implement abstract method '{}'",
                               class_decl.name, method_name),
                        class_decl.position.clone(),
                    ));
                }
            }
        }

        // Validate interface implementations
        for trait_name in &class_decl.implemented_traits {
            self.validate_trait_implementation(class_decl, trait_name);
        }

        // Update the class type with complete information (it was already defined in collection phase)
        // Get the existing class type to preserve methods that were collected
        let existing_methods = if let Some(Type::Class { methods, .. }) = self.type_env.get_type(&class_decl.name) {
            methods.clone()
        } else {
            Vec::new()
        };

        let complete_class_type = Type::Class {
            name: class_decl.name.clone(),
            generic_parameters: class_decl.generic_parameters.iter().map(|p| p.name.clone()).collect(),
            base_class: class_decl.base_class.clone(),
            implemented_traits: class_decl.implemented_traits.clone(),
            fields: class_fields,
            methods: existing_methods, // Preserve methods from collection phase
            static_fields: static_fields,
            static_methods: static_methods,
        };

        // Update the existing type instead of defining a new one
        self.type_env.update_type(class_decl.name.clone(), complete_class_type);

        // Exit class context
        self.access_checker.get_context_mut().exit_class();
    }

    /// Validate that a class properly implements a trait
    fn validate_trait_implementation(&mut self, class_decl: &ClassDeclaration, trait_name: &str) {
        // Check if trait exists and get methods
        let trait_methods = if let Some(trait_type) = self.type_env.get_type(trait_name) {
            if let Type::Trait { name: _, methods, .. } = trait_type {
                methods.clone()  // Clone to avoid borrowing issues
            } else {
                self.add_error(TypeError::new(
                    format!("'{}' is not a trait", trait_name),
                    class_decl.position.clone(),
                ));
                return;
            }
        } else {
            self.add_error(TypeError::new(
                format!("Trait '{}' not found", trait_name),
                class_decl.position.clone(),
            ));
            return;
        };

        // Check that all required trait methods are implemented
        for (method_name, method_type) in trait_methods {
            let mut found_implementation = false;

            // First check in this class
            for class_method in &class_decl.methods {
                if class_method.function.name == method_name {
                    // Verify method signature matches
                    let param_types: Vec<Type> = class_method.function.parameters.iter()
                        .map(|p| self.type_env.resolve_optional_type_annotation(&p.type_annotation))
                        .collect();

                    let return_type = class_method.function.return_type.as_ref()
                        .map(|t| self.type_env.resolve_type_annotation(t))
                        .unwrap_or(Type::Void);

                    let class_method_type = Type::Function {
                        parameters: param_types,
                        return_type: Box::new(return_type),
                    };

                    if class_method_type.is_compatible_with(&method_type) {
                        found_implementation = true;
                        break;
                    } else {
                        self.add_error(TypeError::new(
                            format!("Method '{}' in class '{}' does not match trait '{}' signature",
                                   method_name, class_decl.name, trait_name),
                            class_method.position.clone(),
                        ));
                    }
                }
            }

            // If not found in this class, check parent classes
            if !found_implementation {
                if let Some(base_class_name) = &class_decl.base_class {
                    if let Some(base_class) = self.get_class_declaration(base_class_name) {
                        // Check if parent class implements this trait and has the method
                        if base_class.implemented_traits.contains(&trait_name.to_string()) {
                            for base_method in &base_class.methods {
                                if base_method.function.name == method_name {
                                    found_implementation = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if !found_implementation {
                self.add_error(TypeError::new(
                    format!("Class '{}' must implement method '{}' from trait '{}'",
                           class_decl.name, method_name, trait_name),
                    class_decl.position.clone(),
                ));
            }
        }
    }

    /// Check for circular inheritance
    fn has_circular_inheritance(&self, class_name: &str, base_class_name: &str) -> bool {
        let mut visited = std::collections::HashSet::new();
        self.check_inheritance_cycle(base_class_name, class_name, &mut visited)
    }

    /// Recursively check for inheritance cycles
    fn check_inheritance_cycle(&self, current_class: &str, target_class: &str, visited: &mut std::collections::HashSet<String>) -> bool {
        if current_class == target_class {
            return true; // Found a cycle
        }

        if visited.contains(current_class) {
            return false; // Already visited this class in this path
        }

        visited.insert(current_class.to_string());

        // Check if current_class has a base class
        if let Some(Type::Class { base_class: Some(base), .. }) = self.type_env.get_type(current_class) {
            if self.check_inheritance_cycle(base, target_class, visited) {
                return true;
            }
        }

        visited.remove(current_class);
        false
    }

    /// Validate method override
    fn validate_method_override(&mut self, class_decl: &ClassDeclaration, method: &ClassMethod) {
        if let Some(base_class_name) = &class_decl.base_class {
            // Check if the method exists in the base class VTable
            if let Some(base_vtable) = self.vtable_manager.get_vtable(base_class_name) {
                if let Some(base_method_entry) = base_vtable.get_method(&method.function.name) {
                    // Found the method in base class, validate signature
                    let param_types: Vec<Type> = method.function.parameters.iter()
                        .map(|p| self.type_env.resolve_optional_type_annotation(&p.type_annotation))
                        .collect();

                    let return_type = method.function.return_type.as_ref()
                        .map(|t| self.type_env.resolve_type_annotation(t))
                        .unwrap_or(Type::Void);

                    let current_method_type = Type::Function {
                        parameters: param_types,
                        return_type: Box::new(return_type),
                    };

                    // Check if signatures match (inheritance-aware)
                    if !self.type_env.is_method_override_compatible(&current_method_type, &base_method_entry.method_type) {
                        self.add_error(TypeError::new(
                            format!("Override method '{}' signature does not match base class method",
                                   method.function.name),
                            method.position.clone(),
                        ));
                    }
                } else {
                    self.add_error(TypeError::new(
                        format!("Override method '{}' does not override any method in base class '{}'",
                               method.function.name, base_class_name),
                        method.position.clone(),
                    ));
                }
            } else {
                self.add_error(TypeError::new(
                    format!("Base class '{}' VTable not found for override validation", base_class_name),
                    method.position.clone(),
                ));
            }
        } else {
            self.add_error(TypeError::new(
                format!("Override method '{}' cannot be used in class without base class",
                       method.function.name),
                method.position.clone(),
            ));
        }
    }

    /// Register static members in the type environment
    fn register_static_members(
        &mut self,
        class_decl: &ClassDeclaration,
        static_fields: &[(String, Type)],
        static_methods: &[(String, Type)],
    ) {
        let class_name = &class_decl.name;

        // Ensure the class namespace exists
        self.type_env.define_namespace(class_name.clone());
        if self.debug_mode {
            println!("🔧 Defined namespace: {}", class_name);
        }

        // Register static fields in the namespace (as variables)
        for (field_name, field_type) in static_fields {
            let static_field_name = format!("{}::{}", class_name, field_name);

            // Add static field to namespace variables
            if let Some(ns) = self.type_env.get_namespaces_mut().get_mut(class_name) {
                ns.variables.insert(field_name.clone(), field_type.clone());
                println!("🔧 Registered static field: {} -> {:?}", static_field_name, field_type);
            } else {
                self.add_error(TypeError::new(
                    format!("Failed to register static field '{}': Namespace '{}' not found", static_field_name, class_name),
                    class_decl.position.clone(),
                ));
            }
        }

        // Register static methods in the namespace
        for (method_name, method_type) in static_methods {
            let static_method_name = format!("{}::{}", class_name, method_name);

            // Add to namespace with access control information
            if let Err(e) = self.type_env.add_to_namespace(
                class_name,
                method_name.clone(),
                method_type.clone(),
            ) {
                self.add_error(TypeError::new(
                    format!("Failed to register static method '{}': {}", static_method_name, e),
                    class_decl.position.clone(),
                ));
            }

            println!("🔧 Registered static method: {} -> {:?}", static_method_name, method_type);
        }
    }

    /// Validate that all required trait methods are implemented
    fn validate_trait_implementation_completeness(&mut self, impl_decl: &ImplDeclaration, trait_name: &str) {
        // Clone the trait type to avoid borrowing issues
        let trait_type_opt = self.type_env.get_type(trait_name).cloned();

        if let Some(trait_type) = trait_type_opt {
            if let Type::Trait { methods, .. } = trait_type {
                // Get implemented method names
                let implemented_methods: std::collections::HashSet<String> =
                    impl_decl.methods.iter().map(|m| m.name.clone()).collect();

                // Check each required trait method
                for (method_name, method_type) in methods {
                    if !implemented_methods.contains(&method_name) {
                        // Check if this method has a default implementation in the trait
                        let has_default = self.trait_has_default_method(trait_name, &method_name);

                        if !has_default {
                            self.add_error(TypeError::new(
                                format!("Missing implementation for required trait method '{}' in trait '{}'",
                                       method_name, trait_name),
                                impl_decl.position.clone(),
                            ));
                        }
                    } else {
                        // Method is implemented, validate signature compatibility
                        if let Some(impl_method) = impl_decl.methods.iter().find(|m| m.name == method_name) {
                            self.validate_method_signature_compatibility(impl_method, &method_type, trait_name);
                        }
                    }
                }
            }
        } else {
            self.add_error(TypeError::new(
                format!("Unknown trait '{}' in impl declaration", trait_name),
                impl_decl.position.clone(),
            ));
        }
    }

    /// Check if a trait has a default implementation for a method
    fn trait_has_default_method(&self, trait_name: &str, method_name: &str) -> bool {
        // This would need to be implemented based on how trait declarations are stored
        // For now, assume no default methods
        false
    }

    /// Validate that an implemented method signature is compatible with the trait method
    fn validate_method_signature_compatibility(&mut self, impl_method: &FunctionDeclaration, trait_method_type: &Type, trait_name: &str) {
        // Extract expected signature from trait method type
        if let Type::Function { parameters, return_type, .. } = trait_method_type {
            // Check parameter count
            if impl_method.parameters.len() != parameters.len() {
                self.add_error(TypeError::new(
                    format!("Method '{}' parameter count mismatch: expected {}, found {}",
                           impl_method.name, parameters.len(), impl_method.parameters.len()),
                    impl_method.position.clone(),
                ));
                return;
            }

            // Check parameter types
            for (i, (impl_param, expected_type)) in impl_method.parameters.iter().zip(parameters.iter()).enumerate() {
                if let Some(ref impl_param_type) = impl_param.type_annotation {
                    let impl_type = self.type_env.resolve_type_annotation(impl_param_type);
                    if !impl_type.is_compatible_with_env(expected_type, Some(&self.type_env)) {
                        self.add_error(TypeError::new(
                            format!("Method '{}' parameter {} type mismatch: expected {:?}, found {:?}",
                                   impl_method.name, i, expected_type, impl_type),
                            impl_param.position.clone(),
                        ));
                    }
                }
            }

            // Check return type
            if let Some(ref impl_return_type) = impl_method.return_type {
                let impl_return = self.type_env.resolve_type_annotation(impl_return_type);
                if !impl_return.is_compatible_with_env(return_type, Some(&self.type_env)) {
                    self.add_error(TypeError::new(
                        format!("Method '{}' return type mismatch: expected {:?}, found {:?}",
                               impl_method.name, return_type, impl_return),
                        impl_method.position.clone(),
                    ));
                }
            } else if **return_type != Type::Void {
                self.add_error(TypeError::new(
                    format!("Method '{}' missing return type, expected {:?}",
                           impl_method.name, return_type),
                    impl_method.position.clone(),
                ));
            }
        }
    }

    /// Validate abstract method constraints
    fn validate_abstract_method(&mut self, method: &ClassMethod, class_decl: &ClassDeclaration) {
        // Abstract methods can only exist in abstract classes
        if !class_decl.is_abstract {
            self.add_error(TypeError::new(
                format!("Abstract method '{}' can only be declared in abstract classes", method.function.name),
                method.position.clone(),
            ));
        }

        // Abstract methods should not have a body (or should have an empty/stub body)
        if let Statement::Block { statements, .. } = &method.function.body {
            if !statements.is_empty() {
                // Check if it's just a stub implementation (e.g., panic or unimplemented)
                let is_stub = statements.len() == 1 &&
                    matches!(statements[0], Statement::Expression { .. });

                if !is_stub {
                    self.add_error(TypeError::new(
                        format!("Abstract method '{}' should not have an implementation", method.function.name),
                        method.position.clone(),
                    ));
                }
            }
        }

        // Abstract methods cannot be private (they need to be overridable)
        if method.access_modifier == crate::lang::ast::AccessModifier::Private {
            self.add_error(TypeError::new(
                format!("Abstract method '{}' cannot be private", method.function.name),
                method.position.clone(),
            ));
        }

        // Abstract methods cannot be static
        if method.is_static {
            self.add_error(TypeError::new(
                format!("Abstract method '{}' cannot be static", method.function.name),
                method.position.clone(),
            ));
        }
    }

    /// Analyze import declaration and add symbols to current scope
    fn analyze_import_declaration(&mut self, import_decl: &ImportDeclaration) {
        // Get package name from import path
        let package_name = match &import_decl.package_path {
            PackagePath::Local(name) => name.clone(),
            PackagePath::Relative(components) => components.join("/"),
            PackagePath::Absolute(components) => components.join("::"),
        };

        // Create namespace for the package if it doesn't exist
        if !self.type_env.has_namespace(&package_name) {
            self.type_env.define_namespace(package_name.clone());
        }

        // Process selective imports or import all
        if let Some(selective_imports) = &import_decl.selective_imports {
            for import_spec in selective_imports {
                match import_spec {
                    ImportSpec::Symbol(symbol) => {
                        self.import_symbol_from_package(symbol, &package_name, None);
                    }
                    ImportSpec::Aliased { name, alias } => {
                        self.import_symbol_from_package(name, &package_name, Some(alias));
                    }
                    ImportSpec::All => {
                        self.import_all_symbols_from_package(&package_name);
                    }
                }
            }
        } else {
            // Import entire package as namespace
            self.import_all_symbols_from_package(&package_name);
        }
    }

    /// Import a specific symbol from a package
    fn import_symbol_from_package(&mut self, symbol: &str, package_name: &str, alias: Option<&str>) {
        let symbol_name = alias.unwrap_or(symbol);

        // Try to get the actual function type from the package processor
        let function_type = if let Some(package_processor) = &self.package_processor {
            if let Some(actual_type) = package_processor.get_function_type(package_name, symbol) {
                actual_type
            } else {
                // Fallback to a default function type if not found
                self.add_error(TypeError::new(
                    format!("Symbol '{}' not found in package '{}'", symbol, package_name),
                    Position::new(0, 0),
                ));
                Type::Function {
                    parameters: vec![Type::Int],
                    return_type: Box::new(Type::Int),
                }
            }
        } else {
            // No package processor available, use default
            self.add_error(TypeError::new(
                format!("Package processor not available for symbol '{}'", symbol),
                Position::new(0, 0),
            ));
            Type::Function {
                parameters: vec![Type::Int],
                return_type: Box::new(Type::Int),
            }
        };

        // Add to namespace
        if let Err(e) = self.type_env.define_in_namespace(package_name, symbol_name.to_string(), function_type.clone()) {
            self.add_error(TypeError::new(
                format!("Failed to import symbol '{}' from package '{}': {}", symbol_name, package_name, e),
                Position::new(0, 0), // TODO: Use actual position from import declaration
            ));
        }
    }

    /// Import all symbols from a package
    fn import_all_symbols_from_package(&mut self, package_name: &str) {
        // Get all exported symbols from the package processor
        let symbol_names: Vec<String> = if let Some(package_processor) = &self.package_processor {
            if let Some(package) = package_processor.get_package(package_name) {
                // Collect all exported symbol names first
                package.exported_symbols.keys().cloned().collect()
            } else {
                self.add_error(TypeError::new(
                    format!("Package '{}' not found", package_name),
                    Position::new(0, 0),
                ));
                return;
            }
        } else {
            self.add_error(TypeError::new(
                format!("Package processor not available for package '{}'", package_name),
                Position::new(0, 0),
            ));
            return;
        };

        // Now import all symbols
        for symbol_name in symbol_names {
            self.import_symbol_from_package(&symbol_name, package_name, None);
        }
    }
}
