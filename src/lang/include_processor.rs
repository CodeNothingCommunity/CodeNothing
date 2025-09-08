//! Include processor for CodeNothing language
//! 
//! Handles file inclusion, dependency resolution, and circular dependency detection

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::fs;
use crate::lang::ast::{Declaration, IncludeDeclaration, Program, Position, Statement};
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
}

impl IncludeProcessor {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
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
        // Resolve file path
        let file_path = self.resolve_path(&include_decl.path, &context.current_file)?;
        
        // Check for circular dependency
        if context.is_circular_dependency(&file_path) {
            return Err(IncludeError::with_file(
                format!("Circular dependency detected: {}", file_path.display()),
                include_decl.position.clone(),
                context.current_file.to_string_lossy().to_string(),
            ));
        }

        // Check if file is already processed
        if let Some(cached_program) = context.processed_files.get(&file_path).cloned() {
            return self.extract_symbols(&cached_program, include_decl, context);
        }

        // Read and parse the included file
        let included_program = self.parse_file(&file_path, include_decl)?;
        
        // Process includes in the included file
        context.push_file(file_path.clone());
        let processed_program = self.process_program_with_context(included_program, context)?;
        context.pop_file();
        
        // Cache the processed program
        context.processed_files.insert(file_path.clone(), processed_program.clone());
        
        // Extract symbols based on include type
        self.extract_symbols(&processed_program, include_decl, context)
    }

    /// Resolve include path relative to current file
    fn resolve_path(&self, include_path: &str, current_file: &Path) -> Result<PathBuf, IncludeError> {
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
            for import_name in selective_imports {
                if let Some(declaration) = self.find_declaration_by_name(program, import_name) {
                    result.push(declaration.clone());
                    context.add_symbol(import_name.clone(), include_decl.path.clone());
                } else {
                    return Err(IncludeError::with_file(
                        format!("Symbol '{}' not found in included file", import_name),
                        include_decl.position.clone(),
                        include_decl.path.clone(),
                    ));
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
                        context.add_symbol(symbol_name, include_decl.path.clone());
                    }
                }
            }
        }
        
        Ok(result)
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
