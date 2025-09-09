//! Include processor for CodeNothing language
//! 
//! Handles file inclusion, dependency resolution, and circular dependency detection

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::fs;
use crate::lang::ast::{Declaration, IncludeDeclaration, IncludePath, ImportSpec, Program, Position, Statement};
use crate::lang::lexer::Lexer;
use crate::lang::parser::Parser;

/// Include processing errors
#[derive(Debug, Clone)]
pub struct IncludeError {
    pub message: String,
    pub position: Position,
    pub file_path: Option<String>,
}

impl IncludeError {
    pub fn new(message: String, position: Position) -> Self {
        Self {
            message,
            position,
            file_path: None,
        }
    }

    pub fn with_file(message: String, position: Position, file_path: String) -> Self {
        Self {
            message,
            position,
            file_path: Some(file_path),
        }
    }
}

impl std::fmt::Display for IncludeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(file) = &self.file_path {
            write!(f, "Include error in {}: {} at {}:{}", 
                   file, self.message, self.position.line, self.position.column)
        } else {
            write!(f, "Include error: {} at {}:{}", 
                   self.message, self.position.line, self.position.column)
        }
    }
}

impl std::error::Error for IncludeError {}

/// Include context for tracking dependencies
#[derive(Debug)]
pub struct IncludeContext {
    /// Current file being processed
    pub current_file: PathBuf,
    /// Stack of files being processed (for circular dependency detection)
    pub file_stack: Vec<PathBuf>,
    /// Cache of already processed files
    pub processed_files: HashMap<PathBuf, Program>,
    /// Symbol table for included symbols
    pub included_symbols: HashMap<String, String>, // symbol_name -> source_file
}

impl IncludeContext {
    pub fn new(main_file: PathBuf) -> Self {
        Self {
            current_file: main_file,
            file_stack: Vec::new(),
            processed_files: HashMap::new(),
            included_symbols: HashMap::new(),
        }
    }

    /// Check if a file is currently being processed (circular dependency)
    pub fn is_circular_dependency(&self, file_path: &Path) -> bool {
        self.file_stack.contains(&file_path.to_path_buf())
    }

    /// Push a file onto the processing stack
    pub fn push_file(&mut self, file_path: PathBuf) {
        self.file_stack.push(file_path.clone());
        self.current_file = file_path;
    }

    /// Pop a file from the processing stack
    pub fn pop_file(&mut self) {
        self.file_stack.pop();
        if let Some(parent) = self.file_stack.last() {
            self.current_file = parent.clone();
        }
    }

    /// Add a symbol to the included symbols table
    pub fn add_symbol(&mut self, symbol_name: String, source_file: String) {
        self.included_symbols.insert(symbol_name, source_file);
    }

    /// Check if a symbol is already included
    pub fn has_symbol(&self, symbol_name: &str) -> bool {
        self.included_symbols.contains_key(symbol_name)
    }
}

/// Include processor
pub struct IncludeProcessor {
    /// Base directory for resolving relative paths
    pub base_dir: PathBuf,
    /// Search paths for resolving search includes like <std/io>
    pub search_paths: Vec<PathBuf>,
}

impl IncludeProcessor {
    pub fn new(base_dir: PathBuf) -> Self {
        let mut search_paths = Vec::new();

        // Add default search paths
        search_paths.push(base_dir.join("std")); // Standard library
        search_paths.push(base_dir.join("lib")); // Local libraries

        // Add system-wide search paths
        if let Ok(home) = std::env::var("HOME") {
            search_paths.push(PathBuf::from(home).join(".codenothing/lib"));
        }

        Self {
            base_dir,
            search_paths,
        }
    }

    /// Add a custom search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// Set search paths
    pub fn set_search_paths(&mut self, paths: Vec<PathBuf>) {
        self.search_paths = paths;
    }

    /// Process all includes in a program
    pub fn process_program(&mut self, program: Program, main_file: PathBuf) -> Result<Program, IncludeError> {
        let mut context = IncludeContext::new(main_file);
        self.process_program_with_context(program, &mut context)
    }

    /// Process program with existing context
    fn process_program_with_context(&mut self, mut program: Program, context: &mut IncludeContext) -> Result<Program, IncludeError> {
        let mut processed_declarations = Vec::new();
        
        for declaration in program.declarations {
            match declaration {
                Declaration::Include(include_decl) => {
                    // Process include declaration
                    let included_declarations = self.process_include(&include_decl, context)?;
                    processed_declarations.extend(included_declarations);
                }
                other => {
                    // Keep other declarations as-is
                    processed_declarations.push(other);
                }
            }
        }
        
        program.declarations = processed_declarations;
        Ok(program)
    }

    /// Process a single include declaration
    fn process_include(&mut self, include_decl: &IncludeDeclaration, context: &mut IncludeContext) -> Result<Vec<Declaration>, IncludeError> {
        // Resolve file paths (may be multiple for wildcards)
        let file_paths = self.resolve_include_path(&include_decl.path, &context.current_file)?;

        let mut all_declarations = Vec::new();

        for file_path in file_paths {
            // Check for circular dependency
            if context.is_circular_dependency(&file_path) {
                return Err(IncludeError::with_file(
                    format!("Circular dependency detected: {}", file_path.display()),
                    include_decl.position.clone(),
                    context.current_file.to_string_lossy().to_string(),
                ));
            }

            // Check if file is already processed
            let processed_program = if let Some(cached_program) = context.processed_files.get(&file_path).cloned() {
                cached_program
            } else {
                // Read and parse the included file
                let included_program = self.parse_file(&file_path, include_decl)?;

                // Process includes in the included file
                context.push_file(file_path.clone());
                let processed_program = self.process_program_with_context(included_program, context)?;
                context.pop_file();

                // Cache the processed program
                context.processed_files.insert(file_path.clone(), processed_program.clone());
                processed_program
            };

            // Extract symbols from this file
            let declarations = self.extract_symbols(&processed_program, include_decl, context)?;
            all_declarations.extend(declarations);
        }

        Ok(all_declarations)
    }

    /// Resolve include path and return list of matching files
    fn resolve_include_path(&self, include_path: &IncludePath, current_file: &Path) -> Result<Vec<PathBuf>, IncludeError> {
        match include_path {
            IncludePath::Simple(path) => {
                let resolved = self.resolve_simple_path(path, current_file)?;
                Ok(vec![resolved])
            }
            IncludePath::Wildcard(pattern) => {
                self.resolve_wildcard_path(pattern, current_file)
            }
            IncludePath::Search(name) => {
                let resolved = self.resolve_search_path(name)?;
                Ok(vec![resolved])
            }
        }
    }

    /// Resolve simple include path relative to current file
    fn resolve_simple_path(&self, include_path: &str, current_file: &Path) -> Result<PathBuf, IncludeError> {
        let path = Path::new(include_path);
        
        // If absolute path, use as-is
        if path.is_absolute() {
            return Ok(path.to_path_buf());
        }
        
        // Resolve relative to current file's directory
        let current_dir = current_file.parent().unwrap_or(&self.base_dir);
        let resolved = current_dir.join(path);
        
        // Canonicalize to resolve .. and . components
        resolved.canonicalize().map_err(|e| {
            IncludeError::new(
                format!("Failed to resolve path '{}': {}", include_path, e),
                Position::new(0, 0),
            )
        })
    }

    /// Resolve wildcard path pattern
    fn resolve_wildcard_path(&self, pattern: &str, current_file: &Path) -> Result<Vec<PathBuf>, IncludeError> {
        let current_dir = current_file.parent().unwrap_or(&self.base_dir);

        // Use base_dir if current_dir is empty or doesn't exist
        let search_dir = if current_dir.as_os_str().is_empty() || !current_dir.exists() {
            &self.base_dir
        } else {
            current_dir
        };

        let mut matching_files = Vec::new();



        if pattern.contains("**") {
            // Recursive wildcard: **/*.cn
            matching_files.extend(self.find_files_recursive(search_dir, pattern)?);
        } else if pattern.contains('*') {
            // Simple wildcard: *.cn or utils/*.cn
            matching_files.extend(self.find_files_simple(search_dir, pattern)?);
        }

        if matching_files.is_empty() {
            return Err(IncludeError::new(
                format!("No files found matching pattern '{}' in directory '{}'", pattern, search_dir.display()),
                Position::new(0, 0),
            ));
        }

        Ok(matching_files)
    }

    /// Resolve search path like <std/io>
    fn resolve_search_path(&self, name: &str) -> Result<PathBuf, IncludeError> {
        // Try each search path
        for search_path in &self.search_paths {
            let candidate = search_path.join(format!("{}.cn", name));
            if candidate.exists() {
                return Ok(candidate);
            }

            // Also try with directory structure: <std/io> -> std/io.cn or std/io/mod.cn
            let dir_candidate = search_path.join(name).join("mod.cn");
            if dir_candidate.exists() {
                return Ok(dir_candidate);
            }
        }

        Err(IncludeError::new(
            format!("Search path '{}' not found in any search directories", name),
            Position::new(0, 0),
        ))
    }

    /// Find files matching simple wildcard pattern
    fn find_files_simple(&self, search_dir: &Path, pattern: &str) -> Result<Vec<PathBuf>, IncludeError> {
        let mut files = Vec::new();



        if let Some(dir_path) = pattern.rfind('/') {
            // Pattern has directory: utils/*.cn
            let dir_part = &pattern[..dir_path];
            let file_part = &pattern[dir_path + 1..];
            let target_dir = search_dir.join(dir_part);



            if target_dir.is_dir() {
                files.extend(self.match_files_in_dir(&target_dir, file_part)?);
            }
        } else {
            // Pattern is just filename: *.cn

            files.extend(self.match_files_in_dir(search_dir, pattern)?);
        }


        Ok(files)
    }

    /// Find files matching recursive wildcard pattern
    fn find_files_recursive(&self, base_dir: &Path, pattern: &str) -> Result<Vec<PathBuf>, IncludeError> {
        let mut files = Vec::new();

        // Extract the file pattern after **
        let file_pattern = if let Some(pos) = pattern.find("**/") {
            &pattern[pos + 3..]
        } else {
            pattern.trim_start_matches("**")
        };

        self.walk_directory_recursive(base_dir, file_pattern, &mut files)?;
        Ok(files)
    }

    /// Walk directory recursively to find matching files
    fn walk_directory_recursive(&self, dir: &Path, pattern: &str, files: &mut Vec<PathBuf>) -> Result<(), IncludeError> {
        if !dir.is_dir() {
            return Ok(());
        }

        let entries = fs::read_dir(dir).map_err(|e| {
            IncludeError::new(
                format!("Failed to read directory '{}': {}", dir.display(), e),
                Position::new(0, 0),
            )
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                IncludeError::new(
                    format!("Failed to read directory entry: {}", e),
                    Position::new(0, 0),
                )
            })?;

            let path = entry.path();
            if path.is_dir() {
                // Recurse into subdirectory
                self.walk_directory_recursive(&path, pattern, files)?;
            } else if self.matches_pattern(&path, pattern) {
                files.push(path);
            }
        }

        Ok(())
    }

    /// Match files in a single directory
    fn match_files_in_dir(&self, dir: &Path, pattern: &str) -> Result<Vec<PathBuf>, IncludeError> {
        let mut files = Vec::new();

        if !dir.is_dir() {
            return Ok(files);
        }

        let entries = fs::read_dir(dir).map_err(|e| {
            IncludeError::new(
                format!("Failed to read directory '{}': {}", dir.display(), e),
                Position::new(0, 0),
            )
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                IncludeError::new(
                    format!("Failed to read directory entry: {}", e),
                    Position::new(0, 0),
                )
            })?;

            let path = entry.path();
            if path.is_file() && self.matches_pattern(&path, pattern) {
                files.push(path);
            }
        }


        Ok(files)
    }

    /// Check if a file path matches a wildcard pattern
    fn matches_pattern(&self, file_path: &Path, pattern: &str) -> bool {
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");



        // Simple glob-style wildcard matching
        self.glob_match(file_name, pattern)
    }

    /// Simple glob pattern matching
    fn glob_match(&self, text: &str, pattern: &str) -> bool {
        self.glob_match_recursive(text, pattern)
    }

    /// Recursive glob pattern matching implementation
    fn glob_match_recursive(&self, text: &str, pattern: &str) -> bool {
        let text_chars: Vec<char> = text.chars().collect();
        let pattern_chars: Vec<char> = pattern.chars().collect();

        self.glob_match_impl(&text_chars, &pattern_chars, 0, 0)
    }

    /// Core glob matching implementation
    fn glob_match_impl(&self, text: &[char], pattern: &[char], text_idx: usize, pattern_idx: usize) -> bool {
        // If we've reached the end of both strings, it's a match
        if pattern_idx >= pattern.len() && text_idx >= text.len() {
            return true;
        }

        // If we've reached the end of pattern but not text, no match
        if pattern_idx >= pattern.len() {
            return false;
        }

        // If current pattern character is '*'
        if pattern[pattern_idx] == '*' {
            // Try matching zero characters (skip the *)
            if self.glob_match_impl(text, pattern, text_idx, pattern_idx + 1) {
                return true;
            }

            // Try matching one or more characters
            for i in text_idx..text.len() {
                if self.glob_match_impl(text, pattern, i + 1, pattern_idx + 1) {
                    return true;
                }
            }

            return false;
        }

        // If we've reached the end of text but not pattern, no match
        if text_idx >= text.len() {
            return false;
        }

        // If current characters match, continue
        if pattern[pattern_idx] == text[text_idx] {
            return self.glob_match_impl(text, pattern, text_idx + 1, pattern_idx + 1);
        }

        // No match
        false
    }

    /// Create an aliased declaration from an existing declaration
    fn create_aliased_declaration(&self, original: &Declaration, alias: &str) -> Declaration {
        match original {
            Declaration::Function(func_decl) => {
                let mut aliased_func = func_decl.clone();
                aliased_func.name = alias.to_string();
                Declaration::Function(aliased_func)
            }
            Declaration::Variable(var_stmt) => {
                if let Statement::VarDeclaration { scope, name: _, type_annotation, initializer, position } = var_stmt {
                    let aliased_var = Statement::VarDeclaration {
                        scope: scope.clone(),
                        name: alias.to_string(),
                        type_annotation: type_annotation.clone(),
                        initializer: initializer.clone(),
                        position: position.clone(),
                    };
                    Declaration::Variable(aliased_var)
                } else {
                    // If it's not a VarDeclaration, return as-is
                    original.clone()
                }
            }
            Declaration::Class(class_decl) => {
                let mut aliased_class = class_decl.clone();
                aliased_class.name = alias.to_string();
                Declaration::Class(aliased_class)
            }
            Declaration::Trait(trait_decl) => {
                let mut aliased_trait = trait_decl.clone();
                aliased_trait.name = alias.to_string();
                Declaration::Trait(aliased_trait)
            }
            Declaration::Enum(enum_decl) => {
                let mut aliased_enum = enum_decl.clone();
                aliased_enum.name = alias.to_string();
                Declaration::Enum(aliased_enum)
            }
            Declaration::Namespace(ns_decl) => {
                let mut aliased_ns = ns_decl.clone();
                aliased_ns.name = alias.to_string();
                Declaration::Namespace(aliased_ns)
            }
            // For other declaration types, return as-is (they might not support aliasing)
            _ => original.clone(),
        }
    }

    /// Parse a file into a Program
    fn parse_file(&self, file_path: &Path, include_decl: &IncludeDeclaration) -> Result<Program, IncludeError> {
        // Read file content
        let content = fs::read_to_string(file_path).map_err(|e| {
            IncludeError::with_file(
                format!("Failed to read file '{}': {}", file_path.display(), e),
                include_decl.position.clone(),
                file_path.to_string_lossy().to_string(),
            )
        })?;

        // Tokenize
        let mut lexer = Lexer::new(&content);
        let tokens = lexer.tokenize().map_err(|e| {
            IncludeError::with_file(
                format!("Lexer error in '{}': {}", file_path.display(), e),
                include_decl.position.clone(),
                file_path.to_string_lossy().to_string(),
            )
        })?;

        // Parse
        let mut parser = Parser::new(tokens);
        parser.parse_program().map_err(|e| {
            IncludeError::with_file(
                format!("Parser error in '{}': {}", file_path.display(), e),
                include_decl.position.clone(),
                file_path.to_string_lossy().to_string(),
            )
        })
    }

    /// Extract symbols from included program based on include type
    fn extract_symbols(&self, program: &Program, include_decl: &IncludeDeclaration, context: &mut IncludeContext) -> Result<Vec<Declaration>, IncludeError> {
        let mut result = Vec::new();

        if let Some(selective_imports) = &include_decl.selective_imports {
            // Selective import: only include specified symbols
            for import_spec in selective_imports {
                match import_spec {
                    ImportSpec::Symbol(name) => {
                        if let Some(declaration) = self.find_declaration_by_name(program, name) {
                            result.push(declaration.clone());
                            context.add_symbol(name.clone(), self.get_path_string(&include_decl.path));
                        } else {
                            return Err(IncludeError::with_file(
                                format!("Symbol '{}' not found in included file", name),
                                include_decl.position.clone(),
                                self.get_path_string(&include_decl.path),
                            ));
                        }
                    }
                    ImportSpec::Aliased { name, alias } => {
                        // Implement aliased imports - create a new declaration with the alias name
                        if let Some(declaration) = self.find_declaration_by_name(program, name) {
                            let aliased_declaration = self.create_aliased_declaration(declaration, alias);
                            result.push(aliased_declaration);
                            context.add_symbol(alias.clone(), self.get_path_string(&include_decl.path));
                        } else {
                            return Err(IncludeError::with_file(
                                format!("Symbol '{}' not found in included file", name),
                                include_decl.position.clone(),
                                self.get_path_string(&include_decl.path),
                            ));
                        }
                    }
                    ImportSpec::All => {
                        // Import all symbols
                        for declaration in &program.declarations {
                            if !matches!(declaration, Declaration::Include(_)) {
                                result.push(declaration.clone());
                                if let Some(symbol_name) = self.get_declaration_name(declaration) {
                                    context.add_symbol(symbol_name, self.get_path_string(&include_decl.path));
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Full import: include all public symbols
            for declaration in &program.declarations {
                // Skip nested includes to avoid infinite recursion
                if !matches!(declaration, Declaration::Include(_)) {
                    result.push(declaration.clone());

                    // Add symbol to context
                    if let Some(symbol_name) = self.get_declaration_name(declaration) {
                        context.add_symbol(symbol_name, self.get_path_string(&include_decl.path));
                    }
                }
            }
        }

        Ok(result)
    }

    /// Get path string from IncludePath
    fn get_path_string(&self, path: &IncludePath) -> String {
        match path {
            IncludePath::Simple(p) => p.clone(),
            IncludePath::Wildcard(p) => p.clone(),
            IncludePath::Search(p) => format!("<{}>", p),
        }
    }

    /// Find a declaration by name in the program
    fn find_declaration_by_name<'a>(&self, program: &'a Program, name: &str) -> Option<&'a Declaration> {
        program.declarations.iter().find(|decl| {
            self.get_declaration_name(decl).as_ref() == Some(&name.to_string())
        })
    }

    /// Get the name of a declaration
    fn get_declaration_name(&self, declaration: &Declaration) -> Option<String> {
        match declaration {
            Declaration::Function(func) => Some(func.name.clone()),
            Declaration::Namespace(ns) => Some(ns.name.clone()),
            Declaration::Enum(enum_decl) => Some(enum_decl.name.clone()),
            Declaration::Struct(struct_decl) => Some(struct_decl.name.clone()),
            Declaration::Class(class_decl) => Some(class_decl.name.clone()),
            Declaration::Trait(trait_decl) => Some(trait_decl.name.clone()),
            Declaration::Variable(stmt) => {
                // Extract variable name from variable declaration statement
                if let Statement::VarDeclaration { name, .. } = stmt {
                    Some(name.clone())
                } else {
                    None
                }
            },
            _ => None,
        }
    }
}
