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
            Declaration::Package(pkg) => {
                self.analyze_package_items(pkg, &[]);
            }
            Declaration::Memo(memo) => {
                self.analyze_memo_declaration(memo);
            }
            Declaration::Using(using) => {
                // Import namespace functions into current scope
                self.analyze_using_declaration(using);
            }
            Declaration::Import(import) => {
                // Import external package functions into current scope
                self.analyze_import_declaration(import);
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

    /// Analyze package items recursively
    fn analyze_package_items(&mut self, pkg: &PackageDeclaration, parent_path: &[String]) {
        let mut current_path = parent_path.to_vec();
        current_path.push(pkg.name.clone());

        // Set package context for top-level packages
        let old_package = if parent_path.is_empty() {
            // This is a top-level package, set it as the current package
            let old_package = self.access_checker.get_package_context().clone();
            self.access_checker.set_package_context(Some(pkg.name.clone()));
            old_package
        } else {
            None
        };

        // Enter package context
        self.access_checker.get_context_mut().enter_namespace(pkg.name.clone());

        for item in &pkg.items {
            match item {
                PackageItem::Function(func) => {
                    self.analyze_nested_package_function(&current_path, func);
                }
                PackageItem::Namespace(nested_ns) => {
                    self.analyze_namespace_items(nested_ns, &current_path);
                }
                PackageItem::Package(nested_pkg) => {
                    self.analyze_package_items(nested_pkg, &current_path);
                }
                PackageItem::Variable(stmt) => {
                    // Handle package variable declarations
                    self.analyze_package_variable(&current_path, stmt);
                }
                PackageItem::Enum(enum_decl) => {
                    self.analyze_enum_declaration(enum_decl);
                }
                PackageItem::Struct(struct_decl) => {
                    self.analyze_struct_declaration(struct_decl);
                }
                PackageItem::Trait(trait_decl) => {
                    self.analyze_trait_declaration(trait_decl);
                }
                PackageItem::Impl(impl_decl) => {
                    self.analyze_impl_declaration(impl_decl);
                }
                PackageItem::Class(class_decl) => {
                    self.analyze_class_declaration(class_decl);
                }
            }
        }

        // Exit package context
        self.access_checker.get_context_mut().exit_namespace();

        // Restore previous package context for top-level packages
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
            let param_type = self.resolve_type_annotation(&param.type_annotation);
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

    /// Analyze nested package function declaration
    fn analyze_nested_package_function(&mut self, package_path: &[String], function: &FunctionDeclaration) {
        // Enter function scope
        self.type_env.enter_scope();

        // Import all functions and variables from the current package and parent packages
        // This allows functions to call each other within the same package and access package variables
        for i in 1..=package_path.len() {
            let current_package_path = &package_path[0..i];
            let package_name = current_package_path.join("::");

            let current_path: Vec<String> = current_package_path.iter().map(|s| s.clone()).collect();

            // Import functions
            let functions_to_import: Vec<(String, Type)> = if let Some(package_functions) = self.type_env.get_namespace_functions(&current_path) {
                package_functions.iter()
                    .filter(|(name, _)| self.type_env.lookup_function(name).is_none())
                    .map(|(name, func_type)| (name.clone(), func_type.clone()))
                    .collect()
            } else {
                Vec::new()
            };

            for (name, func_type) in functions_to_import {
                if let Err(e) = self.type_env.define_function(name.clone(), func_type) {
                    self.add_error(TypeError::new(
                        format!("Failed to import function '{}' from package '{}': {}", name, package_name, e),
                        function.position.clone(),
                    ));
                }
            }

            // Import variables
            let variables_to_import: Vec<(String, Type)> = if let Some(package_variables) = self.type_env.get_namespace_variables(&current_path) {
                package_variables.iter()
                    .filter(|(name, _)| self.type_env.lookup_variable(name).is_none())
                    .map(|(name, var_type)| (name.clone(), var_type.clone()))
                    .collect()
            } else {
                Vec::new()
            };

            for (name, var_type) in variables_to_import {
                if let Err(e) = self.type_env.define_variable(name.clone(), var_type) {
                    self.add_error(TypeError::new(
                        format!("Failed to import variable '{}' from package '{}': {}", name, package_name, e),
                        function.position.clone(),
                    ));
                }
            }
        }

        // Set current function return type for return statement analysis
        self.current_function_return_type = function.return_type.as_ref()
            .map(|rt| self.resolve_type_annotation(rt));

        // Analyze function parameters
        for param in &function.parameters {
            let param_type = self.resolve_type_annotation(&param.type_annotation);

            if let Err(e) = self.type_env.define_variable(param.name.clone(), param_type) {
                self.add_error(TypeError::new(
                    format!("Failed to define parameter '{}': {}", param.name, e),
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
            let param_type = self.resolve_type_annotation(&param.type_annotation);
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
            let param_type = self.resolve_type_annotation(&param.type_annotation);
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

    /// Analyze import declaration for external packages
    pub(super) fn analyze_import_declaration(&mut self, import: &ImportDeclaration) {
        if self.debug_mode {
            println!("🔧 Semantic: Processing import declaration: {}", import.module_path);
        }

        // Try to load the external module file
        match self.load_external_module(&import.module_path) {
            Ok(module_functions) => {
                if self.debug_mode {
                    println!("🔧 Semantic: Successfully loaded module '{}' with {} functions",
                             import.module_path, module_functions.len());
                }

                // Import functions based on import specification
                let functions_to_import = if let Some(ref items) = import.items {
                    // Import only specified items
                    module_functions.into_iter()
                        .filter(|(name, _)| items.contains(name))
                        .collect()
                } else {
                    // Import all functions (wildcard import)
                    module_functions
                };

                // Apply alias if specified and import functions to correct namespaces
                for (func_name, func_type) in functions_to_import {
                    if let Some(ref alias) = import.alias {
                        // With alias: import to nested namespace structure
                        if func_name.contains("::") {
                            // Split package::function into namespace and function name
                            let parts: Vec<&str> = func_name.split("::").collect();
                            if parts.len() == 2 {
                                let namespace = parts[0];
                                let function_name = parts[1];

                                // Create nested namespace: alias::namespace::function
                                let full_namespace = format!("{}::{}", alias, namespace);

                                if self.debug_mode {
                                    println!("🔧 Semantic: Importing function '{}' to nested namespace '{}' as '{}' with type {:?}",
                                             func_name, full_namespace, function_name, func_type);
                                }

                                // Define the nested namespace if it doesn't exist
                                self.type_env.define_namespace(full_namespace.clone());

                                // Import function to the nested namespace
                                match self.type_env.define_function_in_namespace(&full_namespace, function_name.to_string(), func_type) {
                                    Ok(()) => {
                                        if self.debug_mode {
                                            println!("🔧 Semantic: Successfully defined function '{}' in namespace '{}'", function_name, full_namespace);
                                        }
                                    },
                                    Err(e) => {
                                        if self.debug_mode {
                                            println!("🔧 Semantic: Failed to define function '{}' in namespace '{}': {}", function_name, full_namespace, e);
                                        }
                                    }
                                }
                            } else {
                                // Fallback: import as alias::function
                                let final_name = format!("{}::{}", alias, func_name);
                                if self.debug_mode {
                                    println!("🔧 Semantic: Importing function '{}' as '{}' with type {:?}",
                                             func_name, final_name, func_type);
                                }
                                let _ = self.type_env.define_function(final_name, func_type);
                            }
                        } else {
                            // Simple function name: import as alias::function
                            let final_name = format!("{}::{}", alias, func_name);
                            if self.debug_mode {
                                println!("🔧 Semantic: Importing function '{}' as '{}' with type {:?}",
                                         func_name, final_name, func_type);
                            }
                            let _ = self.type_env.define_function(final_name, func_type);
                        }
                    } else {
                        // Without alias: import directly to the package namespace
                        if func_name.contains("::") {
                            // Split package::function into namespace and function name
                            let parts: Vec<&str> = func_name.split("::").collect();
                            if parts.len() == 2 {
                                let namespace = parts[0];
                                let function_name = parts[1];

                                if self.debug_mode {
                                    println!("🔧 Semantic: Importing function '{}' to namespace '{}' as '{}' with type {:?}",
                                             func_name, namespace, function_name, func_type);
                                }

                                let _ = self.type_env.define_function_in_namespace(namespace, function_name.to_string(), func_type);
                            } else {
                                // Fallback: import as global function
                                if self.debug_mode {
                                    println!("🔧 Semantic: Importing function '{}' as global function with type {:?}",
                                             func_name, func_type);
                                }
                                let _ = self.type_env.define_function(func_name.clone(), func_type);
                            }
                        } else {
                            // Simple function name: import as global function
                            if self.debug_mode {
                                println!("🔧 Semantic: Importing function '{}' as global function with type {:?}",
                                         func_name, func_type);
                            }
                            let _ = self.type_env.define_function(func_name.clone(), func_type);
                        }
                    }
                }
            }
            Err(e) => {
                if self.debug_mode {
                    println!("🔧 Semantic: Error loading module '{}': {}", import.module_path, e);
                }
                self.add_error(TypeError::new(
                    format!("Failed to import module '{}': {}", import.module_path, e),
                    import.position.clone(),
                ));
            }
        }
    }

    /// Load external module and extract its package functions
    fn load_external_module(&mut self, module_path: &str) -> Result<Vec<(String, Type)>, String> {
        use std::path::Path;
        use std::fs;

        // Resolve module path to file path
        let file_path = if module_path.starts_with("./") || module_path.starts_with("../") {
            // Relative path
            format!("{}.cn", module_path)
        } else {
            // Absolute module path - look in standard locations
            // First try current directory
            let current_dir_path = format!("{}.cn", module_path.replace(".", "/"));
            if Path::new(&current_dir_path).exists() {
                current_dir_path
            } else {
                // Try with dots as directory separators
                format!("{}.cn", module_path.replace(".", "/"))
            }
        };

        if self.debug_mode {
            println!("🔧 Semantic: Trying to load module file: {}", file_path);
        }

        // Read and parse the external file
        let source_code = fs::read_to_string(&file_path)
            .map_err(|e| format!("Cannot read module file '{}': {}", file_path, e))?;

        // Parse the external module
        let mut lexer = crate::lang::lexer::Lexer::new(&source_code);
        let tokens = lexer.tokenize()
            .map_err(|e| format!("Lexer error in module '{}': {}", file_path, e))?;

        let mut parser = crate::lang::parser::Parser::new(tokens);
        let program = parser.parse_program()
            .map_err(|e| format!("Parser error in module '{}': {}", file_path, e))?;

        // Store the parsed program for later compilation
        if self.debug_mode {
            println!("🔧 Semantic: Storing external module '{}' with {} declarations", module_path, program.declarations.len());
        }
        self.external_modules.insert(module_path.to_string(), program.clone());

        if self.debug_mode {
            println!("🔧 Semantic: Total external modules stored: {}", self.external_modules.len());
        }

        // Create a temporary semantic analyzer for the external module
        let mut temp_analyzer = crate::lang::semantic::analyzer::SemanticAnalyzer::new();
        temp_analyzer.set_debug_mode(self.debug_mode);

        // Analyze the external module
        if let Err(errors) = temp_analyzer.analyze_program(&program) {
            let error_messages: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
            return Err(format!("Semantic errors in module '{}': {}", file_path, error_messages.join(", ")));
        }

        // Extract all package functions from the external module
        let mut module_functions = Vec::new();

        for declaration in &program.declarations {
            if let Declaration::Package(pkg) = declaration {
                // Get functions from this package
                let package_path = vec![pkg.name.clone()];
                if let Some(package_functions) = temp_analyzer.type_env.get_namespace_functions(&package_path) {
                    for (func_name, func_type) in package_functions {
                        // Add package prefix to function name
                        let qualified_name = format!("{}::{}", pkg.name, func_name);
                        module_functions.push((qualified_name, func_type.clone()));
                    }
                }
            }
        }

        if self.debug_mode {
            println!("🔧 Semantic: Extracted {} functions from module '{}'",
                     module_functions.len(), file_path);
        }

        Ok(module_functions)
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

    /// Analyze package variable declaration
    fn analyze_package_variable(&mut self, package_path: &[String], stmt: &Statement) {
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

            // Create package path string
            let package_str = package_path.join("::");

            // Define the variable in the package (treated as namespace)
            if let Err(e) = self.type_env.define_variable_in_namespace(&package_str, name.clone(), var_type) {
                self.add_error(TypeError::new(
                    format!("Failed to define package variable '{}::{}': {}", package_str, name, e),
                    stmt.position().clone(),
                ));
            }

            // Analyze initializer if present
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

        // Replace the placeholder struct type with the complete one
        self.type_env.types.insert(struct_decl.name.clone(), struct_type);
    }

    /// Analyze trait declaration
    fn analyze_trait_declaration(&mut self, trait_decl: &TraitDeclaration) {
        // Collect method signatures
        let mut method_signatures = Vec::new();
        for method in &trait_decl.methods {
            let param_types: Vec<Type> = method.parameters.iter()
                .map(|p| self.resolve_type_annotation(&p.type_annotation))
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
                    let param_type = self.resolve_type_annotation(&param.type_annotation);
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

        // Replace the placeholder trait type with the complete one
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

            // TODO: Check that all required trait methods are implemented
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
                    .map(|p| self.type_env.resolve_type_annotation(&p.type_annotation))
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
                let param_type = self.type_env.resolve_type_annotation(&param.type_annotation);
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
                    self.type_env.resolve_type_annotation(&param.type_annotation)
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
                // For abstract methods, we could check if body is empty or contains only a placeholder
                // For now, we'll allow any body for abstract methods
                // TODO: Implement proper abstract method validation
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
                        .map(|p| self.type_env.resolve_type_annotation(&p.type_annotation))
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
                        .map(|p| self.type_env.resolve_type_annotation(&p.type_annotation))
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
}
