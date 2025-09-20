//! Parser for CodeNothing language
//! 
//! Implements recursive descent parser for the language grammar

use crate::lang::lexer::{Token, TokenType};
use crate::lang::ast::*;
use std::collections::VecDeque;

/// Parser errors
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub position: Position,
}

impl ParseError {
    pub fn new(message: String, position: Position) -> Self {
        Self { message, position }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error at {}:{}: {}", 
               self.position.line, self.position.column, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Recursive descent parser
pub struct Parser {
    tokens: VecDeque<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens: tokens.into(),
            current: 0,
        }
    }
    
    /// Get current token without consuming
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }
    
    /// Get token at offset without consuming
    fn peek_ahead(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.current + offset)
    }
    
    /// Consume and return current token
    fn advance(&mut self) -> Option<Token> {
        if self.current < self.tokens.len() {
            let token = self.tokens[self.current].clone();
            self.current += 1;
            Some(token)
        } else {
            None
        }
    }
    
    /// Check if current token matches expected type
    fn check(&self, token_type: &TokenType) -> bool {
        if let Some(token) = self.peek() {
            std::mem::discriminant(&token.token_type) == std::mem::discriminant(token_type)
        } else {
            false
        }
    }
    
    /// Consume token if it matches expected type
    fn match_token(&mut self, token_type: TokenType) -> bool {
        if self.check(&token_type) {
            self.advance();
            true
        } else {
            false
        }
    }
    
    /// Expect specific token type and consume it
    fn expect(&mut self, token_type: TokenType, message: &str) -> Result<Token, ParseError> {
        if let Some(token) = self.peek() {
            if std::mem::discriminant(&token.token_type) == std::mem::discriminant(&token_type) {
                Ok(self.advance().unwrap())
            } else {
                Err(ParseError::new(
                    format!("{}, found {:?}", message, token.token_type),
                    Position::new(token.line, token.column)
                ))
            }
        } else {
            Err(ParseError::new(
                format!("{}, found EOF", message),
                Position::new(0, 0)
            ))
        }
    }

    /// Expect '>' token, but handle '>>' as two '>' tokens for generic parsing
    fn expect_greater_or_right_shift(&mut self, message: &str) -> Result<(), ParseError> {
        if let Some(token) = self.peek().cloned() {
            match token.token_type {
                TokenType::Greater => {
                    self.advance();
                    Ok(())
                }
                TokenType::RightShift => {
                    // Convert >> to > by replacing the current token with >
                    // and inserting another > token after it
                    self.advance(); // consume the >>

                    // Insert a Greater token at the current position
                    let greater_token = Token {
                        token_type: TokenType::Greater,
                        lexeme: ">".to_string(),
                        line: token.line,
                        column: token.column + 1,
                    };
                    self.tokens.insert(self.current, greater_token);
                    Ok(())
                }
                _ => Err(ParseError::new(
                    format!("{}, found {:?}", message, token.token_type),
                    Position::new(token.line, token.column)
                ))
            }
        } else {
            Err(ParseError::new(
                format!("{}, found EOF", message),
                Position::new(0, 0)
            ))
        }
    }

    /// Skip newlines (now handled by lexer, this is a no-op)
    fn skip_newlines(&mut self) {
        // No-op: newlines are now skipped by the lexer like other whitespace
    }
    
    /// Parse complete program
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut program = Program::new();
        
        self.skip_newlines();
        
        while !self.check(&TokenType::Eof) {
            let declaration = self.parse_declaration()?;
            program.add_declaration(declaration);
            self.skip_newlines();
        }
        
        Ok(program)
    }
    
    /// Parse top-level declaration
    fn parse_declaration(&mut self) -> Result<Declaration, ParseError> {
        self.skip_newlines();
        
        if let Some(token) = self.peek() {
            match &token.token_type {
                TokenType::Fn => {
                    let func = self.parse_function()?;
                    Ok(Declaration::Function(func))
                }
                TokenType::Ns => {
                    let ns = self.parse_namespace()?;
                    Ok(Declaration::Namespace(ns))
                }
                TokenType::Using => {
                    let using = self.parse_using()?;
                    Ok(Declaration::Using(using))
                }
                TokenType::Include => {
                    let include_decl = self.parse_include()?;
                    Ok(Declaration::Include(include_decl))
                }
                TokenType::Package => {
                    let package_decl = self.parse_package()?;
                    Ok(Declaration::Package(package_decl))
                }
                TokenType::Import => {
                    let import_decl = self.parse_import()?;
                    Ok(Declaration::Import(import_decl))
                }
                TokenType::Export => {
                    let export_decl = self.parse_export()?;
                    Ok(Declaration::Export(export_decl))
                }
                TokenType::Memo => {
                    let memo = self.parse_memo()?;
                    Ok(Declaration::Memo(memo))
                }
                TokenType::Enum => {
                    let enum_decl = self.parse_enum()?;
                    Ok(Declaration::Enum(enum_decl))
                }
                TokenType::Struct => {
                    let struct_decl = self.parse_struct()?;
                    Ok(Declaration::Struct(struct_decl))
                }
                TokenType::Class => {
                    let class_decl = self.parse_class()?;
                    Ok(Declaration::Class(class_decl))
                }
                TokenType::Abstract => {
                    // Handle abstract class
                    let class_decl = self.parse_class()?;
                    Ok(Declaration::Class(class_decl))
                }
                TokenType::Trait => {
                    let trait_decl = self.parse_trait()?;
                    Ok(Declaration::Trait(trait_decl))
                }
                TokenType::Impl => {
                    let impl_decl = self.parse_impl()?;
                    Ok(Declaration::Impl(impl_decl))
                }
                TokenType::Lib => {
                    let lib_import = self.parse_lib_import_statement()?;
                    Ok(Declaration::LibImport(lib_import))
                }
                TokenType::Var | TokenType::Let | TokenType::Const => {
                    // Global variable declaration: var/let/const name : type = value;
                    let stmt = self.parse_variable_declaration()?;
                    Ok(Declaration::Variable(stmt))
                }
                TokenType::Identifier(_) => {
                    // Global variable declaration: name : type = value;
                    let stmt = self.parse_global_variable()?;
                    Ok(Declaration::Variable(stmt))
                }
                _ => {
                    Err(ParseError::new(
                        format!("Expected declaration, found {:?}", token.token_type),
                        Position::new(token.line, token.column)
                    ))
                }
            }
        } else {
            Err(ParseError::new(
                "Expected declaration, found EOF".to_string(),
                Position::new(0, 0)
            ))
        }
    }
    
    /// Parse function declaration
    fn parse_function(&mut self) -> Result<FunctionDeclaration, ParseError> {
        let fn_token = self.expect(TokenType::Fn, "Expected 'fn'")?;
        let position = Position::new(fn_token.line, fn_token.column);
        
        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected function name")?;
        let name = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new("Expected function name".to_string(), position));
        };
        
        self.expect(TokenType::LeftParen, "Expected '(' after function name")?;
        
        let mut parameters = Vec::new();
        
        if !self.check(&TokenType::RightParen) {
            loop {
                let param = self.parse_parameter()?;
                parameters.push(param);
                
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        
        self.expect(TokenType::RightParen, "Expected ')' after parameters")?;
        
        let return_type = if self.match_token(TokenType::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        
        let body = self.parse_statement()?;
        
        Ok(FunctionDeclaration {
            name,
            parameters,
            return_type,
            body,
            position,
        })
    }

    /// Parse abstract function declaration (no body, ends with semicolon)
    fn parse_abstract_function(&mut self) -> Result<FunctionDeclaration, ParseError> {
        let fn_token = self.expect(TokenType::Fn, "Expected 'fn'")?;
        let position = Position::new(fn_token.line, fn_token.column);

        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected function name")?;
        let name = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new("Expected function name".to_string(), position));
        };

        self.expect(TokenType::LeftParen, "Expected '(' after function name")?;

        let mut parameters = Vec::new();

        if !self.check(&TokenType::RightParen) {
            loop {
                let param = self.parse_parameter()?;
                parameters.push(param);

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        self.expect(TokenType::RightParen, "Expected ')' after parameters")?;

        let return_type = if self.match_token(TokenType::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        // Abstract methods end with semicolon, not a body
        self.expect(TokenType::Semicolon, "Expected ';' after abstract method signature")?;

        // Create an empty block as the body for abstract methods
        let empty_body = Statement::Block {
            statements: Vec::new(),
            position: position.clone(),
        };

        Ok(FunctionDeclaration {
            name,
            parameters,
            return_type,
            body: empty_body,
            position,
        })
    }

    /// Parse function parameter
    fn parse_parameter(&mut self) -> Result<Parameter, ParseError> {
        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected parameter name")?;
        let name = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new(
                "Expected parameter name".to_string(),
                Position::new(name_token.line, name_token.column)
            ));
        };

        self.expect(TokenType::Colon, "Expected ':' after parameter name")?;
        let type_annotation = self.parse_type_annotation()?;

        Ok(Parameter {
            name,
            type_annotation: Some(type_annotation),
            position: Position::new(name_token.line, name_token.column),
        })
    }
    
    /// Parse type annotation (alias for parse_type_annotation)
    fn parse_type(&mut self) -> Result<TypeAnnotation, ParseError> {
        self.parse_type_annotation()
    }

    /// Parse type annotation
    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, ParseError> {
        if let Some(token) = self.peek() {
            let position = Position::new(token.line, token.column);
            
            match &token.token_type {
                TokenType::Int => {
                    self.advance();
                    Ok(TypeAnnotation::Int)
                }
                TokenType::Long => {
                    self.advance();
                    Ok(TypeAnnotation::Long)
                }
                TokenType::Float32 => {
                    self.advance();
                    if self.match_token(TokenType::LeftParen) {
                        let size_token = self.expect(TokenType::Integer(0), "Expected number")?;
                        if let TokenType::Integer(size) = size_token.token_type {
                            self.expect(TokenType::RightParen, "Expected ')' after size")?;
                            Ok(TypeAnnotation::Float(Some(size as u32)))
                        } else {
                            Err(ParseError::new("Expected number".to_string(), position))
                        }
                    } else {
                        Ok(TypeAnnotation::Float(None))
                    }
                }
                TokenType::StringType => {
                    self.advance();
                    Ok(TypeAnnotation::String)
                }
                TokenType::Num => {
                    self.advance();
                    if self.match_token(TokenType::LeftParen) {
                        let size_token = self.expect(TokenType::Integer(0), "Expected number")?;
                        if let TokenType::Integer(size) = size_token.token_type {
                            self.expect(TokenType::RightParen, "Expected ')' after size")?;
                            Ok(TypeAnnotation::Num(Some(size as u32)))
                        } else {
                            Err(ParseError::new("Expected number".to_string(), position))
                        }
                    } else {
                        Ok(TypeAnnotation::Num(None))
                    }
                }
                TokenType::Bool => {
                    self.advance();
                    Ok(TypeAnnotation::Bool)
                }
                TokenType::Array => {
                    self.advance();
                    // Check for generic syntax: array<type>
                    if self.match_token(TokenType::Less) {
                        let element_type = self.parse_type_annotation()?;
                        self.expect_greater_or_right_shift("Expected '>' after array element type")?;
                        Ok(TypeAnnotation::Array(Box::new(element_type)))
                    } else {
                        // Default to auto if no generic type specified
                        Ok(TypeAnnotation::Array(Box::new(TypeAnnotation::Auto)))
                    }
                }
                TokenType::Obj => {
                    self.advance();
                    Ok(TypeAnnotation::Obj)
                }
                TokenType::Auto => {
                    self.advance();
                    Ok(TypeAnnotation::Auto)
                }
                TokenType::Null => {
                    self.advance();
                    Ok(TypeAnnotation::Null)
                }
                TokenType::Fn => {
                    // Function pointer type: fn(T1, T2) -> T3
                    self.advance(); // consume 'fn'

                    self.expect(TokenType::LeftParen, "Expected '(' after 'fn'")?;

                    let mut parameters = Vec::new();

                    if !self.check(&TokenType::RightParen) {
                        loop {
                            let param_type = self.parse_type_annotation()?;
                            parameters.push(param_type);

                            if !self.match_token(TokenType::Comma) {
                                break;
                            }
                        }
                    }

                    self.expect(TokenType::RightParen, "Expected ')' after function parameters")?;
                    self.expect(TokenType::Arrow, "Expected '->' after function parameters")?;

                    let return_type = Box::new(self.parse_type_annotation()?);

                    Ok(TypeAnnotation::FunctionPointer {
                        parameters,
                        return_type,
                    })
                }
                TokenType::Pipe => {
                    // Closure type: |T1, T2| -> T3
                    self.advance(); // consume first '|'

                    let mut parameters = Vec::new();

                    if !self.check(&TokenType::Pipe) {
                        loop {
                            let param_type = self.parse_type_annotation()?;
                            parameters.push(param_type);

                            if !self.match_token(TokenType::Comma) {
                                break;
                            }
                        }
                    }

                    self.expect(TokenType::Pipe, "Expected '|' after closure parameters")?;
                    self.expect(TokenType::Arrow, "Expected '->' after closure parameters")?;

                    let return_type = Box::new(self.parse_type_annotation()?);

                    Ok(TypeAnnotation::Function {
                        parameters,
                        return_type,
                    })
                }
                TokenType::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    Ok(TypeAnnotation::Custom(name))
                }
                TokenType::BitAnd => {
                    // Reference type: &T or &mut T
                    self.advance(); // consume &

                    let is_mutable = if self.match_token(TokenType::Mut) {
                        true
                    } else {
                        false
                    };

                    let target_type = Box::new(self.parse_type_annotation()?);

                    if is_mutable {
                        Ok(TypeAnnotation::MutableReference {
                            target_type,
                            lifetime: None, // NOTE: Lifetime parsing requires lifetime syntax definition
                        })
                    } else {
                        Ok(TypeAnnotation::Reference {
                            target_type,
                            lifetime: None, // NOTE: Lifetime parsing requires lifetime syntax definition
                        })
                    }
                }
                TokenType::Multiply => {
                    // Raw pointer type: *T or *mut T or *const T
                    self.advance(); // consume *

                    let is_mutable = if self.match_token(TokenType::Mut) {
                        true
                    } else if self.match_token(TokenType::ConstPtr) {
                        false
                    } else {
                        // Default to const pointer
                        false
                    };

                    let target_type = Box::new(self.parse_type_annotation()?);

                    Ok(TypeAnnotation::RawPointer {
                        target_type,
                        is_mutable,
                    })
                }
                _ => {
                    Err(ParseError::new(
                        format!("Expected type annotation, found {:?}", token.token_type),
                        position
                    ))
                }
            }
        } else {
            Err(ParseError::new(
                "Expected type annotation, found EOF".to_string(),
                Position::new(0, 0)
            ))
        }
    }
    
    /// Parse namespace declaration
    fn parse_namespace(&mut self) -> Result<NamespaceDeclaration, ParseError> {
        let ns_token = self.expect(TokenType::Ns, "Expected 'ns'")?;
        let position = Position::new(ns_token.line, ns_token.column);
        
        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected namespace name")?;
        let name = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new("Expected namespace name".to_string(), position));
        };
        
        self.expect(TokenType::LeftBrace, "Expected '{' after namespace name")?;
        
        let mut items = Vec::new();
        self.skip_newlines();
        
        while !self.check(&TokenType::RightBrace) && !self.check(&TokenType::Eof) {
            if self.check(&TokenType::Fn) {
                let func = self.parse_function()?;
                items.push(NamespaceItem::Function(func));
            } else if self.check(&TokenType::Ns) {
                // Parse nested namespace
                let nested_ns = self.parse_namespace()?;
                items.push(NamespaceItem::Namespace(nested_ns));
            } else if self.check(&TokenType::Var) {
                // Parse namespace variable
                let var_stmt = self.parse_variable_declaration()?;
                items.push(NamespaceItem::Variable(var_stmt));
            } else {
                // Unknown token in namespace
                break;
            }
            self.skip_newlines();
        }
        
        self.expect(TokenType::RightBrace, "Expected '}' after namespace body")?;
        
        Ok(NamespaceDeclaration {
            name,
            items,
            position,
        })
    }
    
    /// Parse using declaration
    fn parse_using(&mut self) -> Result<UsingDeclaration, ParseError> {
        let using_token = self.expect(TokenType::Using, "Expected 'using'")?;
        let position = Position::new(using_token.line, using_token.column);

        self.expect(TokenType::Ns, "Expected 'ns' after 'using'")?;

        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected namespace name")?;
        let namespace = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new("Expected namespace name".to_string(), position));
        };

        self.expect(TokenType::Semicolon, "Expected ';' after using declaration")?;

        Ok(UsingDeclaration {
            namespace,
            position,
        })
    }

    /// Parse include declaration
    fn parse_include(&mut self) -> Result<IncludeDeclaration, ParseError> {
        let include_token = self.expect(TokenType::Include, "Expected 'include'")?;
        let position = Position::new(include_token.line, include_token.column);

        // Parse include path - can be string literal or search path
        let path = if self.check(&TokenType::String(String::new())) {
            // String literal: "file.cn" or "pattern/*.cn"
            if let Some(path_token) = self.advance() {
                if let TokenType::String(p) = &path_token.token_type {
                    if p.contains('*') {
                        IncludePath::Wildcard(p.clone())
                    } else {
                        IncludePath::Simple(p.clone())
                    }
                } else {
                    return Err(ParseError::new("Expected file path string".to_string(), position));
                }
            } else {
                return Err(ParseError::new("Unexpected end of input".to_string(), position));
            }
        } else if self.check(&TokenType::Less) {
            // Search path: <std/io> or <math>
            self.advance(); // consume '<'
            let name_token = self.expect(TokenType::Identifier(String::new()), "Expected search path name")?;
            let name = if let TokenType::Identifier(n) = &name_token.token_type {
                n.clone()
            } else {
                return Err(ParseError::new("Expected search path name".to_string(), position));
            };

            // Handle nested paths like <std/io>
            let mut full_name = name;
            while self.check(&TokenType::Divide) {
                self.advance(); // consume '/'
                let part_token = self.expect(TokenType::Identifier(String::new()), "Expected path component")?;
                if let TokenType::Identifier(part) = &part_token.token_type {
                    full_name.push('/');
                    full_name.push_str(part);
                }
            }

            self.expect(TokenType::Greater, "Expected '>' after search path")?;
            IncludePath::Search(full_name)
        } else {
            return Err(ParseError::new("Expected file path string or search path".to_string(), position));
        };

        // Parse optional alias: "as alias"
        let alias = if self.match_token(TokenType::Identifier("as".to_string())) {
            let alias_token = self.expect(TokenType::Identifier(String::new()), "Expected alias name after 'as'")?;
            if let TokenType::Identifier(a) = &alias_token.token_type {
                Some(a.clone())
            } else {
                return Err(ParseError::new("Expected alias name".to_string(), position));
            }
        } else {
            None
        };

        // Parse optional selective imports: { item1, item2 as alias, * }
        let selective_imports = if self.match_token(TokenType::LeftBrace) {
            let mut imports = Vec::new();

            // Parse first import
            if !self.check(&TokenType::RightBrace) {
                imports.push(self.parse_import_spec()?);

                // Parse additional imports
                while self.match_token(TokenType::Comma) {
                    if !self.check(&TokenType::RightBrace) {
                        imports.push(self.parse_import_spec()?);
                    }
                }
            }

            self.expect(TokenType::RightBrace, "Expected '}' after selective imports")?;
            Some(imports)
        } else {
            None
        };

        self.expect(TokenType::Semicolon, "Expected ';' after include declaration")?;

        Ok(IncludeDeclaration {
            path,
            alias,
            selective_imports,
            search_paths: Vec::new(), // TODO: Add support for custom search paths
            position,
        })
    }

    /// Parse import specification for selective imports
    fn parse_import_spec(&mut self) -> Result<ImportSpec, ParseError> {
        if self.check(&TokenType::Multiply) {
            // Import all: * (using Multiply token as wildcard in import context)
            self.advance();
            Ok(ImportSpec::All)
        } else {
            // Import symbol or aliased symbol
            let name_token = self.expect(TokenType::Identifier(String::new()), "Expected import name")?;
            let name = if let TokenType::Identifier(n) = &name_token.token_type {
                n.clone()
            } else {
                return Err(ParseError::new("Expected import name".to_string(), Position::new(0, 0)));
            };

            // Check for alias: name as alias
            if self.match_token(TokenType::Identifier("as".to_string())) {
                let alias_token = self.expect(TokenType::Identifier(String::new()), "Expected alias name after 'as'")?;
                let alias = if let TokenType::Identifier(a) = &alias_token.token_type {
                    a.clone()
                } else {
                    return Err(ParseError::new("Expected alias name".to_string(), Position::new(0, 0)));
                };
                Ok(ImportSpec::Aliased { name, alias })
            } else {
                Ok(ImportSpec::Symbol(name))
            }
        }
    }

    /// Parse package declaration
    fn parse_package(&mut self) -> Result<PackageDeclaration, ParseError> {
        let package_token = self.expect(TokenType::Package, "Expected 'package'")?;
        let position = Position::new(package_token.line, package_token.column);

        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected package name")?;
        let name = if let TokenType::Identifier(n) = &name_token.token_type {
            n.clone()
        } else {
            return Err(ParseError::new("Expected package name".to_string(), position));
        };

        // Parse optional version
        let version = if self.match_token(TokenType::Identifier("version".to_string())) {
            let version_token = self.expect(TokenType::String(String::new()), "Expected version string")?;
            if let TokenType::String(v) = &version_token.token_type {
                Some(v.clone())
            } else {
                return Err(ParseError::new("Expected version string".to_string(), position));
            }
        } else {
            None
        };

        // Parse optional author
        let author = if self.match_token(TokenType::Identifier("author".to_string())) {
            let author_token = self.expect(TokenType::String(String::new()), "Expected author string")?;
            if let TokenType::String(a) = &author_token.token_type {
                Some(a.clone())
            } else {
                return Err(ParseError::new("Expected author string".to_string(), position));
            }
        } else {
            None
        };

        self.expect(TokenType::Semicolon, "Expected ';' after package declaration")?;

        Ok(PackageDeclaration {
            name,
            version,
            author,
            dependencies: Vec::new(), // TODO: Add dependency parsing
            position,
        })
    }

    /// Parse import declaration
    fn parse_import(&mut self) -> Result<ImportDeclaration, ParseError> {
        let import_token = self.expect(TokenType::Import, "Expected 'import'")?;
        let position = Position::new(import_token.line, import_token.column);

        // Parse package path
        let package_path = self.parse_package_path()?;

        // Parse optional alias
        let alias = if self.match_token(TokenType::Identifier("as".to_string())) {
            let alias_token = self.expect(TokenType::Identifier(String::new()), "Expected alias name after 'as'")?;
            if let TokenType::Identifier(a) = &alias_token.token_type {
                Some(a.clone())
            } else {
                return Err(ParseError::new("Expected alias name".to_string(), position));
            }
        } else {
            None
        };

        // Parse optional selective imports
        let selective_imports = if self.match_token(TokenType::LeftBrace) {
            let mut imports = Vec::new();

            if !self.check(&TokenType::RightBrace) {
                imports.push(self.parse_import_spec()?);

                while self.match_token(TokenType::Comma) {
                    if !self.check(&TokenType::RightBrace) {
                        imports.push(self.parse_import_spec()?);
                    }
                }
            }

            self.expect(TokenType::RightBrace, "Expected '}' after selective imports")?;
            Some(imports)
        } else {
            None
        };

        self.expect(TokenType::Semicolon, "Expected ';' after import declaration")?;

        Ok(ImportDeclaration {
            package_path,
            alias,
            selective_imports,
            position,
        })
    }

    /// Parse export declaration
    fn parse_export(&mut self) -> Result<ExportDeclaration, ParseError> {
        let export_token = self.expect(TokenType::Export, "Expected 'export'")?;
        let position = Position::new(export_token.line, export_token.column);

        // Parse the declaration to export (without recursing into parse_declaration)
        let declaration = if self.check(&TokenType::Fn) {
            Box::new(Declaration::Function(self.parse_function()?))
        } else if self.check(&TokenType::Const) {
            // Parse const declaration as a statement
            Box::new(Declaration::Variable(self.parse_variable_declaration()?))
        } else if self.check(&TokenType::Var) {
            // Parse var declaration as a statement
            Box::new(Declaration::Variable(self.parse_variable_declaration()?))
        } else if self.check(&TokenType::Struct) {
            Box::new(Declaration::Struct(self.parse_struct()?))
        } else if self.check(&TokenType::Class) {
            Box::new(Declaration::Class(self.parse_class()?))
        } else if self.check(&TokenType::Enum) {
            Box::new(Declaration::Enum(self.parse_enum()?))
        } else {
            return Err(ParseError::new("Expected exportable declaration after 'export'".to_string(), position));
        };

        Ok(ExportDeclaration {
            declaration,
            position,
        })
    }

    /// Parse package path (std::io, ./utils, ../common, math)
    fn parse_package_path(&mut self) -> Result<PackagePath, ParseError> {
        if self.check(&TokenType::String(String::new())) {
            // Relative path: "./utils" or "../common"
            if let Some(path_token) = self.advance() {
                if let TokenType::String(path) = &path_token.token_type {
                    let components: Vec<String> = path.split('/').map(|s| s.to_string()).collect();
                    return Ok(PackagePath::Relative(components));
                }
            }
        }

        // Absolute or local path: std::io or math
        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected package name")?;
        let mut components = Vec::new();

        if let TokenType::Identifier(name) = &name_token.token_type {
            components.push(name.clone());
        }

        // Check for :: separator (absolute path)
        if self.check(&TokenType::DoubleColon) {
            while self.match_token(TokenType::DoubleColon) {
                let part_token = self.expect(TokenType::Identifier(String::new()), "Expected package component")?;
                if let TokenType::Identifier(part) = &part_token.token_type {
                    components.push(part.clone());
                }
            }
            Ok(PackagePath::Absolute(components))
        } else {
            // Local path
            Ok(PackagePath::Local(components[0].clone()))
        }
    }

    /// Parse memo declaration
    fn parse_memo(&mut self) -> Result<MemoDeclaration, ParseError> {
        let memo_token = self.expect(TokenType::Memo, "Expected 'memo'")?;
        let position = Position::new(memo_token.line, memo_token.column);

        // Parse optional parameters: memo(max_size: 100, ttl: 3600)
        let params = if self.check(&TokenType::LeftParen) {
            self.advance(); // consume '('

            let mut max_size = None;
            let mut ttl = None;

            // Parse parameters
            while !self.check(&TokenType::RightParen) && !self.is_at_end() {
                if let Some(token) = self.peek() {
                    if let TokenType::Identifier(param_name) = &token.token_type {
                        let param_name = param_name.clone();
                        self.advance(); // consume parameter name

                        self.expect(TokenType::Colon, "Expected ':' after parameter name")?;

                        if let Some(value_token) = self.peek() {
                            if let TokenType::Integer(value) = value_token.token_type {
                                self.advance(); // consume value

                                match param_name.as_str() {
                                    "max_size" => max_size = Some(value),
                                    "ttl" => ttl = Some(value),
                                    _ => return Err(ParseError::new(
                                        format!("Unknown memo parameter: {}", param_name),
                                        position
                                    )),
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Expected integer value for memo parameter".to_string(),
                                    position
                                ));
                            }
                        }

                        // Optional comma
                        if self.check(&TokenType::Comma) {
                            self.advance();
                        }
                    } else {
                        return Err(ParseError::new(
                            "Expected parameter name".to_string(),
                            position
                        ));
                    }
                } else {
                    break;
                }
            }

            self.expect(TokenType::RightParen, "Expected ')' after memo parameters")?;

            Some(MemoParams { max_size, ttl })
        } else {
            None
        };

        // Parse memo content block
        self.expect(TokenType::LeftBrace, "Expected '{' after memo declaration")?;

        let mut declarations = Vec::new();

        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            // Parse memo content (functions or namespaces, but not nested memos)
            if let Some(token) = self.peek() {
                match &token.token_type {
                    TokenType::Fn => {
                        let func = self.parse_function()?;
                        declarations.push(Declaration::Function(func));
                    }
                    TokenType::Ns => {
                        let ns = self.parse_namespace()?;
                        declarations.push(Declaration::Namespace(ns));
                    }
                    TokenType::Using => {
                        let using = self.parse_using()?;
                        declarations.push(Declaration::Using(using));
                    }
                    TokenType::Memo => {
                        return Err(ParseError::new(
                            "Nested memo declarations are not allowed".to_string(),
                            Position::new(token.line, token.column)
                        ));
                    }
                    _ => {
                        // Skip unexpected tokens for now
                        self.advance();
                    }
                }
            } else {
                break;
            }
        }

        self.expect(TokenType::RightBrace, "Expected '}' after memo block")?;

        // Determine content type
        let content = if declarations.len() == 1 {
            match &declarations[0] {
                Declaration::Function(func) => MemoContent::Function(func.clone()),
                Declaration::Namespace(ns) => MemoContent::Namespace(ns.clone()),
                _ => MemoContent::Block(declarations),
            }
        } else {
            MemoContent::Block(declarations)
        };

        Ok(MemoDeclaration {
            params,
            content,
            position,
        })
    }
    
    /// Parse global variable declaration
    fn parse_global_variable(&mut self) -> Result<Statement, ParseError> {
        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected variable name")?;
        let name = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new(
                "Expected variable name".to_string(),
                Position::new(name_token.line, name_token.column)
            ));
        };
        
        self.expect(TokenType::Colon, "Expected ':' after variable name")?;
        let type_annotation = Some(self.parse_type_annotation()?);
        
        let initializer = if self.match_token(TokenType::Assign) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.expect(TokenType::Semicolon, "Expected ';' after variable declaration")?;
        
        Ok(Statement::VarDeclaration {
            scope: VarScope::Global,
            name,
            type_annotation,
            initializer,
            position: Position::new(name_token.line, name_token.column),
        })
    }
    
    /// Parse statement
    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        self.skip_newlines();
        
        if let Some(token) = self.peek() {
            match &token.token_type {
                TokenType::Var | TokenType::Let | TokenType::Const => {
                    self.parse_variable_declaration()
                }
                TokenType::Return => {
                    self.parse_return_statement()
                }
                TokenType::If => {
                    self.parse_if_statement()
                }
                TokenType::While => {
                    self.parse_while_statement()
                }
                TokenType::For => {
                    self.parse_for_statement()
                }
                TokenType::Do => {
                    self.parse_do_while_statement()
                }
                TokenType::Switch => {
                    self.parse_switch_statement()
                }
                TokenType::LeftBrace => {
                    self.parse_block_statement()
                }
                TokenType::Break => {
                    let token = self.advance().unwrap();
                    self.expect(TokenType::Semicolon, "Expected ';' after 'break'")?;
                    Ok(Statement::Break(Position::new(token.line, token.column)))
                }
                TokenType::Continue => {
                    let token = self.advance().unwrap();
                    self.expect(TokenType::Semicolon, "Expected ';' after 'continue'")?;
                    Ok(Statement::Continue(Position::new(token.line, token.column)))
                }
                TokenType::Try => {
                    self.parse_try_statement()
                }
                TokenType::Throw => {
                    self.parse_throw_statement()
                }
                TokenType::Lib => {
                    self.parse_lib_import_statement()
                }
                _ => {
                    // Expression statement
                    let expr = self.parse_expression()?;
                    self.expect(TokenType::Semicolon, "Expected ';' after expression")?;
                    Ok(Statement::Expression(expr))
                }
            }
        } else {
            Err(ParseError::new(
                "Expected statement, found EOF".to_string(),
                Position::new(0, 0)
            ))
        }
    }
    
    /// Parse variable declaration
    fn parse_variable_declaration(&mut self) -> Result<Statement, ParseError> {
        let scope_token = self.advance().unwrap();
        let scope = match scope_token.token_type {
            TokenType::Var => VarScope::Var,
            TokenType::Let => VarScope::Let,
            TokenType::Const => VarScope::Const,
            _ => unreachable!(),
        };

        // Check for optional 'mut' keyword after 'let'
        let _is_mutable = if matches!(scope, VarScope::Let) && self.match_token(TokenType::Mut) {
            true
        } else {
            false
        };

        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected variable name")?;
        let name = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new(
                "Expected variable name".to_string(),
                Position::new(scope_token.line, scope_token.column)
            ));
        };
        
        let (type_annotation, initializer) = if self.match_token(TokenType::ColonAssign) {
            // := syntax - type inference from initializer
            (None, Some(self.parse_expression()?))
        } else if self.match_token(TokenType::Colon) {
            // : type syntax - explicit type annotation
            let type_ann = Some(self.parse_type_annotation()?);
            let init = if self.match_token(TokenType::Assign) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            (type_ann, init)
        } else if self.match_token(TokenType::Assign) {
            // = expression syntax - no type annotation, just initializer
            (None, Some(self.parse_expression()?))
        } else {
            (None, None)
        };
        
        self.expect(TokenType::Semicolon, "Expected ';' after variable declaration")?;
        
        Ok(Statement::VarDeclaration {
            scope,
            name,
            type_annotation,
            initializer,
            position: Position::new(scope_token.line, scope_token.column),
        })
    }
    
    /// Parse return statement
    fn parse_return_statement(&mut self) -> Result<Statement, ParseError> {
        let return_token = self.expect(TokenType::Return, "Expected 'return'")?;
        let position = Position::new(return_token.line, return_token.column);

        let value = if self.check(&TokenType::Semicolon) || self.check(&TokenType::RightBrace) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        // Semicolon is optional if we're at the end of a block
        if self.check(&TokenType::Semicolon) {
            self.advance();
        } else if !self.check(&TokenType::RightBrace) && !self.check(&TokenType::Eof) {
            return Err(ParseError::new("Expected ';' after return statement".to_string(), position));
        }

        Ok(Statement::Return { value, position })
    }
    
    /// Parse if statement
    fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        let if_token = self.expect(TokenType::If, "Expected 'if'")?;
        let position = Position::new(if_token.line, if_token.column);
        
        self.expect(TokenType::LeftParen, "Expected '(' after 'if'")?;
        let condition = self.parse_expression()?;
        self.expect(TokenType::RightParen, "Expected ')' after if condition")?;
        
        let then_branch = Box::new(self.parse_statement()?);
        
        let else_branch = if self.match_token(TokenType::Else) {
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };
        
        Ok(Statement::If {
            condition,
            then_branch,
            else_branch,
            position,
        })
    }
    
    /// Parse while statement
    fn parse_while_statement(&mut self) -> Result<Statement, ParseError> {
        let while_token = self.expect(TokenType::While, "Expected 'while'")?;
        let position = Position::new(while_token.line, while_token.column);
        
        self.expect(TokenType::LeftParen, "Expected '(' after 'while'")?;
        let condition = self.parse_expression()?;
        self.expect(TokenType::RightParen, "Expected ')' after while condition")?;
        
        let body = Box::new(self.parse_statement()?);
        
        Ok(Statement::While {
            condition,
            body,
            position,
        })
    }
    
    /// Parse for statement
    fn parse_for_statement(&mut self) -> Result<Statement, ParseError> {
        let for_token = self.expect(TokenType::For, "Expected 'for'")?;
        let position = Position::new(for_token.line, for_token.column);

        self.expect(TokenType::LeftParen, "Expected '(' after 'for'")?;

        // Check if this is a for-in or range-based for loop
        if let Some(token) = self.peek() {
            if let TokenType::Identifier(_) = token.token_type {
                // Look ahead to see what comes after the identifier
                if let Some(next_token) = self.peek_ahead(1) {
                    if next_token.token_type == TokenType::In {
                        // This is a for-in loop: for (variable in iterable)
                        let var_token = self.advance().unwrap();
                        let variable = if let TokenType::Identifier(name) = var_token.token_type {
                            name
                        } else {
                            return Err(ParseError::new("Expected variable name".to_string(), position));
                        };

                        self.expect(TokenType::In, "Expected 'in' after variable")?;
                        let iterable = self.parse_expression()?;
                        self.expect(TokenType::RightParen, "Expected ')' after iterable")?;

                        let body = Box::new(self.parse_statement()?);

                        return Ok(Statement::ForIn {
                            variable,
                            iterable,
                            body,
                            position,
                        });
                    } else if next_token.token_type == TokenType::Colon {
                        // This is a range-based for loop: for (variable : range)
                        let var_token = self.advance().unwrap();
                        let variable = if let TokenType::Identifier(name) = var_token.token_type {
                            name
                        } else {
                            return Err(ParseError::new("Expected variable name".to_string(), position));
                        };

                        self.expect(TokenType::Colon, "Expected ':' after variable")?;
                        let range = self.parse_expression()?;
                        self.expect(TokenType::RightParen, "Expected ')' after range")?;

                        let body = Box::new(self.parse_statement()?);

                        return Ok(Statement::ForRange {
                            variable,
                            range,
                            body,
                            position,
                        });
                    }
                }
            }
        }

        // Traditional for loop: for (init; condition; increment)
        let initializer = if self.check(&TokenType::Semicolon) {
            None
        } else {
            Some(Box::new(self.parse_statement()?))
        };

        let condition = if self.check(&TokenType::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect(TokenType::Semicolon, "Expected ';' after for condition")?;

        let increment = if self.check(&TokenType::RightParen) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect(TokenType::RightParen, "Expected ')' after for clauses")?;

        let body = Box::new(self.parse_statement()?);

        Ok(Statement::For {
            initializer,
            condition,
            increment,
            body,
            position,
        })
    }

    /// Parse do-while statement
    fn parse_do_while_statement(&mut self) -> Result<Statement, ParseError> {
        let do_token = self.expect(TokenType::Do, "Expected 'do'")?;
        let position = Position::new(do_token.line, do_token.column);

        let body = Box::new(self.parse_statement()?);

        self.expect(TokenType::While, "Expected 'while' after do body")?;
        self.expect(TokenType::LeftParen, "Expected '(' after 'while'")?;
        let condition = self.parse_expression()?;
        self.expect(TokenType::RightParen, "Expected ')' after while condition")?;
        self.expect(TokenType::Semicolon, "Expected ';' after do-while statement")?;

        Ok(Statement::DoWhile {
            body,
            condition,
            position,
        })
    }
    
    /// Parse switch statement
    fn parse_switch_statement(&mut self) -> Result<Statement, ParseError> {
        let switch_token = self.advance().unwrap(); // consume 'switch'
        let position = Position::new(switch_token.line, switch_token.column);

        self.expect(TokenType::LeftParen, "Expected '(' after 'switch'")?;
        let expression = self.parse_expression()?;
        self.expect(TokenType::RightParen, "Expected ')' after switch expression")?;

        self.expect(TokenType::LeftBrace, "Expected '{' after switch expression")?;

        let mut cases = Vec::new();
        let mut default_case = None;

        self.skip_newlines();

        while !self.check(&TokenType::RightBrace) && !self.check(&TokenType::Eof) {
            if self.match_token(TokenType::Case) {
                let case_value = self.parse_expression()?;
                self.expect(TokenType::Colon, "Expected ':' after case value")?;

                // Parse case body (can be multiple statements)
                let mut case_statements = Vec::new();
                self.skip_newlines();

                while !self.check(&TokenType::Case) &&
                      !self.check(&TokenType::Default) &&
                      !self.check(&TokenType::RightBrace) &&
                      !self.check(&TokenType::Eof) {
                    case_statements.push(self.parse_statement()?);
                    self.skip_newlines();
                }

                let case_body = if case_statements.len() == 1 {
                    case_statements.into_iter().next().unwrap()
                } else {
                    Statement::Block {
                        statements: case_statements,
                        position: position.clone(),
                    }
                };

                cases.push(CaseClause {
                    value: case_value,
                    body: case_body,
                    position: position.clone(),
                });

            } else if self.match_token(TokenType::Default) {
                if default_case.is_some() {
                    return Err(ParseError::new(
                        "Multiple default cases in switch statement".to_string(),
                        position
                    ));
                }

                self.expect(TokenType::Colon, "Expected ':' after 'default'")?;

                // Parse default body (can be multiple statements)
                let mut default_statements = Vec::new();
                self.skip_newlines();

                while !self.check(&TokenType::Case) &&
                      !self.check(&TokenType::Default) &&
                      !self.check(&TokenType::RightBrace) &&
                      !self.check(&TokenType::Eof) {
                    default_statements.push(self.parse_statement()?);
                    self.skip_newlines();
                }

                let default_body = if default_statements.len() == 1 {
                    default_statements.into_iter().next().unwrap()
                } else {
                    Statement::Block {
                        statements: default_statements,
                        position: position.clone(),
                    }
                };

                default_case = Some(Box::new(default_body));

            } else {
                return Err(ParseError::new(
                    "Expected 'case' or 'default' in switch statement".to_string(),
                    position
                ));
            }
        }

        self.expect(TokenType::RightBrace, "Expected '}' after switch body")?;

        Ok(Statement::Switch {
            expression,
            cases,
            default_case,
            position,
        })
    }

    /// Parse block (alias for parse_block_statement)
    fn parse_block(&mut self) -> Result<Statement, ParseError> {
        self.parse_block_statement()
    }

    /// Parse block statement
    fn parse_block_statement(&mut self) -> Result<Statement, ParseError> {
        let brace_token = self.expect(TokenType::LeftBrace, "Expected '{'")?;
        let position = Position::new(brace_token.line, brace_token.column);
        
        let mut statements = Vec::new();
        self.skip_newlines();
        
        while !self.check(&TokenType::RightBrace) && !self.check(&TokenType::Eof) {
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }
        
        self.expect(TokenType::RightBrace, "Expected '}' after block")?;
        
        Ok(Statement::Block { statements, position })
    }
    
    /// Parse expression (entry point for expression parsing)
    pub fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_assignment()
    }
    
    /// Parse assignment expression
    fn parse_assignment(&mut self) -> Result<Expression, ParseError> {
        // Check if this looks like an arrow function before trying to parse it
        if self.looks_like_arrow_function() {
            if let Ok(arrow_fn) = self.try_parse_arrow_function() {
                return Ok(arrow_fn);
            }
        }

        let expr = self.parse_pipe()?;

        if let Some(token) = self.peek() {
            let operator = match &token.token_type {
                TokenType::Assign => {
                    self.advance();
                    BinaryOperator::Assign
                }
                TokenType::PlusEqual => {
                    self.advance();
                    BinaryOperator::AddAssign
                }
                TokenType::MinusEqual => {
                    self.advance();
                    BinaryOperator::SubtractAssign
                }
                TokenType::MultiplyEqual => {
                    self.advance();
                    BinaryOperator::MultiplyAssign
                }
                TokenType::DivideEqual => {
                    self.advance();
                    BinaryOperator::DivideAssign
                }
                TokenType::ModuloEqual => {
                    self.advance();
                    BinaryOperator::ModuloAssign
                }
                _ => return Ok(expr),
            };

            let position = expr.position().clone();
            let right = self.parse_assignment()?;
            return Ok(Expression::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                position,
            });
        }

        Ok(expr)
    }

    /// Parse pipe expression (left |> right)
    fn parse_pipe(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_ternary()?;

        while self.match_token(TokenType::PipeForward) {
            let position = expr.position().clone();
            let right = self.parse_ternary()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator: BinaryOperator::PipeForward,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }

    /// Parse ternary conditional expression (condition ? true_expr : false_expr)
    fn parse_ternary(&mut self) -> Result<Expression, ParseError> {
        let expr = self.parse_null_coalescing()?;

        if self.match_token(TokenType::Question) {
            let position = expr.position().clone();
            self.skip_newlines(); // Allow newlines after ?
            let true_expr = self.parse_expression()?;
            self.skip_newlines(); // Allow newlines before :
            self.expect(TokenType::Colon, "Expected ':' after true expression in ternary operator")?;
            self.skip_newlines(); // Allow newlines after :
            let false_expr = self.parse_ternary()?;

            return Ok(Expression::Ternary {
                condition: Box::new(expr),
                true_expr: Box::new(true_expr),
                false_expr: Box::new(false_expr),
                position,
            });
        }

        Ok(expr)
    }

    /// Parse null coalescing expression (left ?? right)
    fn parse_null_coalescing(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_logical_or()?;

        while self.match_token(TokenType::DoubleQuestion) {
            let position = expr.position().clone();
            let right = self.parse_logical_or()?;
            expr = Expression::NullCoalescing {
                left: Box::new(expr),
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }

    /// Parse logical OR expression
    fn parse_logical_or(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_bit_or()?;

        while self.match_token(TokenType::Or) {
            let position = expr.position().clone();
            let right = self.parse_bit_or()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator: BinaryOperator::Or,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }

    /// Parse bitwise OR expression
    fn parse_bit_or(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_bit_xor()?;

        while self.match_token(TokenType::BitOr) {
            let position = expr.position().clone();
            let right = self.parse_bit_xor()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator: BinaryOperator::BitOr,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }

    /// Parse bitwise XOR expression
    fn parse_bit_xor(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_bit_and()?;

        while self.match_token(TokenType::BitXor) {
            let position = expr.position().clone();
            let right = self.parse_bit_and()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator: BinaryOperator::BitXor,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }

    /// Parse bitwise AND expression
    fn parse_bit_and(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_logical_and()?;

        while self.match_token(TokenType::BitAnd) {
            let position = expr.position().clone();
            let right = self.parse_logical_and()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator: BinaryOperator::BitAnd,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }

    /// Parse logical AND expression
    fn parse_logical_and(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_equality()?;

        while self.match_token(TokenType::And) {
            let position = expr.position().clone();
            let right = self.parse_equality()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator: BinaryOperator::And,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }
    
    /// Parse equality expression
    fn parse_equality(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_comparison()?;

        while let Some(token) = self.peek() {
            let operator = match token.token_type {
                TokenType::Equal => {
                    self.advance();
                    BinaryOperator::Equal
                }
                TokenType::NotEqual => {
                    self.advance();
                    BinaryOperator::NotEqual
                }
                _ => break,
            };

            let position = expr.position().clone();
            let right = self.parse_comparison()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }
    
    /// Parse comparison expression
    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_shift()?;

        while let Some(token) = self.peek() {
            let operator = match token.token_type {
                TokenType::Greater => {
                    self.advance();
                    BinaryOperator::Greater
                }
                TokenType::GreaterEqual => {
                    self.advance();
                    BinaryOperator::GreaterEqual
                }
                TokenType::Less => {
                    self.advance();
                    BinaryOperator::Less
                }
                TokenType::LessEqual => {
                    self.advance();
                    BinaryOperator::LessEqual
                }
                _ => break,
            };

            let position = expr.position().clone();
            let right = self.parse_shift()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }

    /// Parse shift expression (<< >>)
    fn parse_shift(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_range()?;

        while let Some(token) = self.peek() {
            let operator = match token.token_type {
                TokenType::LeftShift => {
                    self.advance();
                    BinaryOperator::LeftShift
                }
                TokenType::RightShift => {
                    self.advance();
                    BinaryOperator::RightShift
                }
                _ => break,
            };

            let position = expr.position().clone();
            let right = self.parse_range()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }

    /// Parse range expression (start..end, start..=end, start...end)
    fn parse_range(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_term()?;

        // Check for range operators
        let range_op = if self.match_token(TokenType::DotDot) {
            Some(false) // exclusive range
        } else if self.match_token(TokenType::DotDotEqual) {
            Some(true) // inclusive range
        } else if self.match_token(TokenType::Range) {
            Some(false) // ... is treated as exclusive for backward compatibility
        } else {
            None
        };

        if let Some(inclusive) = range_op {
            let position = expr.position().clone();
            let end = self.parse_term()?;

            expr = Expression::Range {
                start: Box::new(expr),
                end: Box::new(end),
                inclusive,
                position,
            };
        }

        Ok(expr)
    }

    /// Parse term expression (+ -)
    fn parse_term(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_factor()?;

        while let Some(token) = self.peek() {
            let operator = match token.token_type {
                TokenType::Plus => {
                    self.advance();
                    BinaryOperator::Add
                }
                TokenType::Minus => {
                    self.advance();
                    BinaryOperator::Subtract
                }
                _ => break,
            };

            let position = expr.position().clone();
            let right = self.parse_factor()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }
    
    /// Parse factor expression (* / %)
    fn parse_factor(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_unary()?;

        while let Some(token) = self.peek() {
            let operator = match token.token_type {
                TokenType::Multiply => {
                    self.advance();
                    BinaryOperator::Multiply
                }
                TokenType::Divide => {
                    self.advance();
                    BinaryOperator::Divide
                }
                TokenType::Modulo => {
                    self.advance();
                    BinaryOperator::Modulo
                }
                _ => break,
            };

            let position = expr.position().clone();
            let right = self.parse_unary()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                position,
            };
        }

        Ok(expr)
    }
    
    /// Parse unary expression
    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        if let Some(token) = self.peek() {
            let position = Position::new(token.line, token.column);
            
            match token.token_type {
                TokenType::Minus => {
                    self.advance();
                    let operand = self.parse_unary()?;
                    Ok(Expression::Unary {
                        operator: UnaryOperator::Minus,
                        operand: Box::new(operand),
                        position,
                    })
                }
                TokenType::Not => {
                    self.advance();
                    let operand = self.parse_unary()?;
                    Ok(Expression::Unary {
                        operator: UnaryOperator::Not,
                        operand: Box::new(operand),
                        position,
                    })
                }
                TokenType::BitNot => {
                    self.advance();
                    let operand = self.parse_unary()?;
                    Ok(Expression::Unary {
                        operator: UnaryOperator::BitNot,
                        operand: Box::new(operand),
                        position,
                    })
                }
                TokenType::PlusPlus => {
                    self.advance();
                    let operand = self.parse_unary()?;
                    Ok(Expression::Unary {
                        operator: UnaryOperator::PreIncrement,
                        operand: Box::new(operand),
                        position,
                    })
                }
                TokenType::MinusMinus => {
                    self.advance();
                    let operand = self.parse_unary()?;
                    Ok(Expression::Unary {
                        operator: UnaryOperator::PreDecrement,
                        operand: Box::new(operand),
                        position,
                    })
                }
                TokenType::BitAnd => {
                    // Address-of operator: &expr or &mut expr
                    self.advance(); // consume &

                    let is_mutable = if self.match_token(TokenType::Mut) {
                        true
                    } else {
                        false
                    };

                    let operand = self.parse_unary()?;
                    Ok(Expression::AddressOf {
                        operand: Box::new(operand),
                        is_mutable,
                        position,
                    })
                }
                TokenType::Multiply => {
                    // Dereference operator: *expr
                    self.advance(); // consume *
                    let operand = self.parse_unary()?;
                    Ok(Expression::Dereference {
                        operand: Box::new(operand),
                        position,
                    })
                }
                _ => self.parse_postfix(),
            }
        } else {
            Err(ParseError::new(
                "Expected expression, found EOF".to_string(),
                Position::new(0, 0)
            ))
        }
    }

    /// Parse postfix expression (for postfix increment/decrement)
    fn parse_postfix(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_call()?;

        while let Some(token) = self.peek() {
            match token.token_type {
                TokenType::PlusPlus => {
                    let position = expr.position().clone();
                    self.advance();
                    expr = Expression::Unary {
                        operator: UnaryOperator::PostIncrement,
                        operand: Box::new(expr),
                        position,
                    };
                }
                TokenType::MinusMinus => {
                    let position = expr.position().clone();
                    self.advance();
                    expr = Expression::Unary {
                        operator: UnaryOperator::PostDecrement,
                        operand: Box::new(expr),
                        position,
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse call expression
    fn parse_call(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_token(TokenType::LeftParen) {
                // Function call
                let mut arguments = Vec::new();

                if !self.check(&TokenType::RightParen) {
                    loop {
                        arguments.push(self.parse_expression()?);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                }

                self.expect(TokenType::RightParen, "Expected ')' after arguments")?;

                let position = expr.position().clone();
                expr = Expression::Call {
                    function: Box::new(expr),
                    arguments,
                    position,
                };
            } else if self.match_token(TokenType::LeftBracket) {
                // Array access
                let index = self.parse_expression()?;
                self.expect(TokenType::RightBracket, "Expected ']' after array index")?;

                let position = expr.position().clone();
                expr = Expression::ArrayAccess {
                    array: Box::new(expr),
                    index: Box::new(index),
                    position,
                };
            } else if self.match_token(TokenType::Dot) {
                // Field access or method call
                let member_token = self.peek().ok_or_else(|| ParseError::new("Expected member name after '.'".to_string(),
                    Position::new(0, 0)))?;
                let member_name = if let TokenType::Identifier(name) = &member_token.token_type {
                    let name = name.clone();
                    self.advance();
                    name
                } else {
                    return Err(ParseError::new("Expected identifier after '.'".to_string(),
                        Position::new(member_token.line, member_token.column)));
                };

                let position = expr.position().clone();

                // Check if this is a method call (followed by '(')
                if self.check(&TokenType::LeftParen) {
                    self.advance(); // consume '('
                    let mut arguments = Vec::new();

                    if !self.check(&TokenType::RightParen) {
                        loop {
                            let arg = self.parse_expression()?;
                            arguments.push(arg);

                            if self.match_token(TokenType::Comma) {
                                continue;
                            } else if self.check(&TokenType::RightParen) {
                                break;
                            } else {
                                return Err(ParseError::new("Expected ',' or ')' in method call".to_string(), position));
                            }
                        }
                    }

                    self.expect(TokenType::RightParen, "Expected ')' after method arguments")?;
                    expr = Expression::MethodCall {
                        object: Box::new(expr),
                        method_name: member_name,
                        arguments,
                        position,
                    };
                } else {
                    // Field access
                    expr = Expression::FieldAccess {
                        object: Box::new(expr),
                        field: member_name,
                        position,
                    };
                }
            } else {
                break;
            }
        }

        Ok(expr)
    }
    
    /// Parse primary expression
    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        if let Some(token) = self.peek() {
            let position = Position::new(token.line, token.column);
            
            match &token.token_type {
                TokenType::Integer(value) => {
                    let value = *value;
                    self.advance();
                    Ok(Expression::Integer(value, position))
                }
                TokenType::Float(value) => {
                    let value = *value;
                    self.advance();
                    Ok(Expression::Float(value, position))
                }
                TokenType::String(value) => {
                    let value = value.clone();
                    self.advance();
                    Ok(Expression::String(value, position))
                }
                TokenType::InterpolatedString(parts) => {
                    let parts = parts.clone();
                    self.advance();
                    Ok(Expression::InterpolatedString(parts, position))
                }
                TokenType::True => {
                    self.advance();
                    Ok(Expression::Boolean(true, position))
                }
                TokenType::False => {
                    self.advance();
                    Ok(Expression::Boolean(false, position))
                }
                TokenType::Null => {
                    self.advance();
                    Ok(Expression::Null(position))
                }
                TokenType::Super => {
                    self.advance(); // consume 'super'

                    if self.match_token(TokenType::Dot) {
                        // super.field or super.method()
                        let member_token = self.expect(TokenType::Identifier(String::new()), "Expected member name after 'super.'")?;
                        let member_name = if let TokenType::Identifier(n) = member_token.token_type {
                            n
                        } else {
                            return Err(ParseError::new("Expected member name".to_string(), position));
                        };

                        if self.check(&TokenType::LeftParen) {
                            // super.method_name(args...)
                            self.advance(); // consume '('
                            let mut arguments = Vec::new();

                            if !self.check(&TokenType::RightParen) {
                                loop {
                                    let arg = self.parse_expression()?;
                                    arguments.push(arg);

                                    if self.match_token(TokenType::Comma) {
                                        continue;
                                    } else if self.check(&TokenType::RightParen) {
                                        break;
                                    } else {
                                        return Err(ParseError::new("Expected ',' or ')' in super method call".to_string(), position));
                                    }
                                }
                            }

                            self.expect(TokenType::RightParen, "Expected ')' after super method arguments")?;
                            Ok(Expression::SuperCall {
                                method_name: member_name,
                                arguments,
                                position,
                            })
                        } else {
                            // super.field_name
                            Ok(Expression::SuperFieldAccess {
                                field_name: member_name,
                                position,
                            })
                        }
                    } else {
                        return Err(ParseError::new("Expected '.' after 'super'".to_string(), position));
                    }
                }
                TokenType::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    
                    // Check for namespace access or static member access (support multiple levels)
                    if self.match_token(TokenType::DoubleColon) {
                        let mut namespace_path = vec![name.clone()];

                        // Parse the rest of the namespace path
                        loop {
                            let member_token = self.expect(TokenType::Identifier(String::new()), "Expected member name after '::'")?;
                            if let TokenType::Identifier(member) = member_token.token_type {
                                // Check if there's another :: (more nesting)
                                if self.match_token(TokenType::DoubleColon) {
                                    namespace_path.push(member);
                                } else {
                                    // This is the final member name
                                    // Always treat as namespace access first, let semantic analysis decide
                                    return Ok(Expression::NamespaceAccess {
                                        namespace_path,
                                        name: member,
                                        position,
                                    });
                                }
                            } else {
                                return Err(ParseError::new("Expected member name".to_string(), position));
                            }
                        }
                    } else if self.check(&TokenType::LeftBrace) {
                        // This might be a struct construction: StructName { field: value, ... }
                        self.advance(); // consume '{'
                        let mut fields = Vec::new();

                        // Handle empty struct
                        if self.match_token(TokenType::RightBrace) {
                            return Ok(Expression::Object(fields, position));
                        }

                        // Parse struct fields
                        loop {
                            // Parse field name (must be identifier)
                            let field_token = self.expect(TokenType::Identifier(String::new()), "Expected field name")?;
                            let field_name = if let TokenType::Identifier(n) = field_token.token_type {
                                n
                            } else {
                                return Err(ParseError::new("Expected field name".to_string(), position));
                            };

                            // Expect colon
                            self.expect(TokenType::Colon, "Expected ':' after field name")?;

                            // Parse value
                            let value = self.parse_expression()?;
                            fields.push((field_name, value));

                            if self.match_token(TokenType::Comma) {
                                continue;
                            } else if self.match_token(TokenType::RightBrace) {
                                break;
                            } else {
                                return Err(ParseError::new("Expected ',' or '}' in struct literal".to_string(), position));
                            }
                        }

                        Ok(Expression::Object(fields, position))
                    } else {
                        // Simple identifier
                        Ok(Expression::Identifier(name, position))
                    }
                }
                TokenType::LeftParen => {
                    self.advance();
                    let expr = self.parse_expression()?;
                    self.expect(TokenType::RightParen, "Expected ')' after expression")?;
                    Ok(expr)
                }
                TokenType::LeftBracket => {
                    self.advance();
                    let mut elements = Vec::new();

                    // Handle empty array
                    if self.match_token(TokenType::RightBracket) {
                        return Ok(Expression::Array(elements, position));
                    }

                    // Parse array elements
                    loop {
                        elements.push(self.parse_expression()?);

                        if self.match_token(TokenType::Comma) {
                            continue;
                        } else if self.match_token(TokenType::RightBracket) {
                            break;
                        } else {
                            return Err(ParseError::new("Expected ',' or ']' in array literal".to_string(), position));
                        }
                    }

                    Ok(Expression::Array(elements, position))
                }
                TokenType::LeftBrace => {
                    self.advance();
                    let mut fields = Vec::new();

                    // Handle empty object
                    if self.match_token(TokenType::RightBrace) {
                        return Ok(Expression::Object(fields, position));
                    }

                    // Parse object fields
                    loop {
                        // Parse key (must be identifier or string)
                        let key = match &self.peek().unwrap().token_type {
                            TokenType::Identifier(name) => {
                                let key = name.clone();
                                self.advance();
                                key
                            }
                            TokenType::String(s) => {
                                let key = s.clone();
                                self.advance();
                                key
                            }
                            _ => return Err(ParseError::new("Expected identifier or string as object key".to_string(), position)),
                        };

                        // Expect colon
                        self.expect(TokenType::Colon, "Expected ':' after object key")?;

                        // Parse value
                        let value = self.parse_expression()?;
                        fields.push((key, value));

                        if self.match_token(TokenType::Comma) {
                            continue;
                        } else if self.match_token(TokenType::RightBrace) {
                            break;
                        } else {
                            return Err(ParseError::new("Expected ',' or '}' in object literal".to_string(), position));
                        }
                    }

                    Ok(Expression::Object(fields, position))
                }
                TokenType::Pipe => {
                    // Lambda expression: |param1, param2| body
                    self.parse_lambda()
                }
                TokenType::Match => {
                    // Match expression: match expr { ... }
                    self.advance(); // consume 'match' token
                    self.parse_match()
                }
                TokenType::When => {
                    // When expression: when { condition => expr, ... }
                    self.advance(); // consume 'when' token
                    self.parse_when()
                }
                _ => {
                    Err(ParseError::new(
                        format!("Unexpected token {:?}", token.token_type),
                        position
                    ))
                }
            }
        } else {
            Err(ParseError::new(
                "Expected expression, found EOF".to_string(),
                Position::new(0, 0)
            ))
        }
    }

    /// Parse try statement
    fn parse_try_statement(&mut self) -> Result<Statement, ParseError> {
        let try_token = self.advance().unwrap(); // consume 'try'
        let position = Position::new(try_token.line, try_token.column);

        // Parse try block
        let try_block = Box::new(self.parse_statement()?);

        // Parse catch clauses
        let mut catch_clauses = Vec::new();
        while self.match_token(TokenType::Catch) {
            let catch_token = self.peek().unwrap();
            let catch_position = Position::new(catch_token.line, catch_token.column);

            // Parse optional exception variable and type
            let (exception_variable, exception_type) = if self.match_token(TokenType::LeftParen) {
                let var_name = if let Some(token) = self.peek() {
                    if let TokenType::Identifier(name) = &token.token_type {
                        let name = name.clone();
                        self.advance();
                        Some(name)
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Optional type annotation
                let exception_type = if self.match_token(TokenType::Colon) {
                    if let Some(token) = self.peek() {
                        if let TokenType::Identifier(type_name) = &token.token_type {
                            let type_name = type_name.clone();
                            self.advance();
                            Some(type_name)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                self.expect(TokenType::RightParen, "Expected ')' after catch clause")?;
                (var_name, exception_type)
            } else {
                (None, None)
            };

            // Parse catch body
            let body = Box::new(self.parse_statement()?);

            catch_clauses.push(CatchClause {
                exception_variable,
                exception_type,
                body,
                position: catch_position,
            });
        }

        // Parse optional finally block
        let finally_block = if self.match_token(TokenType::Finally) {
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        // Must have at least one catch or finally
        if catch_clauses.is_empty() && finally_block.is_none() {
            return Err(ParseError::new(
                "Try statement must have at least one catch or finally clause".to_string(),
                position
            ));
        }

        Ok(Statement::Try {
            try_block,
            catch_clauses,
            finally_block,
            position,
        })
    }

    /// Parse throw statement
    fn parse_throw_statement(&mut self) -> Result<Statement, ParseError> {
        let throw_token = self.advance().unwrap(); // consume 'throw'
        let position = Position::new(throw_token.line, throw_token.column);

        let value = self.parse_expression()?;
        self.expect(TokenType::Semicolon, "Expected ';' after throw statement")?;

        Ok(Statement::Throw {
            value,
            position,
        })
    }

    /// Parse library import statement
    fn parse_lib_import_statement(&mut self) -> Result<Statement, ParseError> {
        let lib_token = self.advance().unwrap(); // consume 'lib'
        let position = Position::new(lib_token.line, lib_token.column);

        // Check for system library import: lib <library_name>;
        if self.match_token(TokenType::Less) {
            // Parse system library name
            let library_name = if let Some(token) = self.advance() {
                match &token.token_type {
                    TokenType::Identifier(name) => name.clone(),
                    _ => return Err(ParseError::new(
                        "Expected library name after 'lib <'".to_string(),
                        Position::new(token.line, token.column),
                    )),
                }
            } else {
                return Err(ParseError::new(
                    "Expected library name after 'lib <'".to_string(),
                    position,
                ));
            };

            self.expect(TokenType::Greater, "Expected '>' after system library name")?;
            self.expect(TokenType::Semicolon, "Expected ';' after library import")?;

            return Ok(Statement::LibImport {
                library_name,
                import_type: LibImportType::System,
                position,
            });
        }

        // Check for local library import: lib "library_name";
        if let Some(token) = self.peek() {
            match &token.token_type {
                TokenType::String(library_name) => {
                    let library_name = library_name.clone();
                    self.advance(); // consume string
                    self.expect(TokenType::Semicolon, "Expected ';' after library import")?;

                    return Ok(Statement::LibImport {
                        library_name,
                        import_type: LibImportType::Local,
                        position,
                    });
                }
                _ => {
                    return Err(ParseError::new(
                        "Expected '<library_name>' or \"library_name\" after 'lib'".to_string(),
                        Position::new(token.line, token.column),
                    ));
                }
            }
        }

        Err(ParseError::new(
            "Expected library import syntax: 'lib <name>;' or 'lib \"name\";'".to_string(),
            position,
        ))
    }

    /// Check if we're at the end of tokens
    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }
    /// Parse enum declaration
    fn parse_enum(&mut self) -> Result<EnumDeclaration, ParseError> {
        let enum_token = self.expect(TokenType::Enum, "Expected 'enum'")?;
        let position = Position::new(enum_token.line, enum_token.column);

        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected enum name")?;
        let name = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new("Expected enum name".to_string(), position));
        };

        self.expect(TokenType::LeftBrace, "Expected '{' after enum name")?;

        let mut variants = Vec::new();

        while !self.check(&TokenType::RightBrace) && !self.check(&TokenType::Eof) {
            // Check if we've reached the end
            if self.check(&TokenType::RightBrace) || self.check(&TokenType::Eof) {
                break;
            }

            let variant_token = self.expect(TokenType::Identifier(String::new()), "Expected variant name")?;
            let variant_name = if let TokenType::Identifier(n) = variant_token.token_type {
                n
            } else {
                return Err(ParseError::new("Expected variant name".to_string(), position));
            };

            let variant_position = Position::new(variant_token.line, variant_token.column);

            // Check for associated types (tuple-like variants)
            let value = if self.check(&TokenType::LeftParen) {
                self.advance(); // consume '('

                // Parse type annotations for tuple-like enum variants
                let mut types = Vec::new();

                if !self.check(&TokenType::RightParen) {
                    loop {
                        let type_annotation = self.parse_type_annotation()?;
                        // Convert type annotation to a dummy expression for compatibility
                        // NOTE: This requires AST modification to support proper enum variant types
                        let dummy_expr = Expression::String(
                            format!("{:?}", type_annotation),
                            variant_position.clone(),
                        );
                        types.push(dummy_expr);

                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                }

                self.expect(TokenType::RightParen, "Expected ')' after variant types")?;

                // Currently uses the first type as the value
                // NOTE: Multiple types per variant requires AST enhancement
                types.into_iter().next()
            } else {
                None
            };

            variants.push(EnumVariant {
                name: variant_name,
                value,
                position: variant_position,
            });

            // Skip optional comma
            if self.check(&TokenType::Comma) {
                self.advance();
            }
        }

        self.expect(TokenType::RightBrace, "Expected '}' after enum variants")?;

        Ok(EnumDeclaration {
            name,
            variants,
            position,
        })
    }

    /// Parse struct declaration
    fn parse_struct(&mut self) -> Result<StructDeclaration, ParseError> {
        let struct_token = self.expect(TokenType::Struct, "Expected 'struct'")?;
        let position = Position::new(struct_token.line, struct_token.column);

        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected struct name")?;
        let name = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new("Expected struct name".to_string(), position));
        };

        self.expect(TokenType::LeftBrace, "Expected '{' after struct name")?;

        let mut fields = Vec::new();

        while !self.check(&TokenType::RightBrace) && !self.check(&TokenType::Eof) {
            // Check if we've reached the end
            if self.check(&TokenType::RightBrace) || self.check(&TokenType::Eof) {
                break;
            }

            let field_token = self.expect(TokenType::Identifier(String::new()), "Expected field name")?;
            let field_name = if let TokenType::Identifier(n) = field_token.token_type {
                n
            } else {
                return Err(ParseError::new("Expected field name".to_string(), position));
            };

            let field_position = Position::new(field_token.line, field_token.column);

            self.expect(TokenType::Colon, "Expected ':' after field name")?;
            let field_type = self.parse_type()?;

            fields.push(StructField {
                name: field_name,
                type_annotation: field_type,
                position: field_position,
            });

            // Skip optional comma
            if self.check(&TokenType::Comma) {
                self.advance();
            }
        }

        self.expect(TokenType::RightBrace, "Expected '}' after struct fields")?;

        Ok(StructDeclaration {
            name,
            fields,
            position,
        })
    }

    /// Parse trait declaration
    fn parse_trait(&mut self) -> Result<TraitDeclaration, ParseError> {
        let trait_token = self.expect(TokenType::Trait, "Expected 'trait'")?;
        let position = Position::new(trait_token.line, trait_token.column);

        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected trait name")?;
        let name = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new("Expected trait name".to_string(), position));
        };

        // Parse super traits (trait inheritance)
        let mut super_traits = Vec::new();
        if self.check(&TokenType::Colon) {
            self.advance(); // consume ':'

            // Parse first super trait
            let super_trait_token = self.expect(TokenType::Identifier(String::new()), "Expected super trait name")?;
            if let TokenType::Identifier(super_trait_name) = super_trait_token.token_type {
                super_traits.push(super_trait_name);
            }

            // Parse additional super traits separated by '+'
            while self.check(&TokenType::Plus) {
                self.advance(); // consume '+'
                let super_trait_token = self.expect(TokenType::Identifier(String::new()), "Expected super trait name")?;
                if let TokenType::Identifier(super_trait_name) = super_trait_token.token_type {
                    super_traits.push(super_trait_name);
                }
            }
        }

        self.expect(TokenType::LeftBrace, "Expected '{' after trait declaration")?;

        let mut methods = Vec::new();

        while !self.check(&TokenType::RightBrace) && !self.check(&TokenType::Eof) {
            // Check if we've reached the end
            if self.check(&TokenType::RightBrace) || self.check(&TokenType::Eof) {
                break;
            }

            self.expect(TokenType::Fn, "Expected 'fn' for trait method")?;

            let method_token = self.expect(TokenType::Identifier(String::new()), "Expected method name")?;
            let method_name = if let TokenType::Identifier(n) = method_token.token_type {
                n
            } else {
                return Err(ParseError::new("Expected method name".to_string(), position));
            };

            let method_position = Position::new(method_token.line, method_token.column);

            self.expect(TokenType::LeftParen, "Expected '(' after method name")?;

            let mut parameters = Vec::new();

            if !self.check(&TokenType::RightParen) {
                loop {
                    let param_token = self.expect(TokenType::Identifier(String::new()), "Expected parameter name")?;
                    let param_name = if let TokenType::Identifier(n) = param_token.token_type {
                        n
                    } else {
                        return Err(ParseError::new("Expected parameter name".to_string(), position));
                    };

                    self.expect(TokenType::Colon, "Expected ':' after parameter name")?;
                    let param_type = self.parse_type()?;

                    parameters.push(Parameter {
                        name: param_name,
                        type_annotation: Some(param_type),
                        position: Position::new(param_token.line, param_token.column),
                    });

                    if self.check(&TokenType::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }

            self.expect(TokenType::RightParen, "Expected ')' after parameters")?;

            let return_type = if self.check(&TokenType::Colon) {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };

            // Check for default implementation
            let default_body = if self.check(&TokenType::LeftBrace) {
                Some(self.parse_block()?)
            } else {
                self.expect(TokenType::Semicolon, "Expected ';' after trait method signature")?;
                None
            };

            let is_required = default_body.is_none();
            methods.push(TraitMethod {
                name: method_name,
                parameters,
                return_type,
                default_body,
                is_required,
                position: method_position,
            });
        }

        self.expect(TokenType::RightBrace, "Expected '}' after trait methods")?;

        // Separate methods with default implementations from those without
        let mut trait_methods = Vec::new();
        let mut default_methods = Vec::new();

        for method in methods {
            if method.default_body.is_some() {
                default_methods.push(method);
            } else {
                trait_methods.push(method);
            }
        }

        Ok(TraitDeclaration {
            name,
            super_traits,
            methods: trait_methods,
            default_methods,
            associated_types: Vec::new(),  // NOTE: Associated types require syntax definition
            position,
        })
    }

    /// Parse impl declaration
    fn parse_impl(&mut self) -> Result<ImplDeclaration, ParseError> {
        let impl_token = self.expect(TokenType::Impl, "Expected 'impl'")?;
        let position = Position::new(impl_token.line, impl_token.column);

        // Parse either "impl TraitName for TypeName" or "impl TypeName"
        let first_name_token = self.expect(TokenType::Identifier(String::new()), "Expected type or trait name")?;
        let first_name = if let TokenType::Identifier(n) = first_name_token.token_type {
            n
        } else {
            return Err(ParseError::new("Expected type or trait name".to_string(), position));
        };

        let (trait_name, type_name) = if self.check(&TokenType::For) {
            // impl TraitName for TypeName
            self.advance(); // consume 'for'
            let type_token = self.expect(TokenType::Identifier(String::new()), "Expected type name after 'for'")?;
            let type_name = if let TokenType::Identifier(n) = type_token.token_type {
                n
            } else {
                return Err(ParseError::new("Expected type name".to_string(), position));
            };
            (Some(first_name), type_name)
        } else {
            // impl TypeName
            (None, first_name)
        };

        self.expect(TokenType::LeftBrace, "Expected '{' after impl declaration")?;

        let mut methods = Vec::new();

        while !self.check(&TokenType::RightBrace) && !self.check(&TokenType::Eof) {
            // Check if we've reached the end
            if self.check(&TokenType::RightBrace) || self.check(&TokenType::Eof) {
                break;
            }

            let method = self.parse_function()?;
            methods.push(method);
        }

        self.expect(TokenType::RightBrace, "Expected '}' after impl methods")?;

        Ok(ImplDeclaration {
            trait_name,
            type_name,
            methods,
            position,
        })
    }

    /// Parse class declaration
    fn parse_class(&mut self) -> Result<ClassDeclaration, ParseError> {
        // Check for abstract keyword
        let is_abstract = if self.check(&TokenType::Abstract) {
            self.advance(); // consume 'abstract'
            true
        } else {
            false
        };

        let class_token = self.expect(TokenType::Class, "Expected 'class'")?;
        let position = Position::new(class_token.line, class_token.column);

        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected class name")?;
        let name = if let TokenType::Identifier(n) = name_token.token_type {
            n
        } else {
            return Err(ParseError::new("Expected class name".to_string(), position));
        };

        // Parse generic parameters if present
        let mut generic_parameters = Vec::new();
        if self.match_token(TokenType::Less) {
            loop {
                let param_token = self.expect(TokenType::Identifier(String::new()), "Expected generic parameter name")?;
                let param_name = if let TokenType::Identifier(name) = param_token.token_type {
                    name
                } else {
                    return Err(ParseError::new("Expected generic parameter name".to_string(), position));
                };

                // Parse type bounds (e.g., T: Drawable + Movable)
                let mut bounds = Vec::new();
                if self.check(&TokenType::Colon) {
                    self.advance(); // consume ':'

                    // Parse first bound
                    let bound_token = self.expect(TokenType::Identifier(String::new()), "Expected type bound")?;
                    if let TokenType::Identifier(bound_name) = bound_token.token_type {
                        bounds.push(bound_name);
                    }

                    // Parse additional bounds separated by '+'
                    while self.check(&TokenType::Plus) {
                        self.advance(); // consume '+'
                        let bound_token = self.expect(TokenType::Identifier(String::new()), "Expected type bound")?;
                        if let TokenType::Identifier(bound_name) = bound_token.token_type {
                            bounds.push(bound_name);
                        }
                    }
                }

                generic_parameters.push(crate::lang::ast::GenericParameter {
                    name: param_name,
                    bounds,
                    position: Position::new(param_token.line, param_token.column),
                });

                if self.match_token(TokenType::Comma) {
                    continue;
                } else if self.match_token(TokenType::Greater) {
                    break;
                } else {
                    return Err(ParseError::new("Expected ',' or '>' in generic parameter list".to_string(), position));
                }
            }
        }

        // Check for inheritance (extends keyword)
        let base_class = if self.match_token(TokenType::Extends) {
            let base_name_token = self.expect(TokenType::Identifier(String::new()), "Expected base class name after 'extends'")?;
            if let TokenType::Identifier(base_name) = base_name_token.token_type {
                Some(base_name)
            } else {
                return Err(ParseError::new("Expected base class name".to_string(), position));
            }
        } else {
            None
        };

        // Check for interface implementation (implements keyword)
        let mut implemented_traits = Vec::new();
        if self.match_token(TokenType::Implements) {
            loop {
                let trait_name_token = self.expect(TokenType::Identifier(String::new()), "Expected trait name after 'implements'")?;
                if let TokenType::Identifier(trait_name) = trait_name_token.token_type {
                    implemented_traits.push(trait_name);
                } else {
                    return Err(ParseError::new("Expected trait name".to_string(), position));
                }

                if self.match_token(TokenType::Comma) {
                    continue; // Parse next trait
                } else {
                    break; // No more traits
                }
            }
        }

        self.expect(TokenType::LeftBrace, "Expected '{' after class name or base class")?;

        let mut fields = Vec::new();
        let mut constructor = None;
        let mut destructor = None;
        let mut methods = Vec::new();
        let mut friend_classes = Vec::new();

        while !self.check(&TokenType::RightBrace) && !self.check(&TokenType::Eof) {
            // Check if we've reached the end
            if self.check(&TokenType::RightBrace) || self.check(&TokenType::Eof) {
                break;
            }

            // Parse class members
            if let Some(token) = self.peek() {
                match &token.token_type {
                    // Access modifiers
                    TokenType::Public | TokenType::Private | TokenType::Protected | TokenType::Package => {
                        let access_modifier = self.parse_access_modifier()?;

                        // Check what follows the access modifier
                        if let Some(next_token) = self.peek() {
                            match &next_token.token_type {
                                TokenType::Static => {
                                    self.advance(); // consume 'static'
                                    if let Some(after_static) = self.peek() {
                                        match &after_static.token_type {
                                            TokenType::Fn => {
                                                let line = after_static.line;
                                                let column = after_static.column;
                                                let (method, ctor) = self.parse_class_method_or_constructor(&name, access_modifier, true, false, false, false)?;
                                                if let Some(method) = method {
                                                    methods.push(method);
                                                }
                                                if let Some(_ctor) = ctor {
                                                    return Err(ParseError::new("Constructors cannot be static".to_string(), Position::new(line, column)));
                                                }
                                            }
                                            TokenType::Identifier(_) => {
                                                let field = self.parse_class_field(access_modifier, true)?;
                                                fields.push(field);
                                            }
                                            _ => {
                                                return Err(ParseError::new(
                                                    "Expected field or method after 'static'".to_string(),
                                                    Position::new(after_static.line, after_static.column)
                                                ));
                                            }
                                        }
                                    }
                                }
                                TokenType::Fn => {
                                    let (method, ctor) = self.parse_class_method_or_constructor(&name, access_modifier, false, false, false, false)?;
                                    if let Some(method) = method {
                                        methods.push(method);
                                    }
                                    if let Some(ctor) = ctor {
                                        if constructor.is_some() {
                                            return Err(ParseError::new("Multiple constructors not allowed".to_string(), Position::new(self.peek().unwrap().line, self.peek().unwrap().column)));
                                        }
                                        constructor = Some(ctor);
                                    }
                                }
                                TokenType::Virtual => {
                                    self.advance(); // consume 'virtual'
                                    if self.check(&TokenType::Fn) {
                                        let (method, ctor) = self.parse_class_method_or_constructor(&name, access_modifier, false, true, false, false)?;
                                        if let Some(method) = method {
                                            methods.push(method);
                                        }
                                        if let Some(_ctor) = ctor {
                                            return Err(ParseError::new("Constructors cannot be virtual".to_string(), Position::new(self.peek().unwrap().line, self.peek().unwrap().column)));
                                        }
                                    } else {
                                        return Err(ParseError::new(
                                            "Expected 'fn' after 'virtual'".to_string(),
                                            Position::new(self.peek().unwrap().line, self.peek().unwrap().column)
                                        ));
                                    }
                                }
                                TokenType::Override => {
                                    self.advance(); // consume 'override'
                                    if self.check(&TokenType::Fn) {
                                        let (method, ctor) = self.parse_class_method_or_constructor(&name, access_modifier, false, false, true, false)?;
                                        if let Some(method) = method {
                                            methods.push(method);
                                        }
                                        if let Some(_ctor) = ctor {
                                            return Err(ParseError::new("Constructors cannot be override".to_string(), Position::new(self.peek().unwrap().line, self.peek().unwrap().column)));
                                        }
                                    } else {
                                        return Err(ParseError::new(
                                            "Expected 'fn' after 'override'".to_string(),
                                            Position::new(self.peek().unwrap().line, self.peek().unwrap().column)
                                        ));
                                    }
                                }
                                TokenType::Abstract => {
                                    self.advance(); // consume 'abstract'
                                    if self.check(&TokenType::Fn) {
                                        let (method, ctor) = self.parse_class_method_or_constructor(&name, access_modifier, false, false, false, true)?;
                                        if let Some(method) = method {
                                            methods.push(method);
                                        }
                                        if let Some(_ctor) = ctor {
                                            return Err(ParseError::new("Constructors cannot be abstract".to_string(), Position::new(self.peek().unwrap().line, self.peek().unwrap().column)));
                                        }
                                    } else {
                                        return Err(ParseError::new(
                                            "Expected 'fn' after 'abstract'".to_string(),
                                            Position::new(self.peek().unwrap().line, self.peek().unwrap().column)
                                        ));
                                    }
                                }
                                TokenType::Identifier(id) => {
                                    // Check if this is a constructor (identifier matches class name)
                                    if id == &name {
                                        if constructor.is_some() {
                                            return Err(ParseError::new("Multiple constructors not allowed".to_string(), Position::new(next_token.line, next_token.column)));
                                        }
                                        constructor = Some(self.parse_constructor(&name, access_modifier)?);
                                    } else {
                                        let field = self.parse_class_field(access_modifier, false)?;
                                        fields.push(field);
                                    }
                                }
                                _ => {
                                    return Err(ParseError::new(
                                        "Expected field or method after access modifier".to_string(),
                                        Position::new(next_token.line, next_token.column)
                                    ));
                                }
                            }
                        }
                    }
                    // Static members without explicit access modifier
                    TokenType::Static => {
                        self.advance(); // consume 'static'
                        if let Some(next_token) = self.peek() {
                            match &next_token.token_type {
                                TokenType::Fn => {
                                    let method = self.parse_class_method(AccessModifier::Public, true, false, false, false)?;
                                    methods.push(method);
                                }
                                TokenType::Identifier(_) => {
                                    let field = self.parse_class_field(AccessModifier::Public, true)?;
                                    fields.push(field);
                                }
                                _ => {
                                    return Err(ParseError::new(
                                        "Expected field or method after 'static'".to_string(),
                                        Position::new(next_token.line, next_token.column)
                                    ));
                                }
                            }
                        }
                    }
                    // Constructor: class_name(params) { ... }
                    TokenType::Identifier(id) if id == &name => {
                        if constructor.is_some() {
                            return Err(ParseError::new("Multiple constructors not allowed".to_string(), position));
                        }
                        constructor = Some(self.parse_constructor(&name, AccessModifier::Public)?);
                    }
                    // Destructor: ~class_name() { ... }
                    TokenType::BitNot => {
                        if destructor.is_some() {
                            return Err(ParseError::new("Multiple destructors not allowed".to_string(), position));
                        }
                        destructor = Some(self.parse_destructor(&name)?);
                    }
                    // Virtual method: virtual fn method_name(params) { ... }
                    TokenType::Virtual => {
                        self.advance(); // consume 'virtual'
                        if self.check(&TokenType::Fn) {
                            let method = self.parse_class_method(AccessModifier::Public, false, true, false, false)?;
                            methods.push(method);
                        } else {
                            return Err(ParseError::new(
                                "Expected 'fn' after 'virtual'".to_string(),
                                Position::new(self.peek().unwrap().line, self.peek().unwrap().column)
                            ));
                        }
                    }
                    // Override method: override fn method_name(params) { ... }
                    TokenType::Override => {
                        self.advance(); // consume 'override'
                        if self.check(&TokenType::Fn) {
                            let method = self.parse_class_method(AccessModifier::Public, false, false, true, false)?;
                            methods.push(method);
                        } else {
                            return Err(ParseError::new(
                                "Expected 'fn' after 'override'".to_string(),
                                Position::new(self.peek().unwrap().line, self.peek().unwrap().column)
                            ));
                        }
                    }
                    // Abstract method: abstract fn method_name(params);
                    TokenType::Abstract => {
                        self.advance(); // consume 'abstract'
                        if self.check(&TokenType::Fn) {
                            let method = self.parse_class_method(AccessModifier::Public, false, false, false, true)?;
                            methods.push(method);
                        } else {
                            return Err(ParseError::new(
                                "Expected 'fn' after 'abstract'".to_string(),
                                Position::new(self.peek().unwrap().line, self.peek().unwrap().column)
                            ));
                        }
                    }
                    // Friend class: friend ClassName;
                    TokenType::Friend => {
                        self.advance(); // consume 'friend'

                        // Expect 'class' keyword (optional, for clarity)
                        if self.match_token(TokenType::Class) {
                            // friend class ClassName;
                        }

                        let friend_name_token = self.expect(TokenType::Identifier(String::new()), "Expected friend class name after 'friend'")?;
                        if let TokenType::Identifier(friend_name) = friend_name_token.token_type {
                            friend_classes.push(friend_name);
                        } else {
                            return Err(ParseError::new("Expected friend class name".to_string(), position));
                        }

                        self.expect(TokenType::Semicolon, "Expected ';' after friend class declaration")?;
                    }
                    // Method: fn method_name(params) { ... }
                    TokenType::Fn => {
                        let method = self.parse_class_method(AccessModifier::Public, false, false, false, false)?;
                        methods.push(method);
                    }
                    // Field: field_name: type;
                    TokenType::Identifier(_) => {
                        let field = self.parse_class_field(AccessModifier::Public, false)?;
                        fields.push(field);
                    }
                    _ => {
                        return Err(ParseError::new(
                            format!("Unexpected token in class body: {:?}", token.token_type),
                            Position::new(token.line, token.column)
                        ));
                    }
                }
            } else {
                break;
            }
        }

        self.expect(TokenType::RightBrace, "Expected '}' after class body")?;

        Ok(ClassDeclaration {
            name,
            generic_parameters,
            base_class,
            implemented_traits,
            is_abstract,
            friend_classes,
            fields,
            constructor,
            destructor,
            methods,
            nested_classes: Vec::new(),  // NOTE: Nested classes require syntax extension
            position,
        })
    }

    /// Parse access modifier
    fn parse_access_modifier(&mut self) -> Result<AccessModifier, ParseError> {
        if let Some(token) = self.peek() {
            let access_modifier = match &token.token_type {
                TokenType::Public => {
                    self.advance();
                    AccessModifier::Public
                }
                TokenType::Private => {
                    self.advance();
                    AccessModifier::Private
                }
                TokenType::Protected => {
                    self.advance();
                    AccessModifier::Protected
                }
                TokenType::Package => {
                    self.advance();
                    AccessModifier::Package
                }
                _ => {
                    return Err(ParseError::new(
                        "Expected access modifier (public, private, protected, package)".to_string(),
                        Position::new(token.line, token.column)
                    ));
                }
            };
            Ok(access_modifier)
        } else {
            Err(ParseError::new(
                "Expected access modifier, found EOF".to_string(),
                Position::new(0, 0)
            ))
        }
    }

    /// Parse class field with access modifier
    fn parse_class_field(&mut self, access_modifier: AccessModifier, is_static: bool) -> Result<ClassField, ParseError> {
        let field_token = self.expect(TokenType::Identifier(String::new()), "Expected field name")?;
        let field_name = if let TokenType::Identifier(n) = field_token.token_type {
            n
        } else {
            return Err(ParseError::new("Expected field name".to_string(),
                Position::new(field_token.line, field_token.column)));
        };

        let field_position = Position::new(field_token.line, field_token.column);

        self.expect(TokenType::Colon, "Expected ':' after field name")?;
        let field_type = self.parse_type()?;

        // Check for field initialization
        let initializer = if self.match_token(TokenType::Assign) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect(TokenType::Semicolon, "Expected ';' after field declaration")?;

        Ok(ClassField {
            name: field_name,
            type_annotation: field_type,
            access_modifier,
            is_static,
            initializer,
            position: field_position,
        })
    }

    /// Parse constructor
    fn parse_constructor(&mut self, class_name: &str, access_modifier: AccessModifier) -> Result<Constructor, ParseError> {
        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected constructor name")?;
        let position = Position::new(name_token.line, name_token.column);

        self.expect(TokenType::LeftParen, "Expected '(' after constructor name")?;

        let mut parameters = Vec::new();
        if !self.check(&TokenType::RightParen) {
            loop {
                let param_token = self.expect(TokenType::Identifier(String::new()), "Expected parameter name")?;
                let param_name = if let TokenType::Identifier(n) = param_token.token_type {
                    n
                } else {
                    return Err(ParseError::new("Expected parameter name".to_string(), position));
                };

                let param_position = Position::new(param_token.line, param_token.column);

                self.expect(TokenType::Colon, "Expected ':' after parameter name")?;
                let param_type = self.parse_type()?;

                parameters.push(Parameter {
                    name: param_name,
                    type_annotation: Some(param_type),
                    position: param_position,
                });

                if self.match_token(TokenType::Comma) {
                    continue;
                } else if self.check(&TokenType::RightParen) {
                    break;
                } else {
                    return Err(ParseError::new("Expected ',' or ')' in parameter list".to_string(), position));
                }
            }
        }

        self.expect(TokenType::RightParen, "Expected ')' after parameters")?;
        let body = self.parse_block()?;

        Ok(Constructor {
            parameters,
            body,
            access_modifier,
            position,
        })
    }

    /// Parse destructor
    fn parse_destructor(&mut self, class_name: &str) -> Result<Destructor, ParseError> {
        let tilde_token = self.expect(TokenType::BitNot, "Expected '~'")?;
        let position = Position::new(tilde_token.line, tilde_token.column);

        let name_token = self.expect(TokenType::Identifier(String::new()), "Expected destructor name")?;
        if let TokenType::Identifier(n) = name_token.token_type {
            if n != class_name {
                return Err(ParseError::new(
                    format!("Destructor name '{}' must match class name '{}'", n, class_name),
                    position
                ));
            }
        }

        self.expect(TokenType::LeftParen, "Expected '(' after destructor name")?;
        self.expect(TokenType::RightParen, "Expected ')' after destructor name")?;
        let body = self.parse_block()?;

        Ok(Destructor {
            body,
            position,
        })
    }

    /// Parse class method with access modifier and modifiers
    fn parse_class_method(&mut self, access_modifier: AccessModifier, is_static: bool, is_virtual: bool, is_override: bool, is_abstract: bool) -> Result<ClassMethod, ParseError> {
        let function = if is_abstract {
            self.parse_abstract_function()?
        } else {
            self.parse_function()?
        };
        let position = function.position.clone();

        Ok(ClassMethod {
            function,
            access_modifier,
            is_static,
            is_virtual,
            is_override,
            is_abstract,
            position,
        })
    }

    /// Parse class method with access modifier and modifiers, checking for constructor
    fn parse_class_method_or_constructor(&mut self, class_name: &str, access_modifier: AccessModifier, is_static: bool, is_virtual: bool, is_override: bool, is_abstract: bool) -> Result<(Option<ClassMethod>, Option<Constructor>), ParseError> {
        let function = if is_abstract {
            self.parse_abstract_function()?
        } else {
            self.parse_function()?
        };
        let position = function.position.clone();

        // Check if this is a constructor (method name matches class name)
        if function.name == class_name && !is_static {
            // This is a constructor
            let constructor = Constructor {
                parameters: function.parameters,
                body: function.body,
                access_modifier,
                position,
            };
            Ok((None, Some(constructor)))
        } else {
            // This is a regular method
            let method = ClassMethod {
                function,
                access_modifier,
                is_static,
                is_virtual,
                is_override,
                is_abstract,
                position,
            };
            Ok((Some(method), None))
        }
    }

    /// Parse lambda expression: |param1, param2| body
    fn parse_lambda(&mut self) -> Result<Expression, ParseError> {
        let pipe_token = self.expect(TokenType::Pipe, "Expected '|' for lambda")?;
        let position = Position::new(pipe_token.line, pipe_token.column);

        let mut parameters = Vec::new();

        // Parse parameters
        if !self.check(&TokenType::Pipe) {
            loop {
                let param_token = self.expect(TokenType::Identifier(String::new()), "Expected parameter name")?;
                let param_name = if let TokenType::Identifier(n) = param_token.token_type {
                    n
                } else {
                    return Err(ParseError::new("Expected parameter name".to_string(), position));
                };

                // Optional type annotation
                let param_type = if self.check(&TokenType::Colon) {
                    self.advance();
                    Some(self.parse_type()?)
                } else {
                    None // Infer type
                };

                parameters.push(Parameter {
                    name: param_name,
                    type_annotation: param_type,
                    position: Position::new(param_token.line, param_token.column),
                });

                if self.check(&TokenType::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        self.expect(TokenType::Pipe, "Expected '|' after lambda parameters")?;

        // Parse body expression
        let body = Box::new(self.parse_expression()?);

        Ok(Expression::Lambda {
            parameters,
            body,
            position,
        })
    }

    /// Parse simple expression for match (no object literals)
    fn parse_match_expression(&mut self) -> Result<Expression, ParseError> {
        // Parse a simple expression that doesn't include object literals
        // This avoids the conflict with { } in match expressions
        if let Some(token) = self.peek() {
            let position = Position::new(token.line, token.column);

            match &token.token_type {
                TokenType::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    Ok(Expression::Identifier(name, position))
                }
                TokenType::Integer(value) => {
                    let value = *value;
                    self.advance();
                    Ok(Expression::Integer(value, position))
                }
                TokenType::String(value) => {
                    let value = value.clone();
                    self.advance();
                    Ok(Expression::String(value, position))
                }
                TokenType::Boolean(value) => {
                    let value = *value;
                    self.advance();
                    Ok(Expression::Boolean(value, position))
                }
                TokenType::LeftParen => {
                    self.advance();
                    let expr = self.parse_expression()?;
                    self.expect(TokenType::RightParen, "Expected ')' after expression")?;
                    Ok(expr)
                }
                _ => {
                    Err(ParseError::new(
                        format!("Expected simple expression in match, found {:?}", token.token_type),
                        position
                    ))
                }
            }
        } else {
            Err(ParseError::new("Unexpected end of input in match expression".to_string(), Position::new(0, 0)))
        }
    }

    /// Parse match expression: match expr { pattern => body, ... }
    fn parse_match(&mut self) -> Result<Expression, ParseError> {
        // Note: 'match' token has already been consumed in parse_primary
        let position = if let Some(token) = self.peek() {
            Position::new(token.line, token.column)
        } else {
            Position::new(0, 0)
        };

        // Parse the expression to match against (simple expression only)
        let expression = Box::new(self.parse_match_expression()?);

        self.expect(TokenType::LeftBrace, "Expected '{' after match expression")?;

        let mut arms = Vec::new();

        self.skip_newlines(); // Skip any newlines after '{'

        while !self.check(&TokenType::RightBrace) && !self.check(&TokenType::Eof) {
            let pattern = self.parse_pattern()?;

            // Optional guard
            let guard = if self.check(&TokenType::If) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };

            self.expect(TokenType::FatArrow, "Expected '=>' after match pattern")?;

            let body = self.parse_expression()?;
            let arm_position = pattern.position().clone();

            arms.push(MatchArm {
                pattern,
                guard,
                body,
                position: arm_position,
            });

            if self.check(&TokenType::Comma) {
                self.advance();
            } else if !self.check(&TokenType::RightBrace) {
                return Err(ParseError::new("Expected ',' or '}' after match arm".to_string(), position));
            }

            self.skip_newlines(); // Skip newlines after each arm
        }

        self.expect(TokenType::RightBrace, "Expected '}' after match arms")?;

        Ok(Expression::Match {
            expression,
            arms,
            position,
        })
    }

    /// Parse when expression: when { condition => expr, condition => expr, else => expr }
    fn parse_when(&mut self) -> Result<Expression, ParseError> {
        let position = if let Some(token) = self.peek() {
            Position::new(token.line, token.column)
        } else {
            Position::new(0, 0)
        };

        self.expect(TokenType::LeftBrace, "Expected '{' after 'when'")?;

        let mut arms = Vec::new();
        let mut else_expr = None;

        while !self.check(&TokenType::RightBrace) && !self.check(&TokenType::Eof) {
            if self.match_token(TokenType::Else) {
                // else => expression
                self.expect(TokenType::FatArrow, "Expected '=>' after 'else'")?;
                else_expr = Some(Box::new(self.parse_expression()?));

                // Optional comma before closing brace
                self.match_token(TokenType::Comma);
                break;
            } else {
                // condition => expression
                let condition = self.parse_expression()?;
                self.expect(TokenType::FatArrow, "Expected '=>' after when condition")?;
                let body = self.parse_expression()?;

                arms.push(crate::lang::ast::WhenArm {
                    condition,
                    body,
                    position: position.clone(),
                });

                // Optional comma
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        self.expect(TokenType::RightBrace, "Expected '}' after when expression")?;

        Ok(Expression::When {
            arms,
            else_expr,
            position,
        })
    }

    /// Parse pattern for match expressions
    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        if let Some(token) = self.peek() {
            let position = Position::new(token.line, token.column);

            match &token.token_type {
                TokenType::Identifier(name) => {
                    let name = name.clone();
                    self.advance();

                    // Check for enum variant pattern: EnumName::Variant(pattern)
                    if self.check(&TokenType::DoubleColon) {
                        self.advance();
                        let variant_token = self.expect(TokenType::Identifier(String::new()), "Expected variant name")?;
                        let variant = if let TokenType::Identifier(v) = variant_token.token_type {
                            v
                        } else {
                            return Err(ParseError::new("Expected variant name".to_string(), position));
                        };

                        let value = if self.check(&TokenType::LeftParen) {
                            self.advance();
                            let pattern = Box::new(self.parse_pattern()?);
                            self.expect(TokenType::RightParen, "Expected ')' after variant pattern")?;
                            Some(pattern)
                        } else {
                            None
                        };

                        Ok(Pattern::EnumVariant {
                            enum_name: name,
                            variant,
                            value,
                            position,
                        })
                    } else if name == "_" {
                        // Wildcard pattern
                        Ok(Pattern::Wildcard(position))
                    } else {
                        // Identifier pattern
                        Ok(Pattern::Identifier(name, position))
                    }
                }
                TokenType::Integer(_) | TokenType::Float(_) | TokenType::String(_) |
                TokenType::True | TokenType::False | TokenType::Null => {
                    // Literal pattern
                    let expr = self.parse_primary()?;
                    Ok(Pattern::Literal(expr))
                }
                TokenType::LeftParen => {
                    // Tuple pattern: (a, b, c)
                    self.advance();
                    let mut patterns = Vec::new();

                    if !self.check(&TokenType::RightParen) {
                        loop {
                            patterns.push(self.parse_pattern()?);

                            if self.check(&TokenType::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }

                    self.expect(TokenType::RightParen, "Expected ')' after tuple pattern")?;
                    Ok(Pattern::Tuple(patterns, position))
                }
                TokenType::LeftBracket => {
                    // Array pattern: [a, b, c]
                    self.advance();
                    let mut patterns = Vec::new();

                    if !self.check(&TokenType::RightBracket) {
                        loop {
                            patterns.push(self.parse_pattern()?);

                            if self.check(&TokenType::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }

                    self.expect(TokenType::RightBracket, "Expected ']' after array pattern")?;
                    Ok(Pattern::Array(patterns, position))
                }
                _ => {
                    Err(ParseError::new(
                        format!("Unexpected token in pattern: {:?}", token.token_type),
                        position
                    ))
                }
            }
        } else {
            Err(ParseError::new(
                "Expected pattern, found EOF".to_string(),
                Position::new(0, 0)
            ))
        }
    }

    /// Try to parse arrow function: (x, y) => x + y or x => x + 1
    fn try_parse_arrow_function(&mut self) -> Result<Expression, ParseError> {
        let start_position = self.current;

        // Try to parse parameters
        let (parameters, position) = if self.check(&TokenType::LeftParen) {
            // Multi-parameter case: (x, y) => ...
            self.advance(); // consume '('
            let position = if let Some(token) = self.peek() {
                Position::new(token.line, token.column)
            } else {
                Position::new(0, 0)
            };

            let mut params = Vec::new();

            if !self.check(&TokenType::RightParen) {
                loop {
                    // Parse parameter name
                    if let Some(token) = self.peek() {
                        if let TokenType::Identifier(name) = &token.token_type {
                            let param_name = name.clone();
                            let token_line = token.line;
                            let token_column = token.column;
                            self.advance();

                            // Optional type annotation
                            let type_annotation = if self.match_token(TokenType::Colon) {
                                Some(self.parse_type()?)
                            } else {
                                None
                            };

                            params.push(Parameter {
                                name: param_name,
                                type_annotation: type_annotation,
                                position: Position::new(token_line, token_column),
                            });
                        } else {
                            // Reset position and return error
                            self.current = start_position;
                            return Err(ParseError::new("Expected parameter name in arrow function".to_string(), Position::new(0, 0)));
                        }
                    }

                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }

            self.expect(TokenType::RightParen, "Expected ')' after arrow function parameters")?;
            (params, position)
        } else if let Some(token) = self.peek() {
            // Single parameter case: x => ... or x: type => ...
            if let TokenType::Identifier(name) = &token.token_type {
                let param_name = name.clone();
                let position = Position::new(token.line, token.column);
                self.advance();

                // Check for type annotation: x: type => ...
                let type_annotation = if self.match_token(TokenType::Colon) {
                    Some(self.parse_type()?)
                } else {
                    None
                };

                let param = Parameter {
                    name: param_name,
                    type_annotation,
                    position: position.clone(),
                };

                (vec![param], position)
            } else {
                // Reset position and return error
                self.current = start_position;
                return Err(ParseError::new("Expected parameter in arrow function".to_string(), Position::new(0, 0)));
            }
        } else {
            // Reset position and return error
            self.current = start_position;
            return Err(ParseError::new("Expected arrow function parameters".to_string(), Position::new(0, 0)));
        };

        // Check for arrow operator
        if !self.match_token(TokenType::FatArrow) {
            // Reset position and return error
            self.current = start_position;
            return Err(ParseError::new("Expected '=>' in arrow function".to_string(), Position::new(0, 0)));
        }

        // Parse return type (optional)
        let return_type = None; // For now, we'll infer the return type

        // Parse function body
        let body = if self.check(&TokenType::LeftBrace) {
            // Block body: x => { return x + 1; }
            self.advance(); // consume '{'
            let mut statements = Vec::new();

            while !self.check(&TokenType::RightBrace) && !self.check(&TokenType::Eof) {
                // Parse statements within the arrow function block
                // We need to be careful to avoid infinite recursion
                statements.push(self.parse_arrow_function_statement()?);
            }

            self.expect(TokenType::RightBrace, "Expected '}' after arrow function block")?;
            crate::lang::ast::ArrowFunctionBody::Block(statements)
        } else {
            // Expression body: x => x + 1
            // 使用 parse_assignment 来支持嵌套箭头函数
            let expr = self.parse_assignment()?;
            crate::lang::ast::ArrowFunctionBody::Expression(Box::new(expr))
        };

        Ok(Expression::ArrowFunction {
            parameters,
            return_type,
            body,
            position,
        })
    }

    /// Parse a statement within an arrow function block
    /// This is similar to parse_statement but avoids infinite recursion with arrow functions
    fn parse_arrow_function_statement(&mut self) -> Result<Statement, ParseError> {
        match self.peek() {
            Some(token) => match &token.token_type {
                TokenType::Var => self.parse_variable_declaration(),
                TokenType::If => self.parse_if_statement(),
                TokenType::While => self.parse_while_statement(),
                TokenType::For => self.parse_for_statement(),
                TokenType::Do => self.parse_do_while_statement(),
                TokenType::Return => self.parse_return_statement(),
                TokenType::Break => {
                    // Simple break statement
                    let position = Position::new(self.peek().unwrap().line, self.peek().unwrap().column);
                    self.advance(); // consume 'break'
                    self.expect(TokenType::Semicolon, "Expected ';' after break")?;
                    Ok(Statement::Break(position))
                }
                TokenType::Continue => {
                    // Simple continue statement
                    let position = Position::new(self.peek().unwrap().line, self.peek().unwrap().column);
                    self.advance(); // consume 'continue'
                    self.expect(TokenType::Semicolon, "Expected ';' after continue")?;
                    Ok(Statement::Continue(position))
                }
                TokenType::LeftBrace => {
                    // Block statement
                    self.parse_block_statement()
                }
                _ => {
                    // Expression statement
                    let expr = self.parse_arrow_function_expression()?;
                    self.expect(TokenType::Semicolon, "Expected ';' after expression")?;
                    Ok(Statement::Expression(expr))
                }
            }
            None => Err(ParseError::new("Unexpected end of input".to_string(), Position::new(0, 0)))
        }
    }

    /// Parse an expression within an arrow function, avoiding arrow function recursion
    fn parse_arrow_function_expression(&mut self) -> Result<Expression, ParseError> {
        // Parse expressions but avoid arrow function recursion by starting from assignment level
        // This skips the arrow function parsing in parse_expression
        self.parse_assignment()
    }

    /// Check if the current position looks like the start of an arrow function
    fn looks_like_arrow_function(&self) -> bool {
        // Look ahead to see if we have patterns like:
        // identifier => ...
        // ( ... ) => ...

        if self.current >= self.tokens.len() {
            return false;
        }

        // Case 1: identifier => ... or identifier : type => ...
        if let Some(token) = self.tokens.get(self.current) {
            if matches!(token.token_type, TokenType::Identifier(_)) {
                // Look for => after identifier
                if let Some(next_token) = self.tokens.get(self.current + 1) {
                    if matches!(next_token.token_type, TokenType::FatArrow) {
                        return true;
                    }
                    // Look for : type => pattern
                    if matches!(next_token.token_type, TokenType::Colon) {
                        // Skip type tokens and look for =>
                        let mut pos = self.current + 2;
                        while pos < self.tokens.len() {
                            if let Some(t) = self.tokens.get(pos) {
                                if matches!(t.token_type, TokenType::FatArrow) {
                                    return true;
                                }
                                // Stop if we hit something that's not part of a type
                                if matches!(t.token_type, TokenType::Semicolon | TokenType::LeftBrace | TokenType::RightBrace) {
                                    break;
                                }
                            }
                            pos += 1;
                            // Limit lookahead
                            if pos - self.current > 10 {
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Case 2: ( ... ) => ...
        if let Some(token) = self.tokens.get(self.current) {
            if matches!(token.token_type, TokenType::LeftParen) {
                // Look for matching ) followed by =>
                let mut paren_count = 0;
                let mut pos = self.current;

                while pos < self.tokens.len() {
                    if let Some(t) = self.tokens.get(pos) {
                        match &t.token_type {
                            TokenType::LeftParen => paren_count += 1,
                            TokenType::RightParen => {
                                paren_count -= 1;
                                if paren_count == 0 {
                                    // Found matching ), check for =>
                                    if let Some(next_token) = self.tokens.get(pos + 1) {
                                        return matches!(next_token.token_type, TokenType::FatArrow);
                                    }
                                    return false;
                                }
                            }
                            _ => {}
                        }
                    }
                    pos += 1;

                    // Avoid infinite loops - limit lookahead
                    if pos - self.current > 20 {
                        break;
                    }
                }
            }
        }

        false
    }
}

impl Pattern {
    pub fn position(&self) -> &Position {
        match self {
            Pattern::Wildcard(pos) => pos,
            Pattern::Literal(expr) => expr.position(),
            Pattern::Identifier(_, pos) => pos,
            Pattern::EnumVariant { position, .. } => position,
            Pattern::Tuple(_, pos) => pos,
            Pattern::Array(_, pos) => pos,
        }
    }


}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::lexer::Lexer;
    
    #[test]
    fn test_parse_simple_function() {
        let source = "fn main() : int { return 42; }";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let program = parser.parse_program().unwrap();
        assert_eq!(program.declarations.len(), 1);
        
        if let Declaration::Function(func) = &program.declarations[0] {
            assert_eq!(func.name, "main");
            assert_eq!(func.parameters.len(), 0);
            assert!(matches!(func.return_type, Some(TypeAnnotation::Int)));
        } else {
            panic!("Expected function declaration");
        }
    }
    
    #[test]
    fn test_parse_variable_declaration() {
        let source = "var x : int = 42;";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);

        let stmt = parser.parse_statement().unwrap();
        if let Statement::VarDeclaration { scope, name, type_annotation, .. } = stmt {
            assert!(matches!(scope, VarScope::Var));
            assert_eq!(name, "x");
            assert!(matches!(type_annotation, Some(TypeAnnotation::Int)));
        } else {
            panic!("Expected variable declaration");
        }
    }

    #[test]
    fn test_parse_variable_declaration_with_type_inference() {
        let source = "var x := 42;";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);

        let stmt = parser.parse_statement().unwrap();
        if let Statement::VarDeclaration { scope, name, type_annotation, initializer, .. } = stmt {
            assert!(matches!(scope, VarScope::Var));
            assert_eq!(name, "x");
            assert!(type_annotation.is_none()); // No explicit type annotation
            assert!(initializer.is_some()); // Has initializer for type inference
        } else {
            panic!("Expected variable declaration");
        }
    }
}
