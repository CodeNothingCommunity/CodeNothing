//! Package processor for CodeNothing language
//! 
//! Handles package resolution, dependency management, and symbol exports

use crate::lang::ast::*;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Package context for tracking symbols and dependencies
#[derive(Debug, Clone)]
pub struct PackageContext {
    pub name: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub dependencies: Vec<String>,
    pub exported_symbols: HashMap<String, Declaration>,
    pub all_functions: HashMap<String, Declaration>, // All functions including private ones
    pub imported_symbols: HashMap<String, String>, // symbol -> source_package
}

impl PackageContext {
    pub fn new(name: String) -> Self {
        Self {
            name,
            version: None,
            author: None,
            dependencies: Vec::new(),
            exported_symbols: HashMap::new(),
            all_functions: HashMap::new(),
            imported_symbols: HashMap::new(),
        }
    }

    pub fn add_export(&mut self, symbol_name: String, declaration: Declaration) {
        self.exported_symbols.insert(symbol_name, declaration);
    }

    pub fn add_function(&mut self, function_name: String, declaration: Declaration) {
        self.all_functions.insert(function_name, declaration);
    }

    pub fn add_import(&mut self, symbol_name: String, source_package: String) {
        self.imported_symbols.insert(symbol_name, source_package);
    }

    pub fn has_symbol(&self, symbol_name: &str) -> bool {
        self.exported_symbols.contains_key(symbol_name)
    }
}

/// Package processor for handling package system
#[derive(Clone)]
pub struct PackageProcessor {
    packages: HashMap<String, PackageContext>,
    search_paths: Vec<PathBuf>,
    processed_files: HashSet<PathBuf>,
    dependency_stack: Vec<String>, // For circular dependency detection
    current_package: Option<String>, // Track the current package being processed
}

impl PackageProcessor {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
            search_paths: vec![
                PathBuf::from("./"),
                PathBuf::from("./packages/"),
                PathBuf::from("../packages/"),
                PathBuf::from("packages/"),
                PathBuf::from("packages"),
            ],
            processed_files: HashSet::new(),
            dependency_stack: Vec::new(),
            current_package: None,
        }
    }

    /// Process a program with package declarations
    pub fn process_program(&mut self, program: &mut Program) -> Result<(), String> {
        // Check if this is a package file (has package declaration but no main function)
        let has_package_decl = program.declarations.iter().any(|d| matches!(d, Declaration::Package(_)));
        let has_main_function = program.declarations.iter().any(|d| {
            if let Declaration::Function(func) = d {
                func.name == "main"
            } else {
                false
            }
        });

        // If this is a package file without main function, add a dummy main to prevent runtime errors
        if has_package_decl && !has_main_function {
            self.add_dummy_main(program);
        }

        // First pass: collect package declarations and set current package
        for declaration in &program.declarations {
            if let Declaration::Package(package_decl) = declaration {
                self.register_package(package_decl)?;
                // Set this as the current package being processed
                self.current_package = Some(package_decl.name.clone());
            }
        }

        // Second pass: collect all functions (including private ones)
        for declaration in &program.declarations {
            if let Declaration::Function(func_decl) = declaration {
                if let Some(current_package) = self.get_current_package_mut() {
                    current_package.add_function(func_decl.name.clone(), declaration.clone());
                }
            }
        }

        // Third pass: process imports and exports
        for declaration in &mut program.declarations {
            match declaration {
                Declaration::Import(import_decl) => {
                    self.process_import(import_decl)?;
                }
                Declaration::Export(export_decl) => {
                    self.process_export(export_decl)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Add a dummy main function to prevent runtime errors in package files
    fn add_dummy_main(&self, program: &mut Program) {
        use crate::lang::ast::{FunctionDeclaration, TypeAnnotation, Statement, Expression, Position};

        // Create a dummy position
        let pos = Position::new(0, 0);

        // Create a dummy main function that returns 0
        let dummy_main = FunctionDeclaration {
            name: "main".to_string(),
            parameters: vec![],
            return_type: Some(TypeAnnotation::Int),
            body: Statement::Block {
                statements: vec![
                    Statement::Return {
                        value: Some(Expression::Integer(0, pos.clone())),
                        position: pos.clone(),
                    }
                ],
                position: pos.clone(),
            },
            position: pos,
        };

        program.declarations.push(Declaration::Function(dummy_main));
    }

    /// Register a package declaration
    fn register_package(&mut self, package_decl: &PackageDeclaration) -> Result<(), String> {
        let mut context = PackageContext::new(package_decl.name.clone());
        context.version = package_decl.version.clone();
        context.author = package_decl.author.clone();
        context.dependencies = package_decl.dependencies.clone();

        self.packages.insert(package_decl.name.clone(), context);
        Ok(())
    }

    /// Process an import declaration
    fn process_import(&mut self, import_decl: &ImportDeclaration) -> Result<(), String> {
        let package_name = self.resolve_package_path(&import_decl.package_path)?;
        
        // Check for circular dependencies
        if self.dependency_stack.contains(&package_name) {
            return Err(format!("Circular dependency detected: {:?} -> {}", 
                self.dependency_stack, package_name));
        }

        // Load package if not already loaded
        if !self.packages.contains_key(&package_name) {
            self.load_package(&package_name)?;
        }

        // Process selective imports
        if let Some(selective_imports) = &import_decl.selective_imports {
            for import_spec in selective_imports {
                match import_spec {
                    ImportSpec::Symbol(symbol) => {
                        self.import_symbol(symbol, &package_name)?;
                    }
                    ImportSpec::Aliased { name, alias } => {
                        self.import_symbol_with_alias(name, alias, &package_name)?;
                    }
                    ImportSpec::All => {
                        self.import_all_symbols(&package_name)?;
                    }
                }
            }
        } else {
            // Import entire package
            self.import_all_symbols(&package_name)?;
        }

        Ok(())
    }

    /// Process an export declaration
    fn process_export(&mut self, export_decl: &ExportDeclaration) -> Result<(), String> {
        // Extract symbol name from declaration
        let symbol_name = self.extract_symbol_name(&export_decl.declaration)?;
        
        // Add to current package's exports
        if let Some(current_package) = self.get_current_package_mut() {
            current_package.add_export(symbol_name, *export_decl.declaration.clone());
        }

        Ok(())
    }

    /// Resolve package path to package name
    fn resolve_package_path(&self, package_path: &PackagePath) -> Result<String, String> {
        match package_path {
            PackagePath::Absolute(components) => {
                Ok(components.join("::"))
            }
            PackagePath::Relative(components) => {
                Ok(components.join("/"))
            }
            PackagePath::Local(name) => {
                Ok(name.clone())
            }
        }
    }

    /// Load a package from file system
    fn load_package(&mut self, package_name: &str) -> Result<(), String> {
        let package_file = self.find_package_file(package_name)?;

        if self.processed_files.contains(&package_file) {
            return Ok(()); // Already processed
        }

        self.dependency_stack.push(package_name.to_string());

        // Read and parse package file
        let content = std::fs::read_to_string(&package_file)
            .map_err(|e| format!("Failed to read package file '{}': {}", package_file.display(), e))?;

        let mut lexer = crate::lang::lexer::Lexer::new(&content);
        let tokens = lexer.tokenize()
            .map_err(|e| format!("Failed to tokenize package file '{}': {}", package_file.display(), e))?;

        let mut parser = crate::lang::parser::Parser::new(tokens);
        let mut program = parser.parse_program()
            .map_err(|e| format!("Failed to parse package file '{}': {}", package_file.display(), e))?;

        // Process the package program
        let mut context = PackageContext::new(package_name.to_string());

        // Extract exports and all functions from the parsed program
        for declaration in &program.declarations {
            match declaration {
                crate::lang::ast::Declaration::Export(export_decl) => {
                    let symbol_name = self.extract_symbol_name(&export_decl.declaration)?;
                    context.add_export(symbol_name.clone(), *export_decl.declaration.clone());
                    // Also add to all_functions for code generation
                    context.add_function(symbol_name, *export_decl.declaration.clone());
                }
                crate::lang::ast::Declaration::Function(func_decl) => {
                    // Add private functions to all_functions
                    context.add_function(func_decl.name.clone(), crate::lang::ast::Declaration::Function(func_decl.clone()));
                }
                _ => {}
            }
        }

        self.packages.insert(package_name.to_string(), context);
        self.processed_files.insert(package_file);
        self.dependency_stack.pop();

        Ok(())
    }

    /// Find package file in search paths
    fn find_package_file(&self, package_name: &str) -> Result<PathBuf, String> {
        let possible_names = vec![
            format!("{}.cn", package_name),
            format!("{}/mod.cn", package_name),
            format!("{}/package.cn", package_name),
        ];

        for search_path in &self.search_paths {
            for name in &possible_names {
                let full_path = search_path.join(name);
                if full_path.exists() {
                    return Ok(full_path);
                }
            }
        }

        Err(format!("Package '{}' not found in search paths", package_name))
    }

    /// Import a symbol from a package
    fn import_symbol(&mut self, symbol: &str, package_name: &str) -> Result<(), String> {
        if let Some(package) = self.packages.get(package_name) {
            if !package.has_symbol(symbol) {
                return Err(format!("Symbol '{}' not found in package '{}'", symbol, package_name));
            }
        }

        if let Some(current_package) = self.get_current_package_mut() {
            current_package.add_import(symbol.to_string(), package_name.to_string());
        }

        Ok(())
    }

    /// Import a symbol with alias
    fn import_symbol_with_alias(&mut self, symbol: &str, alias: &str, package_name: &str) -> Result<(), String> {
        if let Some(package) = self.packages.get(package_name) {
            if !package.has_symbol(symbol) {
                return Err(format!("Symbol '{}' not found in package '{}'", symbol, package_name));
            }
        }

        if let Some(current_package) = self.get_current_package_mut() {
            current_package.add_import(alias.to_string(), package_name.to_string());
        }

        Ok(())
    }

    /// Import all symbols from a package
    fn import_all_symbols(&mut self, package_name: &str) -> Result<(), String> {
        if let Some(package) = self.packages.get(package_name) {
            let symbols: Vec<String> = package.exported_symbols.keys().cloned().collect();
            
            for symbol in symbols {
                if let Some(current_package) = self.get_current_package_mut() {
                    current_package.add_import(symbol, package_name.to_string());
                }
            }
        }

        Ok(())
    }

    /// Extract symbol name from declaration
    fn extract_symbol_name(&self, declaration: &Declaration) -> Result<String, String> {
        match declaration {
            Declaration::Function(func) => Ok(func.name.clone()),
            Declaration::Variable(stmt) => {
                // Extract variable name from variable declaration statement
                match stmt {
                    Statement::VarDeclaration { name, .. } => Ok(name.clone()),
                    _ => Err("Invalid variable declaration statement".to_string()),
                }
            }
            Declaration::Struct(struct_decl) => Ok(struct_decl.name.clone()),
            Declaration::Class(class_decl) => Ok(class_decl.name.clone()),
            Declaration::Enum(enum_decl) => Ok(enum_decl.name.clone()),
            _ => Err("Cannot extract symbol name from this declaration type".to_string()),
        }
    }

    /// Get current package context (mutable)
    fn get_current_package_mut(&mut self) -> Option<&mut PackageContext> {
        if let Some(current_package_name) = &self.current_package {
            self.packages.get_mut(current_package_name)
        } else {
            // Fallback to first package if no current package is set
            self.packages.values_mut().next()
        }
    }

    /// Get current package name
    pub fn get_current_package_name(&self) -> Option<&String> {
        self.current_package.as_ref()
    }

    /// Set current package
    pub fn set_current_package(&mut self, package_name: Option<String>) {
        self.current_package = package_name;
    }

    /// Add search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// Set search paths
    pub fn set_search_paths(&mut self, paths: Vec<PathBuf>) {
        self.search_paths = paths;
    }

    /// Get package information
    pub fn get_package(&self, name: &str) -> Option<&PackageContext> {
        self.packages.get(name)
    }

    /// Get all packages
    pub fn get_all_packages(&self) -> &HashMap<String, PackageContext> {
        &self.packages
    }

    /// Get symbol type from package (functions, constants, etc.)
    pub fn get_function_type(&self, package_name: &str, symbol_name: &str) -> Option<crate::lang::types::Type> {
        if let Some(package) = self.packages.get(package_name) {
            if let Some(declaration) = package.exported_symbols.get(symbol_name) {
                match declaration {
                    Declaration::Function(func_decl) => {
                        return Some(self.convert_function_to_type(func_decl));
                    }
                    Declaration::Variable(stmt) => {
                        // Handle constant declarations
                        return Some(self.convert_variable_to_type(stmt));
                    }
                    _ => {
                        // Other declaration types not yet supported
                        return None;
                    }
                }
            }
        }
        None
    }

    /// Convert function declaration to Type
    fn convert_function_to_type(&self, func_decl: &crate::lang::ast::FunctionDeclaration) -> crate::lang::types::Type {
        use crate::lang::types::Type;
        use crate::lang::ast::TypeAnnotation;

        let mut parameters = Vec::new();
        for param in &func_decl.parameters {
            let param_type = match &param.type_annotation {
                Some(TypeAnnotation::Int) => Type::Int,
                Some(TypeAnnotation::Float(_)) => Type::Float(64),
                Some(TypeAnnotation::String) => Type::String,
                Some(TypeAnnotation::Bool) => Type::Bool,
                Some(TypeAnnotation::Custom(name)) => Type::Custom(name.clone()),
                None => Type::Int, // Default to int if no type annotation
                _ => Type::Int, // Fallback
            };
            parameters.push(param_type);
        }

        let return_type = match &func_decl.return_type {
            Some(TypeAnnotation::Int) => Type::Int,
            Some(TypeAnnotation::Float(_)) => Type::Float(64),
            Some(TypeAnnotation::String) => Type::String,
            Some(TypeAnnotation::Bool) => Type::Bool,
            Some(TypeAnnotation::Custom(name)) => Type::Custom(name.clone()),
            None => Type::Void,
            _ => Type::Void,
        };

        Type::Function {
            parameters,
            return_type: Box::new(return_type),
        }
    }

    /// Convert variable declaration to Type
    fn convert_variable_to_type(&self, stmt: &crate::lang::ast::Statement) -> crate::lang::types::Type {
        use crate::lang::types::Type;
        use crate::lang::ast::{TypeAnnotation, Statement};

        match stmt {
            Statement::VarDeclaration { type_annotation, .. } => {
                match type_annotation {
                    Some(TypeAnnotation::Int) => Type::Int,
                    Some(TypeAnnotation::Float(_)) => Type::Float(64),
                    Some(TypeAnnotation::String) => Type::String,
                    Some(TypeAnnotation::Bool) => Type::Bool,
                    Some(TypeAnnotation::Custom(name)) => Type::Custom(name.clone()),
                    None => Type::Int, // Default to int if no type annotation
                    _ => Type::Int, // Fallback
                }
            }
            _ => Type::Int, // Fallback for other statement types
        }
    }
}
